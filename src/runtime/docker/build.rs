use anyhow::Result;
use bollard::image::BuildImageOptions;
use bytes::Bytes;
use futures::StreamExt;

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
    let options = BuildImageOptions {
        dockerfile: ctx.dockerfile.trim_start_matches("./"),
        t: &ctx.tag,
        rm: true,
        target,
        extrahosts: extra_build_args.extra_hosts.as_deref(),
        nocache: extra_build_args.no_cache,
        memory: memory.map(|m| m as u64),
        memswap: memory, // Set memswap equal to memory to disable swap
        cpuperiod: cpuperiod.map(|p| p as u64),
        cpuquota: cpuquota.map(|q| q as u64),
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
                if let Some(stream) = output.stream {
                    tracing::debug!("{}", stream.trim());
                }
                if let Some(error) = output.error {
                    anyhow::bail!("Build error: {}", error);
                }
            }
            Err(e) => anyhow::bail!("Build failed: {}", e),
        }
    }

    Ok(ctx.tag.clone())
}
