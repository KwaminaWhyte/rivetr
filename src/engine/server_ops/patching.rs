//! OS package patching: detect pending updates, apply them, configure
//! automatic security updates and reboot the server.

use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::{log_line, save_patch_state, HostTarget};
use crate::DbPool;

/// A package with a newer version available.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackageUpdate {
    pub name: String,
    pub current_version: Option<String>,
    pub new_version: String,
    pub arch: Option<String>,
    pub repository: Option<String>,
    pub security: bool,
    /// Updating it restarts the Docker daemon and with it every container.
    pub restarts_docker: bool,
}

/// Result of a package check, cached in `server_maintenance_settings.patch_state`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PatchState {
    /// apt, dnf, yum, apk, pacman, zypper or unknown
    pub package_manager: String,
    pub packages: Vec<PackageUpdate>,
    pub reboot_required: bool,
    pub checked_at: Option<String>,
    pub error: Option<String>,
}

impl PatchState {
    pub fn security_count(&self) -> usize {
        self.packages.iter().filter(|p| p.security).count()
    }
}

/// Package managers whose automatic security updates Rivetr can configure.
pub fn supports_auto_updates(pm: &str) -> bool {
    matches!(pm, "apt" | "dnf" | "yum")
}

/// Docker engine packages: updating them restarts dockerd and every container.
pub fn restarts_docker(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == "docker"
        || n == "docker.io"
        || n == "runc"
        || n.starts_with("docker-ce")
        || n.starts_with("docker-engine")
        || n.starts_with("containerd")
        || n.starts_with("moby-")
        || n == "podman"
}

/// A package name safe to pass to a package manager.
pub fn is_valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 200
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '_' | '-' | ':' | '@'))
}

/// Script that detects the package manager, lists upgradable packages and
/// prints whether a reboot is required.
pub(crate) const CHECK_SCRIPT: &str = r#"if command -v apt-get >/dev/null 2>&1; then
  echo "PKG_MANAGER:apt"
  $SUDO apt-get -qq update >/dev/null 2>&1 || true
  echo "---PACKAGES---"
  apt list --upgradable 2>/dev/null | grep -v '^Listing' || true
elif command -v dnf >/dev/null 2>&1 || command -v yum >/dev/null 2>&1; then
  if command -v dnf >/dev/null 2>&1; then P=dnf; else P=yum; fi
  echo "PKG_MANAGER:$P"
  echo "---PACKAGES---"
  $P -q check-update 2>/dev/null || true
  echo "---SECURITY---"
  $P -q updateinfo list --security 2>/dev/null || true
  echo "---INSTALLED---"
  rpm -qa --qf '%{NAME} %{VERSION}-%{RELEASE}\n' 2>/dev/null || true
elif command -v apk >/dev/null 2>&1; then
  echo "PKG_MANAGER:apk"
  $SUDO apk update -q >/dev/null 2>&1 || true
  echo "---PACKAGES---"
  apk -u list 2>/dev/null || apk version -l '<' 2>/dev/null || true
elif command -v pacman >/dev/null 2>&1; then
  echo "PKG_MANAGER:pacman"
  echo "---PACKAGES---"
  if command -v checkupdates >/dev/null 2>&1; then checkupdates 2>/dev/null; else $SUDO pacman -Sy >/dev/null 2>&1; pacman -Qu 2>/dev/null; fi
  true
elif command -v zypper >/dev/null 2>&1; then
  echo "PKG_MANAGER:zypper"
  echo "---PACKAGES---"
  $SUDO zypper -q --non-interactive list-updates 2>/dev/null || true
  echo "---SECURITY---"
  $SUDO zypper -q --non-interactive list-patches --category security 2>/dev/null || true
else
  echo "PKG_MANAGER:unknown"
fi
echo "---REBOOT---"
if [ -f /var/run/reboot-required ]; then
  echo "yes"
elif command -v needs-restarting >/dev/null 2>&1; then
  if needs-restarting -r >/dev/null 2>&1; then echo "no"; else echo "yes"; fi
elif [ -d /lib/modules ] && [ ! -d "/lib/modules/$(uname -r)" ]; then
  echo "yes"
else
  echo "no"
fi"#;

