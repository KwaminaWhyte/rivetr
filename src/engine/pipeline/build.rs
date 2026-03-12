use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::db::App;
use crate::engine::nixpacks;
use crate::engine::pack_builder;
use crate::engine::railpack;
use crate::engine::static_builder::{StaticSiteBuilder, StaticSiteConfig};
use crate::runtime::{BuildContext, ContainerRuntime};
use crate::DbPool;

use super::super::{add_deployment_log, BuildLimits, KEY_LENGTH};

/// Push a built image to a Docker registry if the app has registry push enabled.
/// Uses `docker tag` + `docker login` + `docker push` CLI commands.
pub(super) async fn push_image_to_registry(
    db: &DbPool,
    deployment_id: &str,
    app: &App,
    image_tag: &str,
    encryption_key: Option<&[u8; super::super::KEY_LENGTH]>,
) -> Result<()> {
    use crate::crypto;
    use tokio::process::Command;

    if !app.is_registry_push_enabled() {
        return Ok(());
    }

    let registry_url = match app.registry_url.as_deref().filter(|s| !s.is_empty()) {
        Some(url) => url.to_string(),
        None => {
            add_deployment_log(
                db,
                deployment_id,
                "warn",
                "Registry push is enabled but no registry URL is configured — skipping push",
            )
            .await?;
            return Ok(());
        }
    };

    // Short deployment ID for the remote tag (first 8 chars)
    let short_id = if deployment_id.len() >= 8 {
        &deployment_id[..8]
    } else {
        deployment_id
    };
    let remote_tag = format!(
        "{}/{app_name}:{short_id}",
        registry_url,
        app_name = app.name
    );

    add_deployment_log(
        db,
        deployment_id,
        "info",
        &format!("Pushing image to registry: {}", remote_tag),
    )
    .await?;

    // Step 1: tag
    let tag_output = Command::new("docker")
        .args(["tag", image_tag, &remote_tag])
        .output()
        .await
        .context("Failed to run docker tag")?;

    if !tag_output.status.success() {
        let stderr = String::from_utf8_lossy(&tag_output.stderr);
        anyhow::bail!("docker tag failed: {}", stderr);
    }

    // Step 2: docker login (only if credentials are provided)
    if let Some(username) = app.registry_username.as_deref().filter(|s| !s.is_empty()) {
        let raw_password = app.registry_password.clone().unwrap_or_default();
        let password =
            crypto::decrypt_if_encrypted(&raw_password, encryption_key).unwrap_or(raw_password);

        if !password.is_empty() {
            let login_output = Command::new("docker")
                .args(["login", &registry_url, "-u", username, "--password-stdin"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("Failed to spawn docker login")?
                .wait_with_output()
                .await;

            // If spawn/piped stdin approach is not available, fall back to -p flag
            // (less secure but simpler)
            let login_output = match login_output {
                Ok(o) if o.status.success() => o,
                _ => {
                    let fallback = Command::new("docker")
                        .args(["login", &registry_url, "-u", username, "-p", &password])
                        .output()
                        .await
                        .context("Failed to run docker login")?;
                    fallback
                }
            };

            if !login_output.status.success() {
                let stderr = String::from_utf8_lossy(&login_output.stderr);
                anyhow::bail!("docker login failed: {}", stderr);
            }
        }
    }

    // Step 3: docker push
    let push_output = Command::new("docker")
        .args(["push", &remote_tag])
        .output()
        .await
        .context("Failed to run docker push")?;

    if !push_output.status.success() {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        anyhow::bail!("docker push failed: {}", stderr);
    }

    add_deployment_log(
        db,
        deployment_id,
        "info",
        &format!("Image pushed to registry: {}", remote_tag),
    )
    .await?;

    Ok(())
}

/// Execute deployment commands (pre or post) in a container
/// For pre-deploy: runs in a temporary container using the built image
/// For post-deploy: runs in the running container using docker exec
pub(super) async fn execute_deployment_commands(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    container_id: &str,
    commands: &[String],
    phase: &str,
) -> Result<()> {
    if commands.is_empty() {
        return Ok(());
    }

    add_deployment_log(
        db,
        deployment_id,
        "info",
        &format!(
            "Executing {} deployment commands ({} total)",
            phase,
            commands.len()
        ),
    )
    .await?;

    for (i, command) in commands.iter().enumerate() {
        add_deployment_log(
            db,
            deployment_id,
            "info",
            &format!("[{}/{}] Running: {}", i + 1, commands.len(), command),
        )
        .await?;

        // Execute command in container using shell
        let cmd = vec!["/bin/sh".to_string(), "-c".to_string(), command.clone()];
        let result = runtime.run_command(container_id, cmd).await?;

        // Log command output
        if !result.stdout.is_empty() {
            // Truncate very long output
            let stdout = if result.stdout.len() > 4000 {
                format!("{}... (truncated)", &result.stdout[..4000])
            } else {
                result.stdout.clone()
            };
            add_deployment_log(
                db,
                deployment_id,
                "info",
                &format!("Output: {}", stdout.trim()),
            )
            .await?;
        }

        if !result.stderr.is_empty() {
            let stderr = if result.stderr.len() > 4000 {
                format!("{}... (truncated)", &result.stderr[..4000])
            } else {
                result.stderr.clone()
            };
            add_deployment_log(
                db,
                deployment_id,
                "warn",
                &format!("Stderr: {}", stderr.trim()),
            )
            .await?;
        }

        // Check exit code
        if result.exit_code != 0 {
            add_deployment_log(
                db,
                deployment_id,
                "error",
                &format!(
                    "Command failed with exit code {}: {}",
                    result.exit_code, command
                ),
            )
            .await?;
            anyhow::bail!(
                "{} command failed with exit code {}: {}",
                phase,
                result.exit_code,
                command
            );
        }

        add_deployment_log(
            db,
            deployment_id,
            "info",
            &format!(
                "[{}/{}] Command completed successfully",
                i + 1,
                commands.len()
            ),
        )
        .await?;
    }

    add_deployment_log(
        db,
        deployment_id,
        "info",
        &format!("All {} deployment commands completed", phase),
    )
    .await?;

    Ok(())
}

/// Build the image for a git-based deployment
pub(super) async fn build_git_image(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    app: &App,
    build_path: &Path,
    build_limits: &BuildLimits,
    encryption_key: Option<&[u8; KEY_LENGTH]>,
) -> Result<String> {
    let image_tag = format!("rivetr-{}:{}", app.name, deployment_id);

    // If a build server is assigned, offload the Docker build step to that remote server
    if let Some(ref build_server_id) = app.build_server_id {
        return run_remote_build(
            db,
            deployment_id,
            app,
            build_path,
            &image_tag,
            build_server_id,
            encryption_key,
        )
        .await;
    }

    let build_type = app.get_build_type();

    match build_type {
        "nixpacks" => {
            // Nixpacks build
            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Building with Nixpacks (auto-detecting language and framework)...",
            )
            .await?;

            // Check if Nixpacks is available
            if !nixpacks::is_available().await {
                anyhow::bail!(
                    "Nixpacks is not installed. Please install it with: curl -sSL https://nixpacks.com/install.sh | bash"
                );
            }

            // Log Nixpacks version
            if let Some(version) = nixpacks::get_version().await {
                add_deployment_log(
                    db,
                    deployment_id,
                    "info",
                    &format!("Using Nixpacks version: {}", version),
                )
                .await?;
            }

            // Get Nixpacks config if provided
            let nixpacks_config = app.get_nixpacks_config();
            if nixpacks_config.is_some() {
                add_deployment_log(
                    db,
                    deployment_id,
                    "info",
                    "Using custom Nixpacks configuration",
                )
                .await?;
            }

            // Get env vars for the build
            let env_vars: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
                "SELECT key, value FROM env_vars WHERE app_id = ?",
            )
            .bind(&app.id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

            // Set up log streaming channel for nixpacks output
            let (nix_log_tx, mut nix_log_rx) = mpsc::unbounded_channel::<String>();
            let db_nix = db.clone();
            let depl_nix = deployment_id.to_string();
            let nix_drain = tokio::spawn(async move {
                while let Some(line) = nix_log_rx.recv().await {
                    let _ = add_deployment_log(&db_nix, &depl_nix, "info", &line).await;
                }
            });

            // Build with Nixpacks (nix_log_tx is moved in, dropped when build_image returns)
            nixpacks::build_image(
                build_path,
                &image_tag,
                nixpacks_config.as_ref(),
                &env_vars,
                Some(nix_log_tx),
            )
            .await
            .context("Nixpacks build failed")?;

            // Wait for all buffered log lines to be written to DB before advancing
            let _ = nix_drain.await;

            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Nixpacks build completed successfully",
            )
            .await?;
        }
        "staticsite" => {
            // Static site build using NGINX-based container
            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Building static site with NGINX...",
            )
            .await?;

            // Get env vars for the build
            let env_vars: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
                "SELECT key, value FROM env_vars WHERE app_id = ?",
            )
            .bind(&app.id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

            // Determine publish directory - use app setting or auto-detect
            let publish_dir = if let Some(ref dir) = app.publish_directory {
                if !dir.is_empty() {
                    add_deployment_log(
                        db,
                        deployment_id,
                        "info",
                        &format!("Using configured publish directory: {}", dir),
                    )
                    .await?;
                    dir.clone()
                } else {
                    let detected = StaticSiteBuilder::detect_publish_dir(build_path).await;
                    add_deployment_log(
                        db,
                        deployment_id,
                        "info",
                        &format!("Auto-detected publish directory: {}", detected),
                    )
                    .await?;
                    detected
                }
            } else {
                let detected = StaticSiteBuilder::detect_publish_dir(build_path).await;
                add_deployment_log(
                    db,
                    deployment_id,
                    "info",
                    &format!("Auto-detected publish directory: {}", detected),
                )
                .await?;
                detected
            };

            // Create static site config
            let static_config = StaticSiteConfig {
                source_dir: build_path.to_string_lossy().to_string(),
                publish_dir,
                env_vars,
                spa_mode: true, // Default to SPA mode for better client-side routing
                cpu_limit: build_limits.cpu_limit.clone(),
                memory_limit: build_limits.memory_limit.clone(),
                port: app.port as u16,
                ..Default::default()
            };

            // Build with StaticSiteBuilder
            let static_builder = StaticSiteBuilder::new(runtime.clone());
            static_builder
                .build(&static_config, &image_tag)
                .await
                .context("Static site build failed")?;

            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Static site build completed successfully",
            )
            .await?;
        }
        "railpack" => {
            // Railpack build (Railway's Nixpacks successor)
            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Building with Railpack (Railway's Nixpacks successor)...",
            )
            .await?;

            // Check if Railpack is available
            if !railpack::is_available().await {
                anyhow::bail!(
                    "Railpack is not installed or not supported on this platform. Note: Windows is not supported. Install with: mise install ubi:railwayapp/railpack@latest"
                );
            }

            // Log Railpack version
            if let Some(version) = railpack::get_version().await {
                add_deployment_log(
                    db,
                    deployment_id,
                    "info",
                    &format!("Using Railpack version: {}", version),
                )
                .await?;
            }

            // Get env vars for the build
            let env_vars: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
                "SELECT key, value FROM env_vars WHERE app_id = ?",
            )
            .bind(&app.id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

            // Set up log streaming channel for railpack output
            let (rail_log_tx, mut rail_log_rx) = mpsc::unbounded_channel::<String>();
            let db_rail = db.clone();
            let depl_rail = deployment_id.to_string();
            let rail_drain = tokio::spawn(async move {
                while let Some(line) = rail_log_rx.recv().await {
                    let _ = add_deployment_log(&db_rail, &depl_rail, "info", &line).await;
                }
            });

            // Build with Railpack (rail_log_tx is moved in, dropped when build_image returns)
            railpack::build_image(build_path, &image_tag, None, &env_vars, Some(rail_log_tx))
                .await
                .context("Railpack build failed")?;

            // Wait for all buffered log lines to be written to DB before advancing
            let _ = rail_drain.await;

            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Railpack build completed successfully",
            )
            .await?;
        }
        "cnb" | "paketo" | "heroku-cnb" => {
            // Cloud Native Buildpacks build (Paketo/Heroku via pack CLI)
            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Building with Cloud Native Buildpacks (pack CLI)...",
            )
            .await?;

            // Check if Pack CLI is available
            if !pack_builder::is_available().await {
                anyhow::bail!(
                    "Pack CLI is not installed. Install from: https://buildpacks.io/docs/tools/pack/"
                );
            }

            // Log Pack CLI version
            if let Some(version) = pack_builder::get_version().await {
                add_deployment_log(
                    db,
                    deployment_id,
                    "info",
                    &format!("Using Pack CLI version: {}", version),
                )
                .await?;
            }

            // Suggest best builder based on project files
            let suggested_builder = pack_builder::suggest_builder(build_path).await;
            add_deployment_log(
                db,
                deployment_id,
                "info",
                &format!("Using CNB builder: {}", suggested_builder.image()),
            )
            .await?;

            // Get env vars for the build
            let env_vars: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
                "SELECT key, value FROM env_vars WHERE app_id = ?",
            )
            .bind(&app.id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

            // Build with Pack CLI
            let pack_config = pack_builder::PackConfig {
                builder: suggested_builder,
                trust_builder: true,
                ..Default::default()
            };

            pack_builder::build_image(build_path, &image_tag, Some(&pack_config), &env_vars)
                .await
                .context("Pack CLI build failed")?;

            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Cloud Native Buildpacks build completed successfully",
            )
            .await?;
        }
        _ => {
            // Default: Dockerfile build
            add_deployment_log(db, deployment_id, "info", "Building Docker image...").await?;

            // Determine the dockerfile to use (dockerfile_path takes precedence over dockerfile)
            let dockerfile = app
                .dockerfile_path
                .as_ref()
                .filter(|p| !p.is_empty())
                .cloned()
                .unwrap_or_else(|| app.dockerfile.clone());

            // Set up a channel so Docker build output streams to deployment_logs
            let (log_tx, mut log_rx) = mpsc::unbounded_channel::<String>();
            let db_clone = db.clone();
            let depl_id = deployment_id.to_string();
            let log_drain = tokio::spawn(async move {
                while let Some(line) = log_rx.recv().await {
                    let _ = add_deployment_log(&db_clone, &depl_id, "info", &line).await;
                }
            });

            let build_ctx = BuildContext {
                path: build_path.to_string_lossy().to_string(),
                dockerfile,
                tag: image_tag.clone(),
                build_args: vec![],
                build_target: app.build_target.clone(),
                custom_options: app.custom_docker_options.clone(),
                cpu_limit: build_limits.cpu_limit.clone(),
                memory_limit: build_limits.memory_limit.clone(),
                log_tx: Some(log_tx),
            };

            // Log build resource limits if configured
            if build_limits.cpu_limit.is_some() || build_limits.memory_limit.is_some() {
                let mut limits = vec![];
                if let Some(ref cpu) = build_limits.cpu_limit {
                    limits.push(format!("cpu={}", cpu));
                }
                if let Some(ref mem) = build_limits.memory_limit {
                    limits.push(format!("memory={}", mem));
                }
                add_deployment_log(
                    db,
                    deployment_id,
                    "info",
                    &format!("Build resource limits: {}", limits.join(", ")),
                )
                .await?;
            }

            // Log build options if any are set
            if app.base_directory.is_some()
                || app.dockerfile_path.is_some()
                || app.build_target.is_some()
            {
                let mut opts = vec![];
                if let Some(ref base_dir) = app.base_directory {
                    if !base_dir.is_empty() {
                        opts.push(format!("base_directory={}", base_dir));
                    }
                }
                if let Some(ref df_path) = app.dockerfile_path {
                    if !df_path.is_empty() {
                        opts.push(format!("dockerfile_path={}", df_path));
                    }
                }
                if let Some(ref target) = app.build_target {
                    if !target.is_empty() {
                        opts.push(format!("target={}", target));
                    }
                }
                if !opts.is_empty() {
                    add_deployment_log(
                        db,
                        deployment_id,
                        "info",
                        &format!("Build options: {}", opts.join(", ")),
                    )
                    .await?;
                }
            }

            runtime.build(&build_ctx).await.context("Build failed")?;
            // Drop build_ctx (closes log_tx) then wait for all buffered logs to reach DB
            drop(build_ctx);
            let _ = log_drain.await;
            add_deployment_log(db, deployment_id, "info", "Image built successfully").await?;
        }
    }

    Ok(image_tag)
}

