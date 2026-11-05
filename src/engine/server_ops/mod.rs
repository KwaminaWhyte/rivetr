//! Server operations: image inventory, Docker cleanup and OS package patching
//! for the local runtime (server id `localhost`) and remote SSH servers.
//!
//! Every operation runs a POSIX shell script on the target host, locally via
//! `sh -c` or remotely over SSH, so the same parsers cover both. Long running
//! work (cleanup, package updates, reboots) runs as a background task whose
//! status is persisted in `docker_cleanup_runs` / `server_patch_jobs`.

pub mod cleanup;
pub mod images;
pub mod patching;
pub mod scheduler;

use std::io::Write;
use std::str::FromStr;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{Config, RuntimeType};
use crate::crypto;
use crate::db::Server;
use crate::DbPool;

/// Server id used for the runtime Rivetr itself runs on.
pub const LOCALHOST_ID: &str = "localhost";

/// Where a server operation runs.
#[derive(Clone)]
pub enum HostTarget {
    /// The machine Rivetr runs on.
    Local { prefer_podman: bool },
    /// A remote server reached over SSH.
    Remote {
        host: String,
        port: u16,
        username: String,
        private_key: Option<String>,
        password: Option<String>,
    },
}

/// Exit code and captured output of a shell command.
#[derive(Debug, Clone)]
pub struct ExecOutput {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl ExecOutput {
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }

    /// stdout and stderr joined for logs.
    pub fn combined(&self) -> String {
        let out = self.stdout.trim_end();
        let err = self.stderr.trim_end();
        match (out.is_empty(), err.is_empty()) {
            (true, true) => String::new(),
            (false, true) => out.to_string(),
            (true, false) => err.to_string(),
            (false, false) => format!("{out}\n{err}"),
        }
    }
}

impl HostTarget {
    pub fn is_local(&self) -> bool {
        matches!(self, HostTarget::Local { .. })
    }

    /// Shell prelude that sets `$D` to the container CLI and `$SUDO` for
    /// commands that need root when the SSH user is not root.
    pub fn prelude(&self) -> &'static str {
        match self {
            HostTarget::Local {
                prefer_podman: true,
            } => PRELUDE_PODMAN.as_str(),
            _ => PRELUDE_DOCKER.as_str(),
        }
    }

    /// Run `script` (prefixed with [`Self::prelude`]) and wait at most `timeout`.
    pub async fn exec(&self, script: &str, timeout: Duration) -> Result<ExecOutput> {
        let full = format!("{}\n{}", self.prelude(), script);
        let fut = async {
            match self {
                HostTarget::Local { .. } => {
                    let output = tokio::process::Command::new("sh")
                        .arg("-c")
                        .arg(&full)
                        .kill_on_drop(true)
                        .output()
                        .await
                        .context("failed to spawn sh")?;
                    Ok::<_, anyhow::Error>(to_exec_output(output))
                }
                HostTarget::Remote {
                    host,
                    port,
                    username,
                    private_key,
                    password,
                } => {
                    run_ssh(
                        host,
                        *port,
                        username,
                        private_key.as_deref(),
                        password.as_deref(),
                        &full,
                    )
                    .await
                }
            }
        };
        match tokio::time::timeout(timeout, fut).await {
            Ok(res) => res,
            Err(_) => anyhow::bail!("command timed out after {}s", timeout.as_secs()),
        }
    }
}

// `$SUDO` and the `$D` container CLI wrapper come from the shared privilege
// helper (crate::utils::privileged): `$D` goes through `sudo -n` when the SSH
// user cannot reach the daemon directly.
static PRELUDE_DOCKER: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    use crate::utils::privileged::{container_cli_line, SUDO_LINE};
    format!(
        "{SUDO_LINE}\n{}",
        container_cli_line("D", &["docker", "podman"])
    )
});

static PRELUDE_PODMAN: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    use crate::utils::privileged::{container_cli_line, SUDO_LINE};
    format!(
        "{SUDO_LINE}\n{}",
        container_cli_line("D", &["podman", "docker"])
    )
});

