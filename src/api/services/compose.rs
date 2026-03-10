//! Docker Compose helper utilities used by CRUD and control handlers.

use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Validate docker-compose content
/// Checks that it's valid YAML with a 'services' key
pub fn validate_compose_content(content: &str) -> Result<(), String> {
    // Check it's not completely empty/whitespace
    if content.trim().is_empty() {
        return Err("Compose file is empty".to_string());
    }

    // Parse as YAML
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;

    // Check for 'services' key
    let mapping = yaml
        .as_mapping()
        .ok_or_else(|| "Compose file must be a YAML mapping".to_string())?;

    // Check for 'services' key (required in docker-compose)
    if !mapping.contains_key(serde_yaml::Value::String("services".to_string())) {
        return Err("Compose file must contain a 'services' key".to_string());
    }

    Ok(())
}

/// Get the compose file path for a service
pub fn get_compose_dir(data_dir: &Path, service_name: &str) -> PathBuf {
    data_dir.join("services").join(service_name)
}

/// Get the compose directory for a service, checking both data dir and temp dir
pub fn get_service_compose_dir(data_dir: &Path, service_name: &str) -> PathBuf {
    // First try the standard data directory
    let data_compose_dir = get_compose_dir(data_dir, service_name);
    if data_compose_dir.join("docker-compose.yml").exists() {
        return data_compose_dir;
    }

    // Try the temp directory (used by template deployments)
    let temp_compose_dir = std::env::temp_dir().join(format!("rivetr-svc-{}", service_name));
    if temp_compose_dir.join("docker-compose.yml").exists() {
        return temp_compose_dir;
    }

    // Default to data directory even if it doesn't exist
    data_compose_dir
}

/// Namespace container names in compose content to prevent global conflicts
/// Prefixes all container_name values with "rivetr-{service_name}-"
pub fn namespace_container_names(content: &str, service_name: &str) -> Result<String, String> {
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;

    let prefix = format!("rivetr-{}-", service_name);

    if let Some(mapping) = yaml.as_mapping_mut() {
        if let Some(services) = mapping.get_mut(serde_yaml::Value::String("services".to_string())) {
            if let Some(services_map) = services.as_mapping_mut() {
                for (_service_key, service_config) in services_map.iter_mut() {
                    if let Some(config_map) = service_config.as_mapping_mut() {
                        let container_name_key =
                            serde_yaml::Value::String("container_name".to_string());
                        if let Some(container_name_val) = config_map.get_mut(&container_name_key) {
                            if let Some(name) = container_name_val.as_str() {
                                // Only add prefix if not already prefixed
                                if !name.starts_with(&prefix) && !name.starts_with("rivetr-") {
                                    *container_name_val =
                                        serde_yaml::Value::String(format!("{}{}", prefix, name));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    serde_yaml::to_string(&yaml).map_err(|e| format!("Failed to serialize YAML: {}", e))
}

/// Write compose content to file
/// Namespaces container names to prevent global conflicts
pub async fn write_compose_file(
    data_dir: &Path,
    service_name: &str,
    content: &str,
) -> Result<PathBuf, std::io::Error> {
    let dir = get_compose_dir(data_dir, service_name);
    tokio::fs::create_dir_all(&dir).await?;
    let compose_file = dir.join("docker-compose.yml");

    // Namespace container names to prevent global conflicts
    let namespaced_content = namespace_container_names(content, service_name).unwrap_or_else(|e| {
        tracing::warn!(
            "Failed to namespace container names: {}. Using original content.",
            e
        );
        content.to_string()
    });

    tokio::fs::write(&compose_file, namespaced_content).await?;
    Ok(dir)
}

/// Run docker compose command
pub async fn run_compose_command(
    project_dir: &std::path::Path,
    project_name: &str,
    args: &[&str],
) -> Result<String, String> {
    // Try docker compose first (modern), then docker-compose (legacy)
    let result = Command::new("docker")
        .arg("compose")
        .arg("-p")
        .arg(project_name)
        .args(args)
        .current_dir(project_dir)
        .output()
        .await;

    match result {
        Ok(output) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
        Err(_) => {
            // Try legacy docker-compose command
            let output = Command::new("docker-compose")
                .arg("-p")
                .arg(project_name)
                .args(args)
                .current_dir(project_dir)
                .output()
                .await
                .map_err(|e| format!("Failed to execute docker-compose: {}", e))?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
    }
}
