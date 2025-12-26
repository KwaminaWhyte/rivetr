//! Input validation for API requests.
//!
//! This module provides validation functions for API request data,
//! ensuring all inputs meet the required format and constraints.
//!
//! For collecting multiple validation errors and returning them as an ApiError,
//! use the `ValidationErrorBuilder` from the `error` module.

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Regex for validating HTTP/HTTPS Git URLs
    static ref GIT_HTTP_URL_REGEX: Regex = Regex::new(
        r"^https?://[a-zA-Z0-9][-a-zA-Z0-9]*(\.[a-zA-Z0-9][-a-zA-Z0-9]*)*(:\d+)?(/[-a-zA-Z0-9_%&=+@~.]+)*/?$"
    ).unwrap();

    /// Regex for validating SSH Git URLs
    static ref GIT_SSH_URL_REGEX: Regex = Regex::new(
        r"^(git@[a-zA-Z0-9][-a-zA-Z0-9]*(\.[a-zA-Z0-9][-a-zA-Z0-9]*)*:[-a-zA-Z0-9_./]+\.git|ssh://[a-zA-Z0-9@][-a-zA-Z0-9@.]*(/[-a-zA-Z0-9_.]+)+\.git)$"
    ).unwrap();

    /// Regex for validating app names (alphanumeric with dashes, 1-63 chars)
    static ref APP_NAME_REGEX: Regex = Regex::new(
        r"^[a-z0-9]([a-z0-9-]*[a-z0-9])?$"
    ).unwrap();

    /// Regex for validating domain names
    static ref DOMAIN_REGEX: Regex = Regex::new(
        r"^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?)*$"
    ).unwrap();

    /// Regex for validating branch names
    static ref BRANCH_REGEX: Regex = Regex::new(
        r"^[a-zA-Z0-9]([a-zA-Z0-9._/-]*[a-zA-Z0-9])?$"
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

/// Validate a Git URL (HTTP/HTTPS or SSH format)
pub fn validate_git_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("Git URL is required".to_string());
    }

    if url.len() > 2048 {
        return Err("Git URL is too long (max 2048 characters)".to_string());
    }

    // Check HTTP/HTTPS URL
    if url.starts_with("http://") || url.starts_with("https://") {
        if GIT_HTTP_URL_REGEX.is_match(url) {
            return Ok(());
        }
    }

    // Check SSH URL
    if url.starts_with("git@") || url.starts_with("ssh://") {
        if GIT_SSH_URL_REGEX.is_match(url) {
            return Ok(());
        }
    }

    Err("Invalid Git URL format. Must be HTTP(S) or SSH URL".to_string())
}

/// Validate an app name
pub fn validate_app_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("App name is required".to_string());
    }

    if name.len() > 63 {
        return Err("App name is too long (max 63 characters)".to_string());
    }

    if name.len() < 2 {
        return Err("App name is too short (min 2 characters)".to_string());
    }

    if !APP_NAME_REGEX.is_match(name) {
        return Err(
            "App name must be lowercase alphanumeric with dashes, starting and ending with alphanumeric".to_string()
        );
    }

    Ok(())
}

/// Validate a port number
pub fn validate_port(port: i32) -> Result<(), String> {
    if port < 1 || port > 65535 {
        return Err("Port must be between 1 and 65535".to_string());
    }

    // Warn about privileged ports but don't reject
    if port < 1024 {
        // Could add warning logging here if needed
    }

    Ok(())
}

/// Validate a domain name (optional field)
pub fn validate_domain(domain: &Option<String>) -> Result<(), String> {
    if let Some(d) = domain {
        if d.is_empty() {
            return Ok(()); // Empty string treated as no domain
        }

        if d.len() > 253 {
            return Err("Domain name is too long (max 253 characters)".to_string());
        }

        if !DOMAIN_REGEX.is_match(d) {
            return Err("Invalid domain name format".to_string());
        }
    }

    Ok(())
}

