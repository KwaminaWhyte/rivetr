//! Advanced Docker Swarm service spec for apps deployed to Swarm.
//!
//! An app's spec (placement constraints and spread preferences, update and
//! rollback config, resource reservations, restart policy) is stored as JSON in
//! `app_swarm_specs` (migration 130). It is used when a Swarm service is created
//! for the app and re-applied to the app's existing Swarm services on save.
//!
//! Endpoints:
//! - `GET /api/apps/{id}/swarm-spec`
//! - `PUT /api/apps/{id}/swarm-spec`

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use bollard::models::{
    LocalNodeState, ResourceObject, ServiceSpec, ServiceSpecMode, ServiceSpecModeReplicated,
    ServiceSpecRollbackConfig, ServiceSpecRollbackConfigFailureActionEnum,
    ServiceSpecRollbackConfigOrderEnum, ServiceSpecUpdateConfig,
    ServiceSpecUpdateConfigFailureActionEnum, ServiceSpecUpdateConfigOrderEnum, TaskSpec,
    TaskSpecContainerSpec, TaskSpecPlacement, TaskSpecPlacementPreferences,
    TaskSpecPlacementSpread, TaskSpecResources, TaskSpecRestartPolicy,
    TaskSpecRestartPolicyConditionEnum,
};
use bollard::query_parameters::UpdateServiceOptionsBuilder;
use bollard::Docker;
use serde::{Deserialize, Serialize};

use crate::db::{SwarmService, User};
use crate::AppState;

use super::authz;
use super::error::ApiError;

const NANOS_PER_SEC: i64 = 1_000_000_000;

/// Rolling update or rollback behaviour (`docker service create --update-*` / `--rollback-*`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SwarmRolloutConfig {
    /// Tasks updated at once (0 = all at once).
    #[serde(default)]
    pub parallelism: Option<i64>,
    /// Delay between batches, in seconds.
    #[serde(default)]
    pub delay_secs: Option<i64>,
    /// `continue`, `pause` or (update only) `rollback`.
    #[serde(default)]
    pub failure_action: Option<String>,
    /// `stop-first` or `start-first`.
    #[serde(default)]
    pub order: Option<String>,
    /// How long to watch each task for failure after it is updated, in seconds.
    #[serde(default)]
    pub monitor_secs: Option<i64>,
    /// Failure ratio tolerated during the rollout (0.0 to 1.0).
    #[serde(default)]
    pub max_failure_ratio: Option<f64>,
}

/// Task restart policy (`--restart-*`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SwarmRestartPolicy {
    /// `none`, `on-failure` or `any`.
    pub condition: String,
    #[serde(default)]
    pub delay_secs: Option<i64>,
    #[serde(default)]
    pub max_attempts: Option<i64>,
    #[serde(default)]
    pub window_secs: Option<i64>,
}

/// The advanced spec stored per app. Every field is optional; unset fields keep
/// Docker's defaults.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SwarmSpecConfig {
    /// Placement constraints, e.g. `node.role==worker`, `node.labels.zone!=eu-1`.
    #[serde(default)]
    pub constraints: Vec<String>,
    /// Spread preferences, e.g. `node.labels.zone`.
    #[serde(default)]
    pub spread_preferences: Vec<String>,
    /// Maximum replicas of this service per node.
    #[serde(default)]
    pub max_replicas_per_node: Option<i64>,
    #[serde(default)]
    pub update_config: Option<SwarmRolloutConfig>,
    #[serde(default)]
    pub rollback_config: Option<SwarmRolloutConfig>,
    /// CPU reservation in cores (e.g. 0.5).
    #[serde(default)]
    pub reserve_cpus: Option<f64>,
    /// Memory reservation in MB.
    #[serde(default)]
    pub reserve_memory_mb: Option<i64>,
    #[serde(default)]
    pub restart_policy: Option<SwarmRestartPolicy>,
}

fn is_valid_constraint(c: &str) -> bool {
    let op = if c.contains("==") {
        "=="
    } else if c.contains("!=") {
        "!="
    } else {
        return false;
    };
    let mut parts = c.splitn(2, op);
    let key = parts.next().unwrap_or("").trim();
    let value = parts.next().unwrap_or("").trim();
    !key.is_empty()
        && !value.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '/'))
        && !value.contains(['\n', '\r', '=', '!'])
}