/// Split check output into named sections.
fn sections(output: &str) -> (String, Vec<(String, Vec<String>)>) {
    let mut pm = "unknown".to_string();
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for raw in output.lines() {
        let line = raw.trim_end();
        if let Some(m) = line.trim().strip_prefix("PKG_MANAGER:") {
            pm = m.trim().to_string();
            continue;
        }
        let t = line.trim();
        if t.starts_with("---") && t.ends_with("---") && t.len() > 6 {
            out.push((t.trim_matches('-').to_string(), Vec::new()));
            continue;
        }
        if let Some((_, lines)) = out.last_mut() {
            if !t.is_empty() {
                lines.push(line.to_string());
            }
        }
    }
    (pm, out)
}

fn section<'a>(secs: &'a [(String, Vec<String>)], name: &str) -> &'a [String] {
    secs.iter()
        .find(|(n, _)| n == name)
        .map(|(_, l)| l.as_slice())
        .unwrap_or(&[])
}

fn make(
    name: &str,
    current: Option<String>,
    new: &str,
    arch: Option<String>,
    repo: Option<String>,
    security: bool,
) -> PackageUpdate {
    PackageUpdate {
        name: name.to_string(),
        current_version: current,
        new_version: new.to_string(),
        arch,
        repository: repo,
        security,
        restarts_docker: restarts_docker(name),
    }
}

/// `pkg/noble-updates,noble-security 1.2.3 amd64 [upgradable from: 1.2.2]`
pub fn parse_apt(lines: &[String]) -> Vec<PackageUpdate> {
    lines
        .iter()
        .filter_map(|line| {
            let line = line.trim();
            let (name, rest) = line.split_once('/')?;
            let mut parts = rest.split_whitespace();
            let suites = parts.next()?;
            let version = parts.next()?;
            let arch = parts.next().map(|s| s.to_string());
            let current = line
                .split_once("upgradable from: ")
                .map(|(_, v)| v.trim_end_matches(']').trim().to_string());
            let security = suites.contains("-security") || suites.contains("security");
            Some(make(
                name,
                current,
                version,
                arch,
                Some(suites.to_string()),
                security,
            ))
        })
        .collect()
}

/// Split `name.arch` as printed by dnf / yum.
fn split_name_arch(s: &str) -> (String, Option<String>) {
    match s.rsplit_once('.') {
        Some((n, a))
            if matches!(
                a,
                "x86_64" | "noarch" | "aarch64" | "i686" | "i386" | "armv7hl" | "ppc64le" | "s390x"
            ) =>
        {
            (n.to_string(), Some(a.to_string()))
        }
        _ => (s.to_string(), None),
    }
}

/// `dnf check-update` lines: `name.arch  version  repo`. Security advisories
/// come from `dnf updateinfo list --security` (last column is the NEVRA) and
/// current versions from `rpm -qa`.
pub fn parse_dnf(
    packages: &[String],
    security: &[String],
    installed: &[String],
) -> Vec<PackageUpdate> {
    let security_nevras: Vec<&str> = security
        .iter()
        .filter_map(|l| l.split_whitespace().last())
        .collect();
    let installed_map: std::collections::HashMap<&str, &str> = installed
        .iter()
        .filter_map(|l| l.split_once(' '))
        .map(|(n, v)| (n.trim(), v.trim()))
        .collect();

    let mut out = Vec::new();
    for line in packages {
        let t = line.trim();
        // Sections after the updates list (obsoletes, security notes) end it.
        if t.starts_with("Obsoleting") || t.starts_with("Security:") {
            break;
        }
        let parts: Vec<&str> = t.split_whitespace().collect();
        if parts.len() != 3 || t.starts_with("Last metadata") {
            continue;
        }
        let (name, arch) = split_name_arch(parts[0]);
        if arch.is_none() {
            continue;
        }
        let version = parts[1];
        let is_security = security_nevras
            .iter()
            .any(|nevra| nevra.starts_with(&format!("{name}-")) && nevra.contains(version));
        out.push(make(
            &name,
            installed_map.get(name.as_str()).map(|v| v.to_string()),
            version,
            arch,
            Some(parts[2].to_string()),
            is_security,
        ));
    }
    out
}

