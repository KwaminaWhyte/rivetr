//! App-specific validation functions.
//!
//! Covers: app names, dockerfile paths, healthchecks, build config, resource limits,
//! deployment commands, port mappings, network aliases, extra hosts, and docker images.

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Regex for validating app names: lowercase alphanumeric and dashes, 1-63 chars
    static ref APP_NAME_REGEX: Regex = Regex::new(
        r"^[a-z0-9][a-z0-9-]*$"
    ).unwrap();

    /// Regex for validating memory limit format (e.g., 256m, 1g, 512M, 2G, 256mb, 1gb)
    static ref MEMORY_LIMIT_REGEX: Regex = Regex::new(
        r"^[1-9]\d*([mMgG][bB]?|[bB])$"
    ).unwrap();

    /// Regex for validating CPU limit format (e.g., 0.5, 1, 2.0)
    static ref CPU_LIMIT_REGEX: Regex = Regex::new(
        r"^([0-9]+(\.[0-9]+)?|[0-9]*\.[0-9]+)$"
    ).unwrap();
}

/// Validate an app name
pub fn validate_app_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("App name is required".to_string());
    }

    if name.len() > 63 {
        return Err("App name is too long (max 63 characters)".to_string());
    }

    if !APP_NAME_REGEX.is_match(name) {
        return Err(
            "App name must start with a letter or number and contain only lowercase letters, numbers, and dashes".to_string()
        );
    }

    Ok(())
}

/// Validate a dockerfile path
pub fn validate_dockerfile(dockerfile: &str) -> Result<(), String> {
    if dockerfile.is_empty() {
        return Err("Dockerfile path is required".to_string());
    }

    if dockerfile.len() > 512 {
        return Err("Dockerfile path is too long (max 512 characters)".to_string());
    }

    // Check for path traversal attempts
    if dockerfile.contains("..") {
        return Err("Dockerfile path cannot contain '..'".to_string());
    }

    // Must be a relative path or start with ./
    if dockerfile.starts_with('/') && !dockerfile.starts_with("./") {
        return Err("Dockerfile path must be relative".to_string());
    }

    Ok(())
}

/// Validate a healthcheck path (optional field)
pub fn validate_healthcheck(healthcheck: &Option<String>) -> Result<(), String> {
    if let Some(h) = healthcheck {
        if h.is_empty() {
            return Ok(()); // Empty string treated as no healthcheck
        }

        if h.len() > 512 {
            return Err("Healthcheck path is too long (max 512 characters)".to_string());
        }

        // Must start with /
        if !h.starts_with('/') {
            return Err("Healthcheck path must start with '/'".to_string());
        }
    }

    Ok(())
}

/// Validate memory limit format (optional field)
pub fn validate_memory_limit(memory_limit: &Option<String>) -> Result<(), String> {
    if let Some(m) = memory_limit {
        if m.is_empty() {
            return Ok(()); // Empty string treated as no limit
        }

        if !MEMORY_LIMIT_REGEX.is_match(m) {
            return Err("Invalid memory limit format. Use format like '256m', '1g', '512M', '2G', '256mb', '1gb'".to_string());
        }
    }

    Ok(())
}

/// Validate CPU limit format (optional field)
pub fn validate_cpu_limit(cpu_limit: &Option<String>) -> Result<(), String> {
    if let Some(c) = cpu_limit {
        if c.is_empty() {
            return Ok(()); // Empty string treated as no limit
        }

        if !CPU_LIMIT_REGEX.is_match(c) {
            return Err("Invalid CPU limit format. Use format like '0.5', '1', '2.0'".to_string());
        }

        // Parse and validate range
        if let Ok(cpu) = c.parse::<f64>() {
            if cpu <= 0.0 {
                return Err("CPU limit must be greater than 0".to_string());
            }
            if cpu > 128.0 {
                return Err("CPU limit is too high (max 128)".to_string());
            }
        }
    }

    Ok(())
}

/// Validate base_directory (build context subdirectory)
pub fn validate_base_directory(base_dir: &Option<String>) -> Result<(), String> {
    if let Some(dir) = base_dir {
        if dir.is_empty() {
            return Ok(()); // Empty string treated as no base directory
        }

        if dir.len() > 512 {
            return Err("Base directory path is too long (max 512 characters)".to_string());
        }

        // Check for path traversal attempts
        if dir.contains("..") {
            return Err("Base directory path cannot contain '..'".to_string());
        }

        // Must be a relative path
        if dir.starts_with('/') {
            return Err("Base directory must be a relative path".to_string());
        }

        // Check for dangerous characters
        if dir.contains('\0') || dir.contains('\\') {
            return Err("Base directory contains invalid characters".to_string());
        }
    }

    Ok(())
}

