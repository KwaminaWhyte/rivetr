// Dashboard UI module - Phase 1.9-1.11
// Uses Askama templates + HTMX for server-side rendering

mod templates;

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use std::sync::Arc;

use crate::db::{App, Deployment};
use crate::AppState;

pub use templates::*;

// Helper to render templates and handle errors
fn render_template<T: Template>(template: T) -> Response {
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Template error: {}", e)).into_response(),
    }
}

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        // Public routes
        .route("/", get(dashboard))
        .route("/login", get(login_page))
        .route("/login", post(login_submit))
        .route("/logout", get(logout))
        // Protected routes
        .route("/apps", get(apps_list))
        .route("/apps/new", get(app_new_form))
        .route("/apps/new", post(app_create))
        .route("/apps/:id", get(app_detail))
        .route("/apps/:id/edit", get(app_edit_form))
        .route("/deployments/:id", get(deployment_detail))
        .route("/settings", get(settings_page))
}

// Session token cookie name
const SESSION_COOKIE: &str = "rivetr_session";

// Check if user is authenticated
fn is_authenticated(jar: &CookieJar, state: &AppState) -> bool {
    jar.get(SESSION_COOKIE)
        .map(|c| c.value() == state.config.auth.admin_token)
        .unwrap_or(false)
}

// Get token from cookie
fn get_token(jar: &CookieJar) -> Option<String> {
    jar.get(SESSION_COOKIE).map(|c| c.value().to_string())
}