/// Split an apk `name-1.2.3-r0` into name and version.
fn split_apk(s: &str) -> Option<(String, String)> {
    let (rest, rel) = s.rsplit_once('-')?;
    let (name, ver) = rest.rsplit_once('-')?;
    if !rel.starts_with('r') {
        return None;
    }
    Some((name.to_string(), format!("{ver}-{rel}")))
}

/// `apk -u list`: `curl-8.5.0-r0 x86_64 {curl} (curl) [upgradable from: curl-8.4.0-r0]`
pub fn parse_apk(lines: &[String]) -> Vec<PackageUpdate> {
    lines
        .iter()
        .filter_map(|line| {
            let t = line.trim();
            let mut parts = t.split_whitespace();
            let (name, version) = split_apk(parts.next()?)?;
            let arch = parts.next().map(|s| s.to_string());
            let current = t
                .split_once("upgradable from: ")
                .and_then(|(_, v)| split_apk(v.trim_end_matches(']').trim()))
                .map(|(_, v)| v);
            Some(make(&name, current, &version, arch, None, false))
        })
        .collect()
}

/// `checkupdates` / `pacman -Qu`: `linux 6.6.1.arch1-1 -> 6.6.2.arch1-1`
pub fn parse_pacman(lines: &[String]) -> Vec<PackageUpdate> {
    lines
        .iter()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 && parts[2] == "->" {
                Some(make(
                    parts[0],
                    Some(parts[1].to_string()),
                    parts[3],
                    None,
                    None,
                    false,
                ))
            } else {
                None
            }
        })
        .collect()
}

/// `zypper list-updates` table:
/// `v | Main Update Repository | curl | 8.0.1-1.1 | 8.0.1-2.1 | x86_64`
pub fn parse_zypper(lines: &[String], security: &[String]) -> Vec<PackageUpdate> {
    let security_text = security.join("\n");
    lines
        .iter()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split('|').map(|c| c.trim()).collect();
            if cols.len() < 6 || cols[0] == "S" || cols[2] == "Name" || cols[0].starts_with("--") {
                return None;
            }
            let name = cols[2];
            if name.is_empty() {
                return None;
            }
            Some(make(
                name,
                Some(cols[3].to_string()).filter(|s| !s.is_empty()),
                cols[4],
                Some(cols[5].to_string()).filter(|s| !s.is_empty()),
                Some(cols[1].to_string()).filter(|s| !s.is_empty()),
                security_text.contains(name),
            ))
        })
        .collect()
}

/// Parse the whole output of [`CHECK_SCRIPT`].
pub fn parse_check_output(output: &str) -> PatchState {
    let (pm, secs) = sections(output);
    let packages_lines = section(&secs, "PACKAGES");
    let mut packages = match pm.as_str() {
        "apt" => parse_apt(packages_lines),
        "dnf" | "yum" => parse_dnf(
            packages_lines,
            section(&secs, "SECURITY"),
            section(&secs, "INSTALLED"),
        ),
        "apk" => parse_apk(packages_lines),
        "pacman" => parse_pacman(packages_lines),
        "zypper" => parse_zypper(packages_lines, section(&secs, "SECURITY")),
        _ => Vec::new(),
    };
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages.dedup_by(|a, b| a.name == b.name);
    let reboot_required = parse_reboot_required(section(&secs, "REBOOT"));
    PatchState {
        package_manager: pm,
        packages,
        reboot_required,
        checked_at: Some(chrono::Utc::now().to_rfc3339()),
        error: None,
    }
}

/// The REBOOT section is a single `yes` or `no`.
pub fn parse_reboot_required(lines: &[String]) -> bool {
    lines.iter().any(|l| l.trim().eq_ignore_ascii_case("yes"))
}

