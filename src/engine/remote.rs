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
    /// Plain-text SSH password (used when no key is available, via sshpass).
    pub ssh_password: Option<String>,
}

impl RemoteContext {
    /// Run an arbitrary shell command on the remote server via SSH.
    ///
    /// Returns `(stdout, stderr)` on success. The call succeeds even when the
    /// remote command exits with a non-zero status code; callers should inspect
    /// stderr if needed.
    ///
    /// When `ssh_password` is set and no `key_path` is available, `sshpass` is
    /// used to supply the password non-interactively.
    pub async fn run_command(&self, cmd: &str) -> Result<(String, String)> {
        let port_str = self.port.to_string();
        let target = format!("{}@{}", self.username, self.host);

        // Use sshpass for password authentication when no key is available
        let use_sshpass = self.ssh_password.is_some() && self.key_path.is_none();

        let mut command = if use_sshpass {
            let mut c = Command::new("sshpass");
            c.arg("-p").arg(self.ssh_password.as_deref().unwrap());
            c.arg("ssh");
            c
        } else {
            Command::new("ssh")
        };

        command
            .arg("-o")
            .arg("StrictHostKeyChecking=no")
            .arg("-o")
            .arg("ConnectTimeout=10");

        // BatchMode=yes disables interactive password prompts; skip it when using sshpass
        if !use_sshpass {
            command.arg("-o").arg("BatchMode=yes");
        }

        command.arg("-p").arg(&port_str);

        if let Some(ref key) = self.key_path {
            command.arg("-i").arg(key.as_str());
        }

        command.arg(&target).arg(cmd);

        let output = command
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