fn is_valid_spread(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 200
        && s.chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '/'))
}

fn validate_rollout(cfg: &SwarmRolloutConfig, field: &str, is_update: bool) -> Result<(), String> {
    for (name, v) in [
        ("parallelism", cfg.parallelism),
        ("delay_secs", cfg.delay_secs),
        ("monitor_secs", cfg.monitor_secs),
    ] {
        if v.is_some_and(|v| v < 0) {
            return Err(format!("{field}.{name} must be 0 or more"));
        }
    }
    if let Some(r) = cfg.max_failure_ratio {
        if !(0.0..=1.0).contains(&r) {
            return Err(format!("{field}.max_failure_ratio must be between 0 and 1"));
        }
    }
    match cfg.failure_action.as_deref() {
        None | Some("") | Some("continue") | Some("pause") => {}
        Some("rollback") if is_update => {}
        Some(other) => return Err(format!("{field}.failure_action '{other}' is not valid")),
    }
    match cfg.order.as_deref() {
        None | Some("") | Some("stop-first") | Some("start-first") => {}
        Some(other) => return Err(format!("{field}.order '{other}' is not valid")),
    }
    Ok(())
}

impl SwarmSpecConfig {
    /// Validate user input before it is stored or sent to Docker.
    pub fn validate(&self) -> Result<(), String> {
        if self.constraints.len() > 32 || self.spread_preferences.len() > 8 {
            return Err("Too many constraints or spread preferences".to_string());
        }
        for c in &self.constraints {
            if !is_valid_constraint(c.trim()) {
                return Err(format!(
                    "Constraint '{c}' must look like node.role==worker or node.labels.zone!=eu"
                ));
            }
        }
        for s in &self.spread_preferences {
            if !is_valid_spread(s.trim()) {
                return Err(format!(
                    "Spread preference '{s}' must be a node attribute like node.labels.zone"
                ));
            }
        }
        if self.max_replicas_per_node.is_some_and(|v| v < 0) {
            return Err("max_replicas_per_node must be 0 or more".to_string());
        }
        if let Some(u) = &self.update_config {
            validate_rollout(u, "update_config", true)?;
        }
        if let Some(r) = &self.rollback_config {
            validate_rollout(r, "rollback_config", false)?;
        }
        if self
            .reserve_cpus
            .is_some_and(|v| !(0.0..=1024.0).contains(&v))
        {
            return Err("reserve_cpus must be between 0 and 1024".to_string());
        }
        if self.reserve_memory_mb.is_some_and(|v| v < 0) {
            return Err("reserve_memory_mb must be 0 or more".to_string());
        }
        if let Some(p) = &self.restart_policy {
            if !matches!(p.condition.as_str(), "none" | "on-failure" | "any") {
                return Err("restart_policy.condition must be none, on-failure or any".into());
            }
            for (name, v) in [
                ("delay_secs", p.delay_secs),
                ("max_attempts", p.max_attempts),
                ("window_secs", p.window_secs),
            ] {
                if v.is_some_and(|v| v < 0) {
                    return Err(format!("restart_policy.{name} must be 0 or more"));
                }
            }
        }
        Ok(())
    }

    /// Whether any field is set.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Write this config into a bollard `ServiceSpec`, keeping fields it does not
    /// manage (image, env, networks, resource limits, platforms, ...).
    pub fn apply_to(&self, spec: &mut ServiceSpec) {
        let task = spec.task_template.get_or_insert_with(TaskSpec::default);

        // Placement
        let placement = task
            .placement
            .get_or_insert_with(TaskSpecPlacement::default);
        let constraints: Vec<String> = self
            .constraints
            .iter()
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .collect();
        placement.constraints = (!constraints.is_empty()).then_some(constraints);
        let prefs: Vec<TaskSpecPlacementPreferences> = self
            .spread_preferences
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| TaskSpecPlacementPreferences {
                spread: Some(TaskSpecPlacementSpread {
                    spread_descriptor: Some(s.to_string()),
                }),
            })
            .collect();
        placement.preferences = (!prefs.is_empty()).then_some(prefs);
        placement.max_replicas = self.max_replicas_per_node.filter(|v| *v > 0);