/// Shell command that upgrades one package (or every package when `package`
/// is `None`) with the given package manager.
pub fn update_command(pm: &str, package: Option<&str>) -> Option<String> {
    let cmd = match (pm, package) {
        ("apt", Some(p)) => format!(
            "$SUDO env DEBIAN_FRONTEND=noninteractive apt-get install --only-upgrade -y \
             -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold {p}"
        ),
        ("apt", None) => "$SUDO env DEBIAN_FRONTEND=noninteractive apt-get update && \
             $SUDO env DEBIAN_FRONTEND=noninteractive apt-get upgrade -y \
             -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold"
            .to_string(),
        ("dnf", Some(p)) => format!("$SUDO dnf upgrade -y {p}"),
        ("dnf", None) => "$SUDO dnf upgrade -y".to_string(),
        ("yum", Some(p)) => format!("$SUDO yum update -y {p}"),
        ("yum", None) => "$SUDO yum update -y".to_string(),
        ("apk", Some(p)) => format!("$SUDO apk upgrade {p}"),
        ("apk", None) => "$SUDO apk update && $SUDO apk upgrade".to_string(),
        ("pacman", Some(p)) => format!("$SUDO pacman -S --noconfirm --needed {p}"),
        ("pacman", None) => "$SUDO pacman -Syu --noconfirm".to_string(),
        ("zypper", Some(p)) => format!("$SUDO zypper -n update {p}"),
        ("zypper", None) => "$SUDO zypper -n update".to_string(),
        _ => return None,
    };
    Some(cmd)
}

/// Script enabling or disabling unattended security updates (apt and dnf),
/// running at `window` (HH:MM, server local time).
pub fn auto_updates_script(enabled: bool, window: &str) -> String {
    if enabled {
        format!(
            r#"set -e
if command -v apt-get >/dev/null 2>&1; then
  $SUDO env DEBIAN_FRONTEND=noninteractive apt-get install -y unattended-upgrades
  printf 'APT::Periodic::Update-Package-Lists "1";\nAPT::Periodic::Unattended-Upgrade "1";\n' | $SUDO tee /etc/apt/apt.conf.d/20auto-upgrades >/dev/null
  $SUDO mkdir -p /etc/systemd/system/apt-daily-upgrade.timer.d
  printf '[Timer]\nOnCalendar=\nOnCalendar=*-*-* {window}:00\nRandomizedDelaySec=0\n' | $SUDO tee /etc/systemd/system/apt-daily-upgrade.timer.d/rivetr.conf >/dev/null
  $SUDO systemctl daemon-reload
  $SUDO systemctl enable --now apt-daily-upgrade.timer
  echo "Unattended security upgrades enabled at {window}"
elif command -v dnf >/dev/null 2>&1; then
  $SUDO dnf install -y dnf-automatic
  $SUDO sed -i 's/^upgrade_type.*/upgrade_type = security/; s/^apply_updates.*/apply_updates = yes/' /etc/dnf/automatic.conf
  $SUDO mkdir -p /etc/systemd/system/dnf-automatic.timer.d
  printf '[Timer]\nOnCalendar=\nOnCalendar=*-*-* {window}:00\nRandomizedDelaySec=0\n' | $SUDO tee /etc/systemd/system/dnf-automatic.timer.d/rivetr.conf >/dev/null
  $SUDO systemctl daemon-reload
  $SUDO systemctl enable --now dnf-automatic.timer
  echo "dnf-automatic security updates enabled at {window}"
else
  echo "Automatic security updates are only supported with apt or dnf" >&2
  exit 2
fi"#
        )
    } else {
        r#"if command -v apt-get >/dev/null 2>&1; then
  printf 'APT::Periodic::Update-Package-Lists "1";\nAPT::Periodic::Unattended-Upgrade "0";\n' | $SUDO tee /etc/apt/apt.conf.d/20auto-upgrades >/dev/null
  $SUDO rm -f /etc/systemd/system/apt-daily-upgrade.timer.d/rivetr.conf
  $SUDO systemctl daemon-reload || true
  echo "Unattended security upgrades disabled"
elif command -v dnf >/dev/null 2>&1; then
  $SUDO systemctl disable --now dnf-automatic.timer || true
  $SUDO rm -f /etc/systemd/system/dnf-automatic.timer.d/rivetr.conf
  $SUDO systemctl daemon-reload || true
  echo "dnf-automatic disabled"
else
  echo "Nothing to disable"
fi"#
        .to_string()
    }
}

