use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use super::{BuildContext, CommandResult, ContainerInfo, ContainerRuntime, ContainerStats, ExecConfig, ExecHandle, LogLine, LogStream, RunConfig, TtySize};

pub struct PodmanRuntime;

impl PodmanRuntime {
    pub fn new() -> Self {
        Self
    }

    async fn run_command(&self, args: &[String]) -> Result<String> {
        let output = Command::new("podman")
            .args(args)
            .output()
            .await
            .context("Failed to execute podman command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Podman command failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[async_trait]
impl ContainerRuntime for PodmanRuntime {
    async fn build(&self, ctx: &BuildContext) -> Result<String> {
        let dockerfile = ctx.dockerfile.trim_start_matches("./");

        let mut args = vec![
            "build".to_string(),
            "-t".to_string(),
            ctx.tag.clone(),
            "-f".to_string(),
            dockerfile.to_string(),
        ];

        // Add build target if specified (multi-stage builds)
        if let Some(ref target) = ctx.build_target {
            if !target.is_empty() {
                args.push("--target".to_string());
                args.push(target.clone());
            }
        }

        // Parse and add custom options
        if let Some(ref options) = ctx.custom_options {
            parse_podman_custom_options(options, &mut args);
        }

        for (key, value) in &ctx.build_args {
            args.push("--build-arg".to_string());
            args.push(format!("{}={}", key, value));
        }

        args.push(ctx.path.clone());

        self.run_command(&args).await?;
        Ok(ctx.tag.clone())
    }

    async fn run(&self, config: &RunConfig) -> Result<String> {
        let mut args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(),
            config.name.clone(),
        ];

        // Add primary port mapping (let podman auto-assign host port)
        args.push("-p".to_string());
        args.push(format!(":{}", config.port));

        // Add additional port mappings
        for mapping in &config.port_mappings {
            args.push("-p".to_string());
            if mapping.host_port == 0 {
                // Auto-assign host port
                args.push(format!(":{}/{}", mapping.container_port, mapping.protocol));
            } else {
                // Use specified host port
                args.push(format!(
                    "{}:{}/{}",
                    mapping.host_port, mapping.container_port, mapping.protocol
                ));
            }
        }

        // Add extra hosts (--add-host flag)
        for extra_host in &config.extra_hosts {
            args.push("--add-host".to_string());
            args.push(extra_host.clone());
        }

        // Add network aliases if provided
        // Note: Full network alias support requires creating/joining a podman network
        for alias in &config.network_aliases {
            args.push("--network-alias".to_string());
            args.push(alias.clone());
        }

        for (key, value) in &config.env {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, value));
        }

        if let Some(mem) = &config.memory_limit {
            args.push("-m".to_string());
            args.push(mem.clone());
        }

        if let Some(cpu) = &config.cpu_limit {
            args.push("--cpus".to_string());
            args.push(cpu.clone());
        }

        args.push(config.image.clone());

        self.run_command(&args).await
    }

    async fn start(&self, container_id: &str) -> Result<()> {
        self.run_command(&["start".to_string(), container_id.to_string()])
            .await?;
        Ok(())
    }

    async fn stop(&self, container_id: &str) -> Result<()> {
        self.run_command(&["stop".to_string(), container_id.to_string()])
            .await?;
        Ok(())
    }

    async fn remove(&self, container_id: &str) -> Result<()> {
        self.run_command(&["rm".to_string(), "-f".to_string(), container_id.to_string()])
            .await?;
        Ok(())
    }