        // Reservations (limits are left as they are)
        let reservations = ResourceObject {
            nano_cpus: self
                .reserve_cpus
                .filter(|v| *v > 0.0)
                .map(|cpus| (cpus * NANOS_PER_SEC as f64).round() as i64),
            memory_bytes: self
                .reserve_memory_mb
                .filter(|v| *v > 0)
                .map(|mb| mb * 1024 * 1024),
            generic_resources: None,
        };
        let has_reservations =
            reservations.nano_cpus.is_some() || reservations.memory_bytes.is_some();
        if has_reservations || task.resources.is_some() {
            let resources = task
                .resources
                .get_or_insert_with(TaskSpecResources::default);
            resources.reservations = has_reservations.then_some(reservations);
        }

        // Restart policy
        task.restart_policy = self.restart_policy.as_ref().map(|p| TaskSpecRestartPolicy {
            condition: Some(match p.condition.as_str() {
                "none" => TaskSpecRestartPolicyConditionEnum::NONE,
                "on-failure" => TaskSpecRestartPolicyConditionEnum::ON_FAILURE,
                _ => TaskSpecRestartPolicyConditionEnum::ANY,
            }),
            delay: p.delay_secs.map(|s| s * NANOS_PER_SEC),
            max_attempts: p.max_attempts,
            window: p.window_secs.map(|s| s * NANOS_PER_SEC),
        });

        spec.update_config = self
            .update_config
            .as_ref()
            .map(|u| ServiceSpecUpdateConfig {
                parallelism: u.parallelism,
                delay: u.delay_secs.map(|s| s * NANOS_PER_SEC),
                failure_action: match u.failure_action.as_deref() {
                    Some("continue") => Some(ServiceSpecUpdateConfigFailureActionEnum::CONTINUE),
                    Some("pause") => Some(ServiceSpecUpdateConfigFailureActionEnum::PAUSE),
                    Some("rollback") => Some(ServiceSpecUpdateConfigFailureActionEnum::ROLLBACK),
                    _ => None,
                },
                monitor: u.monitor_secs.map(|s| s * NANOS_PER_SEC),
                max_failure_ratio: u.max_failure_ratio,
                order: match u.order.as_deref() {
                    Some("stop-first") => Some(ServiceSpecUpdateConfigOrderEnum::STOP_FIRST),
                    Some("start-first") => Some(ServiceSpecUpdateConfigOrderEnum::START_FIRST),
                    _ => None,
                },
            });

        spec.rollback_config = self
            .rollback_config
            .as_ref()
            .map(|r| ServiceSpecRollbackConfig {
                parallelism: r.parallelism,
                delay: r.delay_secs.map(|s| s * NANOS_PER_SEC),
                failure_action: match r.failure_action.as_deref() {
                    Some("continue") => Some(ServiceSpecRollbackConfigFailureActionEnum::CONTINUE),
                    Some("pause") => Some(ServiceSpecRollbackConfigFailureActionEnum::PAUSE),
                    _ => None,
                },
                monitor: r.monitor_secs.map(|s| s * NANOS_PER_SEC),
                max_failure_ratio: r.max_failure_ratio,
                order: match r.order.as_deref() {
                    Some("stop-first") => Some(ServiceSpecRollbackConfigOrderEnum::STOP_FIRST),
                    Some("start-first") => Some(ServiceSpecRollbackConfigOrderEnum::START_FIRST),
                    _ => None,
                },
            });
    }
}