/// Reboot in the background so the command returns before the connection drops.
// systemd-run hands the delayed reboot to the host's systemd, so it survives the end
// of this shell (and of the helper container on a hardened local install).
pub(crate) const REBOOT_SCRIPT: &str = r#"echo "Rebooting in 2 seconds"
if command -v systemd-run >/dev/null 2>&1 && $SUDO systemd-run --on-active=2 --unit=rivetr-reboot systemctl reboot >/dev/null 2>&1; then
  exit 0
fi
nohup sh -c "sleep 2; $SUDO shutdown -r now 'Reboot requested from Rivetr' || $SUDO reboot" >/dev/null 2>&1 &
exit 0"#;

/// Run a package check and cache the result.
pub async fn run_check(db: &DbPool, server_id: &str, target: &HostTarget) -> Result<PatchState> {
    let out = target
        .exec_privileged(CHECK_SCRIPT, Duration::from_secs(300))
        .await
        .map_err(|e| anyhow::anyhow!("{e}"));
    let state = match out {
        Ok(out) => {
            let mut s = parse_check_output(&out.stdout);
            if s.package_manager == "unknown" {
                s.error = Some("No supported package manager found".to_string());
            }
            s
        }
        Err(e) => PatchState {
            package_manager: "unknown".to_string(),
            checked_at: Some(chrono::Utc::now().to_rfc3339()),
            error: Some(e.to_string()),
            ..Default::default()
        },
    };
    save_patch_state(db, server_id, &state).await?;
    Ok(state)
}

// ---------------------------------------------------------------------------
// Background jobs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PatchJob {
    pub id: String,
    pub server_id: String,
    pub kind: String,
    pub package: Option<String>,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub log: String,
}

/// What a patch job does.
#[derive(Debug, Clone)]
pub enum PatchJobKind {
    Check,
    UpdatePackage(String),
    UpdateAll,
    Reboot,
    ConfigureAutoUpdates { enabled: bool, window: String },
}

impl PatchJobKind {
    fn as_str(&self) -> &'static str {
        match self {
            PatchJobKind::Check => "check",
            PatchJobKind::UpdatePackage(_) => "update_package",
            PatchJobKind::UpdateAll => "update_all",
            PatchJobKind::Reboot => "reboot",
            PatchJobKind::ConfigureAutoUpdates { .. } => "configure_auto_updates",
        }
    }
}

/// Is a patch job already running for this server?
pub async fn running_job(db: &DbPool, server_id: &str) -> Result<Option<PatchJob>> {
    Ok(sqlx::query_as::<_, PatchJob>(
        "SELECT * FROM server_patch_jobs WHERE server_id = ? AND status = 'running' \
         ORDER BY started_at DESC LIMIT 1",
    )
    .bind(server_id)
    .fetch_optional(db)
    .await?)
}

pub async fn recent_jobs(db: &DbPool, server_id: &str, limit: i64) -> Result<Vec<PatchJob>> {
    Ok(sqlx::query_as::<_, PatchJob>(
        "SELECT * FROM server_patch_jobs WHERE server_id = ? ORDER BY started_at DESC LIMIT ?",
    )
    .bind(server_id)
    .bind(limit)
    .fetch_all(db)
    .await?)
}

/// Record a job row and run it in the background. Returns the job id.
pub async fn start_job(
    db: DbPool,
    server_id: String,
    target: HostTarget,
    kind: PatchJobKind,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let package = match &kind {
        PatchJobKind::UpdatePackage(p) => Some(p.clone()),
        _ => None,
    };
    sqlx::query(
        "INSERT INTO server_patch_jobs (id, server_id, kind, package, status, started_at, log) \
         VALUES (?, ?, ?, ?, 'running', ?, '')",
    )
    .bind(&id)
    .bind(&server_id)
    .bind(kind.as_str())
    .bind(&package)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(&db)
    .await?;

    let job_id = id.clone();
    tokio::spawn(async move {
        let (ok, log) = execute_job(&db, &server_id, &target, &kind).await;
        let status = if ok { "success" } else { "failed" };
        if let Err(e) = sqlx::query(
            "UPDATE server_patch_jobs SET status = ?, finished_at = ?, log = ? WHERE id = ?",
        )
        .bind(status)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&log)
        .bind(&job_id)
        .execute(&db)
        .await
        {
            tracing::error!(job_id = %job_id, "Failed to store patch job result: {}", e);
        }
    });
    Ok(id)
}

