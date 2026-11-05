//! One app across several servers behind the Rivetr proxy.
//!
//! An app with rows in `app_server_placements` is deployed here instead of the
//! single-host pipeline: the image is built once (locally or on a build
//! server), distributed to every target server (registry pull or
//! `docker save | ssh docker load`), started with a rolling strategy, health
//! checked over the network path the proxy will use, and added to an upstream
//! pool for the app's domains. When too many targets fail the whole rollout is
//! rolled back and the previous containers keep serving.

mod deploy;
pub mod distribute;
mod lifecycle;
pub mod plan;
pub mod routes;
mod shell;

pub use deploy::run_and_finalize;
pub use lifecycle::{remove_all, set_running};
pub use routes::{healthy_upstreams, install_routes, restore_routes};

use crate::db::{AppServerPlacement, MultiServerSettings, MultiServerSettingsRow};
use crate::DbPool;

/// Placement rows of an app, in placement order.
pub async fn load_placements(db: &DbPool, app_id: &str) -> Vec<AppServerPlacement> {
    sqlx::query_as::<_, AppServerPlacement>(
        "SELECT * FROM app_server_placements WHERE app_id = ? ORDER BY sort_order ASC, created_at ASC",
    )
    .bind(app_id)
    .fetch_all(db)
    .await
    .unwrap_or_default()
}

/// Rollout and load balancing settings of an app (defaults when unset).
pub async fn load_settings(db: &DbPool, app_id: &str) -> MultiServerSettings {
    sqlx::query_as::<_, MultiServerSettingsRow>(
        "SELECT app_id, max_unavailable, failure_threshold_pct, distribution, lb_policy, \
                sticky_sessions, bind_private \
         FROM app_multi_server_settings WHERE app_id = ?",
    )
    .bind(app_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .map(MultiServerSettings::from)
    .unwrap_or_default()
}

/// Whether an app deploys through the multi-server pipeline (it has a placement).
pub async fn is_enabled(db: &DbPool, app_id: &str) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM app_server_placements WHERE app_id = ?")
        .bind(app_id)
        .fetch_one(db)
        .await
        .map(|n| n > 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::RouteTable;
    use arc_swap::ArcSwap;
    use std::sync::Arc;

    async fn db_with_app() -> (tempfile::TempDir, DbPool) {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::init(dir.path()).await.unwrap();
        sqlx::query(
            "INSERT INTO apps (id, name, git_url, branch, port, domain) \
             VALUES ('a1', 'web', 'https://example.com/r.git', 'main', 3000, 'web.example.com')",
        )
        .execute(&pool)
        .await
        .unwrap();
        (dir, pool)
    }

    #[tokio::test]
    async fn migration_is_idempotent_and_placements_load_in_order() {
        let (dir, pool) = db_with_app().await;
        assert!(!is_enabled(&pool, "a1").await);
        assert_eq!(
            load_settings(&pool, "a1").await,
            MultiServerSettings::default()
        );
        sqlx::query(
            "INSERT INTO app_server_placements (id, app_id, server_id, replicas, sort_order) VALUES \
             ('p2', 'a1', 's2', 2, 1), ('p1', 'a1', 'localhost', 1, 0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO app_multi_server_settings (app_id, max_unavailable, lb_policy, sticky_sessions) \
             VALUES ('a1', 2, 'least_conn', 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        // Running the migrations again keeps the rows.
        let pool = crate::db::init(dir.path()).await.unwrap();
        assert!(is_enabled(&pool, "a1").await);
        let placements = load_placements(&pool, "a1").await;
        let ids: Vec<&str> = placements.iter().map(|p| p.server_id.as_str()).collect();
        assert_eq!(ids, ["localhost", "s2"]);
        let s = load_settings(&pool, "a1").await;
        assert_eq!(s.max_unavailable, 2);
        assert_eq!(s.lb_policy, "least_conn");
        assert!(s.sticky_sessions);
        assert_eq!(plan::plan_slots(&placements).len(), 3);

        // Deleting the app removes its placement.
        sqlx::query("DELETE FROM apps WHERE id = 'a1'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(!is_enabled(&pool, "a1").await);
    }

    #[tokio::test]
    async fn restore_routes_pools_healthy_targets() {
        let (_dir, pool) = db_with_app().await;
        sqlx::query(
            "INSERT INTO app_server_placements (id, app_id, server_id) VALUES ('p1', 'a1', 'localhost')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO deployments (id, app_id, status) VALUES ('d1', 'a1', 'running')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO deployment_targets (id, deployment_id, app_id, server_id, status, upstream) VALUES \
             ('t1', 'd1', 'a1', 'localhost', 'healthy', '127.0.0.1:41000'), \
             ('t2', 'd1', 'a1', 's2', 'healthy', '100.64.0.2:41001'), \
             ('t3', 'd1', 'a1', 's3', 'failed', '100.64.0.3:41002')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let routes = Arc::new(ArcSwap::new(Arc::new(RouteTable::new())));
        restore_routes(&pool, &routes).await;
        let table = routes.load();
        let p = table.pool("web.example.com").expect("pooled route");
        let mut ups = p.upstreams().to_vec();
        ups.sort();
        assert_eq!(ups, ["100.64.0.2:41001", "127.0.0.1:41000"]);
    }
}