/// Build the spec for a new Swarm service.
pub fn new_service_spec(
    name: &str,
    image: &str,
    mode: &str,
    replicas: i64,
    config: Option<&SwarmSpecConfig>,
) -> ServiceSpec {
    let mode = if mode == "global" {
        ServiceSpecMode {
            global: Some(Default::default()),
            ..Default::default()
        }
    } else {
        ServiceSpecMode {
            replicated: Some(ServiceSpecModeReplicated {
                replicas: Some(replicas),
            }),
            ..Default::default()
        }
    };
    let mut spec = ServiceSpec {
        name: Some(name.to_string()),
        mode: Some(mode),
        task_template: Some(TaskSpec {
            container_spec: Some(TaskSpecContainerSpec {
                image: Some(image.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    if let Some(cfg) = config {
        cfg.apply_to(&mut spec);
    }
    spec
}

/// Load an app's stored spec (default when none is saved).
pub async fn load_app_spec(state: &AppState, app_id: &str) -> Result<SwarmSpecConfig, ApiError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT spec FROM app_swarm_specs WHERE app_id = ?")
            .bind(app_id)
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::from)?;
    Ok(row
        .and_then(|(s,)| serde_json::from_str(&s).ok())
        .unwrap_or_default())
}

/// Re-apply the spec to an existing Swarm service (inspect, merge, update).
pub async fn apply_to_existing(
    docker: &Docker,
    service_name: &str,
    config: &SwarmSpecConfig,
) -> anyhow::Result<()> {
    let current = docker.inspect_service(service_name, None).await?;
    let version = current
        .version
        .and_then(|v| v.index)
        .ok_or_else(|| anyhow::anyhow!("service has no version"))?;
    let mut spec = current.spec.unwrap_or_default();
    config.apply_to(&mut spec);
    let options = UpdateServiceOptionsBuilder::new()
        .version(i32::try_from(version)?)
        .build();
    docker
        .update_service(service_name, spec, options, None)
        .await?;
    Ok(())
}

fn local_docker() -> Result<Docker, ApiError> {
    Docker::connect_with_local_defaults()
        .map_err(|e| ApiError::internal(format!("Failed to connect to Docker: {e}")))
}

#[derive(Debug, Serialize)]
pub struct AppSwarmSpecResponse {
    pub spec: SwarmSpecConfig,
    /// Whether this Docker host is part of an active swarm.
    pub swarm_active: bool,
    /// Swarm services bound to the app.
    pub services: Vec<SwarmService>,
}

#[derive(Debug, Serialize)]
pub struct SaveSwarmSpecResponse {
    pub spec: SwarmSpecConfig,
    /// Services the spec was applied to.
    pub applied: Vec<String>,
    /// Services that could not be updated, with the error.
    pub errors: Vec<String>,
}

async fn app_services(state: &AppState, app_id: &str) -> Result<Vec<SwarmService>, ApiError> {
    sqlx::query_as::<_, SwarmService>(
        "SELECT * FROM swarm_services WHERE app_id = ? ORDER BY created_at",
    )
    .bind(app_id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::from)
}

/// GET /api/apps/{id}/swarm-spec
pub async fn get_app_swarm_spec(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(app_id): Path<String>,
) -> Result<Json<AppSwarmSpecResponse>, ApiError> {
    authz::authorize_app(&state, &user, &app_id).await?;
    let spec = load_app_spec(&state, &app_id).await?;
    let services = app_services(&state, &app_id).await?;
    let swarm_active = match Docker::connect_with_local_defaults() {
        Ok(docker) => docker
            .info()
            .await
            .ok()
            .and_then(|i| i.swarm)
            .and_then(|s| s.local_node_state)
            .is_some_and(|s| s == LocalNodeState::ACTIVE),
        Err(_) => false,
    };
    Ok(Json(AppSwarmSpecResponse {
        spec,
        swarm_active,
        services,
    }))
}

/// PUT /api/apps/{id}/swarm-spec
pub async fn update_app_swarm_spec(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(app_id): Path<String>,
    Json(spec): Json<SwarmSpecConfig>,
) -> Result<Json<SaveSwarmSpecResponse>, ApiError> {
    authz::authorize_app_write(&state, &user, &app_id).await?;
    spec.validate().map_err(ApiError::bad_request)?;

    let json = serde_json::to_string(&spec).map_err(|e| ApiError::internal(e.to_string()))?;
    sqlx::query(
        "INSERT INTO app_swarm_specs (app_id, spec) VALUES (?, ?)
         ON CONFLICT(app_id) DO UPDATE SET spec = excluded.spec, updated_at = datetime('now')",
    )
    .bind(&app_id)
    .bind(&json)
    .execute(&state.db)
    .await
    .map_err(ApiError::from)?;

    let services = app_services(&state, &app_id).await?;
    let mut applied = Vec::new();
    let mut errors = Vec::new();
    if !services.is_empty() {
        let docker = local_docker()?;
        for svc in services {
            match apply_to_existing(&docker, &svc.service_name, &spec).await {
                Ok(()) => applied.push(svc.service_name),
                Err(e) => errors.push(format!("{}: {e}", svc.service_name)),
            }
        }
    }

    Ok(Json(SaveSwarmSpecResponse {
        spec,
        applied,
        errors,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::models::Limit;

    fn full() -> SwarmSpecConfig {
        SwarmSpecConfig {
            constraints: vec![
                "node.role==worker".into(),
                " node.labels.zone!=eu-1 ".into(),
            ],
            spread_preferences: vec!["node.labels.zone".into()],
            max_replicas_per_node: Some(2),
            update_config: Some(SwarmRolloutConfig {
                parallelism: Some(1),
                delay_secs: Some(10),
                failure_action: Some("rollback".into()),
                order: Some("start-first".into()),
                monitor_secs: Some(5),
                max_failure_ratio: Some(0.25),
            }),
            rollback_config: Some(SwarmRolloutConfig {
                parallelism: Some(2),
                delay_secs: Some(3),
                failure_action: Some("pause".into()),
                order: Some("stop-first".into()),
                monitor_secs: None,
                max_failure_ratio: None,
            }),
            reserve_cpus: Some(0.5),
            reserve_memory_mb: Some(256),
            restart_policy: Some(SwarmRestartPolicy {
                condition: "on-failure".into(),
                delay_secs: Some(5),
                max_attempts: Some(3),
                window_secs: Some(120),
            }),
        }
    }

    #[test]
    fn maps_full_spec_to_bollard_types() {
        let cfg = full();
        cfg.validate().unwrap();
        let spec = new_service_spec("web", "nginx:1.27", "replicated", 3, Some(&cfg));

        assert_eq!(spec.name.as_deref(), Some("web"));
        assert_eq!(
            spec.mode
                .as_ref()
                .unwrap()
                .replicated
                .as_ref()
                .unwrap()
                .replicas,
            Some(3)
        );
        let task = spec.task_template.as_ref().unwrap();
        assert_eq!(
            task.container_spec.as_ref().unwrap().image.as_deref(),
            Some("nginx:1.27")
        );

        let placement = task.placement.as_ref().unwrap();
        assert_eq!(
            placement.constraints.as_deref().unwrap(),
            ["node.role==worker", "node.labels.zone!=eu-1"]
        );
        assert_eq!(
            placement.preferences.as_ref().unwrap()[0]
                .spread
                .as_ref()
                .unwrap()
                .spread_descriptor
                .as_deref(),
            Some("node.labels.zone")
        );
        assert_eq!(placement.max_replicas, Some(2));

        let res = task
            .resources
            .as_ref()
            .unwrap()
            .reservations
            .as_ref()
            .unwrap();
        assert_eq!(res.nano_cpus, Some(500_000_000));
        assert_eq!(res.memory_bytes, Some(256 * 1024 * 1024));

        let restart = task.restart_policy.as_ref().unwrap();
        assert_eq!(
            restart.condition,
            Some(TaskSpecRestartPolicyConditionEnum::ON_FAILURE)
        );
        assert_eq!(restart.delay, Some(5 * NANOS_PER_SEC));
        assert_eq!(restart.max_attempts, Some(3));
        assert_eq!(restart.window, Some(120 * NANOS_PER_SEC));

        let update = spec.update_config.as_ref().unwrap();
        assert_eq!(update.parallelism, Some(1));
        assert_eq!(update.delay, Some(10 * NANOS_PER_SEC));
        assert_eq!(
            update.failure_action,
            Some(ServiceSpecUpdateConfigFailureActionEnum::ROLLBACK)
        );
        assert_eq!(
            update.order,
            Some(ServiceSpecUpdateConfigOrderEnum::START_FIRST)
        );
        assert_eq!(update.monitor, Some(5 * NANOS_PER_SEC));
        assert_eq!(update.max_failure_ratio, Some(0.25));

        let rollback = spec.rollback_config.as_ref().unwrap();
        assert_eq!(rollback.parallelism, Some(2));
        assert_eq!(
            rollback.failure_action,
            Some(ServiceSpecRollbackConfigFailureActionEnum::PAUSE)
        );
        assert_eq!(
            rollback.order,
            Some(ServiceSpecRollbackConfigOrderEnum::STOP_FIRST)
        );

        // Serialises to the Docker Engine API field names.
        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(
            json["TaskTemplate"]["Placement"]["Preferences"][0]["Spread"]["SpreadDescriptor"],
            "node.labels.zone"
        );
        assert_eq!(json["UpdateConfig"]["Order"], "start-first");
        assert_eq!(
            json["TaskTemplate"]["RestartPolicy"]["Condition"],
            "on-failure"
        );
    }

    #[test]
    fn global_mode_and_empty_config() {
        let spec = new_service_spec("agent", "img:1", "global", 5, None);
        let mode = spec.mode.unwrap();
        assert!(mode.global.is_some());
        assert!(mode.replicated.is_none());
        assert!(spec.update_config.is_none());
        assert!(SwarmSpecConfig::default().is_empty());
    }

    #[test]
    fn apply_preserves_unmanaged_fields_and_clears_removed_ones() {
        let mut spec = new_service_spec("web", "nginx:1.27", "replicated", 1, Some(&full()));
        let task = spec.task_template.as_mut().unwrap();
        task.resources.as_mut().unwrap().limits = Some(Limit {
            nano_cpus: Some(1_000_000_000),
            ..Default::default()
        });
        task.placement.as_mut().unwrap().platforms = Some(vec![]);

        // Clearing the config removes what it managed but keeps the rest.
        SwarmSpecConfig::default().apply_to(&mut spec);
        let task = spec.task_template.as_ref().unwrap();
        assert_eq!(
            task.container_spec.as_ref().unwrap().image.as_deref(),
            Some("nginx:1.27")
        );
        let resources = task.resources.as_ref().unwrap();
        assert!(resources.reservations.is_none());
        assert_eq!(
            resources.limits.as_ref().unwrap().nano_cpus,
            Some(1_000_000_000)
        );
        let placement = task.placement.as_ref().unwrap();
        assert!(placement.constraints.is_none());
        assert!(placement.preferences.is_none());
        assert!(placement.platforms.is_some());
        assert!(task.restart_policy.is_none());
        assert!(spec.update_config.is_none());
        assert!(spec.rollback_config.is_none());
    }

    #[test]
    fn validation_rejects_bad_input() {
        let bad = |f: fn(&mut SwarmSpecConfig)| {
            let mut c = SwarmSpecConfig::default();
            f(&mut c);
            c.validate().is_err()
        };
        assert!(bad(|c| c.constraints = vec!["node.role".into()]));
        assert!(bad(|c| c.constraints = vec!["node.role==".into()]));
        assert!(bad(|c| c.constraints = vec!["node role==x".into()]));
        assert!(bad(|c| c.spread_preferences = vec!["node labels".into()]));
        assert!(bad(|c| c.reserve_cpus = Some(-1.0)));
        assert!(bad(|c| c.reserve_memory_mb = Some(-5)));
        assert!(bad(|c| {
            c.rollback_config = Some(SwarmRolloutConfig {
                failure_action: Some("rollback".into()),
                ..Default::default()
            })
        }));
        assert!(bad(|c| {
            c.update_config = Some(SwarmRolloutConfig {
                order: Some("random".into()),
                ..Default::default()
            })
        }));
        assert!(bad(|c| {
            c.update_config = Some(SwarmRolloutConfig {
                max_failure_ratio: Some(1.5),
                ..Default::default()
            })
        }));
        assert!(bad(|c| {
            c.restart_policy = Some(SwarmRestartPolicy {
                condition: "always".into(),
                ..Default::default()
            })
        }));
        assert!(full().validate().is_ok());
    }
}