async fn execute_job(
    db: &DbPool,
    server_id: &str,
    target: &HostTarget,
    kind: &PatchJobKind,
) -> (bool, String) {
    let mut log = String::new();
    let pm = super::load_settings(db, server_id)
        .await
        .ok()
        .and_then(|s| s.patch_state())
        .map(|s| s.package_manager)
        .unwrap_or_default();

    let script = match kind {
        PatchJobKind::Check => {
            return match run_check(db, server_id, target).await {
                Ok(state) => {
                    log_line(
                        &mut log,
                        format!(
                            "Found {} package update(s) via {}{}",
                            state.packages.len(),
                            state.package_manager,
                            if state.reboot_required {
                                ", reboot required"
                            } else {
                                ""
                            }
                        ),
                    );
                    if let Some(err) = &state.error {
                        log_line(&mut log, err);
                    }
                    (state.error.is_none(), log)
                }
                Err(e) => (false, format!("Check failed: {e}")),
            };
        }
        PatchJobKind::UpdatePackage(p) => match update_command(&pm, Some(p)) {
            Some(c) => c,
            None => return (false, "Run a package check first".to_string()),
        },
        PatchJobKind::UpdateAll => match update_command(&pm, None) {
            Some(c) => c,
            None => return (false, "Run a package check first".to_string()),
        },
        PatchJobKind::Reboot => REBOOT_SCRIPT.to_string(),
        PatchJobKind::ConfigureAutoUpdates { enabled, window } => {
            auto_updates_script(*enabled, window)
        }
    };

    log_line(
        &mut log,
        format!("$ {}", script.lines().next().unwrap_or("")),
    );
    let result = target
        .exec_privileged(&format!("{script} 2>&1"), Duration::from_secs(1800))
        .await;
    let ok = match result {
        Ok(out) => {
            log_line(&mut log, out.combined());
            out.success()
        }
        Err(e) => {
            // A reboot can drop the SSH session before it reports back.
            if matches!(kind, PatchJobKind::Reboot) {
                log_line(&mut log, format!("Connection closed: {e}"));
                true
            } else {
                log_line(&mut log, format!("Error: {e}"));
                false
            }
        }
    };

    // Refresh the package list after changing packages.
    if matches!(
        kind,
        PatchJobKind::UpdatePackage(_) | PatchJobKind::UpdateAll
    ) {
        match run_check(db, server_id, target).await {
            Ok(state) => log_line(
                &mut log,
                format!(
                    "Re-checked: {} package update(s) left{}",
                    state.packages.len(),
                    if state.reboot_required {
                        ", reboot required"
                    } else {
                        ""
                    }
                ),
            ),
            Err(e) => log_line(&mut log, format!("Re-check failed: {e}")),
        }
    }
    (ok, log)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Vec<String> {
        s.lines().map(|l| l.to_string()).collect()
    }

    #[test]
    fn parses_apt_upgradable() {
        let pk = parse_apt(&lines(
            "curl/noble-updates,noble-security 8.5.0-2ubuntu10.4 amd64 [upgradable from: 8.5.0-2ubuntu10.3]\n\
             docker-ce/noble 5:27.3.1-1~ubuntu.24.04~noble amd64 [upgradable from: 5:27.2.0-1~ubuntu.24.04~noble]\n\
             garbage line",
        ));
        assert_eq!(pk.len(), 2);
        assert_eq!(pk[0].name, "curl");
        assert_eq!(pk[0].new_version, "8.5.0-2ubuntu10.4");
        assert_eq!(pk[0].current_version.as_deref(), Some("8.5.0-2ubuntu10.3"));
        assert_eq!(pk[0].arch.as_deref(), Some("amd64"));
        assert!(pk[0].security);
        assert!(!pk[0].restarts_docker);
        assert_eq!(pk[1].name, "docker-ce");
        assert!(!pk[1].security);
        assert!(pk[1].restarts_docker);
    }

    #[test]
    fn parses_dnf_check_update() {
        let packages = lines(
            "\nkernel.x86_64    5.14.0-427.el9    baseos\n\
             containerd.io.x86_64   1.7.22-3.1.el9   docker-ce-stable\n\
             Obsoleting Packages\n\
             foo.x86_64  1-1  baseos\n",
        );
        let security = lines("RHSA-2024:1234 Important/Sec. kernel-5.14.0-427.el9.x86_64");
        let installed = lines("kernel 5.14.0-362.el9\ncontainerd.io 1.7.20-3.1.el9");
        let pk = parse_dnf(&packages, &security, &installed);
        assert_eq!(pk.len(), 2);
        assert_eq!(pk[0].name, "kernel");
        assert!(pk[0].security);
        assert_eq!(pk[0].current_version.as_deref(), Some("5.14.0-362.el9"));
        assert_eq!(pk[1].name, "containerd.io");
        assert!(pk[1].restarts_docker);
        assert!(!pk[1].security);
    }

    #[test]
    fn parses_apk_pacman_zypper() {
        let apk = parse_apk(&lines(
            "curl-8.5.0-r0 x86_64 {curl} (curl) [upgradable from: curl-8.4.0-r0]",
        ));
        assert_eq!(apk[0].name, "curl");
        assert_eq!(apk[0].new_version, "8.5.0-r0");
        assert_eq!(apk[0].current_version.as_deref(), Some("8.4.0-r0"));

        let pac = parse_pacman(&lines("linux 6.6.1.arch1-1 -> 6.6.2.arch1-1"));
        assert_eq!(pac[0].name, "linux");
        assert_eq!(pac[0].current_version.as_deref(), Some("6.6.1.arch1-1"));
        assert_eq!(pac[0].new_version, "6.6.2.arch1-1");

        let zy = parse_zypper(
            &lines(
                "S | Repository | Name | Current Version | Available Version | Arch\n\
                 --+------------+------+-----------------+-------------------+-------\n\
                 v | Main Update | curl | 8.0.1-1.1 | 8.0.1-2.1 | x86_64",
            ),
            &lines("SUSE-2024-1 | security | important | curl"),
        );
        assert_eq!(zy.len(), 1);
        assert_eq!(zy[0].name, "curl");
        assert_eq!(zy[0].new_version, "8.0.1-2.1");
        assert!(zy[0].security);
    }

    #[test]
    fn parses_full_check_output_and_reboot() {
        let out = "PKG_MANAGER:apt\n---PACKAGES---\n\
                   openssl/jammy-security 3.0.2-0ubuntu1.18 amd64 [upgradable from: 3.0.2-0ubuntu1.17]\n\
                   ---REBOOT---\nyes\n";
        let state = parse_check_output(out);
        assert_eq!(state.package_manager, "apt");
        assert_eq!(state.packages.len(), 1);
        assert_eq!(state.security_count(), 1);
        assert!(state.reboot_required);

        let none = parse_check_output("PKG_MANAGER:unknown\n---REBOOT---\nno\n");
        assert_eq!(none.package_manager, "unknown");
        assert!(none.packages.is_empty());
        assert!(!none.reboot_required);
    }

    #[test]
    fn reboot_detection() {
        assert!(parse_reboot_required(&lines("yes")));
        assert!(!parse_reboot_required(&lines("no")));
        assert!(!parse_reboot_required(&[]));
    }

    #[test]
    fn package_name_validation_blocks_injection() {
        assert!(is_valid_package_name("docker-ce"));
        assert!(is_valid_package_name("libstdc++6"));
        assert!(is_valid_package_name("python3.12"));
        assert!(!is_valid_package_name(""));
        assert!(!is_valid_package_name("-y"));
        assert!(!is_valid_package_name("curl; reboot"));
        assert!(!is_valid_package_name("curl $(id)"));
    }

    #[test]
    fn update_commands_per_manager() {
        assert!(update_command("apt", Some("curl"))
            .unwrap()
            .contains("--only-upgrade -y"));
        assert_eq!(
            update_command("dnf", None).as_deref(),
            Some("$SUDO dnf upgrade -y")
        );
        assert!(update_command("unknown", None).is_none());
        assert!(supports_auto_updates("apt"));
        assert!(!supports_auto_updates("pacman"));
    }
}
