mod models;

pub use models::*;

use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;
use tracing::info;

pub type DbPool = SqlitePool;

/// Execute a SQL migration file, properly handling comments
async fn execute_sql(pool: &SqlitePool, sql: &str) -> Result<()> {
    for statement in sql.split(';') {
        // Strip SQL comment lines (lines starting with --)
        let cleaned: String = statement
            .lines()
            .filter(|line| !line.trim().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        let trimmed = cleaned.trim();
        if !trimmed.is_empty() {
            sqlx::query(trimmed).execute(pool).await?;
        }
    }
    Ok(())
}

pub async fn init(data_dir: &Path) -> Result<DbPool> {
    let db_path = data_dir.join("rivetr.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    info!("Initializing database at {}", db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Enable WAL mode for better concurrency
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    // Run migrations
    run_migrations(&pool).await?;

    info!("Database initialized successfully");
    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    info!("Running database migrations...");

    // Migration 001: Initial schema
    execute_sql(pool, include_str!("../../migrations/001_initial.sql")).await?;

    // Migration 002: Users table
    execute_sql(pool, include_str!("../../migrations/002_users.sql")).await?;

    // Migration 003: Add image_tag column for rollback support
    let has_image_tag: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('deployments') WHERE name = 'image_tag'"
    )
    .fetch_optional(pool)
    .await?;
    if has_image_tag.is_none() {
        execute_sql(pool, include_str!("../../migrations/003_rollback.sql")).await?;
    }

    // Migration 004: Add SSH keys table for private repository authentication
    let has_ssh_keys_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='ssh_keys'"
    )
    .fetch_optional(pool)
    .await?;
    if has_ssh_keys_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/004_ssh_keys.sql")).await?;
    }

    // Migration 005: Add git_providers table for OAuth connections
    let has_git_providers_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='git_providers'"
    )
    .fetch_optional(pool)
    .await?;
    if has_git_providers_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/005_git_providers.sql")).await?;
    }

    // Migration 006: Add environment field to apps
    let has_environment: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'environment'"
    )
    .fetch_optional(pool)
    .await?;
    if has_environment.is_none() {
        execute_sql(pool, include_str!("../../migrations/006_environment.sql")).await?;
    }

    // Migration 007: Add projects table and project_id to apps
    let has_projects_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='projects'"
    )
    .fetch_optional(pool)
    .await?;
    if has_projects_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/007_projects.sql")).await?;
    }

    // Migration 008: Add is_secret and updated_at columns to env_vars
    let has_is_secret: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('env_vars') WHERE name = 'is_secret'"
    )
    .fetch_optional(pool)
    .await?;
    if has_is_secret.is_none() {
        execute_sql(pool, include_str!("../../migrations/008_env_vars_update.sql")).await?;
    }

    // Migration 009: Add advanced build options to apps
    let has_dockerfile_path: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'dockerfile_path'"
    )
    .fetch_optional(pool)
    .await?;
    if has_dockerfile_path.is_none() {
        execute_sql(pool, include_str!("../../migrations/009_build_options.sql")).await?;
    }

    // Migration 010: Add domain management to apps
    let has_domains: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'domains'"
    )
    .fetch_optional(pool)
    .await?;
    if has_domains.is_none() {
        execute_sql(pool, include_str!("../../migrations/010_domains.sql")).await?;
    }

    // Migration 011: Add network configuration to apps
    let has_port_mappings: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'port_mappings'"
    )
    .fetch_optional(pool)
    .await?;
    if has_port_mappings.is_none() {
        execute_sql(pool, include_str!("../../migrations/011_network_config.sql")).await?;
    }

    // Migration 012: Add HTTP basic auth to apps
    let has_basic_auth: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'basic_auth_enabled'"
    )
    .fetch_optional(pool)
    .await?;
    if has_basic_auth.is_none() {
        execute_sql(pool, include_str!("../../migrations/012_basic_auth.sql")).await?;
    }

    // Migration 013: Add pre/post deployment commands to apps
    let has_pre_deploy: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'pre_deploy_commands'"
    )
    .fetch_optional(pool)
    .await?;
    if has_pre_deploy.is_none() {
        execute_sql(pool, include_str!("../../migrations/013_deployment_commands.sql")).await?;
    }

    // Migration 014: Add docker registry support
    let has_docker_image: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'docker_image'"
    )
    .fetch_optional(pool)
    .await?;
    if has_docker_image.is_none() {
        execute_sql(pool, include_str!("../../migrations/014_docker_registry.sql")).await?;
    }

    // Migration 015: Add teams and team_members tables for multi-user support
    let has_teams_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='teams'"
    )
    .fetch_optional(pool)
    .await?;
    if has_teams_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/015_teams.sql")).await?;
    }

    // Migration 016: Add notification channels and subscriptions
    let has_notification_channels: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='notification_channels'"
    )
    .fetch_optional(pool)
    .await?;
    if has_notification_channels.is_none() {
        execute_sql(pool, include_str!("../../migrations/016_notifications.sql")).await?;
    }

    // Migration 017: Add container_labels to apps
    let has_container_labels: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'container_labels'"
    )
    .fetch_optional(pool)
    .await?;
    if has_container_labels.is_none() {
        execute_sql(pool, include_str!("../../migrations/017_container_labels.sql")).await?;
    }

    // Migration 018: Add volumes table for persistent storage
    let has_volumes_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='volumes'"
    )
    .fetch_optional(pool)
    .await?;
    if has_volumes_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/018_volumes.sql")).await?;
    }

    // Migration 019: Add databases table for managed database deployments
    let has_databases_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='databases'"
    )
    .fetch_optional(pool)
    .await?;
    if has_databases_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/019_databases.sql")).await?;
    }

    // Migration 020: Add project_id to databases table
    let has_db_project_id: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('databases') WHERE name = 'project_id'"
    )
    .fetch_optional(pool)
    .await?;
    if has_db_project_id.is_none() {
        execute_sql(pool, include_str!("../../migrations/020_databases_project.sql")).await?;
    }

    // Migration 021: Add database backups and backup schedules tables
    let has_database_backups_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='database_backups'"
    )
    .fetch_optional(pool)
    .await?;
    if has_database_backups_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/021_database_backups.sql")).await?;
    }

    // Migration 022: Add services table for Docker Compose services
    let has_services_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='services'"
    )
    .fetch_optional(pool)
    .await?;
    if has_services_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/022_services.sql")).await?;
    }

    // Migration 023: Add service_templates table
    let has_service_templates_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='service_templates'"
    )
    .fetch_optional(pool)
    .await?;
    if has_service_templates_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/023_service_templates.sql")).await?;
    }

    // Migration 024: Add audit_logs table for tracking user actions
    let has_audit_logs_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='audit_logs'"
    )
    .fetch_optional(pool)
    .await?;
    if has_audit_logs_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/024_audit_logs.sql")).await?;
    }

    // Migration 025: Add stats_history table for dashboard charts
    let has_stats_history_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='stats_history'"
    )
    .fetch_optional(pool)
    .await?;
    if has_stats_history_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/025_stats_history.sql")).await?;
    }

    // Seed/update built-in templates (runs on every startup to add new templates)
    seed_builtin_templates(pool).await?;

    info!("Migrations completed");
    Ok(())
}