/// Offload a Dockerfile build to a remote build server via SSH.
///
/// Steps:
/// 1. Look up the build server record and decrypt its credentials.
/// 2. Create a temporary directory on the remote machine.
/// 3. rsync the local build context to the remote temp dir.
/// 4. Run `docker build` on the remote machine.
/// 5. Stream the built image back with `docker save | gzip` → `docker load`.
/// 6. Clean up the remote temp dir.
///
/// Returns the `image_tag` on success (same as a local build).
async fn run_remote_build(
    db: &DbPool,
    deployment_id: &str,
    app: &App,
    build_path: &Path,
    image_tag: &str,
    build_server_id: &str,
    encryption_key: Option<&[u8; KEY_LENGTH]>,
) -> Result<String> {
    use crate::crypto;
    use crate::db::BuildServer;
    use std::io::Write;
    use tokio::process::Command;

    // ------------------------------------------------------------------
    // 1. Look up build server
    // ------------------------------------------------------------------
    let server = sqlx::query_as::<_, BuildServer>(
        "SELECT * FROM build_servers WHERE id = ?",
    )
    .bind(build_server_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        anyhow::anyhow!(
            "Build server '{}' not found in database",
            build_server_id
        )
    })?;

    add_deployment_log(
        db,
        deployment_id,
        "info",
        &format!("[Build Server] Using remote build server: {}", server.name),
    )
    .await?;

    // ------------------------------------------------------------------
    // 2. Decrypt SSH credentials
    // ------------------------------------------------------------------
    let private_key_content = if let Some(ref encrypted_key) = server.ssh_private_key {
        match crypto::decrypt_if_encrypted(encrypted_key, encryption_key) {
            Ok(k) => Some(k),
            Err(e) => {
                tracing::warn!(
                    "Failed to decrypt SSH key for build server {}: {}",
                    build_server_id,
                    e
                );
                None
            }
        }
    } else {
        None
    };

    let ssh_password_content = if let Some(ref encrypted_pwd) = server.ssh_password {
        match crypto::decrypt_if_encrypted(encrypted_pwd, encryption_key) {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::warn!(
                    "Failed to decrypt SSH password for build server {}: {}",
                    build_server_id,
                    e
                );
                None
            }
        }
    } else {
        None
    };

    // Write private key to a temp file if available
    let key_tmpfile = if let Some(ref key_content) = private_key_content {
        let mut tmpfile = tempfile::Builder::new()
            .prefix("rivetr-bsrv-key-")
            .suffix(".pem")
            .tempfile()
            .context("Failed to create temp file for SSH key")?;

        tmpfile
            .write_all(key_content.as_bytes())
            .context("Failed to write SSH key to temp file")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                tmpfile.path(),
                std::fs::Permissions::from_mode(0o600),
            )?;
        }

        Some(tmpfile)
    } else {
        None
    };

    let key_path: Option<String> = key_tmpfile
        .as_ref()
        .map(|f| f.path().to_string_lossy().to_string());
    let port = server.port;
    let target = format!("{}@{}", server.username, server.host);
    let use_sshpass = ssh_password_content.is_some() && key_path.is_none();

    // Helper closure: build a tokio SSH command (key or sshpass)
    let make_ssh_cmd =
        |password: Option<&str>, key: Option<&str>| -> Command {
            let use_pass = password.is_some() && key.is_none();
            let mut c = if use_pass {
                let mut cmd = Command::new("sshpass");
                cmd.arg("-p").arg(password.unwrap());
                cmd.arg("ssh");
                cmd
            } else {
                Command::new("ssh")
            };
            c.arg("-o")
                .arg("StrictHostKeyChecking=no")
                .arg("-o")
                .arg("ConnectTimeout=15");
            if !use_pass {
                c.arg("-o").arg("BatchMode=yes");
            }
            c.arg("-p").arg(port.to_string());
            if let Some(k) = key {
                c.arg("-i").arg(k);
            }
            c
        };

    // ------------------------------------------------------------------
    // 3. Create a temp directory on the remote
    // ------------------------------------------------------------------
    add_deployment_log(
        db,
        deployment_id,
        "info",
        "[Build Server] Creating remote work directory...",
    )
    .await?;

    let mut mkdir_cmd = make_ssh_cmd(
        ssh_password_content.as_deref(),
        key_path.as_deref(),
    );
    mkdir_cmd.arg(&target).arg("mktemp -d");

    let mkdir_out = mkdir_cmd
        .output()
        .await
        .context("Failed to run mktemp on remote build server")?;

    if !mkdir_out.status.success() {
        let stderr = String::from_utf8_lossy(&mkdir_out.stderr);
        anyhow::bail!(
            "[Build Server] Failed to create remote temp directory: {}",
            stderr
        );
    }

    let remote_dir = String::from_utf8_lossy(&mkdir_out.stdout)
        .trim()
        .to_string();

    add_deployment_log(
        db,
        deployment_id,
        "info",
        &format!("[Build Server] Remote work directory: {}", remote_dir),
    )
    .await?;

    // Clones for cleanup
    let remote_dir_cleanup = remote_dir.clone();
    let cleanup_target = target.clone();
    let cleanup_key = key_path.clone();
    let cleanup_password = ssh_password_content.clone();

    // Wrap the main build steps so we can guarantee cleanup afterwards
    let result: Result<String> = async {
        // ------------------------------------------------------------------
        // 4. rsync build context to remote
        // ------------------------------------------------------------------
        add_deployment_log(
            db,
            deployment_id,
            "info",
            "[Build Server] Syncing build context to remote...",
        )
        .await?;

        let build_path_str = build_path.to_string_lossy().to_string();
        let src = if build_path_str.ends_with('/') {
            build_path_str.clone()
        } else {
            format!("{}/", build_path_str)
        };
        let dst = format!("{}:{}", target, remote_dir);

        // Build the ssh command string for rsync -e option
        let ssh_e_opt = if use_sshpass {
            let pass = ssh_password_content.as_deref().unwrap_or("");
            format!(
                "sshpass -p '{}' ssh -o StrictHostKeyChecking=no -p {}",
                pass, port
            )
        } else if let Some(ref kp) = key_path {
            format!(
                "ssh -o StrictHostKeyChecking=no -o BatchMode=yes -p {} -i {}",
                port, kp
            )
        } else {
            format!(
                "ssh -o StrictHostKeyChecking=no -o BatchMode=yes -p {}",
                port
            )
        };

        let rsync_out = Command::new("rsync")
            .arg("-az")
            .arg("--delete")
            .arg("-e")
            .arg(&ssh_e_opt)
            .arg(&src)
            .arg(&dst)
            .output()
            .await
            .context("Failed to run rsync to remote build server")?;

        if !rsync_out.status.success() {
            let stderr = String::from_utf8_lossy(&rsync_out.stderr);
            anyhow::bail!("[Build Server] rsync failed: {}", stderr);
        }

        add_deployment_log(
            db,
            deployment_id,
            "info",
            "[Build Server] Build context synced. Running docker build on remote...",
        )
        .await?;

        // ------------------------------------------------------------------
        // 5. Run docker build on remote
        // ------------------------------------------------------------------
        let dockerfile = app
            .dockerfile_path
            .as_ref()
            .filter(|p| !p.is_empty())
            .cloned()
            .unwrap_or_else(|| app.dockerfile.clone());

        let docker_build_cmd = format!(
            "docker build -t '{}' -f '{}/{}' '{}'",
            image_tag, remote_dir, dockerfile, remote_dir
        );

        let mut build_ssh =
            make_ssh_cmd(ssh_password_content.as_deref(), key_path.as_deref());
        build_ssh.arg(&target).arg(&docker_build_cmd);

        let build_out = build_ssh
            .output()
            .await
            .context("Failed to run docker build on remote build server")?;

        let build_stdout = String::from_utf8_lossy(&build_out.stdout);
        for line in build_stdout.lines() {
            if !line.trim().is_empty() {
                add_deployment_log(db, deployment_id, "info", line).await?;
            }
        }

        if !build_out.status.success() {
            let stderr = String::from_utf8_lossy(&build_out.stderr);
            anyhow::bail!(
                "[Build Server] Remote docker build failed: {}",
                stderr
            );
        }

        add_deployment_log(
            db,
            deployment_id,
            "info",
            "[Build Server] Remote build completed. Transferring image to local daemon...",
        )
        .await?;

        // ------------------------------------------------------------------
        // 6. Stream image back: docker save on remote | gzip → local docker load
        // ------------------------------------------------------------------
        let save_cmd = format!("docker save '{}' | gzip", image_tag);

        // Use std::process for synchronous pipeline (stdin→stdout piping)
        let mut stream_ssh_cmd = if use_sshpass {
            let mut c = std::process::Command::new("sshpass");
            c.arg("-p")
                .arg(ssh_password_content.as_deref().unwrap_or(""))
                .arg("ssh");
            c
        } else {
            std::process::Command::new("ssh")
        };

        stream_ssh_cmd
            .arg("-o")
            .arg("StrictHostKeyChecking=no")
            .arg("-o")
            .arg("ConnectTimeout=15");
        if !use_sshpass {
            stream_ssh_cmd.arg("-o").arg("BatchMode=yes");
        }
        stream_ssh_cmd.arg("-p").arg(port.to_string());
        if let Some(ref kp) = key_path {
            stream_ssh_cmd.arg("-i").arg(kp);
        }
        stream_ssh_cmd
            .arg(&target)
            .arg(&save_cmd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());

        let mut ssh_child = stream_ssh_cmd
            .spawn()
            .context("Failed to spawn SSH for docker save")?;

        let ssh_stdout = ssh_child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdout from SSH process"))?;

        let load_child = std::process::Command::new("docker")
            .arg("load")
            .stdin(ssh_stdout)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn docker load")?;

        let load_out = load_child
            .wait_with_output()
            .context("Failed to wait for docker load")?;

        let ssh_status = ssh_child
            .wait()
            .context("Failed to wait for SSH process")?;

        if !ssh_status.success() {
            anyhow::bail!(
                "[Build Server] docker save/stream SSH process exited with status: {}",
                ssh_status
            );
        }

        if !load_out.status.success() {
            let stderr = String::from_utf8_lossy(&load_out.stderr);
            anyhow::bail!("[Build Server] docker load failed: {}", stderr);
        }

        add_deployment_log(
            db,
            deployment_id,
            "info",
            "[Build Server] Image transferred and loaded successfully",
        )
        .await?;

        Ok(image_tag.to_string())
    }
    .await;

    // ------------------------------------------------------------------
    // 7. Clean up remote temp dir (best-effort)
    // ------------------------------------------------------------------
    let use_cleanup_sshpass = cleanup_password.is_some() && cleanup_key.is_none();
    let mut cleanup_cmd = if use_cleanup_sshpass {
        let mut c = Command::new("sshpass");
        c.arg("-p")
            .arg(cleanup_password.as_deref().unwrap_or(""))
            .arg("ssh");
        c
    } else {
        Command::new("ssh")
    };
    cleanup_cmd
        .arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("ConnectTimeout=10");
    if !use_cleanup_sshpass {
        cleanup_cmd.arg("-o").arg("BatchMode=yes");
    }
    cleanup_cmd.arg("-p").arg(port.to_string());
    if let Some(ref kp) = cleanup_key {
        cleanup_cmd.arg("-i").arg(kp);
    }
    cleanup_cmd
        .arg(&cleanup_target)
        .arg(format!("rm -rf '{}'", remote_dir_cleanup));

    if let Err(e) = cleanup_cmd.output().await {
        tracing::warn!(
            "[Build Server] Failed to clean up remote temp dir '{}': {}",
            remote_dir_cleanup,
            e
        );
    }

    result
}

