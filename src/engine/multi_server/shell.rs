//! Runs multi-server deployment scripts on a target: locally through `sh`, or
//! remotely over SSH, optionally feeding stdin (env files, registry passwords)
//! or a locally produced stream (`docker save`).

use std::io::Write;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::engine::server_ops::{ExecOutput, HostTarget};

use super::distribute::ssh_args;

fn to_output(o: std::process::Output) -> ExecOutput {
    ExecOutput {
        code: o.status.code(),
        stdout: String::from_utf8_lossy(&o.stdout).to_string(),
        stderr: String::from_utf8_lossy(&o.stderr).to_string(),
    }
}

/// A private key written to a 0600 temp file for the lifetime of one command.
fn key_file(private_key: Option<&str>) -> Result<Option<tempfile::NamedTempFile>> {
    let Some(key) = private_key else {
        return Ok(None);
    };
    let mut f = tempfile::Builder::new()
        .prefix("rivetr-ssh-key-")
        .suffix(".pem")
        .tempfile()?;
    f.write_all(key.as_bytes())?;
    if !key.ends_with('\n') {
        f.write_all(b"\n")?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(Some(f))
}

/// Build the process for `script` on `target` (prelude included). The returned
/// temp file must outlive the process.
fn command_for(
    target: &HostTarget,
    script: &str,
) -> Result<(Command, Option<tempfile::NamedTempFile>)> {
    let full = format!("{}\n{}", target.prelude(), script);
    match target {
        HostTarget::Local { .. } => {
            let mut c = Command::new("sh");
            c.arg("-c").arg(full);
            Ok((c, None))
        }
        HostTarget::Remote {
            host,
            port,
            username,
            private_key,
            password,
        } => {
            let key = key_file(private_key.as_deref())?;
            let use_password = password.is_some() && key.is_none();
            let key_path = key.as_ref().map(|k| k.path().display().to_string());
            let args = ssh_args(
                host,
                *port,
                username,
                key_path.as_deref(),
                use_password,
                &crate::utils::ssh::known_hosts_option(),
            );
            let mut c = if use_password {
                let mut c = Command::new("sshpass");
                c.env("SSHPASS", password.clone().unwrap_or_default());
                c.arg("-e").arg("ssh");
                c
            } else {
                Command::new("ssh")
            };
            c.args(args).arg(format!(
                "sh -c {}",
                crate::engine::server_ops::shell_quote(&full)
            ));
            Ok((c, key))
        }
    }
}

/// Run `script` on `target`, writing `stdin` to it when given.
pub async fn exec(
    target: &HostTarget,
    script: &str,
    stdin: Option<&[u8]>,
    timeout: Duration,
) -> Result<ExecOutput> {
    let (mut cmd, _key) = command_for(target, script)?;
    cmd.stdin(if stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .kill_on_drop(true);
    let fut = async {
        let mut child = cmd.spawn().context("failed to spawn shell")?;
        if let (Some(bytes), Some(mut pipe)) = (stdin, child.stdin.take()) {
            pipe.write_all(bytes)
                .await
                .context("failed to write stdin")?;
            drop(pipe);
        }
        let out = child.wait_with_output().await?;
        Ok::<_, anyhow::Error>(to_output(out))
    };
    let out = match tokio::time::timeout(timeout, fut).await {
        Ok(res) => res?,
        Err(_) => anyhow::bail!("command timed out after {}s", timeout.as_secs()),
    };
    if !target.is_local() && out.code == Some(255) {
        anyhow::bail!("SSH connection failed: {}", out.stderr.trim());
    }
    Ok(out)
}

/// Pipe the stdout of `local_script` (run on the Rivetr host) into
/// `remote_script` on `target`. Used for `docker save | ssh docker load`.
/// Returns the remote output; a local failure is reported in its stderr.
pub async fn pipe_to(
    local: &HostTarget,
    local_script: &str,
    target: &HostTarget,
    remote_script: &str,
    timeout: Duration,
) -> Result<ExecOutput> {
    let (mut producer, _k1) = command_for(local, local_script)?;
    let (mut consumer, _k2) = command_for(target, remote_script)?;
    let fut = async {
        let mut prod = producer
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("failed to spawn image save")?;
        let prod_out: Stdio = prod
            .stdout
            .take()
            .context("image save has no stdout")?
            .try_into()
            .context("failed to hand image stream to ssh")?;
        let cons = consumer
            .stdin(prod_out)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("failed to spawn ssh for image load")?;
        let (prod_res, cons_res) = tokio::join!(prod.wait_with_output(), cons.wait_with_output());
        let prod_res = prod_res?;
        let mut out = to_output(cons_res?);
        if !prod_res.status.success() {
            out.code = out.code.filter(|c| *c != 0).or(Some(1));
            out.stderr = format!(
                "{}\nimage save failed: {}",
                out.stderr,
                String::from_utf8_lossy(&prod_res.stderr).trim()
            );
        }
        Ok::<_, anyhow::Error>(out)
    };
    match tokio::time::timeout(timeout, fut).await {
        Ok(res) => res,
        Err(_) => anyhow::bail!("image transfer timed out after {}s", timeout.as_secs()),
    }
}
