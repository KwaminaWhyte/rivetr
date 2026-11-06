//! Shell commands for multi-server deployments: getting the image onto each
//! server (registry pull or `docker save | ssh docker load`), starting and
//! stopping containers, and reading the published port. Everything here only
//! builds strings; `shell.rs` runs them. Scripts assume the server-ops prelude,
//! which sets `$D` to the container CLI (through `sudo -n` when needed).

use std::collections::BTreeMap;

use crate::db::ImageDistribution;
use crate::engine::server_ops::shell_quote;

use super::plan::BindAddr;

/// How the built image reaches the remote targets of one deployment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistributionStrategy {
    /// Every target pulls this reference (pushed registry image or a prebuilt
    /// registry image the app deploys from).
    Registry {
        image_ref: String,
        login: Option<RegistryLogin>,
    },
    /// Stream the local image to each remote target over SSH.
    SshStream { image: String },
}

/// Registry credentials for `docker login` on a target. The password is sent on
/// stdin, never on a command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryLogin {
    pub registry: String,
    pub username: String,
    pub password: String,
}

/// Pick the strategy. `pushed_ref` is the registry reference of this build when
/// it was pushed, `prebuilt_ref` the image of an app that deploys a registry image.
/// `Registry` mode without a pushed image falls back to SSH with a reason.
pub fn choose_strategy(
    mode: ImageDistribution,
    local_image: &str,
    pushed_ref: Option<&str>,
    prebuilt_ref: Option<&str>,
    login: Option<RegistryLogin>,
) -> (DistributionStrategy, Option<&'static str>) {
    // An explicit "ssh" choice wins even for a registry image: the target may be
    // unable or unwilling to pull (private networks, offline hosts, rate limits).
    if mode == ImageDistribution::Ssh {
        return (
            DistributionStrategy::SshStream {
                image: local_image.to_string(),
            },
            None,
        );
    }
    if let Some(image_ref) = prebuilt_ref {
        return (
            DistributionStrategy::Registry {
                image_ref: image_ref.to_string(),
                login,
            },
            None,
        );
    }
    match (mode, pushed_ref) {
        (_, Some(image_ref)) => (
            DistributionStrategy::Registry {
                image_ref: image_ref.to_string(),
                login,
            },
            None,
        ),
        (ImageDistribution::Registry, None) => (
            DistributionStrategy::SshStream {
                image: local_image.to_string(),
            },
            Some("Registry distribution needs registry push with a registry URL; streaming over SSH instead"),
        ),
        (ImageDistribution::Auto, None) | (ImageDistribution::Ssh, None) => (
            DistributionStrategy::SshStream {
                image: local_image.to_string(),
            },
            None,
        ),
    }
}

/// Image reference the containers run from under a strategy.
pub fn run_image(strategy: &DistributionStrategy) -> &str {
    match strategy {
        DistributionStrategy::Registry { image_ref, .. } => image_ref,
        DistributionStrategy::SshStream { image } => image,
    }
}

/// Remote script that logs in (password on stdin) and pulls `image_ref`.
pub fn registry_pull_script(image_ref: &str, login: Option<&RegistryLogin>) -> String {
    let mut s = String::from("set -e\n");
    if let Some(l) = login {
        s.push_str(&format!(
            "$D login {} -u {} --password-stdin >/dev/null\n",
            shell_quote(&l.registry),
            shell_quote(&l.username)
        ));
    }
    s.push_str(&format!("$D pull {}\n", shell_quote(image_ref)));
    s
}

/// Local producer half of SSH streaming: save and compress the image to stdout.
/// A failed save leaves a truncated stream, which makes the remote load fail.
pub fn local_save_script(image: &str) -> String {
    format!("$D save {} | gzip -1", shell_quote(image))
}

/// Remote consumer half of SSH streaming: decompress stdin into the image store.
pub const REMOTE_LOAD_SCRIPT: &str = "gunzip -c | $D load";

/// `ssh` arguments (without the remote command) for a target. `key_path` is a
/// private key file; with `use_password` the caller wraps the command in `sshpass -e`.
pub fn ssh_args(
    host: &str,
    port: u16,
    username: &str,
    key_path: Option<&str>,
    use_password: bool,
    known_hosts_option: &str,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-o".into(),
        "StrictHostKeyChecking=accept-new".into(),
        "-o".into(),
        known_hosts_option.to_string(),
        "-o".into(),
        "ConnectTimeout=10".into(),
        "-o".into(),
        "ServerAliveInterval=30".into(),
    ];
    if !use_password {
        args.push("-o".into());
        args.push("BatchMode=yes".into());
    }
    args.push("-p".into());
    args.push(port.to_string());
    if let Some(k) = key_path {
        args.push("-i".into());
        args.push(k.to_string());
    }
    args.push(format!("{}@{}", username, host));
    args
}

