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

/// Inject top-level `cpus` / `mem_limit` keys into every service in the compose
/// file, so the service-level CPU/memory caps configured in Rivetr are enforced
/// by Docker's cgroups. These keys (unlike `deploy.resources.limits`, which is
/// swarm-only) are honored by `docker compose up`. A service that already sets a
/// key keeps its own value (explicit config wins).
///
/// `cpu_limit` (e.g. "0.5") and `memory_limit` (e.g. "512M") are optional; only
/// the provided ones are injected. If neither is set the content is returned
/// unchanged.
pub fn inject_resource_limits(
    content: &str,
    cpu_limit: Option<&str>,
    memory_limit: Option<&str>,
) -> Result<String, String> {
    let cpu = cpu_limit.map(|s| s.trim()).filter(|s| !s.is_empty());
    let mem = memory_limit.map(|s| s.trim()).filter(|s| !s.is_empty());
    if cpu.is_none() && mem.is_none() {
        return Ok(content.to_string());
    }

    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;

    let mapping = match yaml.as_mapping_mut() {
        Some(m) => m,
        None => return Ok(content.to_string()),
    };

    let services_key = serde_yaml::Value::String("services".to_string());
    let Some(services_val) = mapping.get_mut(&services_key) else {
        return Ok(content.to_string());
    };
    let Some(services_map) = services_val.as_mapping_mut() else {
        return Ok(content.to_string());
    };

    let str_val = |s: &str| serde_yaml::Value::String(s.to_string());

    for (_svc_key, svc_val) in services_map.iter_mut() {
        let Some(svc_map) = svc_val.as_mapping_mut() else {
            continue;
        };

        // Use the top-level `cpus` / `mem_limit` keys: unlike
        // `deploy.resources.limits` (swarm-only), these are honored by
        // `docker compose up` and map directly to container cgroup limits.
        // A service that already sets a key keeps its own value.
        if let Some(c) = cpu {
            let cpus_key = str_val("cpus");
            if !svc_map.contains_key(&cpus_key) {
                // `cpus` is numeric in the Compose spec; fall back to a string
                // if the value somehow doesn't parse as a float.
                let val = match c.parse::<f64>() {
                    Ok(f) => serde_yaml::Value::Number(serde_yaml::Number::from(f)),
                    Err(_) => str_val(c),
                };
                svc_map.insert(cpus_key, val);
            }
        }
        if let Some(m) = mem {
            let mem_key = str_val("mem_limit");
            if !svc_map.contains_key(&mem_key) {
                svc_map.insert(mem_key, str_val(m));
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
    use rand::RngExt as _;

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

/// Run a docker compose command while streaming each stdout/stderr line to a
/// callback. Used by the start path so the deploy side panel can render
/// `docker compose up` output (image pulls, container creation) live.
///
/// Returns the captured stdout on success, or stderr on failure (matching
/// `run_compose_command` semantics).
pub async fn run_compose_command_streaming<F>(
    project_dir: &std::path::Path,
    project_name: &str,
    args: &[&str],
    mut on_line: F,
) -> Result<String, String>
where
    F: FnMut(&str, bool),
{
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, BufReader};

    // Build command — try modern `docker compose` first, fall back to legacy
    // `docker-compose` if Docker is missing.
    let mut cmd = tokio::process::Command::new("docker");
    cmd.arg("compose")
        .arg("-p")
        .arg(project_name)
        .args(args)
        .current_dir(project_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => {
            // Fallback to docker-compose (legacy)
            let mut legacy = tokio::process::Command::new("docker-compose");
            legacy
                .arg("-p")
                .arg(project_name)
                .args(args)
                .current_dir(project_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            legacy
                .spawn()
                .map_err(|e| format!("Failed to execute docker-compose: {}", e))?
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let mut captured_stdout = String::new();
    let mut captured_stderr = String::new();

    let stdout_task = stdout.map(|s| {
        let reader = BufReader::new(s);
        reader.lines()
    });
    let stderr_task = stderr.map(|s| {
        let reader = BufReader::new(s);
        reader.lines()
    });

    if let (Some(mut out_lines), Some(mut err_lines)) = (stdout_task, stderr_task) {
        loop {
            tokio::select! {
                line = out_lines.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            on_line(&l, false);
                            captured_stdout.push_str(&l);
                            captured_stdout.push('\n');
                        }
                        _ => break,
                    }
                }
                line = err_lines.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            // Docker writes pull progress to stderr. Treat it as
                            // info for the panel, not an error.
                            on_line(&l, true);
                            captured_stderr.push_str(&l);
                            captured_stderr.push('\n');
                        }
                        _ => break,
                    }
                }
            }
        }

        // Drain any remaining lines after one of the readers returned None.
        while let Ok(Some(l)) = out_lines.next_line().await {
            on_line(&l, false);
            captured_stdout.push_str(&l);
            captured_stdout.push('\n');
        }
        while let Ok(Some(l)) = err_lines.next_line().await {
            on_line(&l, true);
            captured_stderr.push_str(&l);
            captured_stderr.push('\n');
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| format!("Failed to await child: {}", e))?;

    if status.success() {
        Ok(captured_stdout)
    } else if !captured_stderr.is_empty() {
        Err(captured_stderr)
    } else {
        Err(format!("docker compose exited with status {}", status))
    }
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

/// Why rendering a service's compose file failed.
#[derive(Debug)]
pub enum RenderError {
    /// Magic variable substitution failed (e.g. a required `${VAR:?}` is unset).
    Substitution(String),
    /// The compose file could not be written to disk.
    Write(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Substitution(e) => write!(f, "{}", e),
            Self::Write(e) => write!(f, "Failed to write compose file: {}", e),
        }
    }
}

/// Render a service's compose content the way every start path does it:
/// magic variables, the public port binding and CPU/memory limits (the last two
/// skipped in raw mode). The result is what [`write_service_compose`] writes.
pub async fn render_service_compose(
    db: &sqlx::SqlitePool,
    service: &crate::db::Service,
    key: Option<&[u8; 32]>,
) -> Result<String, RenderError> {
    render_service_compose_with(db, service, None, key).await
}

/// Like [`render_service_compose`], with explicit image pins. `None` uses the
/// pins a rollback stored on the service (what start/restart need); a deploy
/// passes its own (empty for a regular deploy, which moves back to the tags).
pub async fn render_service_compose_with(
    db: &sqlx::SqlitePool,
    service: &crate::db::Service,
    pins: Option<&std::collections::BTreeMap<String, String>>,
    key: Option<&[u8; 32]>,
) -> Result<String, RenderError> {
    let gen_vars_rows: Vec<ServiceGeneratedVar> =
        sqlx::query_as("SELECT * FROM service_generated_vars WHERE service_id = ?")
            .bind(&service.id)
            .fetch_all(db)
            .await
            .unwrap_or_default();
    // Service env vars count as set for `${VAR:?}` checks; generated vars win
    // on a name clash.
    let env_vars = load_service_env(db, &service.id, key).await;
    let mut existing_vars: std::collections::HashMap<String, String> =
        env_vars.iter().cloned().collect();
    existing_vars.extend(gen_vars_rows.into_iter().map(|r| (r.key, r.value)));

    let substituted = substitute_magic_vars(
        &service.compose_content,
        &service.id,
        service.domain.as_deref(),
        &existing_vars,
        db,
        false,
    )
    .await
    .map_err(RenderError::Substitution)?;

    let raw_mode = service.raw_compose_mode != 0;

    let content = if !raw_mode
        && service.public_access != 0
        && service.external_port != 0
        && service.expose_container_port != 0
    {
        match inject_public_ports(
            &substituted,
            service.external_port,
            service.expose_container_port,
        ) {
            Ok(injected) => injected,
            Err(e) => {
                tracing::warn!(
                    "Failed to inject public ports for service {}: {}. Using compose without port injection.",
                    service.name,
                    e
                );
                substituted
            }
        }
    } else {
        substituted
    };

    let cfg_row: Option<(Option<String>, i32)> = sqlx::query_as(
        "SELECT pinned_images, inject_env_file FROM service_deploy_configs WHERE service_id = ?",
    )
    .bind(&service.id)
    .fetch_optional(db)
    .await
    .unwrap_or_default();
    let (stored_json, inject_env) = cfg_row.unwrap_or((None, 0));
    let stored_pins: std::collections::BTreeMap<String, String> = stored_json
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default();
    let pins = pins.unwrap_or(&stored_pins);
    let content = if pins.is_empty() {
        content
    } else {
        apply_image_pins(&content, pins).unwrap_or_else(|e| {
            tracing::warn!("Failed to pin images for service {}: {}", service.name, e);
            content
        })
    };
    let content = if inject_env != 0 && !env_vars.is_empty() {
        inject_env_file_ref(&content, ENV_FILE_NAME).unwrap_or_else(|e| {
            tracing::warn!("Failed to add env_file to service {}: {}", service.name, e);
            content
        })
    } else {
        content
    };

    let content = if !raw_mode && (service.cpu_limit.is_some() || service.memory_limit.is_some()) {
        match inject_resource_limits(
            &content,
            service.cpu_limit.as_deref(),
            service.memory_limit.as_deref(),
        ) {
            Ok(injected) => injected,
            Err(e) => {
                tracing::warn!(
                    "Failed to inject resource limits for service {}: {}. Using compose without limit injection.",
                    service.name,
                    e
                );
                content
            }
        }
    } else {
        content
    };

    Ok(content)
}

/// Write rendered compose content for a service, applying container name
/// namespacing and network injection unless raw mode is on.
pub async fn write_service_compose(
    data_dir: &Path,
    service: &crate::db::Service,
    content: &str,
) -> Result<PathBuf, RenderError> {
    let raw_mode = service.raw_compose_mode != 0;
    let isolated_id = if !raw_mode && service.isolated_network != 0 {
        Some(service.id.as_str())
    } else {
        None
    };
    write_compose_file_with_options(data_dir, &service.name, content, isolated_id, raw_mode)
        .await
        .map_err(|e| RenderError::Write(e.to_string()))
}

/// Render and write a service's compose file in one step.
pub async fn render_and_write_service_compose(
    db: &sqlx::SqlitePool,
    data_dir: &Path,
    service: &crate::db::Service,
    key: Option<&[u8; 32]>,
) -> Result<PathBuf, RenderError> {
    let content = render_service_compose(db, service, key).await?;
    let dir = write_service_compose(data_dir, service, &content).await?;
    write_service_env_file(db, data_dir, service, key).await?;
    Ok(dir)
}

/// Name of the Rivetr-managed env file in a service's compose directory.
pub const ENV_FILE_NAME: &str = ".env";
const ENV_FILE_HEADER: &str = "# Managed by Rivetr: edit these in the service's Environment tab.";

/// Decrypted `(key, value)` pairs of a service's env vars, sorted by key.
pub async fn load_service_env(
    db: &sqlx::SqlitePool,
    service_id: &str,
    key: Option<&[u8; 32]>,
) -> Vec<(String, String)> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM service_env_vars WHERE service_id = ? ORDER BY key")
            .bind(service_id)
            .fetch_all(db)
            .await
            .unwrap_or_default();
    rows.into_iter()
        .map(|(k, v)| {
            let plain = crate::crypto::decrypt_if_encrypted(&v, key).unwrap_or(v);
            (k, plain)
        })
        .collect()
}

