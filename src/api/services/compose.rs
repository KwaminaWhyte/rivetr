//! Docker Compose helper utilities used by CRUD and control handlers.

use std::path::{Path, PathBuf};
use tokio::process::Command;

use crate::db::ServiceGeneratedVar;

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

    Ok(format!(
        "{}{}",
        content.trim_end_matches('\n'),
        network_block
    ))
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

/// Inject a host port binding into the first service definition in a compose YAML.
///
/// Adds `"external_port:container_port"` to the `ports:` list of the **first** service
/// in the `services:` map.  If the entry is already present it is not duplicated.
///
/// Returns the original content unchanged if YAML parsing fails (non-fatal).
pub fn inject_public_ports(
    content: &str,
    external_port: i32,
    container_port: i32,
) -> Result<String, String> {
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;

    let port_entry = format!("{}:{}", external_port, container_port);
    let port_val = serde_yaml::Value::String(port_entry.clone());

    let mapping = match yaml.as_mapping_mut() {
        Some(m) => m,
        None => return Ok(content.to_string()),
    };

    let services_key = serde_yaml::Value::String("services".to_string());
    if let Some(services_val) = mapping.get_mut(&services_key) {
        if let Some(services_map) = services_val.as_mapping_mut() {
            // Inject into the first service definition only
            if let Some((_svc_key, svc_val)) = services_map.iter_mut().next() {
                if let Some(svc_map) = svc_val.as_mapping_mut() {
                    let ports_key = serde_yaml::Value::String("ports".to_string());
                    let ports = svc_map
                        .entry(ports_key)
                        .or_insert_with(|| serde_yaml::Value::Sequence(vec![]));

                    if let Some(seq) = ports.as_sequence_mut() {
                        if !seq.contains(&port_val) {
                            seq.push(port_val);
                        }
                    }
                    // If ports is a mapping (unusual), leave it untouched.
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
    write_compose_file_with_options(data_dir, service_name, content, None, false).await
}

/// Write compose content to file with optional network isolation.
///
/// `service_id` — when `Some`, injects a per-service Docker network so the
/// stack is isolated from other Rivetr-managed stacks.
///
/// `raw_mode` — when `true`, the compose file is written verbatim: no
/// container name namespacing, no isolated-network injection, and no shared
/// `rivetr` network injection are applied.
pub async fn write_compose_file_with_options(
    data_dir: &Path,
    service_name: &str,
    content: &str,
    service_id: Option<&str>,
    raw_mode: bool,
) -> Result<PathBuf, std::io::Error> {
    let dir = get_compose_dir(data_dir, service_name);
    tokio::fs::create_dir_all(&dir).await?;
    let compose_file = dir.join("docker-compose.yml");

    if raw_mode {
        // Raw mode: write the compose file exactly as provided, no modifications.
        tokio::fs::write(&compose_file, content).await?;
        return Ok(dir);
    }

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

/// Substitute magic variables in a compose YAML string.
///
/// Handles:
/// - `${SERVICE_PASSWORD_<SUFFIX>}` — random 32-char alphanumeric, persisted to DB
/// - `${SERVICE_BASE64_<SUFFIX>}` — random 32-byte base64-encoded, persisted to DB
/// - `${SERVICE_BASE64_64_<SUFFIX>}` — random 64-byte base64-encoded, persisted to DB
/// - `${SERVICE_FQDN_<SUFFIX>}` — the service domain (if set), or empty
/// - `${SERVICE_URL_<SUFFIX>}` — `https://` + domain (if set), or empty
/// - `${VAR:?error message}` — required var: if VAR is missing from `existing_vars`, returns `Err`
///
/// `existing_vars` maps key → value (unencrypted) for variables already known.
/// `service_domain` is the service's configured domain, if any.
/// `db` is used to load/persist generated PASSWORD/BASE64 values.
/// `service_id` is the DB id of the service.
///
/// When `dry_run` is `true` the function generates values but does NOT write to the DB
/// (used for the preview endpoint).
pub async fn substitute_magic_vars(
    compose_yaml: &str,
    service_id: &str,
    service_domain: Option<&str>,
    existing_vars: &std::collections::HashMap<String, String>,
    db: &sqlx::SqlitePool,
    dry_run: bool,
) -> Result<String, String> {
    use base64::Engine as _;
    use rand::Rng as _;

    let mut content = compose_yaml.to_string();

    // -----------------------------------------------------------------------
    // 1. ${VAR:?message} — required variable check
    // -----------------------------------------------------------------------
    {
        let req_re = regex::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\:\?([^}]*)\}")
            .map_err(|e| format!("regex error: {e}"))?;

        for cap in req_re.captures_iter(&content.clone()) {
            let var_name = &cap[1];
            let error_msg = &cap[2];
            if !existing_vars.contains_key(var_name) {
                return Err(format!(
                    "Required variable {var_name} is not set: {error_msg}"
                ));
            }
        }

        // Substitute required vars that ARE set
        for (key, val) in existing_vars {
            let re = regex::Regex::new(&format!(r"\$\{{{key}:[?][^}}]*\}}"))
                .map_err(|e| format!("regex error: {e}"))?;
            content = re.replace_all(&content, val.as_str()).to_string();
        }
    }

    // -----------------------------------------------------------------------
    // 2. ${SERVICE_FQDN_<SUFFIX>} and ${SERVICE_URL_<SUFFIX>}
    // -----------------------------------------------------------------------
    {
        let fqdn_re = regex::Regex::new(r"\$\{SERVICE_FQDN_([A-Z0-9_]+)(?::-[^}]*)?\}")
            .map_err(|e| format!("regex error: {e}"))?;
        let url_re = regex::Regex::new(r"\$\{SERVICE_URL_([A-Z0-9_]+)(?::-[^}]*)?\}")
            .map_err(|e| format!("regex error: {e}"))?;

        let domain_value = service_domain.unwrap_or("").to_string();
        let url_value = if domain_value.is_empty() {
            String::new()
        } else {
            format!("https://{}", domain_value)
        };

        // Replace ${SERVICE_FQDN_*} and ${SERVICE_FQDN_*:-default}
        let new_content = fqdn_re
            .replace_all(&content, domain_value.as_str())
            .to_string();
        content = new_content;

        // Replace ${SERVICE_URL_*} and ${SERVICE_URL_*:-default}
        let new_content = url_re.replace_all(&content, url_value.as_str()).to_string();
        content = new_content;
    }

    // -----------------------------------------------------------------------
    // 3. ${SERVICE_PASSWORD_<SUFFIX>} and ${SERVICE_BASE64_*} — generated, persisted
    // -----------------------------------------------------------------------
    {
        let magic_re = regex::Regex::new(
            r"\$\{(SERVICE_(?:PASSWORD|BASE64(?:_64)?)_([A-Z0-9_]+))(?::-[^}]*)?\}",
        )
        .map_err(|e| format!("regex error: {e}"))?;

        // Collect unique variable names that appear in the compose content
        let mut to_resolve: Vec<(String, String)> = Vec::new(); // (full_var, suffix_kind)
        for cap in magic_re.captures_iter(&content.clone()) {
            let full_var = cap[1].to_string();
            if to_resolve.iter().any(|(v, _)| v == &full_var) {
                continue;
            }
            to_resolve.push((full_var, cap[2].to_string()));
        }

        let mut generated: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for (full_var, _suffix) in &to_resolve {
            // Try loading from DB first (for stable values across restarts)
            let existing: Option<ServiceGeneratedVar> = sqlx::query_as(
                "SELECT * FROM service_generated_vars WHERE service_id = ? AND key = ?",
            )
            .bind(service_id)
            .bind(full_var)
            .fetch_optional(db)
            .await
            .map_err(|e| format!("DB error loading generated var: {e}"))?;

            if let Some(row) = existing {
                generated.insert(full_var.clone(), row.value);
                continue;
            }

            // Generate a new value
            let value: String = if full_var.starts_with("SERVICE_PASSWORD_") {
                let mut rng = rand::rng();
                let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
                (0..32)
                    .map(|_| chars[rng.random_range(0..chars.len())] as char)
                    .collect()
            } else if full_var.starts_with("SERVICE_BASE64_64_") {
                // 64-byte random → base64
                let mut rng = rand::rng();
                let bytes: Vec<u8> = (0..64).map(|_| rng.random::<u8>()).collect();
                base64::engine::general_purpose::STANDARD.encode(&bytes)
            } else {
                // SERVICE_BASE64_* (32-byte)
                let mut rng = rand::rng();
                let bytes: Vec<u8> = (0..32).map(|_| rng.random::<u8>()).collect();
                base64::engine::general_purpose::STANDARD.encode(&bytes)
            };

            if !dry_run {
                // Persist to DB
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT OR REPLACE INTO service_generated_vars (id, service_id, key, value, updated_at) \
                     VALUES (?, ?, ?, ?, datetime('now'))",
                )
                .bind(&id)
                .bind(service_id)
                .bind(full_var)
                .bind(&value)
                .execute(db)
                .await
                .map_err(|e| format!("DB error saving generated var: {e}"))?;
            }

            generated.insert(full_var.clone(), value);
        }

        // Apply substitutions — both simple `${VAR}` and `${VAR:-default}` forms
        for (var, value) in &generated {
            content = content.replace(&format!("${{{var}}}"), value);
            let re2 = regex::Regex::new(&format!(r"\$\{{{var}:-[^}}]*\}}"))
                .map_err(|e| format!("regex error: {e}"))?;
            content = re2.replace_all(&content, value.as_str()).to_string();
        }
    }

    Ok(content)
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