/// Validate a branch name
pub fn validate_branch(branch: &str) -> Result<(), String> {
    if branch.is_empty() {
        return Err("Branch name is required".to_string());
    }

    if branch.len() > 255 {
        return Err("Branch name is too long (max 255 characters)".to_string());
    }

    if !BRANCH_REGEX.is_match(branch) {
        return Err("Invalid branch name format".to_string());
    }

    // Check for dangerous patterns
    if branch.contains("..") {
        return Err("Branch name cannot contain '..'".to_string());
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

/// Validate a UUID string
pub fn validate_uuid(id: &str, field_name: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err(format!("{} is required", field_name));
    }

    if uuid::Uuid::parse_str(id).is_err() {
        return Err(format!("Invalid {} format", field_name));
    }

    Ok(())
}

/// Valid environment values
const VALID_ENVIRONMENTS: [&str; 3] = ["development", "staging", "production"];

/// Validate an environment value
pub fn validate_environment(environment: &str) -> Result<(), String> {
    let env_lower = environment.to_lowercase();
    if !VALID_ENVIRONMENTS.contains(&env_lower.as_str()) {
        return Err(format!(
            "Invalid environment. Must be one of: {}",
            VALID_ENVIRONMENTS.join(", ")
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_git_url_https() {
        assert!(validate_git_url("https://github.com/user/repo").is_ok());
        assert!(validate_git_url("https://github.com/user/repo.git").is_ok());
        assert!(validate_git_url("https://gitlab.com/org/project/repo").is_ok());
        assert!(validate_git_url("http://gitea.example.com/user/repo").is_ok());
    }

    #[test]
    fn test_validate_git_url_ssh() {
        assert!(validate_git_url("git@github.com:user/repo.git").is_ok());
        assert!(validate_git_url("git@gitlab.com:org/project/repo.git").is_ok());
        assert!(validate_git_url("ssh://git@github.com/user/repo.git").is_ok());
    }

    #[test]
    fn test_validate_git_url_invalid() {
        assert!(validate_git_url("").is_err());
        assert!(validate_git_url("not-a-url").is_err());
        assert!(validate_git_url("ftp://example.com/repo").is_err());
    }

    #[test]
    fn test_validate_app_name() {
        assert!(validate_app_name("my-app").is_ok());
        assert!(validate_app_name("app123").is_ok());
        assert!(validate_app_name("my-cool-app-2").is_ok());

        assert!(validate_app_name("").is_err());
        assert!(validate_app_name("a").is_err()); // too short
        assert!(validate_app_name("-invalid").is_err());
        assert!(validate_app_name("invalid-").is_err());
        assert!(validate_app_name("Invalid").is_err()); // uppercase
        assert!(validate_app_name("my_app").is_err()); // underscore
    }

    #[test]
    fn test_validate_port() {
        assert!(validate_port(80).is_ok());
        assert!(validate_port(443).is_ok());
        assert!(validate_port(3000).is_ok());
        assert!(validate_port(8080).is_ok());
        assert!(validate_port(65535).is_ok());

        assert!(validate_port(0).is_err());
        assert!(validate_port(-1).is_err());
        assert!(validate_port(65536).is_err());
    }

    #[test]
    fn test_validate_domain() {
        assert!(validate_domain(&Some("example.com".to_string())).is_ok());
        assert!(validate_domain(&Some("sub.example.com".to_string())).is_ok());
        assert!(validate_domain(&Some("my-app.example.com".to_string())).is_ok());
        assert!(validate_domain(&None).is_ok());

        assert!(validate_domain(&Some("-invalid.com".to_string())).is_err());
    }

    #[test]
    fn test_validate_branch() {
        assert!(validate_branch("main").is_ok());
        assert!(validate_branch("develop").is_ok());
        assert!(validate_branch("feature/my-feature").is_ok());
        assert!(validate_branch("release/v1.0.0").is_ok());

        assert!(validate_branch("").is_err());
        assert!(validate_branch("..").is_err());
        assert!(validate_branch("branch/../hack").is_err());
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
    fn test_validate_uuid() {
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000", "app_id").is_ok());
        assert!(validate_uuid("", "app_id").is_err());
        assert!(validate_uuid("not-a-uuid", "app_id").is_err());
    }

    #[test]
    fn test_validate_environment() {
        assert!(validate_environment("development").is_ok());
        assert!(validate_environment("staging").is_ok());
        assert!(validate_environment("production").is_ok());
        // Case insensitive
        assert!(validate_environment("Development").is_ok());
        assert!(validate_environment("STAGING").is_ok());
        assert!(validate_environment("Production").is_ok());

        assert!(validate_environment("").is_err());
        assert!(validate_environment("invalid").is_err());
        assert!(validate_environment("test").is_err());
    }
}
