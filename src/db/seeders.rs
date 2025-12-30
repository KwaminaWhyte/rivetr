//! Database seeders for built-in data
//!
//! This module contains functions to seed the database with initial data
//! like service templates, default configurations, etc.

use anyhow::Result;
use sqlx::SqlitePool;
use tracing::info;

/// Seed built-in service templates (runs on every startup to add/update templates)
pub async fn seed_service_templates(pool: &SqlitePool) -> Result<()> {
    info!("Seeding built-in service templates...");

    // Define built-in templates - these get added/updated on every startup
    // Format: (id, name, description, category, icon, compose_template, env_schema)
    let templates: Vec<(&str, &str, &str, &str, &str, &str, &str)> = vec![
        // ==================== DEVELOPMENT ====================
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
        (
            "mailhog",
            "Mailhog",
            "Email testing tool for developers. Capture SMTP emails and view in web UI.",
            "development",
            "mailhog",
            r#"services:
  mailhog:
    image: mailhog/mailhog:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-mailhog}
    restart: unless-stopped
    ports:
      - "${SMTP_PORT:-1025}:1025"
      - "${WEB_PORT:-8025}:8025"
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mailhog","secret":false},{"name":"SMTP_PORT","label":"SMTP Port","required":false,"default":"1025","secret":false},{"name":"WEB_PORT","label":"Web UI Port","required":false,"default":"8025","secret":false}]"#,
        ),
        (
            "watchtower",
            "Watchtower",
            "Automatically update running Docker containers when new images are available.",
            "development",
            "watchtower",
            r#"services:
  watchtower:
    image: containrrr/watchtower:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-watchtower}
    restart: unless-stopped
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    environment:
      - WATCHTOWER_POLL_INTERVAL=${POLL_INTERVAL:-86400}
      - WATCHTOWER_CLEANUP=${CLEANUP:-true}
      - WATCHTOWER_INCLUDE_RESTARTING=${INCLUDE_RESTARTING:-true}
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"watchtower","secret":false},{"name":"POLL_INTERVAL","label":"Poll Interval (seconds)","required":false,"default":"86400","secret":false},{"name":"CLEANUP","label":"Cleanup Old Images","required":false,"default":"true","secret":false},{"name":"INCLUDE_RESTARTING","label":"Include Restarting","required":false,"default":"true","secret":false}]"#,
        ),
        (
            "heimdall",
            "Heimdall",
            "Application dashboard and launcher. Organize all your web services in one place.",
            "development",
            "heimdall",
            r#"services:
  heimdall:
    image: lscr.io/linuxserver/heimdall:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-heimdall}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
      - "${HTTPS_PORT:-8443}:443"
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=${TZ:-UTC}
    volumes:
      - heimdall_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  heimdall_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"heimdall","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"8080","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"8443","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false}]"#,
        ),
        (
            "drone",
            "Drone CI",
            "Container-native continuous integration platform. Build, test, and deploy with ease.",
            "development",
            "drone",
            r#"services:
  drone:
    image: drone/drone:${VERSION:-2}
    container_name: ${CONTAINER_NAME:-drone}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
      - "${HTTPS_PORT:-8443}:443"
    environment:
      - DRONE_GITHUB_CLIENT_ID=${GITHUB_CLIENT_ID:-}
      - DRONE_GITHUB_CLIENT_SECRET=${GITHUB_CLIENT_SECRET:-}
      - DRONE_RPC_SECRET=${RPC_SECRET:-change-me}
      - DRONE_SERVER_HOST=${SERVER_HOST:-localhost}
      - DRONE_SERVER_PROTO=${SERVER_PROTO:-http}
    volumes:
      - drone_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  drone_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"2","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"drone","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"8080","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"8443","secret":false},{"name":"GITHUB_CLIENT_ID","label":"GitHub Client ID","required":false,"default":"","secret":false},{"name":"GITHUB_CLIENT_SECRET","label":"GitHub Client Secret","required":false,"default":"","secret":true},{"name":"RPC_SECRET","label":"RPC Secret","required":true,"default":"","secret":true},{"name":"SERVER_HOST","label":"Server Host","required":false,"default":"localhost","secret":false},{"name":"SERVER_PROTO","label":"Server Protocol","required":false,"default":"http","secret":false}]"#,
        ),
        (
            "nocodb",
            "NocoDB",
            "Open-source Airtable alternative. Turn any database into a smart spreadsheet.",
            "development",
            "nocodb",
            r#"services:
  nocodb:
    image: nocodb/nocodb:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-nocodb}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - NC_DB=sqlite3:///?database=/usr/app/data/noco.db
    volumes:
      - nocodb_data:/usr/app/data
    labels:
      - "rivetr.managed=true"