/// Quote a value for compose's dotenv parser: single quotes keep it literal;
/// values with a `'` or a newline use double quotes with `\`, `\"`, `\$`
/// and `\n` escapes.
pub fn quote_env_value(value: &str) -> String {
    if !value.contains('\'') && !value.contains('\n') && !value.contains('\r') {
        return format!("'{}'", value);
    }
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '$' => out.push_str("\\$"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Contents of the `.env` file for a set of variables.
pub fn format_env_file(vars: &[(String, String)]) -> String {
    let mut out = String::from(ENV_FILE_HEADER);
    out.push('\n');
    for (k, v) in vars {
        out.push_str(k);
        out.push('=');
        out.push_str(&quote_env_value(v));
        out.push('\n');
    }
    out
}

/// Parse a pasted `.env` file into `(key, value)` pairs. Handles comments,
/// `export KEY=...`, single quotes (literal) and double quotes (escapes).
pub fn parse_env_file(content: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let v = v.trim();
        let value = if let Some(inner) = v.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
            inner.to_string()
        } else if let Some(inner) = v.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
            let mut s = String::new();
            let mut chars = inner.chars();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    match chars.next() {
                        Some('n') => s.push('\n'),
                        Some('r') => s.push('\r'),
                        Some('t') => s.push('\t'),
                        Some(o) => s.push(o),
                        None => s.push('\\'),
                    }
                } else {
                    s.push(c);
                }
            }
            s
        } else {
            // Unquoted: an inline comment starts at " #".
            match v.find(" #") {
                Some(i) => v[..i].trim_end().to_string(),
                None => v.to_string(),
            }
        };
        out.push((k.to_string(), value));
    }
    out
}