/// Build the image for an upload-based deployment
pub(super) async fn build_upload_image(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    app: &App,
    build_path: &Path,
    build_limits: &BuildLimits,
) -> Result<String> {
    let image_tag = format!("rivetr-{}:{}", app.name, deployment_id);
    let build_type = app.get_build_type();

    match build_type {
        "nixpacks" => {
            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Building uploaded project with Nixpacks...",
            )
            .await?;

            if !nixpacks::is_available().await {
                anyhow::bail!(
                    "Nixpacks is not installed. Please install it with: curl -sSL https://nixpacks.com/install.sh | bash"
                );
            }

            if let Some(version) = nixpacks::get_version().await {
                add_deployment_log(
                    db,
                    deployment_id,
                    "info",
                    &format!("Using Nixpacks version: {}", version),
                )
                .await?;
            }

            let nixpacks_config = app.get_nixpacks_config();
            let env_vars: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
                "SELECT key, value FROM env_vars WHERE app_id = ?",
            )
            .bind(&app.id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

            let (nix_log_tx2, mut nix_log_rx2) = mpsc::unbounded_channel::<String>();
            let db_nix2 = db.clone();
            let depl_nix2 = deployment_id.to_string();
            let nix_drain2 = tokio::spawn(async move {
                while let Some(line) = nix_log_rx2.recv().await {
                    let _ = add_deployment_log(&db_nix2, &depl_nix2, "info", &line).await;
                }
            });

            nixpacks::build_image(
                build_path,
                &image_tag,
                nixpacks_config.as_ref(),
                &env_vars,
                Some(nix_log_tx2),
            )
            .await
            .context("Nixpacks build failed")?;

            // Wait for all buffered log lines to be written to DB before advancing
            let _ = nix_drain2.await;

            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Nixpacks build completed successfully",
            )
            .await?;
        }
        "staticsite" => {
            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Building uploaded static site with NGINX...",
            )
            .await?;

            let env_vars: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
                "SELECT key, value FROM env_vars WHERE app_id = ?",
            )
            .bind(&app.id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

            let publish_dir = if let Some(ref dir) = app.publish_directory {
                if !dir.is_empty() {
                    add_deployment_log(
                        db,
                        deployment_id,
                        "info",
                        &format!("Using configured publish directory: {}", dir),
                    )
                    .await?;
                    dir.clone()
                } else {
                    let detected = StaticSiteBuilder::detect_publish_dir(build_path).await;
                    add_deployment_log(
                        db,
                        deployment_id,
                        "info",
                        &format!("Auto-detected publish directory: {}", detected),
                    )
                    .await?;
                    detected
                }
            } else {
                let detected = StaticSiteBuilder::detect_publish_dir(build_path).await;
                add_deployment_log(
                    db,
                    deployment_id,
                    "info",
                    &format!("Auto-detected publish directory: {}", detected),
                )
                .await?;
                detected
            };

            let static_config = StaticSiteConfig {
                source_dir: build_path.to_string_lossy().to_string(),
                publish_dir,
                env_vars,
                spa_mode: true,
                cpu_limit: build_limits.cpu_limit.clone(),
                memory_limit: build_limits.memory_limit.clone(),
                port: app.port as u16,
                ..Default::default()
            };

            let static_builder = StaticSiteBuilder::new(runtime.clone());
            static_builder
                .build(&static_config, &image_tag)
                .await
                .context("Static site build failed")?;

            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Static site build completed successfully",
            )
            .await?;
        }
        "railpack" => {
            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Building uploaded project with Railpack...",
            )
            .await?;

            if !railpack::is_available().await {
                anyhow::bail!(
                    "Railpack is not installed or not supported on this platform. Note: Windows is not supported."
                );
            }

            if let Some(version) = railpack::get_version().await {
                add_deployment_log(
                    db,
                    deployment_id,
                    "info",
                    &format!("Using Railpack version: {}", version),
                )
                .await?;
            }

            let env_vars: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
                "SELECT key, value FROM env_vars WHERE app_id = ?",
            )
            .bind(&app.id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

            // Set up log streaming channel for railpack output
            let (rail_log_tx2, mut rail_log_rx2) = mpsc::unbounded_channel::<String>();
            let db_rail2 = db.clone();
            let depl_rail2 = deployment_id.to_string();
            let rail_drain2 = tokio::spawn(async move {
                while let Some(line) = rail_log_rx2.recv().await {
                    let _ = add_deployment_log(&db_rail2, &depl_rail2, "info", &line).await;
                }
            });

            // Build with Railpack (rail_log_tx2 is moved in, dropped when build_image returns)
            railpack::build_image(build_path, &image_tag, None, &env_vars, Some(rail_log_tx2))
                .await
                .context("Railpack build failed")?;

            // Wait for all buffered log lines to be written to DB before advancing
            let _ = rail_drain2.await;

            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Railpack build completed successfully",
            )
            .await?;
        }
        "cnb" | "paketo" | "heroku-cnb" => {
            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Building uploaded project with Cloud Native Buildpacks...",
            )
            .await?;

            if !pack_builder::is_available().await {
                anyhow::bail!(
                    "Pack CLI is not installed. Install from: https://buildpacks.io/docs/tools/pack/"
                );
            }

            if let Some(version) = pack_builder::get_version().await {
                add_deployment_log(
                    db,
                    deployment_id,
                    "info",
                    &format!("Using Pack CLI version: {}", version),
                )
                .await?;
            }

            let suggested_builder = pack_builder::suggest_builder(build_path).await;
            add_deployment_log(
                db,
                deployment_id,
                "info",
                &format!("Using CNB builder: {}", suggested_builder.image()),
            )
            .await?;

            let env_vars: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
                "SELECT key, value FROM env_vars WHERE app_id = ?",
            )
            .bind(&app.id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

            let pack_config = pack_builder::PackConfig {
                builder: suggested_builder,
                trust_builder: true,
                ..Default::default()
            };

            pack_builder::build_image(build_path, &image_tag, Some(&pack_config), &env_vars)
                .await
                .context("Pack CLI build failed")?;

            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Cloud Native Buildpacks build completed successfully",
            )
            .await?;
        }
        _ => {
            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Building uploaded project with Dockerfile...",
            )
            .await?;

            let dockerfile = app
                .dockerfile_path
                .as_ref()
                .filter(|p| !p.is_empty())
                .cloned()
                .unwrap_or_else(|| app.dockerfile.clone());

            let (log_tx2, mut log_rx2) = mpsc::unbounded_channel::<String>();
            let db_clone2 = db.clone();
            let depl_id2 = deployment_id.to_string();
            let log_drain2 = tokio::spawn(async move {
                while let Some(line) = log_rx2.recv().await {
                    let _ = add_deployment_log(&db_clone2, &depl_id2, "info", &line).await;
                }
            });

            let build_ctx = BuildContext {
                path: build_path.to_string_lossy().to_string(),
                dockerfile,
                tag: image_tag.clone(),
                build_args: vec![],
                build_target: app.build_target.clone(),
                custom_options: app.custom_docker_options.clone(),
                cpu_limit: build_limits.cpu_limit.clone(),
                memory_limit: build_limits.memory_limit.clone(),
                log_tx: Some(log_tx2),
            };

            runtime.build(&build_ctx).await.context("Build failed")?;
            // Drop build_ctx (closes log_tx2) then wait for all buffered logs to reach DB
            drop(build_ctx);
            let _ = log_drain2.await;
            add_deployment_log(db, deployment_id, "info", "Image built successfully").await?;
        }
    }

    Ok(image_tag)
}
