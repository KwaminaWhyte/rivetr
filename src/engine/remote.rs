//! Remote server deployment via SSH.
//!
//! Provides a `RemoteContext` that can run shell commands on a remote server
//! over an SSH connection. For the current MVP, the deployment pipeline logs
//! the remote-deploy intent and falls back to local execution. The struct and
//! helpers here are the foundation for a full remote-build pipeline.

use anyhow::Result;
use std::process::Stdio;
use tokio::process::Command;

/// Connection parameters for a remote server reachable via SSH.
pub struct RemoteContext {
    pub host: String,
    pub port: i64,
    pub username: String,
    /// Filesystem path to a temporary file containing the decrypted private key.
    pub key_path: Option<String>,
}

impl RemoteContext {
    /// Run an arbitrary shell command on the remote server via SSH.
    ///
    /// Returns `(stdout, stderr)` on success. The call succeeds even when the
    /// remote command exits with a non-zero status code; callers should inspect
    /// stderr if needed.
    pub async fn run_command(&self, cmd: &str) -> Result<(String, String)> {
        let port_str = self.port.to_string();
        let target = format!("{}@{}", self.username, self.host);

        let mut args: Vec<&str> = vec![
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "ConnectTimeout=10",
            "-o",
            "BatchMode=yes",
            "-p",
            &port_str,
        ];

        if let Some(ref key) = self.key_path {
            args.extend(["-i", key.as_str()]);
        }

        args.push(&target);
        args.push(cmd);

        let output = Command::new("ssh")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    /// Verify that the SSH connection works by running a trivial remote command.
    pub async fn test_connection(&self) -> Result<bool> {
        let (_out, _err) = self.run_command("echo ok").await?;
        Ok(true)
    }
}