    async fn logs(
        &self,
        container_id: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>> {
        let mut child = Command::new("podman")
            .args(["logs", "-f", container_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn podman logs")?;

        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);
        let lines = reader.lines();

        let stream = async_stream::stream! {
            let mut lines = lines;
            while let Ok(Some(line)) = lines.next_line().await {
                yield LogLine {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    message: line,
                    stream: LogStream::Stdout,
                };
            }
        };

        Ok(Box::pin(stream))
    }

    async fn inspect(&self, container_id: &str) -> Result<ContainerInfo> {
        let output = self
            .run_command(&[
                "inspect".to_string(),
                "--format".to_string(),
                "{{.Id}}|{{.Name}}|{{.State.Status}}|{{.State.Running}}|{{range $p, $conf := .NetworkSettings.Ports}}{{range $conf}}{{.HostPort}}{{end}}{{end}}".to_string(),
                container_id.to_string(),
            ])
            .await?;

        let parts: Vec<&str> = output.split('|').collect();
        if parts.len() >= 4 {
            let is_running = parts.get(3).map(|s| *s == "true").unwrap_or(false);
            let host_port = parts.get(4)
                .and_then(|s| s.parse::<u16>().ok());

            Ok(ContainerInfo {
                id: parts[0].to_string(),
                name: parts[1].to_string(),
                status: parts[2].to_string(),
                port: host_port,
                running: is_running,
                host_port,
            })
        } else {
            anyhow::bail!("Invalid inspect output")
        }
    }

    async fn is_available(&self) -> bool {
        Command::new("podman")
            .arg("--version")
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn list_containers(&self, name_prefix: &str) -> Result<Vec<ContainerInfo>> {
        let output = self
            .run_command(&[
                "ps".to_string(),
                "--filter".to_string(),
                format!("name={}", name_prefix),
                "--format".to_string(),
                "{{.ID}}|{{.Names}}|{{.State}}|{{.Ports}}".to_string(),
            ])
            .await?;

        let mut result = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                // Parse port from format like "0.0.0.0:32768->3000/tcp"
                let port = parts.get(3).and_then(|ports| {
                    ports
                        .split("->")
                        .next()
                        .and_then(|host_part| host_part.split(':').last())
                        .and_then(|p| p.parse().ok())
                });

                // In ps output, running containers have "running" state
                let is_running = parts[2].to_lowercase() == "running";

                result.push(ContainerInfo {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    status: parts[2].to_string(),
                    port,
                    running: is_running,
                    host_port: port,
                });
            }
        }

        Ok(result)
    }

    async fn stats(&self, container_id: &str) -> Result<ContainerStats> {
        // Use podman stats with JSON output for reliable parsing
        let output = Command::new("podman")
            .args([
                "stats",
                "--no-stream",
                "--format",
                "json",
                container_id,
            ])
            .output()
            .await
            .context("Failed to execute podman stats")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Podman stats failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Podman stats returns an array of stats objects
        let stats: Vec<serde_json::Value> = serde_json::from_str(&stdout)
            .context("Failed to parse podman stats JSON")?;

        if stats.is_empty() {
            anyhow::bail!("No stats returned for container");
        }

        let stat = &stats[0];

        // Parse CPU percentage (e.g., "5.23%")
        let cpu_percent = stat
            .get("cpu_percent")
            .and_then(|v| v.as_str())
            .and_then(|s| s.trim_end_matches('%').parse::<f64>().ok())
            .or_else(|| {
                // Fallback: try getting it as a number directly
                stat.get("CPUPerc")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.trim_end_matches('%').parse::<f64>().ok())
            })
            .unwrap_or(0.0);

        // Parse memory usage and limit
        // Podman returns mem_usage like "128.5MiB / 512MiB"
        let (memory_usage, memory_limit) = stat
            .get("mem_usage")
            .and_then(|v| v.as_str())
            .map(|s| {
                let parts: Vec<&str> = s.split('/').collect();
                let usage = parts.first().map(|p| parse_podman_size(p.trim())).unwrap_or(0);
                let limit = parts.get(1).map(|p| parse_podman_size(p.trim())).unwrap_or(0);
                (usage, limit)
            })
            .or_else(|| {
                // Alternative field names
                stat.get("MemUsage").and_then(|v| v.as_str()).map(|s| {
                    let parts: Vec<&str> = s.split('/').collect();
                    let usage = parts.first().map(|p| parse_podman_size(p.trim())).unwrap_or(0);
                    let limit = parts.get(1).map(|p| parse_podman_size(p.trim())).unwrap_or(0);
                    (usage, limit)
                })
            })
            .unwrap_or((0, 0));

        // Parse network I/O (e.g., "648kB / 0B")
        let (network_rx, network_tx) = stat
            .get("net_io")
            .and_then(|v| v.as_str())
            .map(|s| {
                let parts: Vec<&str> = s.split('/').collect();
                let rx = parts.first().map(|p| parse_podman_size(p.trim())).unwrap_or(0);
                let tx = parts.get(1).map(|p| parse_podman_size(p.trim())).unwrap_or(0);
                (rx, tx)
            })
            .or_else(|| {
                stat.get("NetIO").and_then(|v| v.as_str()).map(|s| {
                    let parts: Vec<&str> = s.split('/').collect();
                    let rx = parts.first().map(|p| parse_podman_size(p.trim())).unwrap_or(0);
                    let tx = parts.get(1).map(|p| parse_podman_size(p.trim())).unwrap_or(0);
                    (rx, tx)
                })
            })
            .unwrap_or((0, 0));

        Ok(ContainerStats {
            cpu_percent,
            memory_usage,
            memory_limit,
            network_rx,
            network_tx,
        })
    }

    async fn remove_image(&self, image: &str) -> Result<()> {
        self.run_command(&["rmi".to_string(), "-f".to_string(), image.to_string()])
            .await?;
        Ok(())
    }

    async fn prune_images(&self) -> Result<u64> {
        // Podman image prune returns the IDs of pruned images
        // We can't easily get space reclaimed from CLI, so return 0
        let output = Command::new("podman")
            .args(["image", "prune", "-f", "--filter", "dangling=true"])
            .output()
            .await
            .context("Failed to execute podman image prune")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Podman image prune failed: {}", stderr);
        }

        // Count lines in output to report number of images pruned
        let stdout = String::from_utf8_lossy(&output.stdout);
        let pruned_count = stdout.lines().filter(|l| !l.is_empty()).count();
        tracing::debug!("Pruned {} images with podman", pruned_count);