/// Seed built-in service templates (runs on every startup to add/update templates)
async fn seed_builtin_templates(pool: &SqlitePool) -> Result<()> {
    info!("Seeding built-in service templates...");

    // Define built-in templates - these get added/updated on every startup
    let templates: Vec<(&str, &str, &str, &str, &str, &str, &str)> = vec![
        // (id, name, description, category, icon, compose_template, env_schema)
        (
            "portainer",
            "Portainer",
            "A powerful, open-source container management UI for Docker and Kubernetes.",
            "development",
            "portainer",
            r#"services:
  portainer:
    image: portainer/portainer-ce:${PORTAINER_VERSION:-latest}
    container_name: ${CONTAINER_NAME:-portainer}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-9000}:9000"
      - "${HTTPS_PORT:-9443}:9443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - portainer_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  portainer_data:
"#,
            r#"[{"name":"PORTAINER_VERSION","label":"Portainer Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"portainer","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"9000","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"9443","secret":false}]"#,
        ),
        (
            "uptime-kuma",
            "Uptime Kuma",
            "A self-hosted monitoring tool like Uptime Robot. Monitor HTTP(s), TCP, DNS, Docker containers, and more.",
            "monitoring",
            "uptime-kuma",
            r#"services:
  uptime-kuma:
    image: louislam/uptime-kuma:${VERSION:-1}
    container_name: ${CONTAINER_NAME:-uptime-kuma}
    restart: unless-stopped
    ports:
      - "${PORT:-3001}:3001"
    volumes:
      - uptime_kuma_data:/app/data
      - /var/run/docker.sock:/var/run/docker.sock:ro
    labels:
      - "rivetr.managed=true"

volumes:
  uptime_kuma_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"1","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"uptime-kuma","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3001","secret":false}]"#,
        ),
        (
            "grafana-prometheus",
            "Grafana + Prometheus",
            "Complete monitoring stack with Prometheus for metrics collection and Grafana for visualization.",
            "monitoring",
            "grafana",
            r#"services:
  prometheus:
    image: prom/prometheus:${PROMETHEUS_VERSION:-latest}
    container_name: ${PROMETHEUS_NAME:-prometheus}
    restart: unless-stopped
    ports:
      - "${PROMETHEUS_PORT:-9090}:9090"
    volumes:
      - prometheus_data:/prometheus
    command:
      - "--config.file=/etc/prometheus/prometheus.yml"
      - "--storage.tsdb.path=/prometheus"
      - "--web.enable-lifecycle"
    labels:
      - "rivetr.managed=true"

  grafana:
    image: grafana/grafana:${GRAFANA_VERSION:-latest}
    container_name: ${GRAFANA_NAME:-grafana}
    restart: unless-stopped
    ports:
      - "${GRAFANA_PORT:-3000}:3000"
    environment:
      - GF_SECURITY_ADMIN_USER=${GRAFANA_USER:-admin}
      - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_PASSWORD:-admin}
      - GF_USERS_ALLOW_SIGN_UP=false
    volumes:
      - grafana_data:/var/lib/grafana
    depends_on:
      - prometheus
    labels:
      - "rivetr.managed=true"

volumes:
  prometheus_data:
  grafana_data:
"#,
            r#"[{"name":"PROMETHEUS_VERSION","label":"Prometheus Version","required":false,"default":"latest","secret":false},{"name":"GRAFANA_VERSION","label":"Grafana Version","required":false,"default":"latest","secret":false},{"name":"PROMETHEUS_NAME","label":"Prometheus Container","required":false,"default":"prometheus","secret":false},{"name":"GRAFANA_NAME","label":"Grafana Container","required":false,"default":"grafana","secret":false},{"name":"PROMETHEUS_PORT","label":"Prometheus Port","required":false,"default":"9090","secret":false},{"name":"GRAFANA_PORT","label":"Grafana Port","required":false,"default":"3000","secret":false},{"name":"GRAFANA_USER","label":"Grafana Admin User","required":false,"default":"admin","secret":false},{"name":"GRAFANA_PASSWORD","label":"Grafana Admin Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "gitea",
            "Gitea",
            "A lightweight, self-hosted Git service with GitHub-like functionality.",
            "development",
            "gitea",
            r#"services:
  gitea:
    image: gitea/gitea:${GITEA_VERSION:-latest}
    container_name: ${CONTAINER_NAME:-gitea}
    restart: unless-stopped
    environment:
      - USER_UID=1000
      - USER_GID=1000
      - GITEA__database__DB_TYPE=sqlite3
      - GITEA__server__ROOT_URL=${ROOT_URL:-http://localhost:3000}
    ports:
      - "${HTTP_PORT:-3000}:3000"
      - "${SSH_PORT:-2222}:22"
    volumes:
      - gitea_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  gitea_data:
"#,
            r#"[{"name":"GITEA_VERSION","label":"Gitea Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"gitea","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"3000","secret":false},{"name":"SSH_PORT","label":"SSH Port","required":false,"default":"2222","secret":false},{"name":"ROOT_URL","label":"Root URL","required":false,"default":"http://localhost:3000","secret":false}]"#,
        ),
        (
            "minio",
            "MinIO",
            "High-performance, S3-compatible object storage. Perfect for storing backups, files, and static assets.",
            "storage",
            "minio",
            r#"services:
  minio:
    image: minio/minio:${MINIO_VERSION:-latest}
    container_name: ${CONTAINER_NAME:-minio}
    restart: unless-stopped
    ports:
      - "${API_PORT:-9000}:9000"
      - "${CONSOLE_PORT:-9001}:9001"
    environment:
      - MINIO_ROOT_USER=${ROOT_USER:-minioadmin}
      - MINIO_ROOT_PASSWORD=${ROOT_PASSWORD:-minioadmin}
    volumes:
      - minio_data:/data
    command: server /data --console-address ":9001"
    labels:
      - "rivetr.managed=true"

volumes:
  minio_data:
"#,
            r#"[{"name":"MINIO_VERSION","label":"MinIO Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"minio","secret":false},{"name":"API_PORT","label":"API Port","required":false,"default":"9000","secret":false},{"name":"CONSOLE_PORT","label":"Console Port","required":false,"default":"9001","secret":false},{"name":"ROOT_USER","label":"Root User","required":true,"default":"minioadmin","secret":false},{"name":"ROOT_PASSWORD","label":"Root Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "n8n",
            "n8n",
            "Workflow automation tool that connects apps and services. Self-hosted alternative to Zapier.",
            "development",
            "n8n",
            r#"services:
  n8n:
    image: n8nio/n8n:${N8N_VERSION:-latest}
    container_name: ${CONTAINER_NAME:-n8n}
    restart: unless-stopped
    ports:
      - "${PORT:-5678}:5678"
    environment:
      - N8N_BASIC_AUTH_ACTIVE=true
      - N8N_BASIC_AUTH_USER=${AUTH_USER:-admin}
      - N8N_BASIC_AUTH_PASSWORD=${AUTH_PASSWORD:-admin}
      - N8N_HOST=${HOST:-localhost}
      - N8N_PORT=5678
      - N8N_PROTOCOL=${PROTOCOL:-http}
      - WEBHOOK_URL=${WEBHOOK_URL:-http://localhost:5678/}
    volumes:
      - n8n_data:/home/node/.n8n
    labels:
      - "rivetr.managed=true"

volumes:
  n8n_data:
"#,
            r#"[{"name":"N8N_VERSION","label":"n8n Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"n8n","secret":false},{"name":"PORT","label":"Port","required":false,"default":"5678","secret":false},{"name":"AUTH_USER","label":"Auth Username","required":false,"default":"admin","secret":false},{"name":"AUTH_PASSWORD","label":"Auth Password","required":true,"default":"","secret":true},{"name":"HOST","label":"Hostname","required":false,"default":"localhost","secret":false},{"name":"PROTOCOL","label":"Protocol","required":false,"default":"http","secret":false},{"name":"WEBHOOK_URL","label":"Webhook URL","required":false,"default":"http://localhost:5678/","secret":false}]"#,
        ),
        (
            "traefik",
            "Traefik",
            "Modern HTTP reverse proxy and load balancer with automatic SSL/TLS via Let's Encrypt.",
            "networking",
            "traefik",
            r#"services:
  traefik:
    image: traefik:${TRAEFIK_VERSION:-v3.0}
    container_name: ${CONTAINER_NAME:-traefik}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
      - "${DASHBOARD_PORT:-8080}:8080"
    command:
      - "--api.dashboard=true"
      - "--api.insecure=true"
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - traefik_certs:/letsencrypt
    labels:
      - "rivetr.managed=true"

volumes:
  traefik_certs:
"#,
            r#"[{"name":"TRAEFIK_VERSION","label":"Traefik Version","required":false,"default":"v3.0","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"traefik","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false},{"name":"DASHBOARD_PORT","label":"Dashboard Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "plausible",
            "Plausible Analytics",
            "Privacy-friendly alternative to Google Analytics. Lightweight, open-source, and GDPR compliant.",
            "analytics",
            "plausible",
            r#"services:
  plausible:
    image: ghcr.io/plausible/community-edition:${VERSION:-v2}
    container_name: ${CONTAINER_NAME:-plausible}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - BASE_URL=${BASE_URL:-http://localhost:8000}
      - SECRET_KEY_BASE=${SECRET_KEY:-please-change-this-to-a-64-char-key}
      - DATABASE_URL=postgres://plausible:plausible@plausible_db:5432/plausible
      - CLICKHOUSE_DATABASE_URL=http://plausible_events_db:8123/plausible_events_db
    depends_on:
      - plausible_db
      - plausible_events_db
    labels:
      - "rivetr.managed=true"

  plausible_db:
    image: postgres:16-alpine
    restart: unless-stopped
    volumes:
      - plausible_db_data:/var/lib/postgresql/data
    environment:
      - POSTGRES_PASSWORD=plausible
      - POSTGRES_USER=plausible
      - POSTGRES_DB=plausible
    labels:
      - "rivetr.managed=true"

  plausible_events_db:
    image: clickhouse/clickhouse-server:24-alpine
    restart: unless-stopped
    volumes:
      - plausible_events_data:/var/lib/clickhouse
    ulimits:
      nofile:
        soft: 262144
        hard: 262144
    labels:
      - "rivetr.managed=true"

volumes:
  plausible_db_data:
  plausible_events_data:
"#,
            r#"[{"name":"VERSION","label":"Plausible Version","required":false,"default":"v2","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"plausible","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false},{"name":"BASE_URL","label":"Base URL","required":true,"default":"http://localhost:8000","secret":false},{"name":"SECRET_KEY","label":"Secret Key (64 chars)","required":true,"default":"","secret":true}]"#,
        ),
        (
            "nginx",
            "Nginx",
            "High-performance web server and reverse proxy. Great for serving static files.",
            "networking",
            "nginx",
            r#"services:
  nginx:
    image: nginx:${NGINX_VERSION:-alpine}
    container_name: ${CONTAINER_NAME:-nginx}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
    volumes:
      - nginx_html:/usr/share/nginx/html:ro
      - nginx_conf:/etc/nginx/conf.d:ro
    labels:
      - "rivetr.managed=true"

volumes:
  nginx_html:
  nginx_conf:
"#,
            r#"[{"name":"NGINX_VERSION","label":"Nginx Version","required":false,"default":"alpine","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"nginx","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false}]"#,
        ),
        (
            "vaultwarden",
            "Vaultwarden",
            "Unofficial Bitwarden-compatible server. Self-hosted password manager.",
            "security",
            "vaultwarden",
            r#"services:
  vaultwarden:
    image: vaultwarden/server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-vaultwarden}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - ADMIN_TOKEN=${ADMIN_TOKEN:-}
      - SIGNUPS_ALLOWED=${SIGNUPS_ALLOWED:-true}
      - DOMAIN=${DOMAIN:-http://localhost:8080}
    volumes:
      - vaultwarden_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  vaultwarden_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"vaultwarden","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"ADMIN_TOKEN","label":"Admin Token","required":false,"default":"","secret":true},{"name":"SIGNUPS_ALLOWED","label":"Allow Signups","required":false,"default":"true","secret":false},{"name":"DOMAIN","label":"Domain URL","required":false,"default":"http://localhost:8080","secret":false}]"#,
        ),
        (
            "wordpress",
            "WordPress",
            "The world's most popular content management system. Includes MySQL database.",
            "development",
            "wordpress",
            r#"services:
  wordpress:
    image: wordpress:${WP_VERSION:-latest}
    container_name: ${CONTAINER_NAME:-wordpress}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - WORDPRESS_DB_HOST=wordpress_db
      - WORDPRESS_DB_USER=wordpress
      - WORDPRESS_DB_PASSWORD=${DB_PASSWORD:-wordpress}
      - WORDPRESS_DB_NAME=wordpress
    volumes:
      - wordpress_data:/var/www/html
    depends_on:
      - wordpress_db
    labels:
      - "rivetr.managed=true"

  wordpress_db:
    image: mysql:8.0
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=wordpress
      - MYSQL_USER=wordpress
      - MYSQL_PASSWORD=${DB_PASSWORD:-wordpress}
      - MYSQL_RANDOM_ROOT_PASSWORD=yes
    volumes:
      - wordpress_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  wordpress_data:
  wordpress_db_data:
"#,
            r#"[{"name":"WP_VERSION","label":"WordPress Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"wordpress","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "ghost",
            "Ghost",
            "Professional publishing platform. Modern alternative to WordPress for blogs and newsletters.",
            "development",
            "ghost",
            r#"services:
  ghost:
    image: ghost:${GHOST_VERSION:-5-alpine}
    container_name: ${CONTAINER_NAME:-ghost}
    restart: unless-stopped
    ports:
      - "${PORT:-2368}:2368"
    environment:
      - url=${URL:-http://localhost:2368}
      - database__client=sqlite3
      - database__connection__filename=/var/lib/ghost/content/data/ghost.db
    volumes:
      - ghost_data:/var/lib/ghost/content
    labels:
      - "rivetr.managed=true"

volumes:
  ghost_data:
"#,
            r#"[{"name":"GHOST_VERSION","label":"Ghost Version","required":false,"default":"5-alpine","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"ghost","secret":false},{"name":"PORT","label":"Port","required":false,"default":"2368","secret":false},{"name":"URL","label":"Site URL","required":true,"default":"http://localhost:2368","secret":false}]"#,
        ),
    ];

    let template_count = templates.len();
    for (id, name, description, category, icon, compose, env_schema) in templates {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO service_templates
            (id, name, description, category, icon, compose_template, env_schema, is_builtin, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, 1, COALESCE((SELECT created_at FROM service_templates WHERE id = ?), datetime('now')))
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(category)
        .bind(icon)
        .bind(compose)
        .bind(env_schema)
        .bind(id)
        .execute(pool)
        .await?;
    }

    info!("Seeded {} built-in service templates", template_count);
    Ok(())
}
