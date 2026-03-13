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

/// Inject a dedicated per-service Docker network into compose content for isolation.
///
/// Appends a top-level `networks:` block that names the default network
/// `rivetr-svc-{network_suffix}` (first 8 chars of the service ID).  All
/// services in the stack can still reach each other (they share the default
/// network), but they are isolated from other Rivetr-managed stacks.
///
/// If the compose YAML already defines a top-level `networks:` key the
/// content is returned unchanged so user-defined network configs are not
/// overwritten.
pub fn inject_isolated_network(content: &str, service_id: &str) -> Result<String, String> {
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;

    // If networks key already exists, do not overwrite.
    if let Some(mapping) = yaml.as_mapping() {
        if mapping.contains_key(serde_yaml::Value::String("networks".to_string())) {
            return Ok(content.to_string());
        }
    }

    // Use the first 8 characters of the service ID for a short, stable name.
    let suffix = &service_id[..service_id.len().min(8)];
    let network_name = format!("rivetr-svc-{}", suffix);

    let network_block = format!(
        "\nnetworks:\n  default:\n    name: {}\n    driver: bridge\n",
        network_name
    );

    Ok(format!("{}{}", content.trim_end_matches('\n'), network_block))
}

/// Inject the shared `rivetr` Docker network into compose YAML.
///
/// Adds (or extends) the top-level `networks:` block with an external
/// `rivetr` network, and appends `rivetr` to each service's `networks` list.
/// This allows compose services to communicate with Rivetr-managed app
/// containers (which are all connected to the `rivetr` bridge) by hostname.
pub fn inject_rivetr_network(content: &str) -> Result<String, String> {
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;

    let mapping = match yaml.as_mapping_mut() {
        Some(m) => m,
        None => return Ok(content.to_string()),
    };

    // Ensure the top-level networks block contains rivetr as external
    {
        let networks_key = serde_yaml::Value::String("networks".to_string());
        let rivetr_key = serde_yaml::Value::String("rivetr".to_string());

        let networks_entry = mapping
            .entry(networks_key)
            .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));

        if let Some(nets_map) = networks_entry.as_mapping_mut() {
            if !nets_map.contains_key(&rivetr_key) {
                let mut rivetr_def = serde_yaml::Mapping::new();
                rivetr_def.insert(
                    serde_yaml::Value::String("external".to_string()),
                    serde_yaml::Value::Bool(true),
                );
                rivetr_def.insert(
                    serde_yaml::Value::String("name".to_string()),
                    serde_yaml::Value::String("rivetr".to_string()),
                );
                nets_map.insert(rivetr_key, serde_yaml::Value::Mapping(rivetr_def));
            }
        }
    }

    // Append rivetr to each service's networks list (sequence form only)
    let services_key = serde_yaml::Value::String("services".to_string());
    if let Some(services) = mapping.get_mut(&services_key) {
        if let Some(services_map) = services.as_mapping_mut() {
            for (_svc_key, svc_val) in services_map.iter_mut() {
                if let Some(svc_map) = svc_val.as_mapping_mut() {
                    let net_key = serde_yaml::Value::String("networks".to_string());
                    let rivetr_val = serde_yaml::Value::String("rivetr".to_string());

                    let svc_networks = svc_map
                        .entry(net_key)
                        .or_insert_with(|| serde_yaml::Value::Sequence(vec![]));

                    if let Some(seq) = svc_networks.as_sequence_mut() {
                        if !seq.contains(&rivetr_val) {
                            seq.push(rivetr_val);
                        }
                    }
                    // If networks is a mapping (named network configs), skip injection
                    // to avoid breaking user-defined complex network setups.
                }
            }
        }
    }

    serde_yaml::to_string(&yaml).map_err(|e| format!("Failed to serialize YAML: {}", e))
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

/// Write compose content to file.
///
/// Namespaces container names to prevent global conflicts and optionally
/// injects an isolated Docker network configuration.
pub async fn write_compose_file(
    data_dir: &Path,
    service_name: &str,
    content: &str,
) -> Result<PathBuf, std::io::Error> {
    write_compose_file_with_options(data_dir, service_name, content, None).await
}

/// Write compose content to file with optional network isolation.
///
/// `service_id` — when `Some`, injects a per-service Docker network so the
/// stack is isolated from other Rivetr-managed stacks.
pub async fn write_compose_file_with_options(
    data_dir: &Path,
    service_name: &str,
    content: &str,
    service_id: Option<&str>,
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

    // Inject per-service isolated network if requested
    let after_isolation = if let Some(id) = service_id {
        inject_isolated_network(&namespaced_content, id).unwrap_or_else(|e| {
            tracing::warn!(
                "Failed to inject isolated network for service {}: {}. Using content without isolation.",
                service_name,
                e
            );
            namespaced_content
        })
    } else {
        namespaced_content
    };

    // Always inject the shared rivetr external network so compose services can
    // communicate with Rivetr app containers by hostname.
    let final_content = inject_rivetr_network(&after_isolation).unwrap_or_else(|e| {
        tracing::warn!(
            "Failed to inject rivetr network into compose file for service {}: {}.",
            service_name,
            e
        );
        after_isolation
    });

    tokio::fs::write(&compose_file, final_content).await?;
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