/// Everything needed to start one container on a target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunSpec {
    pub name: String,
    pub image: String,
    pub app_port: u16,
    pub bind: Option<BindAddr>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub restart_policy: String,
    pub labels: BTreeMap<String, String>,
    /// `host:container[:ro]` bind mounts (paths on the target server).
    pub volumes: Vec<String>,
    pub extra_hosts: Vec<String>,
    pub cap_add: Vec<String>,
    pub cap_drop: Vec<String>,
    pub privileged: bool,
    pub init: bool,
    pub shm_size: Option<String>,
}

/// Docker CLI memory value: `512mb` becomes `512m` (the CLI only takes b, k, m, g).
pub fn cli_memory(value: &str) -> Option<String> {
    let v = value.trim().to_ascii_lowercase();
    if v.is_empty() {
        return None;
    }
    let v = ["gb", "mb", "kb"]
        .iter()
        .find_map(|suffix| {
            v.strip_suffix(suffix)
                .map(|n| format!("{}{}", n, &suffix[..1]))
        })
        .unwrap_or(v);
    let digits = v.trim_end_matches(['b', 'k', 'm', 'g']);
    if digits.is_empty() || digits.parse::<f64>().is_err() {
        return None;
    }
    Some(v)
}

fn restart_flag(policy: &str) -> &'static str {
    match policy {
        "always" => "always",
        "on-failure" => "on-failure:5",
        "never" | "no" => "no",
        _ => "unless-stopped",
    }
}

/// The env file content (`KEY=VALUE` lines) and the keys skipped because the
/// format cannot carry them (multi-line values, invalid names).
pub fn env_file(env: &[(String, String)]) -> (String, Vec<String>) {
    let mut out = String::new();
    let mut skipped = Vec::new();
    for (k, v) in env {
        let valid_key = !k.is_empty()
            && !k.contains('=')
            && !k.chars().any(|c| c.is_whitespace() || c.is_control());
        if !valid_key || v.contains('\n') || v.contains('\r') {
            skipped.push(k.clone());
            continue;
        }
        out.push_str(k);
        out.push('=');
        out.push_str(v);
        out.push('\n');
    }
    (out, skipped)
}

/// Script that starts the container with env from stdin (written to a private
/// temp file that is removed on exit) and prints the container id, then the
/// published host address of the app port.
pub fn run_script(spec: &RunSpec) -> String {
    let q = shell_quote;
    let mut s = String::from(
        "set -e\numask 077\nENVF=$(mktemp)\ntrap 'rm -f \"$ENVF\"' EXIT\ncat > \"$ENVF\"\n",
    );
    s.push_str(&format!(
        "$D rm -f {} >/dev/null 2>&1 || true\n",
        q(&spec.name)
    ));
    let publish = match &spec.bind {
        Some(BindAddr::Ip(ip)) => format!("-p {}", q(&format!("{}::{}", ip, spec.app_port))),
        Some(BindAddr::TailscaleLookup) => {
            s.push_str("BIND=$(tailscale ip -4 2>/dev/null | head -n1)\n");
            s.push_str("[ -n \"$BIND\" ] || BIND=0.0.0.0\n");
            format!("-p \"$BIND::{}\"", spec.app_port)
        }
        Some(BindAddr::All) | None => format!("-p {}", spec.app_port),
    };
    let mut cmd: Vec<String> = vec![
        "$D run -d".into(),
        format!("--name {}", q(&spec.name)),
        "--env-file \"$ENVF\"".into(),
        publish,
        format!("--restart {}", restart_flag(&spec.restart_policy)),
    ];
    if let Some(m) = spec.memory_limit.as_deref().and_then(cli_memory) {
        cmd.push(format!("--memory {}", q(&m)));
    }
    if let Some(c) = spec
        .cpu_limit
        .as_deref()
        .filter(|c| c.trim().parse::<f64>().is_ok_and(|n| n > 0.0))
    {
        cmd.push(format!("--cpus {}", q(c.trim())));
    }
    for (k, v) in &spec.labels {
        cmd.push(format!("--label {}", q(&format!("{}={}", k, v))));
    }
    for v in &spec.volumes {
        cmd.push(format!("-v {}", q(v)));
    }
    for h in &spec.extra_hosts {
        cmd.push(format!("--add-host {}", q(h)));
    }
    for c in &spec.cap_add {
        cmd.push(format!("--cap-add {}", q(c)));
    }
    for c in &spec.cap_drop {
        cmd.push(format!("--cap-drop {}", q(c)));
    }
    if spec.privileged {
        cmd.push("--privileged".into());
    }
    if spec.init {
        cmd.push("--init".into());
    }
    if let Some(shm) = spec.shm_size.as_deref().and_then(cli_memory) {
        cmd.push(format!("--shm-size {}", q(&shm)));
    }
    cmd.push(q(&spec.image));
    s.push_str(&cmd.join(" "));
    s.push('\n');
    s.push_str(&format!(
        "$D port {} {}/tcp\n",
        q(&spec.name),
        spec.app_port
    ));
    s
}