/// Validate build_target (Docker multi-stage build target name)
pub fn validate_build_target(target: &Option<String>) -> Result<(), String> {
    if let Some(t) = target {
        if t.is_empty() {
            return Ok(()); // Empty string treated as no target
        }

        if t.len() > 128 {
            return Err("Build target name is too long (max 128 characters)".to_string());
        }

        // Build target should be alphanumeric with dashes and underscores
        if !t
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(
                "Build target must contain only alphanumeric characters, dashes, and underscores"
                    .to_string(),
            );
        }
    }

    Ok(())
}

/// Validate watch_paths (JSON array of paths to trigger auto-deploy)
pub fn validate_watch_paths(paths: &Option<String>) -> Result<(), String> {
    if let Some(p) = paths {
        if p.is_empty() {
            return Ok(()); // Empty string treated as no watch paths
        }

        if p.len() > 4096 {
            return Err("Watch paths JSON is too long (max 4096 characters)".to_string());
        }

        // Must be valid JSON array
        match serde_json::from_str::<Vec<String>>(p) {
            Ok(arr) => {
                // Validate each path
                for path in arr {
                    if path.contains("..") {
                        return Err(format!("Watch path '{}' cannot contain '..'", path));
                    }
                    if path.starts_with('/') {
                        return Err(format!("Watch path '{}' must be a relative path", path));
                    }
                }
            }
            Err(_) => {
                return Err(
                    "Watch paths must be a valid JSON array of strings, e.g., [\"src/\", \"Dockerfile\"]"
                        .to_string(),
                );
            }
        }
    }

    Ok(())
}

/// Validate custom_docker_options (extra docker build/run arguments)
pub fn validate_custom_docker_options(options: &Option<String>) -> Result<(), String> {
    if let Some(opts) = options {
        if opts.is_empty() {
            return Ok(()); // Empty string treated as no custom options
        }

        if opts.len() > 2048 {
            return Err("Custom Docker options is too long (max 2048 characters)".to_string());
        }

        // Disallow dangerous options
        let dangerous_patterns = [
            "--privileged",
            "--cap-add",
            "--device",
            "--pid=host",
            "--network=host",
            "--userns=host",
            "--ipc=host",
            "-v /:",                      // Root mount
            "--volume /:",                // Root mount alternative
            "--mount type=bind,source=/", // Root bind mount
        ];

        let opts_lower = opts.to_lowercase();
        for pattern in dangerous_patterns {
            if opts_lower.contains(pattern) {
                return Err(format!(
                    "Custom Docker options cannot contain dangerous flag: {}",
                    pattern
                ));
            }
        }
    }

    Ok(())
}

/// Validate port mappings (JSON array of port mapping objects)
pub fn validate_port_mappings(
    port_mappings: &Option<Vec<crate::db::PortMapping>>,
) -> Result<(), String> {
    if let Some(mappings) = port_mappings {
        if mappings.len() > 50 {
            return Err("Too many port mappings (max 50)".to_string());
        }

        let mut seen_host_ports: std::collections::HashSet<u16> = std::collections::HashSet::new();

        for (i, mapping) in mappings.iter().enumerate() {
            // Validate container port
            if mapping.container_port == 0 {
                return Err(format!(
                    "Port mapping {}: container port must be between 1 and 65535",
                    i + 1
                ));
            }

            // Validate host port if specified (0 means auto-assign)
            if mapping.host_port > 0 && mapping.host_port < 1024 {
                return Err(format!(
                    "Port mapping {}: privileged host ports (1-1023) are not allowed",
                    i + 1
                ));
            }

            // Check for duplicate host ports (only if not auto-assigned)
            if mapping.host_port > 0 {
                if seen_host_ports.contains(&mapping.host_port) {
                    return Err(format!(
                        "Port mapping {}: host port {} is already in use",
                        i + 1,
                        mapping.host_port
                    ));
                }
                seen_host_ports.insert(mapping.host_port);
            }

            // Validate protocol
            let protocol = mapping.protocol.to_lowercase();
            if protocol != "tcp" && protocol != "udp" {
                return Err(format!(
                    "Port mapping {}: protocol must be 'tcp' or 'udp'",
                    i + 1
                ));
            }
        }
    }

    Ok(())
}