fn to_exec_output(output: std::process::Output) -> ExecOutput {
    ExecOutput {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

async fn run_ssh(
    host: &str,
    port: u16,
    username: &str,
    private_key: Option<&str>,
    password: Option<&str>,
    script: &str,
) -> Result<ExecOutput> {
    use tokio::process::Command;

    let key_file = if let Some(key_content) = private_key {
        let mut tmpfile = tempfile::Builder::new()
            .prefix("rivetr-ssh-key-")
            .suffix(".pem")
            .tempfile()?;
        tmpfile.write_all(key_content.as_bytes())?;
        if !key_content.ends_with('\n') {
            tmpfile.write_all(b"\n")?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(tmpfile.path(), std::fs::Permissions::from_mode(0o600))?;
        }
        Some(tmpfile)
    } else {
        None
    };

    let use_sshpass = password.is_some() && key_file.is_none();
    let mut cmd = if use_sshpass {
        let mut c = Command::new("sshpass");
        // -e reads the password from SSHPASS so it never shows up in `ps`.
        c.env("SSHPASS", password.unwrap_or_default());
        c.arg("-e").arg("ssh");
        c
    } else {
        Command::new("ssh")
    };
    cmd.arg("-o")
        .arg("StrictHostKeyChecking=accept-new")
        .arg("-o")
        .arg(crate::utils::ssh::known_hosts_option())
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg("-o")
        .arg("ServerAliveInterval=30");
    if !use_sshpass {
        cmd.arg("-o").arg("BatchMode=yes");
    }
    cmd.arg("-p").arg(port.to_string());
    if let Some(ref kf) = key_file {
        cmd.arg("-i").arg(kf.path());
    }
    // The script is passed as one argument; the remote login shell runs it.
    cmd.arg(format!("{}@{}", username, host))
        .arg(format!("sh -c {}", shell_quote(script)));
    cmd.kill_on_drop(true);

    let output = cmd.output().await.context("failed to spawn ssh")?;
    drop(key_file);
    let out = to_exec_output(output);
    // ssh exits 255 on connection errors.
    if out.code == Some(255) {
        anyhow::bail!("SSH connection failed: {}", out.stderr.trim());
    }
    Ok(out)
}

/// Quote `s` as a single POSIX shell word.
pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

fn encryption_key(config: &Config) -> Option<[u8; 32]> {
    config
        .auth
        .encryption_key
        .as_ref()
        .map(|secret| crypto::derive_key(secret))
}

/// Build the [`HostTarget`] for a server id. `localhost` is the local runtime.
/// Returns `Ok(None)` when no server with that id exists.
pub async fn resolve_target(
    db: &DbPool,
    config: &Config,
    server_id: &str,
) -> Result<Option<(HostTarget, String)>> {
    if server_id == LOCALHOST_ID {
        let prefer_podman = matches!(config.runtime.runtime_type, RuntimeType::Podman);
        return Ok(Some((
            HostTarget::Local { prefer_podman },
            "localhost".to_string(),
        )));
    }
    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(server_id)
        .fetch_optional(db)
        .await?;
    let Some(server) = server else {
        return Ok(None);
    };
    let key = encryption_key(config);
    let private_key = server
        .ssh_private_key
        .as_deref()
        .and_then(|k| crypto::decrypt_if_encrypted(k, key.as_ref()).ok());
    let password = server
        .ssh_password
        .as_deref()
        .and_then(|p| crypto::decrypt_if_encrypted(p, key.as_ref()).ok());
    Ok(Some((
        HostTarget::Remote {
            host: server.host.clone(),
            port: server.port as u16,
            username: server.username.clone(),
            private_key,
            password,
        },
        server.name,
    )))
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow)]
struct MaintenanceSettingsRow {
    server_id: String,
    cleanup_enabled: i64,
    cleanup_cron: String,
    cleanup_trigger: String,
    cleanup_threshold_pct: i64,
    remove_unused_volumes: i64,
    remove_unused_networks: i64,
    remove_retained_images: i64,
    patch_check_enabled: i64,
    patch_check_cron: String,
    auto_security_updates: i64,
    maintenance_window: String,
    last_patch_check_at: Option<String>,
    patch_state: Option<String>,
}

/// Per-server maintenance settings (cleanup schedule and patching).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaintenanceSettings {
    pub server_id: String,
    pub cleanup_enabled: bool,
    pub cleanup_cron: String,
    /// `schedule` or `threshold`
    pub cleanup_trigger: String,
    pub cleanup_threshold_pct: i64,
    pub remove_unused_volumes: bool,
    pub remove_unused_networks: bool,
    pub remove_retained_images: bool,
    pub patch_check_enabled: bool,
    pub patch_check_cron: String,
    pub auto_security_updates: bool,
    /// `HH:MM`, UTC
    pub maintenance_window: String,
    pub last_patch_check_at: Option<String>,
    #[serde(skip)]
    pub patch_state: Option<String>,
}