        // We can't easily get space reclaimed from podman CLI
        Ok(0)
    }

    async fn exec(&self, config: &ExecConfig) -> Result<ExecHandle> {
        // Build podman exec command
        let mut args = vec!["exec".to_string(), "-i".to_string()];

        if config.tty {
            args.push("-t".to_string());
        }

        args.push(config.container_id.clone());
        args.extend(config.cmd.clone());

        // Spawn podman exec process with stdin/stdout piped
        let mut child = Command::new("podman")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn podman exec")?;

        let mut stdin = child.stdin.take().context("Failed to get stdin")?;
        let mut stdout = child.stdout.take().context("Failed to get stdout")?;
        let stderr = child.stderr.take().context("Failed to get stderr")?;

        // Create channels for bidirectional communication
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<Bytes>(32);
        let (stdout_tx, stdout_rx) = mpsc::channel::<Bytes>(32);
        let (resize_tx, mut resize_rx) = mpsc::channel::<TtySize>(8);

        // Spawn stdin writer task
        tokio::spawn(async move {
            while let Some(data) = stdin_rx.recv().await {
                if stdin.write_all(&data).await.is_err() {
                    break;
                }
                if stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        // Spawn stdout reader task
        let stdout_tx_clone = stdout_tx.clone();
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match stdout.read(&mut buf).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if stdout_tx_clone.send(Bytes::copy_from_slice(&buf[..n])).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Podman exec stdout error: {}", e);
                        break;
                    }
                }
            }
        });

        // Spawn stderr reader task (merge into stdout channel)
        let mut stderr_reader = BufReader::new(stderr);
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match stderr_reader.read(&mut buf).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if stdout_tx.send(Bytes::copy_from_slice(&buf[..n])).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Podman exec stderr error: {}", e);
                        break;
                    }
                }
            }
        });

        // Spawn resize handler - Note: podman doesn't support resize via CLI easily
        // For proper TTY resize with podman, we'd need podman-remote or API
        let container_id = config.container_id.clone();
        tokio::spawn(async move {
            while let Some(size) = resize_rx.recv().await {
                // Podman doesn't have a direct CLI command for resize
                // This would require using the podman API
                tracing::debug!(
                    "Resize request for container {} to {}x{} (not supported in CLI mode)",
                    container_id,
                    size.cols,
                    size.rows
                );
            }
        });

        // Spawn task to wait for child and clean up
        tokio::spawn(async move {
            let _ = child.wait().await;
        });

        Ok(ExecHandle {
            stdin_tx,
            stdout_rx,
            resize_tx,
        })
    }

    async fn run_command(&self, container_id: &str, cmd: Vec<String>) -> Result<CommandResult> {
        // Build podman exec command
        let mut args = vec!["exec".to_string(), container_id.to_string()];
        args.extend(cmd);

        let output = Command::new("podman")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute podman exec")?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CommandResult {
            exit_code,
            stdout,
            stderr,
        })
    }
}

/// Parse size strings like "128.5MiB", "512MB", "1.2GiB", "648kB"
fn parse_podman_size(s: &str) -> u64 {
    let s = s.trim();

    // Try to find where the number ends and unit begins
    let (num_str, unit) = s
        .chars()
        .position(|c| c.is_alphabetic())
        .map(|i| s.split_at(i))
        .unwrap_or((s, ""));

    let num: f64 = num_str.trim().parse().unwrap_or(0.0);
    let unit = unit.to_lowercase();

    let multiplier: f64 = match unit.as_str() {
        "b" => 1.0,
        "kb" | "kib" => 1024.0,
        "mb" | "mib" => 1024.0 * 1024.0,
        "gb" | "gib" => 1024.0 * 1024.0 * 1024.0,
        "tb" | "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };

    (num * multiplier) as u64
}

/// Parse custom docker options string into podman build arguments
/// Supports: --no-cache, --add-host
fn parse_podman_custom_options(options: &str, args: &mut Vec<String>) {
    // Parse --no-cache
    if options.contains("--no-cache") {
        args.push("--no-cache".to_string());
    }

    // Parse --add-host (format: --add-host=host:ip)
    for part in options.split_whitespace() {
        if part.starts_with("--add-host=") {
            args.push(part.to_string());
        } else if part == "--add-host" {
            // Handle --add-host host:ip format (two-part)
            // Note: This won't work well with split_whitespace, but we try
            args.push(part.to_string());
        }
    }

    // Parse --build-arg from custom options (format: --build-arg KEY=VALUE)
    let mut iter = options.split_whitespace().peekable();
    while let Some(part) = iter.next() {
        if part == "--build-arg" {
            if let Some(arg) = iter.next() {
                args.push("--build-arg".to_string());
                args.push(arg.to_string());
            }
        } else if part.starts_with("--build-arg=") {
            args.push(part.to_string());
        }
    }
}