/// Parse `run_script` output: the container id (first line) and the published
/// host port (from `$D port`, e.g. `100.64.0.7:49153` or `[::]:49153`).
pub fn parse_run_output(stdout: &str) -> Option<(String, u16)> {
    let mut lines = stdout.lines().map(str::trim).filter(|l| !l.is_empty());
    let id = lines.next()?.to_string();
    let port = lines.find_map(|l| l.rsplit_once(':').and_then(|(_, p)| p.parse::<u16>().ok()))?;
    Some((id, port))
}

/// Script that stops and removes containers by name; missing ones are ignored.
pub fn remove_script(names: &[String]) -> String {
    let mut s = String::new();
    for n in names {
        s.push_str(&format!(
            "$D stop -t 10 {n} >/dev/null 2>&1 || true\n$D rm -f {n} >/dev/null 2>&1 || true\n",
            n = shell_quote(n)
        ));
    }
    s
}

/// Script that runs a health check command inside a container.
pub fn exec_check_script(name: &str, command: &str) -> String {
    format!(
        "$D exec {} sh -c {}",
        shell_quote(name),
        shell_quote(command)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn login() -> RegistryLogin {
        RegistryLogin {
            registry: "ghcr.io".into(),
            username: "bot".into(),
            password: "s3cret".into(),
        }
    }

    #[test]
    fn strategy_auto_uses_registry_only_when_pushed() {
        let (s, note) =
            choose_strategy(ImageDistribution::Auto, "rivetr-app:abc", None, None, None);
        assert_eq!(
            s,
            DistributionStrategy::SshStream {
                image: "rivetr-app:abc".into()
            }
        );
        assert!(note.is_none());

        let (s, _) = choose_strategy(
            ImageDistribution::Auto,
            "rivetr-app:abc",
            Some("ghcr.io/me/app:abc"),
            None,
            Some(login()),
        );
        assert_eq!(run_image(&s), "ghcr.io/me/app:abc");
        assert!(matches!(
            s,
            DistributionStrategy::Registry { login: Some(_), .. }
        ));
    }

    #[test]
    fn strategy_ssh_forced_and_registry_fallback() {
        let (s, _) = choose_strategy(
            ImageDistribution::Ssh,
            "rivetr-app:abc",
            Some("ghcr.io/me/app:abc"),
            None,
            None,
        );
        assert_eq!(run_image(&s), "rivetr-app:abc");

        let (s, note) = choose_strategy(
            ImageDistribution::Registry,
            "rivetr-app:abc",
            None,
            None,
            None,
        );
        assert!(matches!(s, DistributionStrategy::SshStream { .. }));
        assert!(note.is_some());
    }

    #[test]
    fn strategy_ssh_streams_even_a_registry_image() {
        let (s, _) = choose_strategy(
            ImageDistribution::Ssh,
            "nginx:1.27",
            None,
            Some("nginx:1.27"),
            None,
        );
        assert_eq!(
            s,
            DistributionStrategy::SshStream {
                image: "nginx:1.27".into()
            }
        );
    }

    #[test]
    fn strategy_prebuilt_registry_image_pulls_by_default() {
        let (s, _) = choose_strategy(
            ImageDistribution::Auto,
            "nginx:1.27",
            None,
            Some("nginx:1.27"),
            None,
        );
        assert_eq!(
            s,
            DistributionStrategy::Registry {
                image_ref: "nginx:1.27".into(),
                login: None
            }
        );
    }

    #[test]
    fn pull_script_keeps_password_off_the_command_line() {
        let l = login();
        let s = registry_pull_script("ghcr.io/me/app:abc", Some(&l));
        assert!(s.contains("$D login 'ghcr.io' -u 'bot' --password-stdin"));
        assert!(s.contains("$D pull 'ghcr.io/me/app:abc'"));
        assert!(!s.contains("s3cret"));
        let s = registry_pull_script("nginx:1.27", None);
        assert!(!s.contains("login"));
    }

    #[test]
    fn stream_scripts() {
        assert_eq!(
            local_save_script("rivetr-app:abc"),
            "$D save 'rivetr-app:abc' | gzip -1"
        );
        assert_eq!(REMOTE_LOAD_SCRIPT, "gunzip -c | $D load");
    }

    #[test]
    fn ssh_args_for_key_and_password() {
        let a = ssh_args(
            "10.0.0.2",
            2222,
            "deploy",
            Some("/tmp/k"),
            false,
            "UserKnownHostsFile=/x",
        );
        assert_eq!(a.last().unwrap(), "deploy@10.0.0.2");
        assert!(a.windows(2).any(|w| w == ["-p", "2222"]));
        assert!(a.windows(2).any(|w| w == ["-i", "/tmp/k"]));
        assert!(a.contains(&"BatchMode=yes".to_string()));
        let a = ssh_args("h", 22, "root", None, true, "UserKnownHostsFile=/x");
        assert!(!a.contains(&"BatchMode=yes".to_string()));
        assert!(!a.contains(&"-i".to_string()));
    }

    #[test]
    fn env_file_skips_unsafe_entries() {
        let (content, skipped) = env_file(&[
            ("A".into(), "1".into()),
            ("URL".into(), "postgres://u:p@h/db?x=1".into()),
            ("CERT".into(), "line1\nline2".into()),
            ("BAD KEY".into(), "x".into()),
            ("".into(), "x".into()),
        ]);
        assert_eq!(content, "A=1\nURL=postgres://u:p@h/db?x=1\n");
        assert_eq!(skipped, ["CERT", "BAD KEY", ""]);
    }

    #[test]
    fn memory_values_are_normalised_for_the_cli() {
        assert_eq!(cli_memory("512mb").as_deref(), Some("512m"));
        assert_eq!(cli_memory("1GB").as_deref(), Some("1g"));
        assert_eq!(cli_memory("256m").as_deref(), Some("256m"));
        assert_eq!(cli_memory("1048576").as_deref(), Some("1048576"));
        assert_eq!(cli_memory("lots"), None);
        assert_eq!(cli_memory(""), None);
    }

    #[test]
    fn run_script_publishes_on_the_chosen_interface_and_hides_env() {
        let mut labels = BTreeMap::new();
        labels.insert("rivetr.app_id".to_string(), "a1".to_string());
        let spec = RunSpec {
            name: "rivetr-api-ms-local-0-abc".into(),
            image: "rivetr-api:abc".into(),
            app_port: 3000,
            bind: Some(BindAddr::Ip("100.64.0.7".into())),
            memory_limit: Some("512mb".into()),
            cpu_limit: Some("0.5".into()),
            restart_policy: "never".into(),
            labels,
            volumes: vec!["/data/api:/data".into()],
            ..Default::default()
        };
        let s = run_script(&spec);
        assert!(s.contains("--env-file \"$ENVF\""));
        assert!(s.contains("-p '100.64.0.7::3000'"));
        assert!(s.contains("--memory '512m'"));
        assert!(s.contains("--cpus '0.5'"));
        assert!(s.contains("--restart no"));
        assert!(s.contains("--label 'rivetr.app_id=a1'"));
        assert!(s.contains("-v '/data/api:/data'"));
        assert!(s.contains("$D port 'rivetr-api-ms-local-0-abc' 3000/tcp"));
        assert!(s.contains("trap 'rm -f \"$ENVF\"' EXIT"));

        let spec = RunSpec {
            bind: Some(BindAddr::TailscaleLookup),
            ..spec.clone()
        };
        let s = run_script(&spec);
        assert!(s.contains("tailscale ip -4"));
        assert!(s.contains("-p \"$BIND::3000\""));

        let spec = RunSpec {
            bind: Some(BindAddr::All),
            cpu_limit: Some("bogus".into()),
            ..spec
        };
        let s = run_script(&spec);
        assert!(s.contains("-p 3000 "));
        assert!(!s.contains("--cpus"));
    }

    #[test]
    fn run_output_parsing() {
        let out = "3f2a9c\n100.64.0.7:49153\n";
        assert_eq!(parse_run_output(out), Some(("3f2a9c".into(), 49153)));
        let out = "abc\n0.0.0.0:32768\n[::]:32768\n";
        assert_eq!(parse_run_output(out), Some(("abc".into(), 32768)));
        assert_eq!(parse_run_output("abc\n"), None);
        assert_eq!(parse_run_output(""), None);
    }

    #[test]
    fn remove_and_exec_scripts_quote_names() {
        let s = remove_script(&["a b".to_string()]);
        assert!(s.contains("$D stop -t 10 'a b'"));
        assert!(s.contains("$D rm -f 'a b'"));
        assert_eq!(
            exec_check_script("c1", "curl -f localhost:3000/health"),
            "$D exec 'c1' sh -c 'curl -f localhost:3000/health'"
        );
    }
}
