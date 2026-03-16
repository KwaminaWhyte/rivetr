//! Instance settings API endpoints.
//!
//! Provides GET and PUT endpoints for instance-level configuration such as
//! the instance domain and instance name.

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::db::{InstanceSettings, UpdateInstanceSettingsRequest};
use crate::proxy::Backend;
use crate::AppState;

/// Response for updating instance settings — includes a flag indicating whether
/// the change took effect immediately (no restart required).
#[derive(Debug, Serialize)]
pub struct UpdateInstanceSettingsResponse {
    #[serde(flatten)]
    pub settings: InstanceSettings,
    /// Whether the change took effect immediately (proxy route reloaded in-process).
    pub requires_restart: bool,
}

/// Get instance settings.
///
/// GET /api/settings/instance
pub async fn get_instance_settings(
    State(state): State<Arc<AppState>>,
) -> Result<Json<InstanceSettings>, StatusCode> {
    let settings = InstanceSettings::load(&state.db).await.map_err(|e| {
        tracing::error!("Failed to load instance settings: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(settings))
}

/// Update instance settings.
///
/// PUT /api/settings/instance
///
/// After saving to the DB this handler:
/// 1. Removes the old instance-domain proxy route (if any).
/// 2. Registers the new domain → API server route so the dashboard is immediately reachable.
/// 3. Spawns a background ACME cert-renewal task when TLS is enabled.
pub async fn update_instance_settings(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateInstanceSettingsRequest>,
) -> Result<Json<UpdateInstanceSettingsResponse>, StatusCode> {
    // Load the current setting so we know the old domain to remove.
    let old_settings = InstanceSettings::load(&state.db).await.map_err(|e| {
        tracing::error!("Failed to load instance settings before update: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let settings = InstanceSettings::update(&state.db, &req)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update instance settings: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("Updated instance settings");

    // --- Hot-reload the proxy route ---
    // Determine whether instance_domain actually changed.
    let new_domain = settings.instance_domain.clone();
    let old_domain = old_settings.instance_domain.clone();

    if old_domain != new_domain {
        let route_table = state.routes.load();

        // Remove the old domain route if there was one.
        if let Some(ref old) = old_domain {
            if !old.is_empty() {
                route_table.remove_route(old);
                tracing::info!(domain = %old, "Removed old instance domain proxy route");
            }
        }

        // Register the new domain route if provided.
        if let Some(ref new) = new_domain {
            if !new.is_empty() {
                let backend = Backend::new(
                    "rivetr-api".to_string(),
                    "127.0.0.1".to_string(),
                    state.config.server.api_port,
                );
                route_table.add_route(new.clone(), backend);
                tracing::info!(
                    domain = %new,
                    port = state.config.server.api_port,
                    "Registered new instance domain proxy route"
                );

                // Spawn ACME cert renewal in background if TLS is enabled.
                let acme_enabled =
                    state.config.proxy.acme_enabled && state.config.proxy.acme_email.is_some();

                if acme_enabled {
                    let acme_cfg = crate::proxy::AcmeConfig {
                        email: state.config.proxy.acme_email.clone().unwrap_or_default(),
                        cache_dir: state.config.proxy.acme_cache_dir.clone(),
                        staging: state.config.proxy.acme_staging,
                    };
                    let domain = new.clone();

                    tokio::spawn(async move {
                        tracing::info!(
                            domain = %domain,
                            "Triggering background ACME certificate renewal for new instance domain"
                        );
                        match crate::proxy::AcmeClient::new(acme_cfg).await {
                            Ok(acme_client) => {
                                let cert_dir = acme_client.cert_dir(&domain);
                                // Only request a new cert if one doesn't already exist for this domain.
                                if !cert_dir.join("fullchain.pem").exists() {
                                    match acme_client
                                        .request_certificate(std::slice::from_ref(&domain))
                                        .await
                                    {
                                        Ok(result) => {
                                            let _ = acme_client.save_certificate(&result).await;
                                            tracing::info!(
                                                domain = %domain,
                                                "ACME certificate obtained for new instance domain"
                                            );
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                domain = %domain,
                                                error = %e,
                                                "ACME certificate request failed for new instance domain"
                                            );
                                        }
                                    }
                                } else {
                                    tracing::info!(
                                        domain = %domain,
                                        "Cached TLS certificate already exists for instance domain"
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    "Failed to init ACME client for instance domain renewal"
                                );
                            }
                        }
                    });
                }
            }
        }
    }

    Ok(Json(UpdateInstanceSettingsResponse {
        settings,
        requires_restart: false,
    }))
}