impl MaintenanceSettings {
    pub fn defaults(server_id: &str) -> Self {
        Self {
            server_id: server_id.to_string(),
            cleanup_enabled: false,
            cleanup_cron: "0 3 * * *".to_string(),
            cleanup_trigger: "schedule".to_string(),
            cleanup_threshold_pct: 80,
            remove_unused_volumes: false,
            remove_unused_networks: false,
            remove_retained_images: false,
            patch_check_enabled: true,
            patch_check_cron: "0 0 * * *".to_string(),
            auto_security_updates: false,
            maintenance_window: "03:00".to_string(),
            last_patch_check_at: None,
            patch_state: None,
        }
    }

    /// Cached result of the last package check.
    pub fn patch_state(&self) -> Option<patching::PatchState> {
        self.patch_state
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
    }

    /// Validate user-editable fields.
    pub fn validate(&self) -> Result<(), String> {
        validate_cron(&self.cleanup_cron).map_err(|e| format!("Cleanup frequency: {e}"))?;
        validate_cron(&self.patch_check_cron).map_err(|e| format!("Check frequency: {e}"))?;
        if !matches!(self.cleanup_trigger.as_str(), "schedule" | "threshold") {
            return Err("Trigger must be 'schedule' or 'threshold'".to_string());
        }
        if !(1..=99).contains(&self.cleanup_threshold_pct) {
            return Err("Disk threshold must be between 1 and 99".to_string());
        }
        if !is_valid_hhmm(&self.maintenance_window) {
            return Err("Maintenance window must be a time like 03:00".to_string());
        }
        Ok(())
    }
}

impl From<MaintenanceSettingsRow> for MaintenanceSettings {
    fn from(r: MaintenanceSettingsRow) -> Self {
        Self {
            server_id: r.server_id,
            cleanup_enabled: r.cleanup_enabled != 0,
            cleanup_cron: r.cleanup_cron,
            cleanup_trigger: r.cleanup_trigger,
            cleanup_threshold_pct: r.cleanup_threshold_pct,
            remove_unused_volumes: r.remove_unused_volumes != 0,
            remove_unused_networks: r.remove_unused_networks != 0,
            remove_retained_images: r.remove_retained_images != 0,
            patch_check_enabled: r.patch_check_enabled != 0,
            patch_check_cron: r.patch_check_cron,
            auto_security_updates: r.auto_security_updates != 0,
            maintenance_window: r.maintenance_window,
            last_patch_check_at: r.last_patch_check_at,
            patch_state: r.patch_state,
        }
    }
}

/// Load settings for a server, falling back to defaults when none are stored.
pub async fn load_settings(db: &DbPool, server_id: &str) -> Result<MaintenanceSettings> {
    let row = sqlx::query_as::<_, MaintenanceSettingsRow>(
        "SELECT server_id, cleanup_enabled, cleanup_cron, cleanup_trigger, cleanup_threshold_pct, \
         remove_unused_volumes, remove_unused_networks, remove_retained_images, \
         patch_check_enabled, patch_check_cron, auto_security_updates, maintenance_window, \
         last_patch_check_at, patch_state \
         FROM server_maintenance_settings WHERE server_id = ?",
    )
    .bind(server_id)
    .fetch_optional(db)
    .await?;
    Ok(row
        .map(MaintenanceSettings::from)
        .unwrap_or_else(|| MaintenanceSettings::defaults(server_id)))
}

/// All stored settings rows (used by the scheduler).
pub async fn load_all_settings(db: &DbPool) -> Result<Vec<MaintenanceSettings>> {
    let rows = sqlx::query_as::<_, MaintenanceSettingsRow>(
        "SELECT server_id, cleanup_enabled, cleanup_cron, cleanup_trigger, cleanup_threshold_pct, \
         remove_unused_volumes, remove_unused_networks, remove_retained_images, \
         patch_check_enabled, patch_check_cron, auto_security_updates, maintenance_window, \
         last_patch_check_at, patch_state \
         FROM server_maintenance_settings",
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(MaintenanceSettings::from).collect())
}

