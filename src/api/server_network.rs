//! Network connectivity check for a managed server, including Tailscale.
//!
//! A server can be reached over its public host or over Tailscale (an IP or
//! MagicDNS name in `tailscale_host`, migration 131). Rivetr does not manage
//! tailnet keys or run tailscaled itself: the Rivetr host must already be
//! logged in to the tailnet (see docs/reference/remote-servers.md).
//!
//! `POST /api/servers/{id}/network-check` reports:
//! - whether the `tailscale` CLI is installed on the Rivetr host and connected,
//! - `tailscale ping` to the server (Tailscale mode only),
//! - a TCP connect to the SSH port on the address Rivetr will use.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use tokio::process::Command;

use crate::db::User;
use crate::AppState;

use super::authz;
use super::error::ApiError;

#[derive(Debug, Serialize, PartialEq)]
pub struct NetworkCheckItem {
    pub name: String,
    /// `pass`, `fail` or `skip`.
    pub status: &'static str,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct NetworkCheckResponse {
    pub network_mode: String,
    /// Address used for SSH.
    pub address: String,
    pub ok: bool,
    pub checks: Vec<NetworkCheckItem>,
}

fn item(name: &str, status: &'static str, detail: impl Into<String>) -> NetworkCheckItem {
    NetworkCheckItem {
        name: name.to_string(),
        status,
        detail: detail.into(),
    }
}

/// Summarise `tailscale status --json` for the local node.
fn parse_tailscale_status(json: &str) -> NetworkCheckItem {
    let name = "Tailscale on the Rivetr host";
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return item(name, "fail", "Could not read `tailscale status --json`"),
    };
    let state = v["BackendState"].as_str().unwrap_or("unknown");
    if state != "Running" {
        return item(
            name,
            "fail",
            format!("tailscaled is {state}. Run `sudo tailscale up` on the Rivetr host"),
        );
    }
    let dns = v["Self"]["DNSName"]
        .as_str()
        .unwrap_or("")
        .trim_end_matches('.');
    let ips: Vec<&str> = v["Self"]["TailscaleIPs"]
        .as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default();
    item(
        name,
        "pass",
        format!("Connected as {} ({})", dns, ips.join(", ")),
    )
}

/// Interpret `tailscale ping` output.
fn parse_tailscale_ping(success: bool, stdout: &str, stderr: &str) -> NetworkCheckItem {
    let name = "Tailscale ping";
    let pong = stdout.lines().find(|l| l.contains("pong from"));
    match (success, pong) {
        (true, Some(line)) => item(name, "pass", line.trim()),
        _ => {
            let msg = stderr
                .lines()
                .chain(stdout.lines())
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or("no reply");
            item(
                name,
                "fail",
                format!("{msg}. Check the server is on the same tailnet and ACLs allow it"),
            )
        }
    }
}

async fn run(cmd: &str, args: &[&str], timeout: Duration) -> Option<std::process::Output> {
    let fut = Command::new(cmd).args(args).kill_on_drop(true).output();
    match tokio::time::timeout(timeout, fut).await {
        Ok(Ok(out)) => Some(out),
        _ => None,
    }
}

/// POST /api/servers/{id}/network-check
pub async fn check_server_network(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<NetworkCheckResponse>, ApiError> {
    let server = authz::authorize_server(&state, &user, &id).await?;
    let address = server.ssh_host().to_string();
    let tailscale_mode = server.network_mode == "tailscale";
    let mut checks = Vec::new();

    // 1. Local tailscaled
    match run("tailscale", &["status", "--json"], Duration::from_secs(5)).await {
        Some(out) if out.status.success() => checks.push(parse_tailscale_status(
            &String::from_utf8_lossy(&out.stdout),
        )),
        Some(out) => checks.push(item(
            "Tailscale on the Rivetr host",
            if tailscale_mode { "fail" } else { "skip" },
            format!(
                "tailscale status failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        )),
        None => checks.push(item(
            "Tailscale on the Rivetr host",
            if tailscale_mode { "fail" } else { "skip" },
            "The tailscale CLI is not installed on the Rivetr host",
        )),
    }

    // 2. tailscale ping (Tailscale mode only; the address is validated on save)
    if tailscale_mode && !address.starts_with('-') {
        match run(
            "tailscale",
            &["ping", "-c", "1", "--timeout", "5s", &address],
            Duration::from_secs(10),
        )
        .await
        {
            Some(out) => checks.push(parse_tailscale_ping(
                out.status.success(),
                &String::from_utf8_lossy(&out.stdout),
                &String::from_utf8_lossy(&out.stderr),
            )),
            None => checks.push(item("Tailscale ping", "fail", "tailscale ping timed out")),
        }
    }

    // 3. SSH port reachable on the address Rivetr will use
    let target = if address.contains(':') && !address.starts_with('[') {
        format!("[{}]:{}", address, server.port)
    } else {
        format!("{}:{}", address, server.port)
    };
    let tcp = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::net::TcpStream::connect(&target),
    )
    .await;
    checks.push(match tcp {
        Ok(Ok(_)) => item(
            "SSH port reachable",
            "pass",
            format!("Connected to {target}"),
        ),
        Ok(Err(e)) => item("SSH port reachable", "fail", format!("{target}: {e}")),
        Err(_) => item(
            "SSH port reachable",
            "fail",
            format!("{target}: timed out after 5s"),
        ),
    });

    let ok = checks.iter().all(|c| c.status != "fail");
    Ok(Json(NetworkCheckResponse {
        network_mode: server.network_mode.clone(),
        address,
        ok,
        checks,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_running() {
        let json = r#"{"BackendState":"Running","Self":{"DNSName":"rivetr.tail1234.ts.net.","TailscaleIPs":["100.64.0.1","fd7a::1"]}}"#;
        let it = parse_tailscale_status(json);
        assert_eq!(it.status, "pass");
        assert_eq!(
            it.detail,
            "Connected as rivetr.tail1234.ts.net (100.64.0.1, fd7a::1)"
        );
    }

    #[test]
    fn status_stopped_or_garbage() {
        let it = parse_tailscale_status(r#"{"BackendState":"NeedsLogin"}"#);
        assert_eq!(it.status, "fail");
        assert!(it.detail.contains("NeedsLogin"));
        assert_eq!(parse_tailscale_status("not json").status, "fail");
    }

    #[test]
    fn ping_output() {
        let ok = parse_tailscale_ping(
            true,
            "pong from web-1 (100.64.0.2) via DERP(fra) in 31ms\n",
            "",
        );
        assert_eq!(ok.status, "pass");
        assert!(ok.detail.starts_with("pong from web-1"));

        let fail = parse_tailscale_ping(false, "", "no matching peer\n");
        assert_eq!(fail.status, "fail");
        assert!(fail.detail.starts_with("no matching peer"));
    }
}
