// Proxy route management API
//
// This module provides endpoints for managing reverse proxy routes.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::proxy::Backend;
use crate::AppState;

/// Route information response
#[derive(Debug, Serialize)]
pub struct RouteInfo {
    pub domain: String,
    pub backend_host: String,
    pub backend_port: u16,
    pub container_id: String,
    pub healthy: bool,
    pub healthcheck_path: Option<String>,
    pub failure_count: u32,
}

impl From<(String, Backend)> for RouteInfo {
    fn from((domain, backend): (String, Backend)) -> Self {
        Self {
            domain,
            backend_host: backend.host,
            backend_port: backend.port,
            container_id: backend.container_id,
            healthy: backend.healthy,
            healthcheck_path: backend.healthcheck_path,
            failure_count: backend.failure_count,
        }
    }
}

/// List all proxy routes response
#[derive(Debug, Serialize)]
pub struct ListRoutesResponse {
    pub routes: Vec<RouteInfo>,
    pub total: usize,
}

/// Request to add a new route
#[derive(Debug, Deserialize)]
pub struct AddRouteRequest {
    pub domain: String,
    pub backend_host: String,
    pub backend_port: u16,
    pub container_id: String,
    #[serde(default)]
    pub healthcheck_path: Option<String>,
}

/// Request to update route health
#[derive(Debug, Deserialize)]
pub struct UpdateHealthRequest {
    pub healthy: bool,
}

/// List all proxy routes
///
/// GET /api/routes
pub async fn list_routes(State(state): State<Arc<AppState>>) -> Json<ListRoutesResponse> {
    let routes = state.routes.load();
    let backends = routes.all_backends();
    let total = backends.len();

    let routes: Vec<RouteInfo> = backends.into_iter().map(RouteInfo::from).collect();

    Json(ListRoutesResponse { routes, total })
}

/// Get a specific route by domain
///
/// GET /api/routes/:domain
pub async fn get_route(
    State(state): State<Arc<AppState>>,
    Path(domain): Path<String>,
) -> Result<Json<RouteInfo>, StatusCode> {
    let routes = state.routes.load();

    match routes.get_backend(&domain) {
        Some(backend) => Ok(Json(RouteInfo::from((domain, backend)))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Add a new proxy route
///
/// POST /api/routes
pub async fn add_route(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddRouteRequest>,
) -> Result<(StatusCode, Json<RouteInfo>), StatusCode> {
    // Validate domain format
    if req.domain.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validate port
    if req.backend_port == 0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let backend = Backend::new(req.container_id.clone(), req.backend_host.clone(), req.backend_port)
        .with_healthcheck(req.healthcheck_path.clone());

    let routes = state.routes.load();
    routes.add_route(req.domain.clone(), backend.clone());

    info!(domain = %req.domain, backend = %backend.addr(), "Route added via API");

    Ok((
        StatusCode::CREATED,
        Json(RouteInfo::from((req.domain, backend))),
    ))
}

/// Remove a proxy route
///
/// DELETE /api/routes/:domain
pub async fn remove_route(
    State(state): State<Arc<AppState>>,
    Path(domain): Path<String>,
) -> StatusCode {
    let routes = state.routes.load();

    if !routes.has_domain(&domain) {
        return StatusCode::NOT_FOUND;
    }

    routes.remove_route(&domain);

    info!(domain = %domain, "Route removed via API");

    StatusCode::NO_CONTENT
}

/// Update route health status
///
/// PUT /api/routes/:domain/health
pub async fn update_route_health(
    State(state): State<Arc<AppState>>,
    Path(domain): Path<String>,
    Json(req): Json<UpdateHealthRequest>,
) -> Result<Json<RouteInfo>, StatusCode> {
    let routes = state.routes.load();

    if !routes.has_domain(&domain) {
        return Err(StatusCode::NOT_FOUND);
    }

    routes.set_health(&domain, req.healthy);

    match routes.get_backend(&domain) {
        Some(backend) => {
            info!(domain = %domain, healthy = req.healthy, "Route health updated via API");
            Ok(Json(RouteInfo::from((domain, backend))))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Get all registered domains
///
/// GET /api/routes/domains
pub async fn list_domains(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    let routes = state.routes.load();
    Json(routes.domains())
}

/// Check if the proxy is healthy (all routes accessible)
///
/// GET /api/routes/health
pub async fn routes_health(State(state): State<Arc<AppState>>) -> Json<HealthSummary> {
    let routes = state.routes.load();
    let backends = routes.all_backends();

    let total = backends.len();
    let healthy = backends.iter().filter(|(_, b)| b.healthy).count();
    let unhealthy = total - healthy;

    Json(HealthSummary {
        total,
        healthy,
        unhealthy,
        all_healthy: unhealthy == 0,
    })
}

/// Health summary response
#[derive(Debug, Serialize)]
pub struct HealthSummary {
    pub total: usize,
    pub healthy: usize,
    pub unhealthy: usize,
    pub all_healthy: bool,
}