/// Insert or update the user-editable settings (not the cached patch state).
pub async fn save_settings(db: &DbPool, s: &MaintenanceSettings) -> Result<()> {
    sqlx::query(
        "INSERT INTO server_maintenance_settings (server_id, cleanup_enabled, cleanup_cron, \
         cleanup_trigger, cleanup_threshold_pct, remove_unused_volumes, remove_unused_networks, \
         remove_retained_images, patch_check_enabled, patch_check_cron, auto_security_updates, \
         maintenance_window, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now')) \
         ON CONFLICT(server_id) DO UPDATE SET \
         cleanup_enabled = excluded.cleanup_enabled, cleanup_cron = excluded.cleanup_cron, \
         cleanup_trigger = excluded.cleanup_trigger, \
         cleanup_threshold_pct = excluded.cleanup_threshold_pct, \
         remove_unused_volumes = excluded.remove_unused_volumes, \
         remove_unused_networks = excluded.remove_unused_networks, \
         remove_retained_images = excluded.remove_retained_images, \
         patch_check_enabled = excluded.patch_check_enabled, \
         patch_check_cron = excluded.patch_check_cron, \
         auto_security_updates = excluded.auto_security_updates, \
         maintenance_window = excluded.maintenance_window, updated_at = datetime('now')",
    )
    .bind(&s.server_id)
    .bind(s.cleanup_enabled as i64)
    .bind(&s.cleanup_cron)
    .bind(&s.cleanup_trigger)
    .bind(s.cleanup_threshold_pct)
    .bind(s.remove_unused_volumes as i64)
    .bind(s.remove_unused_networks as i64)
    .bind(s.remove_retained_images as i64)
    .bind(s.patch_check_enabled as i64)
    .bind(&s.patch_check_cron)
    .bind(s.auto_security_updates as i64)
    .bind(&s.maintenance_window)
    .execute(db)
    .await?;
    Ok(())
}

/// Store the result of a package check, creating the settings row if needed.
pub async fn save_patch_state(
    db: &DbPool,
    server_id: &str,
    state: &patching::PatchState,
) -> Result<()> {
    let json = serde_json::to_string(state)?;
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO server_maintenance_settings (server_id, last_patch_check_at, patch_state) \
         VALUES (?, ?, ?) \
         ON CONFLICT(server_id) DO UPDATE SET last_patch_check_at = excluded.last_patch_check_at, \
         patch_state = excluded.patch_state",
    )
    .bind(server_id)
    .bind(&now)
    .bind(&json)
    .execute(db)
    .await?;
    Ok(())
}

/// Mark jobs left `running` by a previous process as failed.
pub async fn fail_interrupted_jobs(db: &DbPool) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE docker_cleanup_runs SET status = 'failed', finished_at = ?, \
         log = log || char(10) || 'Interrupted: Rivetr restarted while this ran.' \
         WHERE status = 'running'",
    )
    .bind(&now)
    .execute(db)
    .await?;
    sqlx::query(
        "UPDATE server_patch_jobs SET status = 'failed', finished_at = ?, \
         log = log || char(10) || 'Interrupted: Rivetr restarted while this ran.' \
         WHERE status = 'running' AND kind != 'reboot'",
    )
    .bind(&now)
    .execute(db)
    .await?;
    // A reboot of localhost restarts Rivetr itself, so a running reboot job is a success.
    sqlx::query(
        "UPDATE server_patch_jobs SET status = 'success', finished_at = ? \
         WHERE status = 'running' AND kind = 'reboot'",
    )
    .bind(&now)
    .execute(db)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Turn a standard 5-field cron expression into the 6-field form the `cron`
/// crate expects (seconds first). 6 and 7 field expressions pass through.
pub fn normalize_cron(expr: &str) -> String {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() == 5 {
        format!("0 {}", parts.join(" "))
    } else {
        parts.join(" ")
    }
}

/// Validate a cron expression (5 fields, or 6/7 with seconds).
pub fn validate_cron(expr: &str) -> Result<cron::Schedule, String> {
    let fields = expr.split_whitespace().count();
    if !(5..=7).contains(&fields) {
        return Err(format!(
            "expected 5 fields (minute hour day month weekday), got {fields}"
        ));
    }
    cron::Schedule::from_str(&normalize_cron(expr)).map_err(|e| e.to_string())
}

