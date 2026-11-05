//! Privileged commands on managed servers when the SSH user is not root.
//!
//! Every script Rivetr runs over SSH (health checks, Docker install, patching,
//! security checks, remote builds, server operations) is wrapped with
//! [`wrap_script`]. The prelude it adds sets two shell variables:
//!
//! - `$SUDO` is `sudo -n` when the remote user is not root and `sudo` exists,
//!   and empty otherwise. Privileged commands are written as `$SUDO cmd`.
//!   `-n` makes sudo fail instead of hanging on a password prompt.
//! - `$DOCKER` names a shell function that runs `docker` (or `podman`)
//!   directly when the user can reach the daemon (root, or a member of the
//!   `docker` group) and through `$SUDO` otherwise. It is a single word, so
//!   both `$DOCKER ps` and `"$DOCKER" ps` work.
//!
//! [`SUDO_CHECK`] reports whether passwordless sudo works, and
//! [`PrivilegeCheck`] turns that into a validation result with the exact
//! sudoers line to add. Setup is documented in `docs/reference/remote-servers.md`.

use serde::Serialize;

/// Sets `$SUDO` (see module docs).
pub const SUDO_LINE: &str = r#"if [ "$(id -u)" -ne 0 ] && command -v sudo >/dev/null 2>&1; then SUDO="sudo -n"; else SUDO=""; fi"#;

/// Define `$var` as a container CLI wrapper (see module docs). `candidates` are
/// tried in order; the first one found on `PATH` is used. Must come after
/// [`SUDO_LINE`].
pub fn container_cli_line(var: &str, candidates: &[&str]) -> String {
    let fallback = candidates.first().copied().unwrap_or("docker");
    let lookup = candidates
        .iter()
        .map(|c| format!("command -v {c}"))
        .chain(std::iter::once(format!("echo {fallback}")))
        .collect::<Vec<_>>()
        .join(" || ");
    format!(
        r#"__rivetr_cli_bin=$({lookup})
__rivetr_cli() {{ "$__rivetr_cli_bin" "$@"; }}
if [ -n "$SUDO" ] && ! "$__rivetr_cli_bin" info >/dev/null 2>&1; then __rivetr_cli() {{ $SUDO "$__rivetr_cli_bin" "$@"; }}; fi
{var}=__rivetr_cli"#
    )
}

/// Prefix `script` with the privilege prelude so it can use `$SUDO` and `$DOCKER`.
pub fn wrap_script(script: &str) -> String {
    format!(
        "{SUDO_LINE}\n{}\n{script}",
        container_cli_line("DOCKER", &["docker", "podman"])
    )
}

/// Shell snippet that prints `RIVETR_USER:<name>` and `RIVETR_SUDO:<state>`
/// where state is `root`, `ok`, `missing` or `nosudo`.
pub const SUDO_CHECK: &str = r#"echo "RIVETR_USER:$(id -un 2>/dev/null || whoami)"
if [ "$(id -u)" -eq 0 ]; then echo "RIVETR_SUDO:root"
elif ! command -v sudo >/dev/null 2>&1; then echo "RIVETR_SUDO:nosudo"
elif sudo -n true >/dev/null 2>&1; then echo "RIVETR_SUDO:ok"
else echo "RIVETR_SUDO:missing"; fi"#;

/// Result of [`SUDO_CHECK`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SudoStatus {
    /// SSH user is root; no sudo needed.
    Root,
    /// Non-root user with working passwordless sudo.
    Available,
    /// Non-root user; sudo exists but asks for a password.
    Missing,
    /// Non-root user and sudo is not installed.
    NotInstalled,
}

/// Validation result shown in the UI.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PrivilegeCheck {
    pub ssh_user: String,
    pub status: SudoStatus,
    /// True when Rivetr can run privileged commands.
    pub ok: bool,
    pub message: String,
    /// Line to put in `/etc/sudoers.d/rivetr` when sudo is missing.
    pub sudoers_line: Option<String>,
}

/// The sudoers line that grants `user` passwordless sudo.
pub fn sudoers_line(user: &str) -> String {
    format!("{user} ALL=(ALL) NOPASSWD: ALL")
}

