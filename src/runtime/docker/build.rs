use anyhow::{Context, Result};
use bollard::image::BuildImageOptions;
use bytes::Bytes;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;

use crate::runtime::BuildContext;

use super::DockerRuntime;

/// Parsed custom build arguments from custom_docker_options string
#[derive(Default)]
pub(super) struct CustomBuildArgs {
    pub no_cache: bool,
    pub extra_hosts: Option<String>,
}

/// Parse custom docker options string into build arguments.
/// Supports: --no-cache, --add-host
pub(super) fn parse_custom_build_args(options: Option<&str>) -> CustomBuildArgs {
    let mut args = CustomBuildArgs::default();

    let Some(opts) = options else {
        return args;
    };

    // Parse --no-cache
    if opts.contains("--no-cache") {
        args.no_cache = true;
    }

    // Parse --add-host (format: --add-host=host:ip or --add-host host:ip)
    let mut extra_hosts = Vec::new();
    for part in opts.split_whitespace() {
        if part.starts_with("--add-host=") {
            if let Some(host) = part.strip_prefix("--add-host=") {
                extra_hosts.push(host.to_string());
            }
        }
    }
    if !extra_hosts.is_empty() {
        args.extra_hosts = Some(extra_hosts.join(","));
    }

    args
}

/// Parse memory limit for Docker build, returns i64 as required by Bollard BuildImageOptions.
pub(super) fn parse_build_memory(s: &str) -> Option<i64> {
    let s = s.to_lowercase();
    if s.ends_with("gb") {
        s.trim_end_matches("gb")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024 * 1024 * 1024)
    } else if s.ends_with("g") {
        s.trim_end_matches("g")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024 * 1024 * 1024)
    } else if s.ends_with("mb") {
        s.trim_end_matches("mb")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024 * 1024)
    } else if s.ends_with("m") {
        s.trim_end_matches("m")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024 * 1024)
    } else if s.ends_with("kb") {
        s.trim_end_matches("kb")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024)
    } else if s.ends_with("k") {
        s.trim_end_matches("k")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024)
    } else if s.ends_with("b") {
        s.trim_end_matches("b").parse().ok()
    } else {
        // Assume raw bytes if no suffix
        s.parse().ok()
    }
}

/// Parse CPU limits for Docker build.
/// Docker build uses cpu-period (default 100000) and cpu-quota to limit CPU.
/// If cpu-quota = cpu-period * num_cpus, then the container can use num_cpus worth of CPU.
pub(super) fn parse_cpu_limits(cpu_limit: Option<&str>) -> (Option<i64>, Option<i64>) {
    let Some(cpu_str) = cpu_limit else {
        return (None, None);
    };

    let cpu_count = cpu_str.parse::<f64>().ok();

    match cpu_count {
        Some(cpus) if cpus > 0.0 => {
            let period: i64 = 100_000; // Default Docker CPU period (100ms)
            let quota = (cpus * period as f64) as i64;
            (Some(period), Some(quota))
        }
        _ => (None, None),
    }
}

pub async fn build(runtime: &DockerRuntime, ctx: &BuildContext) -> Result<String> {
    // When build secrets are present, or when a non-default platform is requested,
    // we must use `docker buildx build` via CLI because the Bollard API does not
    // support BuildKit --secret flags or --platform builds.
    let needs_buildx = !ctx.build_secrets.is_empty()
        || ctx
            .build_platforms
            .as_deref()
            .map(|p| !p.is_empty() && p != "linux/amd64")
            .unwrap_or(false);

    if needs_buildx {
        return build_with_secrets_cli(ctx).await;
    }

    // Standard path: use the Bollard API (no secrets, default platform)
    build_via_bollard(runtime, ctx).await
}