volumes:
  nocodb_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"nocodb","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "outline",
            "Outline",
            "Modern team knowledge base. Fast, collaborative wiki with Slack integration.",
            "development",
            "outline",
            r#"services:
  outline:
    image: outlinewiki/outline:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-outline}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - SECRET_KEY=${SECRET_KEY:-change-me-to-a-long-random-string}
      - UTILS_SECRET=${UTILS_SECRET:-change-me-to-another-long-random-string}
      - DATABASE_URL=postgres://outline:outline@outline_db:5432/outline
      - REDIS_URL=redis://outline_redis:6379
      - URL=${URL:-http://localhost:3000}
      - PORT=3000
      - FILE_STORAGE=local
    depends_on:
      - outline_db
      - outline_redis
    labels:
      - "rivetr.managed=true"

  outline_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=outline
      - POSTGRES_PASSWORD=outline
      - POSTGRES_DB=outline
    volumes:
      - outline_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  outline_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  outline_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"outline","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"UTILS_SECRET","label":"Utils Secret","required":true,"default":"","secret":true},{"name":"URL","label":"Public URL","required":true,"default":"http://localhost:3000","secret":false}]"#,
        ),

        // ==================== MONITORING ====================
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
            "dozzle",
            "Dozzle",
            "Real-time log viewer for Docker containers. Simple, lightweight, and beautiful.",
            "monitoring",
            "dozzle",
            r#"services:
  dozzle:
    image: amir20/dozzle:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-dozzle}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
    environment:
      - DOZZLE_LEVEL=${LOG_LEVEL:-info}
      - DOZZLE_NO_ANALYTICS=true
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"dozzle","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"LOG_LEVEL","label":"Log Level","required":false,"default":"info","secret":false}]"#,
        ),

        // ==================== ANALYTICS ====================
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
            "metabase",
            "Metabase",
            "Open-source business intelligence tool. Create dashboards and visualizations from your data.",
            "analytics",
            "metabase",
            r#"services:
  metabase:
    image: metabase/metabase:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-metabase}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - MB_DB_FILE=/metabase-data/metabase.db
    volumes:
      - metabase_data:/metabase-data
    labels:
      - "rivetr.managed=true"

volumes:
  metabase_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"metabase","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false}]"#,
        ),

        // ==================== DATABASE ====================
        (
            "redis",
            "Redis",
            "In-memory data store used as a database, cache, and message broker.",
            "database",
            "redis",
            r#"services:
  redis:
    image: redis:${REDIS_VERSION:-7-alpine}
    container_name: ${CONTAINER_NAME:-redis}
    restart: unless-stopped
    ports:
      - "${PORT:-6379}:6379"
    command: redis-server --appendonly yes --requirepass ${REDIS_PASSWORD:-}
    volumes:
      - redis_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  redis_data:
"#,
            r#"[{"name":"REDIS_VERSION","label":"Redis Version","required":false,"default":"7-alpine","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"redis","secret":false},{"name":"PORT","label":"Port","required":false,"default":"6379","secret":false},{"name":"REDIS_PASSWORD","label":"Password (optional)","required":false,"default":"","secret":true}]"#,
        ),
        (
            "adminer",
            "Adminer",
            "Full-featured database management tool. Supports MySQL, PostgreSQL, SQLite, MongoDB, and more.",
            "database",
            "adminer",
            r#"services:
  adminer:
    image: adminer:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-adminer}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - ADMINER_DEFAULT_SERVER=${DEFAULT_SERVER:-}
      - ADMINER_DESIGN=${DESIGN:-pepa-linha-dark}
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"adminer","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"DEFAULT_SERVER","label":"Default DB Server","required":false,"default":"","secret":false},{"name":"DESIGN","label":"Theme","required":false,"default":"pepa-linha-dark","secret":false}]"#,
        ),
        (
            "pgadmin",
            "pgAdmin",
            "Feature-rich PostgreSQL administration and development platform.",
            "database",
            "pgadmin",
            r#"services:
  pgadmin:
    image: dpage/pgadmin4:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-pgadmin}
    restart: unless-stopped
    ports:
      - "${PORT:-5050}:80"
    environment:
      - PGADMIN_DEFAULT_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - PGADMIN_DEFAULT_PASSWORD=${ADMIN_PASSWORD:-admin}
      - PGADMIN_CONFIG_SERVER_MODE=False
    volumes:
      - pgadmin_data:/var/lib/pgadmin
    labels:
      - "rivetr.managed=true"