/// Validate network aliases (JSON array of alias strings)
pub fn validate_network_aliases(aliases: &Option<Vec<String>>) -> Result<(), String> {
    if let Some(alias_list) = aliases {
        if alias_list.len() > 20 {
            return Err("Too many network aliases (max 20)".to_string());
        }

        for (i, alias) in alias_list.iter().enumerate() {
            if alias.is_empty() {
                return Err(format!("Network alias {} cannot be empty", i + 1));
            }

            if alias.len() > 63 {
                return Err(format!(
                    "Network alias '{}' is too long (max 63 characters)",
                    alias
                ));
            }

            // Alias should be a valid DNS hostname
            if !alias
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                return Err(format!(
                    "Network alias '{}' must contain only alphanumeric characters, dashes, and underscores",
                    alias
                ));
            }

            // Cannot start or end with dash
            if alias.starts_with('-') || alias.ends_with('-') {
                return Err(format!(
                    "Network alias '{}' cannot start or end with a dash",
                    alias
                ));
            }
        }
    }

    Ok(())
}

/// Validate extra hosts (JSON array of "hostname:ip" entries)
pub fn validate_extra_hosts(extra_hosts: &Option<Vec<String>>) -> Result<(), String> {
    if let Some(hosts) = extra_hosts {
        if hosts.len() > 50 {
            return Err("Too many extra hosts (max 50)".to_string());
        }

        for (i, host) in hosts.iter().enumerate() {
            if host.is_empty() {
                return Err(format!("Extra host {} cannot be empty", i + 1));
            }

            // Must be in hostname:ip format
            let parts: Vec<&str> = host.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(format!(
                    "Extra host '{}' must be in 'hostname:ip' format",
                    host
                ));
            }

            let hostname = parts[0];
            let ip = parts[1];

            // Validate hostname
            if hostname.is_empty() {
                return Err(format!("Extra host {}: hostname cannot be empty", i + 1));
            }

            if hostname.len() > 253 {
                return Err(format!(
                    "Extra host {}: hostname is too long (max 253 characters)",
                    i + 1
                ));
            }

            // Validate IP address (basic validation, allow special values)
            if ip.is_empty() {
                return Err(format!("Extra host {}: IP address cannot be empty", i + 1));
            }

            // Allow special Docker values like "host-gateway"
            if ip != "host-gateway" {
                // Check if it looks like an IP address (basic validation)
                let is_ipv4 = ip.split('.').count() == 4
                    && ip.split('.').all(|part| part.parse::<u8>().is_ok());
                let is_ipv6 = ip.contains(':');

                if !is_ipv4 && !is_ipv6 {
                    return Err(format!(
                        "Extra host {}: '{}' is not a valid IP address or 'host-gateway'",
                        i + 1,
                        ip
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Maximum number of deployment commands allowed
const MAX_DEPLOYMENT_COMMANDS: usize = 50;

/// Maximum length of a single deployment command
const MAX_COMMAND_LENGTH: usize = 4096;

/// Dangerous shell metacharacters that could enable command injection
/// These are blocked to prevent arbitrary command execution in containers
const DANGEROUS_SHELL_CHARS: &[&str] = &[
    "$(", "`", // Command substitution
    "&&", "||", // Command chaining
    ";",  // Command separator
    "|",  // Pipe to another command
    ">", ">>", // Output redirection
    "<", "<<", // Input redirection
    "&",  // Background execution
    "\n", "\r", // Newlines (command separation)
];

/// Dangerous patterns in deployment commands
const DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf /",    // Dangerous recursive delete
    "rm -rf /*",   // Dangerous recursive delete
    ":(){:|:&};:", // Fork bomb
    "mkfs",        // Filesystem formatting
    "dd if=",      // Raw disk access
    "/dev/sda",    // Disk device access
    "/dev/null",   // While harmless, may indicate redirection attempt
    "chmod 777",   // Overly permissive permissions
    "curl | sh",   // Remote code execution pattern
    "curl | bash", // Remote code execution pattern
    "wget | sh",   // Remote code execution pattern
    "wget | bash", // Remote code execution pattern
];

/// Validate deployment commands (pre or post deploy)
pub fn validate_deployment_commands(
    commands: &Option<Vec<String>>,
    field_name: &str,
) -> Result<(), String> {
    if let Some(cmd_list) = commands {
        if cmd_list.len() > MAX_DEPLOYMENT_COMMANDS {
            return Err(format!(
                "{}: too many commands (max {})",
                field_name, MAX_DEPLOYMENT_COMMANDS
            ));
        }

        for (i, cmd) in cmd_list.iter().enumerate() {
            if cmd.is_empty() {
                return Err(format!("{}: command {} cannot be empty", field_name, i + 1));
            }

            if cmd.len() > MAX_COMMAND_LENGTH {
                return Err(format!(
                    "{}: command {} is too long (max {} characters)",
                    field_name,
                    i + 1,
                    MAX_COMMAND_LENGTH
                ));
            }

            // Check for null bytes which could cause issues
            if cmd.contains('\0') {
                return Err(format!(
                    "{}: command {} contains invalid null character",
                    field_name,
                    i + 1
                ));
            }

            // Check for dangerous shell metacharacters that could enable injection
            for dangerous in DANGEROUS_SHELL_CHARS {
                if cmd.contains(dangerous) {
                    return Err(format!(
                        "{}: command {} contains dangerous shell character '{}'. \
                        For security, shell metacharacters are not allowed in deployment commands. \
                        Each command should be a simple executable with arguments.",
                        field_name,
                        i + 1,
                        dangerous.escape_default()
                    ));
                }
            }

            // Check for dangerous command patterns
            let cmd_lower = cmd.to_lowercase();
            for pattern in DANGEROUS_PATTERNS {
                if cmd_lower.contains(&pattern.to_lowercase()) {
                    return Err(format!(
                        "{}: command {} contains a potentially dangerous pattern '{}'. \
                        This operation is not allowed for security reasons.",
                        field_name,
                        i + 1,
                        pattern
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Validate a Docker image reference
/// Accepts formats like:
/// - "nginx" (official image)
/// - "nginx:1.19" (with tag)
/// - "user/image" (Docker Hub user image)
/// - "ghcr.io/user/image" (GitHub Container Registry)
/// - "registry.example.com/path/image:tag" (custom registry)
pub fn validate_docker_image(image: Option<&str>) -> Result<(), String> {
    let Some(image) = image else {
        return Ok(());
    };

    if image.is_empty() {
        return Ok(()); // Empty string is treated as "clear"
    }

    if image.len() > 1024 {
        return Err("Docker image reference is too long (max 1024 characters)".to_string());
    }

    // Check for forbidden characters
    if image.contains(char::is_whitespace) {
        return Err("Docker image reference cannot contain whitespace".to_string());
    }

    // Split by @ to handle digest format (image@sha256:...)
    let image_part = image.split('@').next().unwrap_or(image);

    // Split by : to separate image from tag
    let parts: Vec<&str> = image_part.splitn(2, ':').collect();
    let image_name = parts[0];

    if image_name.is_empty() {
        return Err("Docker image name cannot be empty".to_string());
    }

    // Validate the image name (registry/path/name format)
    // Each component should be valid DNS-like or path component
    for component in image_name.split('/') {
        if component.is_empty() {
            return Err("Docker image reference contains empty path component".to_string());
        }

        // Allow alphanumeric, dashes, underscores, and dots
        if !component
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(format!(
                "Invalid character in docker image component: '{}'",
                component
            ));
        }
    }

    // If there's a tag, validate it
    if parts.len() > 1 {
        let tag = parts[1];
        if tag.is_empty() {
            return Err("Docker image tag cannot be empty if specified".to_string());
        }

        if tag.len() > 128 {
            return Err("Docker image tag is too long (max 128 characters)".to_string());
        }

        // Tags allow alphanumeric, dashes, underscores, and dots
        if !tag
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(format!("Invalid character in docker image tag: '{}'", tag));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_app_name() {
        assert!(validate_app_name("my-app").is_ok());
        assert!(validate_app_name("app123").is_ok());
        assert!(validate_app_name("my-cool-app-2").is_ok());
        assert!(validate_app_name("a").is_ok()); // single char is ok now
        assert!(validate_app_name("my-app-").is_ok()); // trailing dash allowed

        assert!(validate_app_name("").is_err());
        assert!(validate_app_name("-invalid").is_err()); // must start with alnum
        assert!(validate_app_name("Invalid").is_err()); // uppercase
        assert!(validate_app_name("my_app").is_err()); // underscore not allowed
    }

    #[test]
    fn test_validate_memory_limit() {
        // Single letter suffix
        assert!(validate_memory_limit(&Some("256m".to_string())).is_ok());
        assert!(validate_memory_limit(&Some("1g".to_string())).is_ok());
        assert!(validate_memory_limit(&Some("512M".to_string())).is_ok());
        assert!(validate_memory_limit(&Some("2G".to_string())).is_ok());
        // Double letter suffix (mb, gb)
        assert!(validate_memory_limit(&Some("256mb".to_string())).is_ok());
        assert!(validate_memory_limit(&Some("1gb".to_string())).is_ok());
        assert!(validate_memory_limit(&Some("512MB".to_string())).is_ok());
        assert!(validate_memory_limit(&Some("2GB".to_string())).is_ok());
        // Bytes
        assert!(validate_memory_limit(&Some("1024b".to_string())).is_ok());
        assert!(validate_memory_limit(&Some("1024B".to_string())).is_ok());
        // None/empty
        assert!(validate_memory_limit(&None).is_ok());

        // Invalid formats
        assert!(validate_memory_limit(&Some("invalid".to_string())).is_err());
        assert!(validate_memory_limit(&Some("256".to_string())).is_err());
        assert!(validate_memory_limit(&Some("0m".to_string())).is_err());
    }

    #[test]
    fn test_validate_cpu_limit() {
        assert!(validate_cpu_limit(&Some("0.5".to_string())).is_ok());
        assert!(validate_cpu_limit(&Some("1".to_string())).is_ok());
        assert!(validate_cpu_limit(&Some("2.0".to_string())).is_ok());
        assert!(validate_cpu_limit(&None).is_ok());

        assert!(validate_cpu_limit(&Some("invalid".to_string())).is_err());
        assert!(validate_cpu_limit(&Some("-1".to_string())).is_err());
    }

    #[test]
    fn test_validate_base_directory() {
        // Valid paths
        assert!(validate_base_directory(&None).is_ok());
        assert!(validate_base_directory(&Some("".to_string())).is_ok());
        assert!(validate_base_directory(&Some("backend".to_string())).is_ok());
        assert!(validate_base_directory(&Some("src/app".to_string())).is_ok());
        assert!(validate_base_directory(&Some("packages/api".to_string())).is_ok());

        // Invalid: path traversal
        assert!(validate_base_directory(&Some("..".to_string())).is_err());
        assert!(validate_base_directory(&Some("../etc".to_string())).is_err());
        assert!(validate_base_directory(&Some("src/../hack".to_string())).is_err());

        // Invalid: absolute path
        assert!(validate_base_directory(&Some("/etc".to_string())).is_err());

        // Invalid: backslash
        assert!(validate_base_directory(&Some("path\\to".to_string())).is_err());
    }

    #[test]
    fn test_validate_build_target() {
        // Valid targets
        assert!(validate_build_target(&None).is_ok());
        assert!(validate_build_target(&Some("".to_string())).is_ok());
        assert!(validate_build_target(&Some("production".to_string())).is_ok());
        assert!(validate_build_target(&Some("build-stage".to_string())).is_ok());
        assert!(validate_build_target(&Some("stage_2".to_string())).is_ok());

        // Invalid: special characters
        assert!(validate_build_target(&Some("stage/prod".to_string())).is_err());
        assert!(validate_build_target(&Some("stage:latest".to_string())).is_err());
        assert!(validate_build_target(&Some("stage@v1".to_string())).is_err());
    }

    #[test]
    fn test_validate_watch_paths() {
        // Valid paths
        assert!(validate_watch_paths(&None).is_ok());
        assert!(validate_watch_paths(&Some("".to_string())).is_ok());
        assert!(validate_watch_paths(&Some(r#"["src/"]"#.to_string())).is_ok());
        assert!(validate_watch_paths(&Some(r#"["src/", "package.json"]"#.to_string())).is_ok());
        assert!(
            validate_watch_paths(&Some(r#"["Dockerfile", "docker-compose.yml"]"#.to_string()))
                .is_ok()
        );

        // Invalid: not valid JSON
        assert!(validate_watch_paths(&Some("src/".to_string())).is_err());
        assert!(validate_watch_paths(&Some("[src/]".to_string())).is_err());

        // Invalid: path traversal
        assert!(validate_watch_paths(&Some(r#"["../etc/passwd"]"#.to_string())).is_err());

        // Invalid: absolute path
        assert!(validate_watch_paths(&Some(r#"["/etc/passwd"]"#.to_string())).is_err());
    }

    #[test]
    fn test_validate_custom_docker_options() {
        // Valid options
        assert!(validate_custom_docker_options(&None).is_ok());
        assert!(validate_custom_docker_options(&Some("".to_string())).is_ok());
        assert!(validate_custom_docker_options(&Some("--no-cache".to_string())).is_ok());
        assert!(validate_custom_docker_options(&Some("--build-arg FOO=bar".to_string())).is_ok());
        assert!(
            validate_custom_docker_options(&Some("--add-host=myhost:192.168.1.1".to_string()))
                .is_ok()
        );

        // Dangerous options (security)
        assert!(validate_custom_docker_options(&Some("--privileged".to_string())).is_err());
        assert!(validate_custom_docker_options(&Some("--cap-add SYS_ADMIN".to_string())).is_err());
        assert!(validate_custom_docker_options(&Some("--network=host".to_string())).is_err());
        assert!(validate_custom_docker_options(&Some("-v /:/mnt".to_string())).is_err());
    }

    #[test]
    fn test_validate_app_name_oversized() {
        // 63 chars is the max allowed
        let max_name = "a".repeat(63);
        assert!(validate_app_name(&max_name).is_ok());

        // 64 chars must be rejected
        let too_long = "a".repeat(64);
        assert!(validate_app_name(&too_long).is_err());
    }

    #[test]
    fn test_validate_app_name_metacharacters() {
        // Shell metacharacters / spaces / path separators must be rejected
        assert!(validate_app_name("my app").is_err());
        assert!(validate_app_name("app;rm").is_err());
        assert!(validate_app_name("app/sub").is_err());
        assert!(validate_app_name("app$(id)").is_err());
        assert!(validate_app_name("app.name").is_err()); // dot not allowed
    }

    #[test]
    fn test_validate_dockerfile() {
        // Valid relative paths
        assert!(validate_dockerfile("Dockerfile").is_ok());
        assert!(validate_dockerfile("./Dockerfile").is_ok());
        assert!(validate_dockerfile("docker/Dockerfile").is_ok());
        assert!(validate_dockerfile("./build/Dockerfile.prod").is_ok());

        // Empty rejected
        assert!(validate_dockerfile("").is_err());

        // Path traversal rejected
        assert!(validate_dockerfile("../Dockerfile").is_err());
        assert!(validate_dockerfile("docker/../../etc/passwd").is_err());

        // Absolute path (not ./) rejected
        assert!(validate_dockerfile("/etc/Dockerfile").is_err());

        // Oversized rejected (>512)
        assert!(validate_dockerfile(&"a".repeat(513)).is_err());
        assert!(validate_dockerfile(&"a".repeat(512)).is_ok());
    }

    #[test]
    fn test_validate_healthcheck() {
        // None and empty treated as no healthcheck
        assert!(validate_healthcheck(&None).is_ok());
        assert!(validate_healthcheck(&Some("".to_string())).is_ok());

        // Valid absolute paths
        assert!(validate_healthcheck(&Some("/".to_string())).is_ok());
        assert!(validate_healthcheck(&Some("/health".to_string())).is_ok());
        assert!(validate_healthcheck(&Some("/api/v1/healthz".to_string())).is_ok());

        // Must start with '/'
        assert!(validate_healthcheck(&Some("health".to_string())).is_err());

        // Oversized rejected (>512)
        assert!(validate_healthcheck(&Some(format!("/{}", "a".repeat(512)))).is_err());
    }

    #[test]
    fn test_validate_cpu_limit_range() {
        // Upper bound: 128 ok, >128 rejected
        assert!(validate_cpu_limit(&Some("128".to_string())).is_ok());
        assert!(validate_cpu_limit(&Some("128.0".to_string())).is_ok());
        assert!(validate_cpu_limit(&Some("129".to_string())).is_err());
        assert!(validate_cpu_limit(&Some("1000".to_string())).is_err());

        // Zero rejected (must be > 0)
        assert!(validate_cpu_limit(&Some("0".to_string())).is_err());
        assert!(validate_cpu_limit(&Some("0.0".to_string())).is_err());

        // Empty treated as no limit
        assert!(validate_cpu_limit(&Some("".to_string())).is_ok());
    }

    #[test]
    fn test_validate_base_directory_null_byte() {
        // Null byte must be rejected
        assert!(validate_base_directory(&Some("src\0app".to_string())).is_err());

        // Oversized rejected (>512)
        assert!(validate_base_directory(&Some("a".repeat(513))).is_err());
    }

    #[test]
    fn test_validate_watch_paths_oversized() {
        // Oversized JSON rejected (>4096)
        let big = format!("[\"{}\"]", "a".repeat(4100));
        assert!(validate_watch_paths(&Some(big)).is_err());
    }

    #[test]
    fn test_validate_custom_docker_options_oversized_and_more() {
        // Oversized rejected (>2048)
        assert!(validate_custom_docker_options(&Some("a".repeat(2049))).is_err());

        // Additional dangerous flags (case-insensitive match)
        assert!(validate_custom_docker_options(&Some("--PID=host".to_string())).is_err());
        assert!(validate_custom_docker_options(&Some("--device /dev/sda".to_string())).is_err());
        assert!(validate_custom_docker_options(&Some("--ipc=host".to_string())).is_err());
        assert!(validate_custom_docker_options(&Some("--volume /:/host".to_string())).is_err());
    }

    #[test]
    fn test_validate_deployment_commands_valid() {
        assert!(validate_deployment_commands(&None, "pre_deploy").is_ok());
        assert!(validate_deployment_commands(&Some(vec![]), "pre_deploy").is_ok());
        assert!(validate_deployment_commands(
            &Some(vec!["npm run migrate".to_string(), "echo done".to_string(),]),
            "pre_deploy",
        )
        .is_ok());
    }

    #[test]
    fn test_validate_deployment_commands_shell_metacharacters() {
        // Each dangerous shell metacharacter must be blocked (command injection)
        let injections = [
            "echo $(whoami)", // command substitution
            "echo `id`",      // backtick substitution
            "ls && rm file",  // command chaining (and)
            "ls || rm file",  // command chaining (or)
            "ls; rm file",    // command separator
            "cat file | sh",  // pipe
            "echo x > out",   // output redirect
            "echo x >> out",  // append redirect
            "sh < input",     // input redirect
            "run &",          // background execution
            "line1\nline2",   // newline injection
            "line1\rline2",   // carriage return injection
        ];
        for inj in injections {
            assert!(
                validate_deployment_commands(&Some(vec![inj.to_string()]), "pre_deploy").is_err(),
                "expected rejection for: {:?}",
                inj
            );
        }
    }

    #[test]
    fn test_validate_deployment_commands_dangerous_patterns() {
        // Destructive / RCE patterns must be blocked (case-insensitive)
        let patterns = [
            "rm -rf /",
            "RM -RF /*",
            "mkfs.ext4 /dev/sdb",
            "dd if=/dev/zero of=disk",
            "chmod 777 secrets",
        ];
        for p in patterns {
            assert!(
                validate_deployment_commands(&Some(vec![p.to_string()]), "post_deploy").is_err(),
                "expected rejection for: {:?}",
                p
            );
        }
    }

    #[test]
    fn test_validate_deployment_commands_edge_cases() {
        // Empty command rejected
        assert!(validate_deployment_commands(&Some(vec!["".to_string()]), "pre_deploy").is_err());

        // Null byte rejected
        assert!(
            validate_deployment_commands(&Some(vec!["echo\0x".to_string()]), "pre_deploy").is_err()
        );

        // Oversized single command rejected (>4096)
        assert!(validate_deployment_commands(&Some(vec!["a".repeat(4097)]), "pre_deploy").is_err());

        // Too many commands rejected (>50)
        let many: Vec<String> = (0..51).map(|i| format!("echo {}", i)).collect();
        assert!(validate_deployment_commands(&Some(many), "pre_deploy").is_err());

        // Exactly 50 commands allowed
        let fifty: Vec<String> = (0..50).map(|i| format!("echo {}", i)).collect();
        assert!(validate_deployment_commands(&Some(fifty), "pre_deploy").is_ok());
    }

    #[test]
    fn test_validate_docker_image_valid() {
        assert!(validate_docker_image(None).is_ok());
        assert!(validate_docker_image(Some("")).is_ok()); // empty = clear
        assert!(validate_docker_image(Some("nginx")).is_ok());
        assert!(validate_docker_image(Some("nginx:1.19")).is_ok());
        assert!(validate_docker_image(Some("user/image")).is_ok());
        assert!(validate_docker_image(Some("ghcr.io/user/image")).is_ok());
        assert!(validate_docker_image(Some("registry.example.com/path/image:tag")).is_ok());
        assert!(validate_docker_image(Some("nginx@sha256:abc123")).is_ok());
    }

    #[test]
    fn test_validate_docker_image_invalid() {
        // Whitespace / injection attempts rejected
        assert!(validate_docker_image(Some("nginx; rm -rf /")).is_err());
        assert!(validate_docker_image(Some("nginx latest")).is_err());
        assert!(validate_docker_image(Some("nginx$(id)")).is_err());

        // Empty path component
        assert!(validate_docker_image(Some("user//image")).is_err());

        // Empty tag after colon
        assert!(validate_docker_image(Some("nginx:")).is_err());

        // Oversized image ref (>1024)
        assert!(validate_docker_image(Some(&"a".repeat(1025))).is_err());

        // Oversized tag (>128)
        let long_tag = format!("nginx:{}", "a".repeat(129));
        assert!(validate_docker_image(Some(&long_tag)).is_err());
    }

    #[test]
    fn test_validate_network_aliases() {
        // Valid
        assert!(validate_network_aliases(&None).is_ok());
        assert!(validate_network_aliases(&Some(vec!["db".to_string()])).is_ok());
        assert!(validate_network_aliases(&Some(vec!["my-svc_1".to_string()])).is_ok());

        // Empty alias rejected
        assert!(validate_network_aliases(&Some(vec!["".to_string()])).is_err());

        // Invalid char rejected
        assert!(validate_network_aliases(&Some(vec!["bad alias".to_string()])).is_err());
        assert!(validate_network_aliases(&Some(vec!["a.b".to_string()])).is_err());

        // Leading/trailing dash rejected
        assert!(validate_network_aliases(&Some(vec!["-svc".to_string()])).is_err());
        assert!(validate_network_aliases(&Some(vec!["svc-".to_string()])).is_err());

        // Too many (>20) rejected
        let many: Vec<String> = (0..21).map(|i| format!("svc{}", i)).collect();
        assert!(validate_network_aliases(&Some(many)).is_err());

        // Oversized alias (>63) rejected
        assert!(validate_network_aliases(&Some(vec!["a".repeat(64)])).is_err());
    }

    #[test]
    fn test_validate_extra_hosts() {
        // Valid
        assert!(validate_extra_hosts(&None).is_ok());
        assert!(validate_extra_hosts(&Some(vec!["myhost:192.168.1.1".to_string()])).is_ok());
        assert!(validate_extra_hosts(&Some(vec!["host:host-gateway".to_string()])).is_ok());
        assert!(validate_extra_hosts(&Some(vec!["host:::1".to_string()])).is_ok()); // ipv6

        // Missing colon / wrong format rejected
        assert!(validate_extra_hosts(&Some(vec!["nohostsep".to_string()])).is_err());

        // Empty entry rejected
        assert!(validate_extra_hosts(&Some(vec!["".to_string()])).is_err());

        // Invalid IP rejected
        assert!(validate_extra_hosts(&Some(vec!["host:not-an-ip".to_string()])).is_err());

        // Empty hostname rejected
        assert!(validate_extra_hosts(&Some(vec![":192.168.1.1".to_string()])).is_err());

        // Too many (>50) rejected
        let many: Vec<String> = (0..51).map(|i| format!("h{}:127.0.0.1", i)).collect();
        assert!(validate_extra_hosts(&Some(many)).is_err());
    }

    #[test]
    fn test_validate_port_mappings() {
        use crate::db::PortMapping;

        let mk = |host: u16, container: u16, proto: &str| PortMapping {
            host_port: host,
            container_port: container,
            protocol: proto.to_string(),
        };

        // Valid
        assert!(validate_port_mappings(&None).is_ok());
        assert!(validate_port_mappings(&Some(vec![mk(8080, 80, "tcp")])).is_ok());
        assert!(validate_port_mappings(&Some(vec![mk(0, 80, "udp")])).is_ok()); // host 0 = auto

        // Container port 0 rejected
        assert!(validate_port_mappings(&Some(vec![mk(8080, 0, "tcp")])).is_err());

        // Privileged host port rejected
        assert!(validate_port_mappings(&Some(vec![mk(80, 80, "tcp")])).is_err());

        // Duplicate host ports rejected
        assert!(
            validate_port_mappings(&Some(vec![mk(8080, 80, "tcp"), mk(8080, 81, "tcp")])).is_err()
        );

        // Invalid protocol rejected
        assert!(validate_port_mappings(&Some(vec![mk(8080, 80, "sctp")])).is_err());

        // Too many (>50) rejected
        let many: Vec<PortMapping> = (0..51).map(|i| mk(0, (i + 1) as u16, "tcp")).collect();
        assert!(validate_port_mappings(&Some(many)).is_err());
    }
}