// Dashboard home
async fn dashboard(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Response {
    if !is_authenticated(&jar, &state) {
        return Redirect::to("/dashboard/login").into_response();
    }

    // Get stats
    let total_apps: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM apps")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let running_apps: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT app_id) FROM deployments WHERE status = 'running'"
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let total_deployments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deployments")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let failed_deployments: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deployments WHERE status = 'failed'"
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // Get recent deployments
    let recent_deployments: Vec<RecentDeployment> = sqlx::query_as(
        r#"
        SELECT d.id, d.app_id, a.name as app_name, d.status, d.started_at, d.finished_at
        FROM deployments d
        JOIN apps a ON d.app_id = a.id
        ORDER BY d.started_at DESC
        LIMIT 10
        "#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // Get apps
    let apps: Vec<AppWithStatus> = get_apps_with_status(&state.db).await;

    let template = DashboardTemplate {
        stats: DashboardStats {
            total_apps: total_apps as u32,
            running_apps: running_apps as u32,
            total_deployments: total_deployments as u32,
            failed_deployments: failed_deployments as u32,
        },
        recent_deployments,
        apps,
        token: get_token(&jar).unwrap_or_default(),
    };

    render_template(template)
}

// Login page
async fn login_page() -> Response {
    let template = LoginTemplate {
        error: None,
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    render_template(template)
}

#[derive(Deserialize)]
struct LoginForm {
    token: String,
}

// Login submit
async fn login_submit(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> Response {
    if form.token == state.config.auth.admin_token {
        let jar = jar.add(
            axum_extra::extract::cookie::Cookie::build((SESSION_COOKIE, form.token))
                .path("/")
                .http_only(true)
                .same_site(axum_extra::extract::cookie::SameSite::Lax)
                .build()
        );
        (jar, Redirect::to("/dashboard")).into_response()
    } else {
        let template = LoginTemplate {
            error: Some("Invalid token".to_string()),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };
        let html = template.render().unwrap_or_else(|e| format!("Error: {}", e));
        (StatusCode::UNAUTHORIZED, Html(html)).into_response()
    }
}

// Logout
async fn logout(jar: CookieJar) -> impl IntoResponse {
    let jar = jar.remove(axum_extra::extract::cookie::Cookie::from(SESSION_COOKIE));
    (jar, Redirect::to("/dashboard/login"))
}

// Apps list
async fn apps_list(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Response {
    if !is_authenticated(&jar, &state) {
        return Redirect::to("/dashboard/login").into_response();
    }

    let apps = get_apps_with_status(&state.db).await;

    let template = AppsTemplate {
        apps,
        token: get_token(&jar).unwrap_or_default(),
    };

    render_template(template)
}

// New app form
async fn app_new_form(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Response {
    if !is_authenticated(&jar, &state) {
        return Redirect::to("/dashboard/login").into_response();
    }

    let template = AppNewTemplate { error: None };
    render_template(template)
}

#[derive(Deserialize)]
struct NewAppForm {
    name: String,
    git_url: String,
    branch: Option<String>,
    domain: Option<String>,
    port: Option<i32>,
    dockerfile: Option<String>,
    healthcheck: Option<String>,
}

// Create app
async fn app_create(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<NewAppForm>,
) -> Response {
    if !is_authenticated(&jar, &state) {
        return Redirect::to("/dashboard/login").into_response();
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query(
        r#"
        INSERT INTO apps (id, name, git_url, branch, dockerfile, domain, port, healthcheck, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&id)
    .bind(&form.name)
    .bind(&form.git_url)
    .bind(form.branch.as_deref().unwrap_or("main"))
    .bind(form.dockerfile.as_deref().unwrap_or("Dockerfile"))
    .bind(&form.domain.filter(|s| !s.is_empty()))
    .bind(form.port.unwrap_or(3000))
    .bind(&form.healthcheck.filter(|s| !s.is_empty()))
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => Redirect::to(&format!("/dashboard/apps/{}", id)).into_response(),
        Err(e) => {
            let template = AppNewTemplate {
                error: Some(format!("Failed to create app: {}", e)),
            };
            let html = template.render().unwrap_or_else(|e| format!("Error: {}", e));
            (StatusCode::BAD_REQUEST, Html(html)).into_response()
        }
    }
}

// App detail
async fn app_detail(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Response {
    if !is_authenticated(&jar, &state) {
        return Redirect::to("/dashboard/login").into_response();
    }

    let app: Option<App> = sqlx::query_as("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

    let app = match app {
        Some(a) => a,
        None => return (StatusCode::NOT_FOUND, "App not found").into_response(),
    };

    // Get latest deployment status
    let status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM deployments WHERE app_id = ? ORDER BY started_at DESC LIMIT 1"
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    // Get deployments
    let deployments: Vec<Deployment> = sqlx::query_as(
        "SELECT * FROM deployments WHERE app_id = ? ORDER BY started_at DESC LIMIT 20"
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let base_url = format!(
        "http://{}:{}",
        state.config.server.host,
        state.config.server.api_port
    );

    let template = AppDetailTemplate {
        app,
        status: status.unwrap_or_else(|| "pending".to_string()),
        deployments,
        base_url,
        token: get_token(&jar).unwrap_or_default(),
    };

    render_template(template)
}

// App edit form (placeholder)
async fn app_edit_form(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Response {
    if !is_authenticated(&jar, &state) {
        return Redirect::to("/dashboard/login").into_response();
    }

    // TODO: Implement edit form
    Redirect::to(&format!("/dashboard/apps/{}", id)).into_response()
}

// Deployment detail (placeholder)
async fn deployment_detail(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Response {
    if !is_authenticated(&jar, &state) {
        return Redirect::to("/dashboard/login").into_response();
    }

    // TODO: Implement deployment detail with logs
    Html(format!("Deployment {} logs - Coming soon", id)).into_response()
}

// Settings page (placeholder)
async fn settings_page(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Response {
    if !is_authenticated(&jar, &state) {
        return Redirect::to("/dashboard/login").into_response();
    }

    Html("Settings - Coming soon").into_response()
}

// Helper to get apps with their status
async fn get_apps_with_status(db: &crate::DbPool) -> Vec<AppWithStatus> {
    let apps: Vec<App> = sqlx::query_as("SELECT * FROM apps ORDER BY name")
        .fetch_all(db)
        .await
        .unwrap_or_default();

    let mut result = Vec::new();
    for app in apps {
        let status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM deployments WHERE app_id = ? ORDER BY started_at DESC LIMIT 1"
        )
        .bind(&app.id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();

        let last_deploy: Option<String> = sqlx::query_scalar(
            "SELECT started_at FROM deployments WHERE app_id = ? ORDER BY started_at DESC LIMIT 1"
        )
        .bind(&app.id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();

        result.push(AppWithStatus {
            id: app.id,
            name: app.name,
            git_url: app.git_url,
            domain: app.domain.unwrap_or_else(|| "-".to_string()),
            status: status.unwrap_or_else(|| "pending".to_string()),
            last_deploy: last_deploy.unwrap_or_else(|| "Never".to_string()),
        });
    }
    result
}