/// `HH:MM` in 24 hour time.
pub fn is_valid_hhmm(s: &str) -> bool {
    let Some((h, m)) = s.split_once(':') else {
        return false;
    };
    h.len() == 2
        && m.len() == 2
        && h.parse::<u8>().is_ok_and(|h| h < 24)
        && m.parse::<u8>().is_ok_and(|m| m < 60)
}

/// Parse sizes printed by docker / podman ("1.2GB", "512MB", "12.3kB", "800B",
/// "1.5 GiB"). Decimal units are powers of 1000, binary (`iB`) of 1024.
pub fn parse_human_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let split = s
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(s.len());
    let (num, unit) = s.split_at(split);
    let value: f64 = num.parse().ok()?;
    let unit = unit.trim().to_ascii_lowercase();
    let mult: f64 = match unit.as_str() {
        "" | "b" => 1.0,
        "kb" | "k" => 1e3,
        "mb" | "m" => 1e6,
        "gb" | "g" => 1e9,
        "tb" | "t" => 1e12,
        "kib" => 1024.0,
        "mib" => 1024.0 * 1024.0,
        "gib" => 1024.0 * 1024.0 * 1024.0,
        "tib" => 1024.0_f64.powi(4),
        _ => return None,
    };
    Some((value * mult).round() as u64)
}

/// Append a timestamped line to a log buffer.
pub(crate) fn log_line(log: &mut String, line: impl AsRef<str>) {
    if !log.is_empty() {
        log.push('\n');
    }
    log.push_str(line.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_validation_accepts_five_and_six_fields() {
        assert!(validate_cron("0 3 * * *").is_ok());
        assert!(validate_cron("*/15 * * * *").is_ok());
        assert!(validate_cron("0 0 3 * * *").is_ok());
        assert!(validate_cron("0 0 * * Sun").is_ok());
    }

    #[test]
    fn cron_validation_rejects_bad_input() {
        assert!(validate_cron("").is_err());
        assert!(validate_cron("* * *").is_err());
        assert!(validate_cron("61 * * * *").is_err());
        assert!(validate_cron("0 25 * * *").is_err());
        assert!(validate_cron("not a cron at all x").is_err());
    }

    #[test]
    fn normalize_cron_prefixes_seconds() {
        assert_eq!(normalize_cron("0 3 * * *"), "0 0 3 * * *");
        assert_eq!(normalize_cron("0 0 3 * * *"), "0 0 3 * * *");
    }

    #[test]
    fn hhmm_validation() {
        assert!(is_valid_hhmm("03:00"));
        assert!(is_valid_hhmm("23:59"));
        assert!(!is_valid_hhmm("24:00"));
        assert!(!is_valid_hhmm("3:00"));
        assert!(!is_valid_hhmm("03:60"));
        assert!(!is_valid_hhmm("0300"));
    }

    #[test]
    fn human_sizes() {
        assert_eq!(parse_human_size("1.2GB"), Some(1_200_000_000));
        assert_eq!(parse_human_size("512MB"), Some(512_000_000));
        assert_eq!(parse_human_size("12.3kB"), Some(12_300));
        assert_eq!(parse_human_size("800B"), Some(800));
        assert_eq!(parse_human_size("1 GiB"), Some(1_073_741_824));
        assert_eq!(parse_human_size("0B"), Some(0));
        assert_eq!(parse_human_size("N/A"), None);
        assert_eq!(parse_human_size(""), None);
    }

    #[test]
    fn settings_validation() {
        let mut s = MaintenanceSettings::defaults("localhost");
        assert!(s.validate().is_ok());
        s.cleanup_trigger = "sometimes".into();
        assert!(s.validate().is_err());
        s = MaintenanceSettings::defaults("localhost");
        s.cleanup_threshold_pct = 100;
        assert!(s.validate().is_err());
        s = MaintenanceSettings::defaults("localhost");
        s.maintenance_window = "3am".into();
        assert!(s.validate().is_err());
        s = MaintenanceSettings::defaults("localhost");
        s.cleanup_cron = "nope".into();
        assert!(s.validate().is_err());
    }

    #[test]
    fn shell_quote_escapes_single_quotes() {
        assert_eq!(shell_quote("abc"), "'abc'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }
}
