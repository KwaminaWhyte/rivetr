//! Input validation for API requests.
//!
//! This module provides validation functions for API request data,
//! ensuring all inputs meet the required format and constraints.
//!
//! For collecting multiple validation errors and returning them as an ApiError,
//! use the `ValidationErrorBuilder` from the `error` module.
//!
//! Submodules:
//! - `apps`      — app-specific validators (names, dockerfile, resources, commands, docker image)
//! - `databases` — database-specific validators (reserved for future use)
//! - `services`  — service-specific validators (reserved for future use)

pub mod apps;
pub mod databases;
pub mod services;

pub use apps::*;

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

    /// Regex for validating domain names
    static ref DOMAIN_REGEX: Regex = Regex::new(
        r"^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?)*$"
    ).unwrap();

    /// Regex for validating branch names
    static ref BRANCH_REGEX: Regex = Regex::new(
        r"^[a-zA-Z0-9]([a-zA-Z0-9._/-]*[a-zA-Z0-9])?$"
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

/// Valid build type values
const VALID_BUILD_TYPES: [&str; 6] = [
    "dockerfile",
    "nixpacks",
    "railpack",
    "cnb",
    "static",
    "staticsite",
];

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

/// Validate a build type value
pub fn validate_build_type(build_type: &str) -> Result<(), String> {
    let build_type_lower = build_type.to_lowercase();
    if !VALID_BUILD_TYPES.contains(&build_type_lower.as_str()) {
        return Err(format!(
            "Invalid build_type. Must be one of: {}",
            VALID_BUILD_TYPES.join(", ")
        ));
    }
    Ok(())
}

/// Validate a single domain name string (non-optional version)
pub fn validate_domain_name(domain: &str) -> Result<(), String> {
    if domain.is_empty() {
        return Err("Domain name cannot be empty".to_string());
    }

    if domain.len() > 253 {
        return Err("Domain name is too long (max 253 characters)".to_string());
    }

    if !DOMAIN_REGEX.is_match(domain) {
        return Err(format!("Invalid domain name format: '{}'", domain));
    }

    Ok(())
}

/// Maximum number of domains per app
const MAX_DOMAINS_PER_APP: usize = 100;

/// Validate domains array (JSON array of Domain objects)
pub fn validate_domains(domains: &Option<Vec<crate::db::Domain>>) -> Result<(), String> {
    if let Some(domain_list) = domains {
        if domain_list.len() > MAX_DOMAINS_PER_APP {
            return Err(format!("Too many domains (max {})", MAX_DOMAINS_PER_APP));
        }

        let mut primary_count = 0;
        let mut seen_domains: std::collections::HashSet<String> = std::collections::HashSet::new();

        for (i, domain) in domain_list.iter().enumerate() {
            // Validate domain name
            validate_domain_name(&domain.domain).map_err(|e| format!("Domain {}: {}", i + 1, e))?;

            // Check for duplicates
            let normalized = domain.domain.to_lowercase();
            if seen_domains.contains(&normalized) {
                return Err(format!("Duplicate domain: '{}'", domain.domain));
            }
            seen_domains.insert(normalized);

            // Count primary domains
            if domain.primary {
                primary_count += 1;
            }
        }

        // Only one primary domain allowed
        if primary_count > 1 {
            return Err("Only one domain can be marked as primary".to_string());
        }
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

    #[test]
    fn test_validate_domain_name() {
        // Valid domain names
        assert!(validate_domain_name("example.com").is_ok());
        assert!(validate_domain_name("sub.example.com").is_ok());
        assert!(validate_domain_name("my-app.example.com").is_ok());
        assert!(validate_domain_name("app123.example.co.uk").is_ok());
        assert!(validate_domain_name("a.b").is_ok());

        // Invalid domain names
        assert!(validate_domain_name("").is_err());
        assert!(validate_domain_name("-invalid.com").is_err());
        assert!(validate_domain_name("invalid-.com").is_err());
        assert!(validate_domain_name(".example.com").is_err());
        assert!(validate_domain_name("example.com.").is_err());
    }

    #[test]
    fn test_validate_domains() {
        use crate::db::Domain;

        // Valid domains
        assert!(validate_domains(&None).is_ok());
        assert!(validate_domains(&Some(vec![])).is_ok());
        assert!(validate_domains(&Some(vec![Domain::new("example.com".to_string()),])).is_ok());
        assert!(validate_domains(&Some(vec![
            Domain::primary("example.com".to_string()),
            Domain::new("www.example.com".to_string()),
        ]))
        .is_ok());

        // Invalid: duplicate domains
        assert!(validate_domains(&Some(vec![
            Domain::new("example.com".to_string()),
            Domain::new("example.com".to_string()),
        ]))
        .is_err());

        // Invalid: multiple primary domains
        assert!(validate_domains(&Some(vec![
            Domain::primary("example.com".to_string()),
            Domain::primary("other.com".to_string()),
        ]))
        .is_err());

        // Invalid: bad domain format
        assert!(validate_domains(&Some(vec![Domain::new("-invalid.com".to_string()),])).is_err());
    }
}