/// Write (or remove) the Rivetr-managed `.env` next to the compose file.
/// A `.env` Rivetr did not write is never overwritten or deleted.
pub async fn write_service_env_file(
    db: &sqlx::SqlitePool,
    data_dir: &Path,
    service: &crate::db::Service,
    key: Option<&[u8; 32]>,
) -> Result<(), RenderError> {
    let path = get_compose_dir(data_dir, &service.name).join(ENV_FILE_NAME);
    let vars = load_service_env(db, &service.id, key).await;
    let existing = tokio::fs::read_to_string(&path).await.ok();
    let ours = existing
        .as_deref()
        .map(|c| c.starts_with(ENV_FILE_HEADER))
        .unwrap_or(true);
    if !ours {
        if !vars.is_empty() {
            tracing::warn!(
                "Not writing {}: the file was not created by Rivetr",
                path.display()
            );
        }
        return Ok(());
    }
    if vars.is_empty() {
        if existing.is_some() {
            let _ = tokio::fs::remove_file(&path).await;
        }
        return Ok(());
    }
    let content = format_env_file(&vars);
    // Write to a 0600 temp file and rename so the secrets are never world-readable.
    let tmp = path.with_extension("rivetr-tmp");
    {
        let mut opts = tokio::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        opts.mode(0o600);
        let mut f = opts
            .open(&tmp)
            .await
            .map_err(|e| RenderError::Write(e.to_string()))?;
        use tokio::io::AsyncWriteExt;
        f.write_all(content.as_bytes())
            .await
            .map_err(|e| RenderError::Write(e.to_string()))?;
    }
    tokio::fs::rename(&tmp, &path)
        .await
        .map_err(|e| RenderError::Write(e.to_string()))?;
    Ok(())
}