/// Use `docker buildx build` CLI when BuildKit secrets or custom platforms are required.
/// Writes each secret value to a tmpfile, passes `--secret id=KEY,src=TMPFILE`,
/// then cleans up tmpfiles on completion (success or failure).
async fn build_with_secrets_cli(ctx: &BuildContext) -> Result<String> {
    use tokio::process::Command;

    let dockerfile = ctx.dockerfile.trim_start_matches("./");

    let mut args: Vec<String> = vec![
        "buildx".to_string(),
        "build".to_string(),
        "--load".to_string(), // export to local Docker daemon
        "-t".to_string(),
        ctx.tag.clone(),
        "-f".to_string(),
        dockerfile.to_string(),
    ];

    // Inject --platform when a target platform is specified
    if let Some(ref platforms) = ctx.build_platforms {
        if !platforms.is_empty() {
            args.push("--platform".to_string());
            args.push(platforms.clone());
        }
    }

    if let Some(ref target) = ctx.build_target {
        if !target.is_empty() {
            args.push("--target".to_string());
            args.push(target.clone());
        }
    }

    // Resource limits via build-arg shim (buildx uses --memory / --cpu-quota on the daemon)
    if let Some(ref memory) = ctx.memory_limit {
        args.push("--memory".to_string());
        args.push(memory.clone());
    }

    if let Some(ref cpu) = ctx.cpu_limit {
        if let Ok(n) = cpu.parse::<f64>() {
            // Convert CPUs to cpu-quota (period = 100000 µs)
            let quota = (n * 100_000.0) as u64;
            args.push("--cpu-quota".to_string());
            args.push(quota.to_string());
        }
    }

    // Custom options (--no-cache, --add-host, …)
    let extra = parse_custom_build_args(ctx.custom_options.as_deref());
    if extra.no_cache || ctx.no_cache {
        args.push("--no-cache".to_string());
    }
    if let Some(ref hosts) = extra.extra_hosts {
        for h in hosts.split(',') {
            args.push("--add-host".to_string());
            args.push(h.to_string());
        }
    }

    for (key, value) in &ctx.build_args {
        args.push("--build-arg".to_string());
        args.push(format!("{}={}", key, value));
    }

    // Inject SOURCE_COMMIT build arg if requested
    if let Some(ref sha) = ctx.source_commit {
        args.push("--build-arg".to_string());
        args.push(format!("SOURCE_COMMIT={}", sha));
    }

    // Write secrets to tmpfiles
    let tag_safe = ctx.tag.replace([':', '/'], "-");
    let mut secret_tmp_paths: Vec<std::path::PathBuf> = vec![];
    for (key, value) in &ctx.build_secrets {
        let tmp_path = std::path::PathBuf::from(format!("/tmp/rivetr-secret-{}-{}", tag_safe, key));
        let mut f = tokio::fs::File::create(&tmp_path)
            .await
            .context(format!("Failed to create secret tmpfile for '{}'", key))?;
        f.write_all(value.as_bytes())
            .await
            .context(format!("Failed to write secret tmpfile for '{}'", key))?;
        args.push("--secret".to_string());
        args.push(format!("id={},src={}", key, tmp_path.display()));
        secret_tmp_paths.push(tmp_path);
    }

    args.push(ctx.path.clone());

    tracing::info!(
        secrets = ?ctx.build_secrets.iter().map(|(k,_)| k.as_str()).collect::<Vec<_>>(),
        "Building image with BuildKit secrets via CLI"
    );

    let output = Command::new("docker")
        .args(&args)
        .env("DOCKER_BUILDKIT", "1")
        .output()
        .await
        .context("Failed to spawn docker buildx build")?;

    // Clean up secret tmpfiles regardless of outcome
    for tmp_path in &secret_tmp_paths {
        if let Err(e) = tokio::fs::remove_file(tmp_path).await {
            tracing::warn!("Failed to remove secret tmpfile {:?}: {}", tmp_path, e);
        }
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Forward output to log_tx if available
        if let Some(ref tx) = ctx.log_tx {
            for line in stdout.lines().chain(stderr.lines()) {
                let _ = tx.send(line.to_string());
            }
        }
        anyhow::bail!("docker buildx build failed:\n{}", stderr);
    }

    // Forward stdout to log_tx
    if let Some(ref tx) = ctx.log_tx {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let _ = tx.send(line.to_string());
        }
    }

    Ok(ctx.tag.clone())
}

/// Standard Bollard-based build (no secrets).
async fn build_via_bollard(runtime: &DockerRuntime, ctx: &BuildContext) -> Result<String> {
    // Create a tar archive of the build context
    let tar_path = format!("{}.tar", ctx.path);
    let tar_file = std::fs::File::create(&tar_path)?;
    let mut tar_builder = tar::Builder::new(tar_file);
    tar_builder.append_dir_all(".", &ctx.path)?;
    tar_builder.finish()?;

    let tar_data = std::fs::read(&tar_path)?;
    std::fs::remove_file(&tar_path)?;

    // Parse custom options for build args
    let extra_build_args = parse_custom_build_args(ctx.custom_options.as_deref());

    // Parse build resource limits
    let memory = ctx
        .memory_limit
        .as_ref()
        .and_then(|m| parse_build_memory(m));
    let (cpuperiod, cpuquota) = parse_cpu_limits(ctx.cpu_limit.as_deref());

    let target = ctx.build_target.as_deref().unwrap_or("");

    // Build args: start with ctx.build_args, then inject SOURCE_COMMIT if requested
    let mut build_args_map: std::collections::HashMap<&str, &str> = ctx
        .build_args
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    if let Some(ref sha) = ctx.source_commit {
        build_args_map.insert("SOURCE_COMMIT", sha.as_str());
    }

    let options = BuildImageOptions {
        dockerfile: ctx.dockerfile.trim_start_matches("./"),
        t: &ctx.tag,
        rm: true,
        target,
        extrahosts: extra_build_args.extra_hosts.as_deref(),
        nocache: extra_build_args.no_cache || ctx.no_cache,
        memory: memory.map(|m| m as u64),
        memswap: memory, // Set memswap equal to memory to disable swap
        cpuperiod: cpuperiod.map(|p| p as u64),
        cpuquota: cpuquota.map(|q| q as u64),
        buildargs: build_args_map,
        ..Default::default()
    };

    if memory.is_some() || cpuquota.is_some() {
        tracing::info!(
            memory = ?memory,
            cpuperiod = ?cpuperiod,
            cpuquota = ?cpuquota,
            "Building image with resource limits"
        );
    }

    let mut stream = runtime
        .client
        .build_image(options, None, Some(Bytes::from(tar_data)));

    while let Some(result) = stream.next().await {
        match result {
            Ok(output) => {
                if let Some(line) = output.stream {
                    let line = line.trim().to_string();
                    if !line.is_empty() {
                        if let Some(ref tx) = ctx.log_tx {
                            let _ = tx.send(line.clone());
                        }
                        tracing::debug!("{}", line);
                    }
                }
                if let Some(error) = output.error {
                    if let Some(ref tx) = ctx.log_tx {
                        let _ = tx.send(format!("ERROR: {}", error));
                    }
                    anyhow::bail!("Build error: {}", error);
                }
            }
            Err(e) => anyhow::bail!("Build failed: {}", e),
        }
    }

    Ok(ctx.tag.clone())
}