volumes:
  pgadmin_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"pgadmin","secret":false},{"name":"PORT","label":"Port","required":false,"default":"5050","secret":false},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "rabbitmq",
            "RabbitMQ",
            "Robust messaging for distributed systems. Feature-rich message broker with management UI.",
            "database",
            "rabbitmq",
            r#"services:
  rabbitmq:
    image: rabbitmq:${VERSION:-3-management-alpine}
    container_name: ${CONTAINER_NAME:-rabbitmq}
    restart: unless-stopped
    ports:
      - "${AMQP_PORT:-5672}:5672"
      - "${MGMT_PORT:-15672}:15672"
    environment:
      - RABBITMQ_DEFAULT_USER=${USERNAME:-guest}
      - RABBITMQ_DEFAULT_PASS=${PASSWORD:-guest}
      - RABBITMQ_DEFAULT_VHOST=${VHOST:-/}
    volumes:
      - rabbitmq_data:/var/lib/rabbitmq
    labels:
      - "rivetr.managed=true"

volumes:
  rabbitmq_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"3-management-alpine","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"rabbitmq","secret":false},{"name":"AMQP_PORT","label":"AMQP Port","required":false,"default":"5672","secret":false},{"name":"MGMT_PORT","label":"Management UI Port","required":false,"default":"15672","secret":false},{"name":"USERNAME","label":"Username","required":false,"default":"guest","secret":false},{"name":"PASSWORD","label":"Password","required":true,"default":"","secret":true},{"name":"VHOST","label":"Virtual Host","required":false,"default":"/","secret":false}]"#,
        ),

        // ==================== NETWORKING ====================
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

        // ==================== SECURITY ====================
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

        // ==================== STORAGE ====================
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
            "nextcloud",
            "Nextcloud",
            "Self-hosted productivity platform with file sync, calendar, contacts, and more.",
            "storage",
            "nextcloud",
            r#"services:
  nextcloud:
    image: nextcloud:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-nextcloud}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - NEXTCLOUD_ADMIN_USER=${ADMIN_USER:-admin}
      - NEXTCLOUD_ADMIN_PASSWORD=${ADMIN_PASSWORD:-admin}
      - NEXTCLOUD_TRUSTED_DOMAINS=${TRUSTED_DOMAINS:-localhost}
    volumes:
      - nextcloud_data:/var/www/html
    labels:
      - "rivetr.managed=true"

volumes:
  nextcloud_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"nextcloud","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"TRUSTED_DOMAINS","label":"Trusted Domains","required":false,"default":"localhost","secret":false}]"#,
        ),
        (
            "filebrowser",
            "Filebrowser",
            "Web-based file manager with upload, download, rename, and edit capabilities.",
            "storage",
            "filebrowser",
            r#"services:
  filebrowser:
    image: filebrowser/filebrowser:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-filebrowser}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    volumes:
      - filebrowser_data:/srv
      - filebrowser_config:/config
    environment:
      - FB_DATABASE=/config/filebrowser.db
    labels:
      - "rivetr.managed=true"

volumes:
  filebrowser_data:
  filebrowser_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"filebrowser","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false}]"#,
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