/// Add `env_file` to every compose service so its containers get the
/// Rivetr-managed variables. An existing env_file (string or list) is kept and
/// the Rivetr file is appended once.
pub fn inject_env_file_ref(content: &str, file: &str) -> Result<String, String> {
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;
    let Some(services) = yaml.get_mut("services").and_then(|s| s.as_mapping_mut()) else {
        return Ok(content.to_string());
    };
    let file_val = serde_yaml::Value::String(file.to_string());
    let key = serde_yaml::Value::String("env_file".to_string());
    for (_, def) in services.iter_mut() {
        let Some(def) = def.as_mapping_mut() else {
            continue;
        };
        let entry = def
            .entry(key.clone())
            .or_insert_with(|| serde_yaml::Value::Sequence(vec![]));
        if let serde_yaml::Value::String(one) = entry.clone() {
            *entry = serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(one)]);
        }
        if let Some(seq) = entry.as_sequence_mut() {
            let present = seq
                .iter()
                .any(|v| v == &file_val || v.get("path").and_then(|p| p.as_str()) == Some(file));
            if !present {
                seq.push(file_val.clone());
            }
        }
    }
    serde_yaml::to_string(&yaml).map_err(|e| format!("Failed to serialize YAML: {}", e))
}

#[cfg(test)]
mod env_file_tests {
    use super::*;

