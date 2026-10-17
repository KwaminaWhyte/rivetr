use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::api::authz::authorize_deployment;
use crate::api::error::ApiError;
use crate::api::validation::validate_uuid;
use crate::db::User;
use crate::engine::deployment_snapshots::{load_restorable_snapshots, DeploymentSnapshot};
use crate::AppState;

/// List the database snapshots restored by rolling back to this deployment
/// GET /api/deployments/:id/snapshots
///
/// A snapshot belongs to the deployment that was running when it was taken (the one
/// whose code last used the data), so these are the snapshots a rollback to `:id` with
/// `restore_data: true` would restore: the newest one per linked database. Empty when
/// the app did not have `snapshot_before_deploy` enabled, or the snapshots were pruned.
pub async fn list_deployment_snapshots(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(deployment_id): Path<String>,
) -> Result<Json<Vec<DeploymentSnapshot>>, ApiError> {
    if let Err(e) = validate_uuid(&deployment_id, "deployment_id") {
        return Err(ApiError::validation_field("deployment_id", e));
    }
    authorize_deployment(&state, &user, &deployment_id).await?;

    let snapshots = load_restorable_snapshots(&state.db, &deployment_id).await?;
    Ok(Json(snapshots))
}