fn is_safe_username(user: &str) -> bool {
    !user.is_empty()
        && user.len() <= 64
        && user
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

impl PrivilegeCheck {
    /// Parse the output of a script that included [`SUDO_CHECK`].
    /// `fallback_user` is the configured SSH user, used if the output lacks one.
    pub fn parse(output: &str, fallback_user: &str) -> Option<Self> {
        let mut user = None;
        let mut state = None;
        for line in output.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("RIVETR_USER:") {
                user = Some(v.trim().to_string());
            } else if let Some(v) = line.strip_prefix("RIVETR_SUDO:") {
                state = Some(v.trim().to_string());
            }
        }
        let user = user
            .filter(|u| is_safe_username(u))
            .unwrap_or_else(|| fallback_user.to_string());
        let status = match state?.as_str() {
            "root" => SudoStatus::Root,
            "ok" => SudoStatus::Available,
            "missing" => SudoStatus::Missing,
            "nosudo" => SudoStatus::NotInstalled,
            _ => return None,
        };
        Some(Self::from_status(user, status))
    }

    pub fn from_status(ssh_user: String, status: SudoStatus) -> Self {
        let line = sudoers_line(&ssh_user);
        let (ok, message, sudoers) = match status {
            SudoStatus::Root => (true, "Connected as root".to_string(), None),
            SudoStatus::Available => (
                true,
                format!("Passwordless sudo available for {ssh_user}"),
                None,
            ),
            SudoStatus::Missing => (
                false,
                format!(
                    "Passwordless sudo missing for {ssh_user}. Run `sudo visudo -f /etc/sudoers.d/rivetr` on the server and add: {line}"
                ),
                Some(line),
            ),
            SudoStatus::NotInstalled => (
                false,
                format!(
                    "sudo is not installed and {ssh_user} is not root. Install sudo, then add to /etc/sudoers.d/rivetr: {line}"
                ),
                Some(line),
            ),
        };
        Self {
            ssh_user,
            status,
            ok,
            message,
            sudoers_line: sudoers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run `script` under `sh` with `id`, `sudo` and `docker` faked by shell
    /// functions. The fake daemon refuses `docker info` (user not in the
    /// docker group).
    fn run_with_fake(uid: &str, sudo_works: bool, script: &str) -> String {
        let fake = format!(
            r#"id() {{ if [ "$1" = "-u" ]; then echo {uid}; else echo deploy; fi; }}
sudo() {{ [ "$1" = "-n" ] && shift; [ "$1" = true ] || echo "via-sudo $*"; return {}; }}
docker() {{ if [ "$1" = info ]; then return 1; fi; echo "docker $*"; }}
"#,
            if sudo_works { 0 } else { 1 }
        );
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("{fake}{script}"))
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).to_string()
    }

    #[test]
    fn prelude_routes_docker_through_sudo_for_non_root() {
        let out = run_with_fake(
            "1000",
            true,
            &wrap_script(r#"echo "[$SUDO]"; $DOCKER ps; "$DOCKER" image ls"#),
        );
        assert_eq!(
            out,
            "[sudo -n]\nvia-sudo docker ps\nvia-sudo docker image ls\n"
        );
    }

    #[test]
    fn prelude_is_direct_for_root() {
        let out = run_with_fake("0", true, &wrap_script(r#"echo "[$SUDO]"; $DOCKER ps"#));
        assert_eq!(out, "[]\ndocker ps\n");
    }

    #[test]
    fn container_cli_prefers_first_candidate() {
        let line = container_cli_line("D", &["podman", "docker"]);
        assert!(line.contains("command -v podman || command -v docker || echo podman"));
        assert!(line.ends_with("D=__rivetr_cli"));
    }

    #[test]
    fn sudo_check_reports_states() {
        let ok = run_with_fake("1000", true, SUDO_CHECK);
        let check = PrivilegeCheck::parse(&ok, "x").unwrap();
        assert_eq!(check.status, SudoStatus::Available);
        assert_eq!(check.ssh_user, "deploy");
        assert!(check.ok);

        let missing = run_with_fake("1000", false, SUDO_CHECK);
        let check = PrivilegeCheck::parse(&missing, "x").unwrap();
        assert_eq!(check.status, SudoStatus::Missing);
        assert!(!check.ok);
        assert_eq!(
            check.sudoers_line.as_deref(),
            Some("deploy ALL=(ALL) NOPASSWD: ALL")
        );

        let root = run_with_fake("0", false, SUDO_CHECK);
        assert_eq!(
            PrivilegeCheck::parse(&root, "x").unwrap().status,
            SudoStatus::Root
        );
    }

    #[test]
    fn parse_falls_back_to_configured_user_and_rejects_junk() {
        let out = "RIVETR_USER:bad user;rm\nRIVETR_SUDO:missing\n";
        let check = PrivilegeCheck::parse(out, "ubuntu").unwrap();
        assert_eq!(check.ssh_user, "ubuntu");
        assert!(PrivilegeCheck::parse("nothing here", "ubuntu").is_none());
    }
}