    #[test]
    fn env_values_round_trip_through_quoting() {
        let vars = vec![
            ("A".to_string(), "plain $HOME ${X} \"q\" # x".to_string()),
            ("B".to_string(), "it's \"dq\"\nline2 \\ $HOME".to_string()),
            ("C".to_string(), String::new()),
        ];
        let file = format_env_file(&vars);
        assert!(file.starts_with(ENV_FILE_HEADER));
        assert!(file.contains("A='plain $HOME ${X} \"q\" # x'"));
        assert!(file.contains("B=\"it's \\\"dq\\\"\\nline2 \\\\ \\$HOME\""));
        assert_eq!(parse_env_file(&file), vars);
    }

    #[test]
    fn parses_pasted_env_files() {
        let parsed = parse_env_file(
            "# comment\nexport DATABASE_URL=postgres://u:p@db/x\nEMPTY=\nPORT=3000 # inline\nbad line\n1X=y\n",
        );
        assert_eq!(
            parsed,
            vec![
                (
                    "DATABASE_URL".to_string(),
                    "postgres://u:p@db/x".to_string()
                ),
                ("EMPTY".to_string(), String::new()),
                ("PORT".to_string(), "3000".to_string()),
                ("1X".to_string(), "y".to_string()),
            ]
        );
    }

    #[test]
    fn env_file_is_added_once_next_to_existing_files() {
        let compose = "services:\n  api:\n    image: a\n    env_file:\n    - /etc/megasign/megasign.env\n  web:\n    image: a\n    env_file: other.env\n  db:\n    image: b\n";
        let out = inject_env_file_ref(compose, ".env").unwrap();
        let again = inject_env_file_ref(&out, ".env").unwrap();
        let y: serde_yaml::Value = serde_yaml::from_str(&again).unwrap();
        assert_eq!(
            y["services"]["api"]["env_file"][0],
            "/etc/megasign/megasign.env"
        );
        assert_eq!(y["services"]["api"]["env_file"][1], ".env");
        assert_eq!(
            y["services"]["api"]["env_file"]
                .as_sequence()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(y["services"]["web"]["env_file"][0], "other.env");
        assert_eq!(y["services"]["web"]["env_file"][1], ".env");
        assert_eq!(y["services"]["db"]["env_file"][0], ".env");
    }
}

/// Replace the `image:` of each listed compose service with a pinned
/// reference (`repo@sha256:...` or a local image ID). Services that are not
/// listed, or have no `image:` key, are left as they are.
pub fn apply_image_pins(
    content: &str,
    pins: &std::collections::BTreeMap<String, String>,
) -> Result<String, String> {
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;
    let Some(services) = yaml.get_mut("services").and_then(|s| s.as_mapping_mut()) else {
        return Ok(content.to_string());
    };
    let mut changed = false;
    for (name, def) in services.iter_mut() {
        let (Some(name), Some(def)) = (name.as_str(), def.as_mapping_mut()) else {
            continue;
        };
        let Some(pin) = pins.get(name) else { continue };
        let key = serde_yaml::Value::String("image".to_string());
        if def.contains_key(&key) {
            def.insert(key, serde_yaml::Value::String(pin.clone()));
            changed = true;
        }
    }
    if !changed {
        return Ok(content.to_string());
    }
    serde_yaml::to_string(&yaml).map_err(|e| format!("Failed to serialize YAML: {}", e))
}

#[cfg(test)]
mod pin_tests {
    use super::*;

    #[test]
    fn pins_replace_only_listed_images() {
        let compose = "services:\n  web:\n    image: localhost:5000/app:latest\n  db:\n    image: postgres:18\n  local:\n    build: .\n";
        let mut pins = std::collections::BTreeMap::new();
        pins.insert(
            "web".to_string(),
            "localhost:5000/app@sha256:aaa".to_string(),
        );
        pins.insert("local".to_string(), "sha256:bbb".to_string());
        pins.insert("gone".to_string(), "x@sha256:ccc".to_string());
        let out = apply_image_pins(compose, &pins).unwrap();
        let y: serde_yaml::Value = serde_yaml::from_str(&out).unwrap();
        assert_eq!(
            y["services"]["web"]["image"],
            "localhost:5000/app@sha256:aaa"
        );
        assert_eq!(y["services"]["db"]["image"], "postgres:18");
        assert!(y["services"]["local"].get("image").is_none());
        assert_eq!(
            apply_image_pins(compose, &Default::default()).unwrap(),
            compose
        );
    }
}
