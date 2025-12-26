use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::{BuildContext, ContainerInfo, ContainerRuntime, ContainerStats, LogLine, LogStream, RunConfig};

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
            "-p".to_string(),
            format!(":{}", config.port),
        ];

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
                "{{.Id}}|{{.Name}}|{{.State.Status}}".to_string(),
                container_id.to_string(),
            ])
            .await?;

        let parts: Vec<&str> = output.split('|').collect();
        if parts.len() >= 3 {
            Ok(ContainerInfo {
                id: parts[0].to_string(),
                name: parts[1].to_string(),
                status: parts[2].to_string(),
                port: None, // Would need additional parsing
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

                result.push(ContainerInfo {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    status: parts[2].to_string(),
                    port,
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
