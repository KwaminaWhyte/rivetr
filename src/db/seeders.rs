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
            "automation",
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
            "cms",
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
            "cms",
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

        // ==================== AI / ML ====================
        (
            "ollama",
            "Ollama",
            "Run large language models locally. Supports Llama, Mistral, Code Llama, and many more.",
            "ai",
            "ollama",
            r#"services:
  ollama:
    image: ollama/ollama:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-ollama}
    restart: unless-stopped
    ports:
      - "${PORT:-11434}:11434"
    volumes:
      - ollama_models:/root/.ollama
    labels:
      - "rivetr.managed=true"

volumes:
  ollama_models:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"ollama","secret":false},{"name":"PORT","label":"Port","required":false,"default":"11434","secret":false}]"#,
        ),
        (
            "open-webui",
            "Open WebUI",
            "ChatGPT-like web interface for Ollama and other LLM backends. Feature-rich and extensible.",
            "ai",
            "open-webui",
            r#"services:
  open-webui:
    image: ghcr.io/open-webui/open-webui:${VERSION:-main}
    container_name: ${CONTAINER_NAME:-open-webui}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - OLLAMA_BASE_URL=${OLLAMA_BASE_URL:-http://host.docker.internal:11434}
    volumes:
      - open_webui_data:/app/backend/data
    labels:
      - "rivetr.managed=true"

volumes:
  open_webui_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"main","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"open-webui","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"OLLAMA_BASE_URL","label":"Ollama Base URL","required":false,"default":"http://host.docker.internal:11434","secret":false}]"#,
        ),
        (
            "litellm",
            "LiteLLM",
            "Unified LLM proxy server. Call 100+ LLM APIs in OpenAI-compatible format.",
            "ai",
            "litellm",
            r#"services:
  litellm:
    image: ghcr.io/berriai/litellm:${VERSION:-main-latest}
    container_name: ${CONTAINER_NAME:-litellm}
    restart: unless-stopped
    ports:
      - "${PORT:-4000}:4000"
    environment:
      - LITELLM_MASTER_KEY=${MASTER_KEY:-sk-litellm-master-key}
    volumes:
      - litellm_data:/app/data
    labels:
      - "rivetr.managed=true"

volumes:
  litellm_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"main-latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"litellm","secret":false},{"name":"PORT","label":"Port","required":false,"default":"4000","secret":false},{"name":"MASTER_KEY","label":"Master API Key","required":true,"default":"","secret":true}]"#,
        ),
        (
            "langflow",
            "Langflow",
            "Visual framework for building multi-agent and RAG applications. Drag-and-drop LLM app builder.",
            "ai",
            "langflow",
            r#"services:
  langflow:
    image: langflowai/langflow:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-langflow}
    restart: unless-stopped
    ports:
      - "${PORT:-7860}:7860"
    volumes:
      - langflow_data:/app/langflow
    labels:
      - "rivetr.managed=true"

volumes:
  langflow_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"langflow","secret":false},{"name":"PORT","label":"Port","required":false,"default":"7860","secret":false}]"#,
        ),
        (
            "flowise",
            "Flowise",
            "Low-code LLM orchestration tool. Build customized LLM flows with drag-and-drop UI.",
            "ai",
            "flowise",
            r#"services:
  flowise:
    image: flowiseai/flowise:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-flowise}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - FLOWISE_USERNAME=${USERNAME:-admin}
      - FLOWISE_PASSWORD=${PASSWORD:-}
    volumes:
      - flowise_data:/root/.flowise
    labels:
      - "rivetr.managed=true"

volumes:
  flowise_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"flowise","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"USERNAME","label":"Username","required":false,"default":"admin","secret":false},{"name":"PASSWORD","label":"Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "chromadb",
            "ChromaDB",
            "Open-source AI-native vector database. Store and query embeddings for LLM applications.",
            "ai",
            "chromadb",
            r#"services:
  chromadb:
    image: chromadb/chroma:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-chromadb}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - IS_PERSISTENT=TRUE
      - ANONYMIZED_TELEMETRY=FALSE
    volumes:
      - chromadb_data:/chroma/chroma
    labels:
      - "rivetr.managed=true"

volumes:
  chromadb_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"chromadb","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false}]"#,
        ),

        // ==================== ANALYTICS (additional) ====================
        (
            "umami",
            "Umami",
            "Simple, fast, privacy-focused alternative to Google Analytics. Self-hosted web analytics.",
            "analytics",
            "umami",
            r#"services:
  umami:
    image: ghcr.io/umami-software/umami:${VERSION:-postgresql-latest}
    container_name: ${CONTAINER_NAME:-umami}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgresql://umami:umami@umami_db:5432/umami
      - DATABASE_TYPE=postgresql
      - APP_SECRET=${APP_SECRET:-change-me-to-random-string}
    depends_on:
      - umami_db
    labels:
      - "rivetr.managed=true"

  umami_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=umami
      - POSTGRES_USER=umami
      - POSTGRES_PASSWORD=umami
    volumes:
      - umami_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  umami_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"postgresql-latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"umami","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"APP_SECRET","label":"App Secret","required":true,"default":"","secret":true}]"#,
        ),
        (
            "posthog",
            "PostHog",
            "Open-source product analytics, session recording, feature flags, and A/B testing platform.",
            "analytics",
            "posthog",
            r#"services:
  posthog:
    image: posthog/posthog:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-posthog}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - SECRET_KEY=${SECRET_KEY:-change-me-to-random-string}
      - DATABASE_URL=postgres://posthog:posthog@posthog_db:5432/posthog
      - REDIS_URL=redis://posthog_redis:6379/
      - SITE_URL=${SITE_URL:-http://localhost:8000}
    depends_on:
      - posthog_db
      - posthog_redis
    labels:
      - "rivetr.managed=true"

  posthog_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=posthog
      - POSTGRES_USER=posthog
      - POSTGRES_PASSWORD=posthog
    volumes:
      - posthog_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  posthog_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  posthog_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"posthog","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"SITE_URL","label":"Site URL","required":false,"default":"http://localhost:8000","secret":false}]"#,
        ),
        (
            "matomo",
            "Matomo",
            "Powerful web analytics platform. Privacy-respecting Google Analytics alternative with full data ownership.",
            "analytics",
            "matomo",
            r#"services:
  matomo:
    image: matomo:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-matomo}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - MATOMO_DATABASE_HOST=matomo_db
      - MATOMO_DATABASE_ADAPTER=mysql
      - MATOMO_DATABASE_TABLES_PREFIX=matomo_
      - MATOMO_DATABASE_USERNAME=matomo
      - MATOMO_DATABASE_PASSWORD=${DB_PASSWORD:-matomo}
      - MATOMO_DATABASE_DBNAME=matomo
    volumes:
      - matomo_data:/var/www/html
    depends_on:
      - matomo_db
    labels:
      - "rivetr.managed=true"

  matomo_db:
    image: mariadb:11
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=matomo
      - MYSQL_USER=matomo
      - MYSQL_PASSWORD=${DB_PASSWORD:-matomo}
      - MYSQL_RANDOM_ROOT_PASSWORD=yes
    volumes:
      - matomo_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  matomo_data:
  matomo_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"matomo","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== AUTOMATION ====================
        (
            "activepieces",
            "Activepieces",
            "No-code workflow automation tool. Open-source alternative to Zapier with 100+ integrations.",
            "automation",
            "activepieces",
            r#"services:
  activepieces:
    image: activepieces/activepieces:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-activepieces}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - AP_ENGINE_EXECUTABLE_PATH=dist/packages/engine/main.js
      - AP_ENCRYPTION_KEY=${ENCRYPTION_KEY:-change-me-32-chars-encryption-key}
      - AP_JWT_SECRET=${JWT_SECRET:-change-me-jwt-secret}
      - AP_FRONTEND_URL=${FRONTEND_URL:-http://localhost:8080}
      - AP_POSTGRES_DATABASE=activepieces
      - AP_POSTGRES_HOST=activepieces_db
      - AP_POSTGRES_PORT=5432
      - AP_POSTGRES_USERNAME=activepieces
      - AP_POSTGRES_PASSWORD=activepieces
      - AP_REDIS_HOST=activepieces_redis
      - AP_REDIS_PORT=6379
    depends_on:
      - activepieces_db
      - activepieces_redis
    labels:
      - "rivetr.managed=true"

  activepieces_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=activepieces
      - POSTGRES_USER=activepieces
      - POSTGRES_PASSWORD=activepieces
    volumes:
      - activepieces_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  activepieces_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  activepieces_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"activepieces","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"ENCRYPTION_KEY","label":"Encryption Key (32 chars)","required":true,"default":"","secret":true},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true},{"name":"FRONTEND_URL","label":"Frontend URL","required":false,"default":"http://localhost:8080","secret":false}]"#,
        ),
        (
            "windmill",
            "Windmill",
            "Developer platform for scripts, workflows, and UIs. Turn scripts into internal tools and workflows.",
            "automation",
            "windmill",
            r#"services:
  windmill:
    image: ghcr.io/windmill-labs/windmill:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-windmill}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - DATABASE_URL=postgres://windmill:windmill@windmill_db:5432/windmill
      - BASE_URL=${BASE_URL:-http://localhost:8000}
    depends_on:
      - windmill_db
    labels:
      - "rivetr.managed=true"

  windmill_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=windmill
      - POSTGRES_USER=windmill
      - POSTGRES_PASSWORD=windmill
    volumes:
      - windmill_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  windmill_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"windmill","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false},{"name":"BASE_URL","label":"Base URL","required":false,"default":"http://localhost:8000","secret":false}]"#,
        ),
        (
            "trigger-dev",
            "Trigger.dev",
            "Open-source background jobs framework. Build reliable workflows with retries and scheduling.",
            "automation",
            "trigger-dev",
            r#"services:
  trigger-dev:
    image: ghcr.io/triggerdotdev/trigger.dev:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-trigger-dev}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgres://trigger:trigger@trigger_db:5432/trigger
      - DIRECT_URL=postgres://trigger:trigger@trigger_db:5432/trigger
      - SESSION_SECRET=${SESSION_SECRET:-change-me-session-secret}
      - MAGIC_LINK_SECRET=${MAGIC_LINK_SECRET:-change-me-magic-link-secret}
      - ENCRYPTION_KEY=${ENCRYPTION_KEY:-change-me-encryption-key}
      - LOGIN_ORIGIN=${LOGIN_ORIGIN:-http://localhost:3000}
      - APP_ORIGIN=${APP_ORIGIN:-http://localhost:3000}
    depends_on:
      - trigger_db
    labels:
      - "rivetr.managed=true"

  trigger_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=trigger
      - POSTGRES_USER=trigger
      - POSTGRES_PASSWORD=trigger
    volumes:
      - trigger_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  trigger_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"trigger-dev","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"SESSION_SECRET","label":"Session Secret","required":true,"default":"","secret":true},{"name":"MAGIC_LINK_SECRET","label":"Magic Link Secret","required":true,"default":"","secret":true},{"name":"ENCRYPTION_KEY","label":"Encryption Key","required":true,"default":"","secret":true},{"name":"LOGIN_ORIGIN","label":"Login Origin URL","required":false,"default":"http://localhost:3000","secret":false},{"name":"APP_ORIGIN","label":"App Origin URL","required":false,"default":"http://localhost:3000","secret":false}]"#,
        ),

        // ==================== CMS ====================
        (
            "strapi",
            "Strapi",
            "Leading open-source headless CMS. Build powerful content APIs with a customizable admin panel.",
            "cms",
            "strapi",
            r#"services:
  strapi:
    image: strapi/strapi:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-strapi}
    restart: unless-stopped
    ports:
      - "${PORT:-1337}:1337"
    environment:
      - DATABASE_CLIENT=sqlite
      - DATABASE_FILENAME=/srv/app/.tmp/data.db
      - APP_KEYS=${APP_KEYS:-key1,key2,key3,key4}
      - API_TOKEN_SALT=${API_TOKEN_SALT:-change-me}
      - ADMIN_JWT_SECRET=${ADMIN_JWT_SECRET:-change-me}
      - JWT_SECRET=${JWT_SECRET:-change-me}
    volumes:
      - strapi_data:/srv/app
    labels:
      - "rivetr.managed=true"

volumes:
  strapi_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"strapi","secret":false},{"name":"PORT","label":"Port","required":false,"default":"1337","secret":false},{"name":"APP_KEYS","label":"App Keys (comma-separated)","required":true,"default":"","secret":true},{"name":"API_TOKEN_SALT","label":"API Token Salt","required":true,"default":"","secret":true},{"name":"ADMIN_JWT_SECRET","label":"Admin JWT Secret","required":true,"default":"","secret":true},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true}]"#,
        ),
        (
            "directus",
            "Directus",
            "Open data platform for managing any SQL database. Instant REST and GraphQL API with a no-code admin app.",
            "cms",
            "directus",
            r#"services:
  directus:
    image: directus/directus:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-directus}
    restart: unless-stopped
    ports:
      - "${PORT:-8055}:8055"
    environment:
      - SECRET=${SECRET:-change-me-to-random-string}
      - DB_CLIENT=pg
      - DB_HOST=directus_db
      - DB_PORT=5432
      - DB_DATABASE=directus
      - DB_USER=directus
      - DB_PASSWORD=directus
      - ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD:-admin}
    volumes:
      - directus_uploads:/directus/uploads
      - directus_extensions:/directus/extensions
    depends_on:
      - directus_db
    labels:
      - "rivetr.managed=true"

  directus_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=directus
      - POSTGRES_USER=directus
      - POSTGRES_PASSWORD=directus
    volumes:
      - directus_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  directus_uploads:
  directus_extensions:
  directus_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"directus","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8055","secret":false},{"name":"SECRET","label":"Secret Key","required":true,"default":"","secret":true},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "payload-cms",
            "Payload CMS",
            "Next-generation TypeScript headless CMS. Code-first with powerful admin panel.",
            "cms",
            "payload-cms",
            r#"services:
  payload:
    image: payloadcms/payload:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-payload}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - PAYLOAD_SECRET=${PAYLOAD_SECRET:-change-me-to-random-string}
      - DATABASE_URI=mongodb://payload_db:27017/payload
    depends_on:
      - payload_db
    labels:
      - "rivetr.managed=true"

  payload_db:
    image: mongo:7
    restart: unless-stopped
    volumes:
      - payload_db_data:/data/db
    labels:
      - "rivetr.managed=true"

volumes:
  payload_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"payload","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"PAYLOAD_SECRET","label":"Payload Secret","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== COMMUNICATION ====================
        (
            "rocketchat",
            "Rocket.Chat",
            "Open-source team communication platform. Chat, video, file sharing, and integrations.",
            "communication",
            "rocketchat",
            r#"services:
  rocketchat:
    image: rocket.chat:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-rocketchat}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - ROOT_URL=${ROOT_URL:-http://localhost:3000}
      - MONGO_URL=mongodb://rocketchat_db:27017/rocketchat?replicaSet=rs0
      - MONGO_OPLOG_URL=mongodb://rocketchat_db:27017/local?replicaSet=rs0
    depends_on:
      - rocketchat_db
    labels:
      - "rivetr.managed=true"

  rocketchat_db:
    image: mongo:6
    restart: unless-stopped
    command: mongod --oplogSize 128 --replSet rs0
    volumes:
      - rocketchat_db_data:/data/db
    labels:
      - "rivetr.managed=true"

volumes:
  rocketchat_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"rocketchat","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"ROOT_URL","label":"Root URL","required":false,"default":"http://localhost:3000","secret":false}]"#,
        ),
        (
            "mattermost",
            "Mattermost",
            "Open-source platform for secure team collaboration. Self-hosted Slack alternative.",
            "communication",
            "mattermost",
            r#"services:
  mattermost:
    image: mattermost/mattermost-team-edition:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-mattermost}
    restart: unless-stopped
    ports:
      - "${PORT:-8065}:8065"
    environment:
      - TZ=UTC
      - MM_SQLSETTINGS_DRIVERNAME=postgres
      - MM_SQLSETTINGS_DATASOURCE=postgres://mattermost:mattermost@mattermost_db:5432/mattermost?sslmode=disable&connect_timeout=10
      - MM_SERVICESETTINGS_SITEURL=${SITE_URL:-http://localhost:8065}
    volumes:
      - mattermost_data:/mattermost/data
      - mattermost_logs:/mattermost/logs
      - mattermost_config:/mattermost/config
      - mattermost_plugins:/mattermost/plugins
    depends_on:
      - mattermost_db
    labels:
      - "rivetr.managed=true"

  mattermost_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=mattermost
      - POSTGRES_USER=mattermost
      - POSTGRES_PASSWORD=mattermost
    volumes:
      - mattermost_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  mattermost_data:
  mattermost_logs:
  mattermost_config:
  mattermost_plugins:
  mattermost_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mattermost","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8065","secret":false},{"name":"SITE_URL","label":"Site URL","required":false,"default":"http://localhost:8065","secret":false}]"#,
        ),
        (
            "matrix-synapse",
            "Matrix Synapse",
            "Decentralized communication server implementing the Matrix protocol. Federated chat and VoIP.",
            "communication",
            "matrix-synapse",
            r#"services:
  synapse:
    image: matrixdotorg/synapse:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-synapse}
    restart: unless-stopped
    ports:
      - "${PORT:-8008}:8008"
    environment:
      - SYNAPSE_SERVER_NAME=${SERVER_NAME:-localhost}
      - SYNAPSE_REPORT_STATS=${REPORT_STATS:-no}
      - SYNAPSE_NO_TLS=true
    volumes:
      - synapse_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  synapse_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"synapse","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8008","secret":false},{"name":"SERVER_NAME","label":"Server Name","required":true,"default":"localhost","secret":false},{"name":"REPORT_STATS","label":"Report Stats","required":false,"default":"no","secret":false}]"#,
        ),

        // ==================== DEVELOPMENT (additional) ====================
        (
            "code-server",
            "Code Server",
            "VS Code in the browser. Full IDE experience accessible from any device with a web browser.",
            "development",
            "code-server",
            r#"services:
  code-server:
    image: lscr.io/linuxserver/code-server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-code-server}
    restart: unless-stopped
    ports:
      - "${PORT:-8443}:8443"
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=${TZ:-UTC}
      - PASSWORD=${PASSWORD:-}
      - SUDO_PASSWORD=${SUDO_PASSWORD:-}
      - DEFAULT_WORKSPACE=/config/workspace
    volumes:
      - code_server_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  code_server_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"code-server","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8443","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"PASSWORD","label":"Password","required":true,"default":"","secret":true},{"name":"SUDO_PASSWORD","label":"Sudo Password","required":false,"default":"","secret":true}]"#,
        ),
        (
            "supabase",
            "Supabase",
            "Open-source Firebase alternative. Postgres database, auth, instant APIs, realtime, and storage.",
            "development",
            "supabase",
            r#"services:
  supabase-studio:
    image: supabase/studio:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-supabase-studio}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - STUDIO_PG_META_URL=http://supabase-meta:8080
      - SUPABASE_URL=http://supabase-kong:8000
      - SUPABASE_REST_URL=http://supabase-kong:8000/rest/v1/
      - SUPABASE_ANON_KEY=${ANON_KEY:-eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9}
      - SUPABASE_SERVICE_KEY=${SERVICE_KEY:-eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9}
    depends_on:
      - supabase-db
    labels:
      - "rivetr.managed=true"

  supabase-db:
    image: supabase/postgres:15.6.1.120
    restart: unless-stopped
    environment:
      - POSTGRES_PASSWORD=${DB_PASSWORD:-supabase}
      - POSTGRES_DB=supabase
    volumes:
      - supabase_db_data:/var/lib/postgresql/data
    ports:
      - "${DB_PORT:-5432}:5432"
    labels:
      - "rivetr.managed=true"

  supabase-meta:
    image: supabase/postgres-meta:v0.83.2
    restart: unless-stopped
    environment:
      - PG_META_PORT=8080
      - PG_META_DB_HOST=supabase-db
      - PG_META_DB_PORT=5432
      - PG_META_DB_NAME=supabase
      - PG_META_DB_USER=supabase_admin
      - PG_META_DB_PASSWORD=${DB_PASSWORD:-supabase}
    depends_on:
      - supabase-db
    labels:
      - "rivetr.managed=true"

volumes:
  supabase_db_data:
"#,
            r#"[{"name":"VERSION","label":"Studio Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"supabase-studio","secret":false},{"name":"PORT","label":"Studio Port","required":false,"default":"3000","secret":false},{"name":"DB_PORT","label":"Database Port","required":false,"default":"5432","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"ANON_KEY","label":"Anon Key","required":true,"default":"","secret":true},{"name":"SERVICE_KEY","label":"Service Role Key","required":true,"default":"","secret":true}]"#,
        ),
        (
            "appwrite",
            "Appwrite",
            "End-to-end backend server for web, mobile, and Flutter developers. Auth, database, storage, functions.",
            "development",
            "appwrite",
            r#"services:
  appwrite:
    image: appwrite/appwrite:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-appwrite}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
    environment:
      - _APP_ENV=production
      - _APP_OPENSSL_KEY_V1=${OPENSSL_KEY:-your-secret-key}
      - _APP_DOMAIN=${DOMAIN:-localhost}
      - _APP_DOMAIN_TARGET=${DOMAIN:-localhost}
      - _APP_REDIS_HOST=appwrite_redis
      - _APP_REDIS_PORT=6379
      - _APP_DB_HOST=appwrite_db
      - _APP_DB_PORT=3306
      - _APP_DB_SCHEMA=appwrite
      - _APP_DB_USER=appwrite
      - _APP_DB_PASS=appwrite
    volumes:
      - appwrite_uploads:/storage/uploads
      - appwrite_cache:/storage/cache
      - appwrite_config:/storage/config
      - appwrite_certs:/storage/certificates
    depends_on:
      - appwrite_db
      - appwrite_redis
    labels:
      - "rivetr.managed=true"

  appwrite_db:
    image: mariadb:11
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=appwrite
      - MYSQL_DATABASE=appwrite
      - MYSQL_USER=appwrite
      - MYSQL_PASSWORD=appwrite
    volumes:
      - appwrite_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

  appwrite_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  appwrite_uploads:
  appwrite_cache:
  appwrite_config:
  appwrite_certs:
  appwrite_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"appwrite","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false},{"name":"OPENSSL_KEY","label":"OpenSSL Secret Key","required":true,"default":"","secret":true},{"name":"DOMAIN","label":"Domain","required":false,"default":"localhost","secret":false}]"#,
        ),
        (
            "pocketbase",
            "PocketBase",
            "Open-source backend in 1 file. Realtime database, auth, file storage, and admin dashboard.",
            "development",
            "pocketbase",
            r#"services:
  pocketbase:
    image: ghcr.io/muchobien/pocketbase:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-pocketbase}
    restart: unless-stopped
    ports:
      - "${PORT:-8090}:8090"
    volumes:
      - pocketbase_data:/pb/pb_data
      - pocketbase_public:/pb/pb_public
    labels:
      - "rivetr.managed=true"

volumes:
  pocketbase_data:
  pocketbase_public:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"pocketbase","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8090","secret":false}]"#,
        ),
        (
            "hoppscotch",
            "Hoppscotch",
            "Open-source API development ecosystem. Test REST, GraphQL, WebSocket, and more from the browser.",
            "development",
            "hoppscotch",
            r#"services:
  hoppscotch:
    image: hoppscotch/hoppscotch:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-hoppscotch}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgresql://hoppscotch:hoppscotch@hoppscotch_db:5432/hoppscotch
      - JWT_SECRET=${JWT_SECRET:-change-me-jwt-secret}
      - SESSION_SECRET=${SESSION_SECRET:-change-me-session-secret}
      - TOKEN_SALT_COMPLEXITY=10
      - MAGIC_LINK_TOKEN_VALIDITY=3
      - REFRESH_TOKEN_VALIDITY=604800000
      - ACCESS_TOKEN_VALIDITY=86400000
      - VITE_ALLOWED_AUTH_PROVIDERS=EMAIL
    depends_on:
      - hoppscotch_db
    labels:
      - "rivetr.managed=true"

  hoppscotch_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=hoppscotch
      - POSTGRES_USER=hoppscotch
      - POSTGRES_PASSWORD=hoppscotch
    volumes:
      - hoppscotch_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  hoppscotch_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"hoppscotch","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true},{"name":"SESSION_SECRET","label":"Session Secret","required":true,"default":"","secret":true}]"#,
        ),
        (
            "forgejo",
            "Forgejo",
            "Self-hosted Git hosting platform. Community fork of Gitea with enhanced features and governance.",
            "development",
            "forgejo",
            r#"services:
  forgejo:
    image: codeberg.org/forgejo/forgejo:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-forgejo}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-3000}:3000"
      - "${SSH_PORT:-2222}:22"
    environment:
      - USER_UID=1000
      - USER_GID=1000
      - FORGEJO__database__DB_TYPE=sqlite3
      - FORGEJO__server__ROOT_URL=${ROOT_URL:-http://localhost:3000}
    volumes:
      - forgejo_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  forgejo_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"forgejo","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"3000","secret":false},{"name":"SSH_PORT","label":"SSH Port","required":false,"default":"2222","secret":false},{"name":"ROOT_URL","label":"Root URL","required":false,"default":"http://localhost:3000","secret":false}]"#,
        ),

        // ==================== BATCH 2: DOCUMENTATION ====================
        (
            "tpl-batch2-bookstack",
            "BookStack",
            "A simple, self-hosted wiki platform for organising and storing information.",
            "documentation",
            "bookstack",
            r#"services:
  bookstack:
    image: lscr.io/linuxserver/bookstack:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-bookstack}
    restart: unless-stopped
    ports:
      - "${PORT:-6875}:80"
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=${TZ:-UTC}
      - APP_URL=${APP_URL:-http://localhost:6875}
      - DB_HOST=bookstack_db
      - DB_PORT=3306
      - DB_USER=bookstack
      - DB_PASS=${DB_PASSWORD:-bookstack}
      - DB_DATABASE=bookstack
    volumes:
      - bookstack_data:/config
    depends_on:
      - bookstack_db
    labels:
      - "rivetr.managed=true"

  bookstack_db:
    image: mysql:8.0
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=bookstack
      - MYSQL_USER=bookstack
      - MYSQL_PASSWORD=${DB_PASSWORD:-bookstack}
      - MYSQL_RANDOM_ROOT_PASSWORD=yes
    volumes:
      - bookstack_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  bookstack_data:
  bookstack_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"bookstack","secret":false},{"name":"PORT","label":"Port","required":false,"default":"6875","secret":false},{"name":"APP_URL","label":"Application URL","required":true,"default":"http://localhost:6875","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false}]"#,
        ),
        (
            "tpl-batch2-wikijs",
            "Wiki.js",
            "A modern, powerful wiki app built on Node.js with beautiful interface and extensive features.",
            "documentation",
            "wikijs",
            r#"services:
  wikijs:
    image: ghcr.io/requarks/wiki:${VERSION:-2}
    container_name: ${CONTAINER_NAME:-wikijs}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DB_TYPE=postgres
      - DB_HOST=wikijs_db
      - DB_PORT=5432
      - DB_USER=wikijs
      - DB_PASS=${DB_PASSWORD:-wikijs}
      - DB_NAME=wikijs
    depends_on:
      - wikijs_db
    labels:
      - "rivetr.managed=true"

  wikijs_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=wikijs
      - POSTGRES_PASSWORD=${DB_PASSWORD:-wikijs}
      - POSTGRES_DB=wikijs
    volumes:
      - wikijs_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  wikijs_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"2","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"wikijs","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-batch2-docmost",
            "Docmost",
            "An open-source collaborative wiki and documentation software. Alternative to Notion and Confluence.",
            "documentation",
            "docmost",
            r#"services:
  docmost:
    image: docmost/docmost:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-docmost}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - APP_URL=${APP_URL:-http://localhost:3000}
      - APP_SECRET=${APP_SECRET:-change-me-to-a-long-random-string}
      - DATABASE_URL=postgresql://docmost:${DB_PASSWORD:-docmost}@docmost_db:5432/docmost
      - REDIS_URL=redis://docmost_redis:6379
    depends_on:
      - docmost_db
      - docmost_redis
    labels:
      - "rivetr.managed=true"

  docmost_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=docmost
      - POSTGRES_PASSWORD=${DB_PASSWORD:-docmost}
      - POSTGRES_DB=docmost
    volumes:
      - docmost_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  docmost_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  docmost_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"docmost","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"APP_URL","label":"Application URL","required":true,"default":"http://localhost:3000","secret":false},{"name":"APP_SECRET","label":"App Secret","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== BATCH 2: FILE & MEDIA ====================
        (
            "tpl-batch2-immich",
            "Immich",
            "High-performance self-hosted photo and video management solution with ML-powered features.",
            "media",
            "immich",
            r#"services:
  immich-server:
    image: ghcr.io/immich-app/immich-server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-immich-server}
    restart: unless-stopped
    ports:
      - "${PORT:-2283}:2283"
    environment:
      - DB_HOSTNAME=immich_db
      - DB_USERNAME=immich
      - DB_PASSWORD=${DB_PASSWORD:-immich}
      - DB_DATABASE_NAME=immich
      - REDIS_HOSTNAME=immich_redis
    volumes:
      - immich_upload:/usr/src/app/upload
    depends_on:
      - immich_db
      - immich_redis
    labels:
      - "rivetr.managed=true"

  immich-machine-learning:
    image: ghcr.io/immich-app/immich-machine-learning:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-immich-ml}
    restart: unless-stopped
    volumes:
      - immich_ml_cache:/cache
    labels:
      - "rivetr.managed=true"

  immich_db:
    image: tensorchord/pgvecto-rs:pg14-v0.2.0
    restart: unless-stopped
    environment:
      - POSTGRES_USER=immich
      - POSTGRES_PASSWORD=${DB_PASSWORD:-immich}
      - POSTGRES_DB=immich
      - POSTGRES_INITDB_ARGS=--data-checksums
    volumes:
      - immich_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  immich_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  immich_upload:
  immich_ml_cache:
  immich_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"immich-server","secret":false},{"name":"PORT","label":"Port","required":false,"default":"2283","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-batch2-jellyfin",
            "Jellyfin",
            "Free software media system for streaming movies, TV shows, music, and more.",
            "media",
            "jellyfin",
            r#"services:
  jellyfin:
    image: jellyfin/jellyfin:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-jellyfin}
    restart: unless-stopped
    ports:
      - "${PORT:-8096}:8096"
    volumes:
      - jellyfin_config:/config
      - jellyfin_cache:/cache
      - jellyfin_media:/media
    labels:
      - "rivetr.managed=true"

volumes:
  jellyfin_config:
  jellyfin_cache:
  jellyfin_media:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"jellyfin","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8096","secret":false}]"#,
        ),
        (
            "tpl-batch2-navidrome",
            "Navidrome",
            "Modern music server and streamer compatible with Subsonic/Airsonic clients.",
            "media",
            "navidrome",
            r#"services:
  navidrome:
    image: deluan/navidrome:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-navidrome}
    restart: unless-stopped
    ports:
      - "${PORT:-4533}:4533"
    environment:
      - ND_SCANSCHEDULE=${SCAN_SCHEDULE:-1h}
      - ND_LOGLEVEL=${LOG_LEVEL:-info}
      - ND_BASEURL=${BASE_URL:-}
    volumes:
      - navidrome_data:/data
      - navidrome_music:/music:ro
    labels:
      - "rivetr.managed=true"

volumes:
  navidrome_data:
  navidrome_music:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"navidrome","secret":false},{"name":"PORT","label":"Port","required":false,"default":"4533","secret":false},{"name":"SCAN_SCHEDULE","label":"Scan Schedule","required":false,"default":"1h","secret":false},{"name":"LOG_LEVEL","label":"Log Level","required":false,"default":"info","secret":false},{"name":"BASE_URL","label":"Base URL","required":false,"default":"","secret":false}]"#,
        ),
        (
            "tpl-batch2-seafile",
            "Seafile",
            "Open-source file sync and share solution with high performance and reliability.",
            "media",
            "seafile",
            r#"services:
  seafile:
    image: seafileltd/seafile-mc:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-seafile}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - DB_HOST=seafile_db
      - DB_ROOT_PASSWD=${DB_ROOT_PASSWORD:-seafile}
      - SEAFILE_ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - SEAFILE_ADMIN_PASSWORD=${ADMIN_PASSWORD:-admin}
      - SEAFILE_SERVER_HOSTNAME=${SERVER_HOSTNAME:-localhost}
    volumes:
      - seafile_data:/shared
    depends_on:
      - seafile_db
      - seafile_memcached
    labels:
      - "rivetr.managed=true"

  seafile_db:
    image: mysql:8.0
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-seafile}
      - MYSQL_LOG_CONSOLE=true
    volumes:
      - seafile_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

  seafile_memcached:
    image: memcached:1.6-alpine
    restart: unless-stopped
    entrypoint: memcached -m 256
    labels:
      - "rivetr.managed=true"

volumes:
  seafile_data:
  seafile_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"seafile","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"DB_ROOT_PASSWORD","label":"DB Root Password","required":true,"default":"","secret":true},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"SERVER_HOSTNAME","label":"Server Hostname","required":false,"default":"localhost","secret":false}]"#,
        ),

        // ==================== BATCH 2: MONITORING ====================
        (
            "tpl-batch2-signoz",
            "SigNoz",
            "Open-source APM and observability platform. Traces, metrics, and logs in a single pane.",
            "monitoring",
            "signoz",
            r#"services:
  signoz:
    image: signoz/signoz:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-signoz}
    restart: unless-stopped
    ports:
      - "${PORT:-3301}:3301"
      - "${OTEL_GRPC_PORT:-4317}:4317"
      - "${OTEL_HTTP_PORT:-4318}:4318"
    environment:
      - SIGNOZ_LOCAL_DB_PATH=/var/lib/signoz/signoz.db
    volumes:
      - signoz_data:/var/lib/signoz
    labels:
      - "rivetr.managed=true"

volumes:
  signoz_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"signoz","secret":false},{"name":"PORT","label":"Web UI Port","required":false,"default":"3301","secret":false},{"name":"OTEL_GRPC_PORT","label":"OTEL gRPC Port","required":false,"default":"4317","secret":false},{"name":"OTEL_HTTP_PORT","label":"OTEL HTTP Port","required":false,"default":"4318","secret":false}]"#,
        ),
        (
            "tpl-batch2-beszel",
            "Beszel",
            "Lightweight server monitoring hub with Docker stats, historical data, and alerting.",
            "monitoring",
            "beszel",
            r#"services:
  beszel:
    image: henrygd/beszel:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-beszel}
    restart: unless-stopped
    ports:
      - "${PORT:-8090}:8090"
    volumes:
      - beszel_data:/beszel_data
    labels:
      - "rivetr.managed=true"

volumes:
  beszel_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"beszel","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8090","secret":false}]"#,
        ),
        (
            "tpl-batch2-checkmate",
            "Checkmate",
            "Open-source uptime and infrastructure monitoring with beautiful dashboards.",
            "monitoring",
            "checkmate",
            r#"services:
  checkmate:
    image: bluewavelabs/checkmate:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-checkmate}
    restart: unless-stopped
    ports:
      - "${PORT:-5000}:5000"
    environment:
      - JWT_SECRET=${JWT_SECRET:-change-me-to-a-random-string}
    volumes:
      - checkmate_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  checkmate_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"checkmate","secret":false},{"name":"PORT","label":"Port","required":false,"default":"5000","secret":false},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== BATCH 2: SECURITY ====================
        (
            "tpl-batch2-authentik",
            "Authentik",
            "Flexible and versatile identity provider. SSO, MFA, user management, and application proxy.",
            "security",
            "authentik",
            r#"services:
  authentik-server:
    image: ghcr.io/goauthentik/server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-authentik-server}
    restart: unless-stopped
    command: server
    ports:
      - "${PORT:-9000}:9000"
      - "${HTTPS_PORT:-9443}:9443"
    environment:
      - AUTHENTIK_SECRET_KEY=${SECRET_KEY:-change-me-to-a-long-random-string}
      - AUTHENTIK_REDIS__HOST=authentik_redis
      - AUTHENTIK_POSTGRESQL__HOST=authentik_db
      - AUTHENTIK_POSTGRESQL__USER=authentik
      - AUTHENTIK_POSTGRESQL__PASSWORD=${DB_PASSWORD:-authentik}
      - AUTHENTIK_POSTGRESQL__NAME=authentik
    depends_on:
      - authentik_db
      - authentik_redis
    labels:
      - "rivetr.managed=true"

  authentik-worker:
    image: ghcr.io/goauthentik/server:${VERSION:-latest}
    restart: unless-stopped
    command: worker
    environment:
      - AUTHENTIK_SECRET_KEY=${SECRET_KEY:-change-me-to-a-long-random-string}
      - AUTHENTIK_REDIS__HOST=authentik_redis
      - AUTHENTIK_POSTGRESQL__HOST=authentik_db
      - AUTHENTIK_POSTGRESQL__USER=authentik
      - AUTHENTIK_POSTGRESQL__PASSWORD=${DB_PASSWORD:-authentik}
      - AUTHENTIK_POSTGRESQL__NAME=authentik
    depends_on:
      - authentik_db
      - authentik_redis
    labels:
      - "rivetr.managed=true"

  authentik_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=authentik
      - POSTGRES_PASSWORD=${DB_PASSWORD:-authentik}
      - POSTGRES_DB=authentik
    volumes:
      - authentik_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  authentik_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  authentik_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"authentik-server","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"9000","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"9443","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-batch2-keycloak",
            "Keycloak",
            "Open-source identity and access management solution. SSO, social login, and user federation.",
            "security",
            "keycloak",
            r#"services:
  keycloak:
    image: quay.io/keycloak/keycloak:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-keycloak}
    restart: unless-stopped
    command: start-dev
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - KEYCLOAK_ADMIN=${ADMIN_USER:-admin}
      - KEYCLOAK_ADMIN_PASSWORD=${ADMIN_PASSWORD:-admin}
      - KC_DB=postgres
      - KC_DB_URL_HOST=keycloak_db
      - KC_DB_URL_DATABASE=keycloak
      - KC_DB_USERNAME=keycloak
      - KC_DB_PASSWORD=${DB_PASSWORD:-keycloak}
    depends_on:
      - keycloak_db
    labels:
      - "rivetr.managed=true"

  keycloak_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=keycloak
      - POSTGRES_PASSWORD=${DB_PASSWORD:-keycloak}
      - POSTGRES_DB=keycloak
    volumes:
      - keycloak_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  keycloak_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"keycloak","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-batch2-infisical",
            "Infisical",
            "Open-source secret management platform. Sync secrets across teams, environments, and infrastructure.",
            "security",
            "infisical",
            r#"services:
  infisical:
    image: infisical/infisical:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-infisical}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - ENCRYPTION_KEY=${ENCRYPTION_KEY:-change-me-to-a-long-random-hex-string}
      - AUTH_SECRET=${AUTH_SECRET:-change-me-to-a-random-string}
      - MONGO_URL=mongodb://infisical_mongo:27017/infisical
      - REDIS_URL=redis://infisical_redis:6379
      - SITE_URL=${SITE_URL:-http://localhost:8080}
    depends_on:
      - infisical_mongo
      - infisical_redis
    labels:
      - "rivetr.managed=true"

  infisical_mongo:
    image: mongo:7
    restart: unless-stopped
    volumes:
      - infisical_mongo_data:/data/db
    labels:
      - "rivetr.managed=true"

  infisical_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  infisical_mongo_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"infisical","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"ENCRYPTION_KEY","label":"Encryption Key","required":true,"default":"","secret":true},{"name":"AUTH_SECRET","label":"Auth Secret","required":true,"default":"","secret":true},{"name":"SITE_URL","label":"Site URL","required":false,"default":"http://localhost:8080","secret":false}]"#,
        ),

        // ==================== BATCH 2: SEARCH ====================
        (
            "tpl-batch2-meilisearch",
            "Meilisearch",
            "Lightning-fast, typo-tolerant search engine. A great open-source alternative to Algolia.",
            "search",
            "meilisearch",
            r#"services:
  meilisearch:
    image: getmeili/meilisearch:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-meilisearch}
    restart: unless-stopped
    ports:
      - "${PORT:-7700}:7700"
    environment:
      - MEILI_MASTER_KEY=${MEILI_MASTER_KEY:-change-me-to-a-secure-key}
      - MEILI_ENV=${MEILI_ENV:-development}
    volumes:
      - meilisearch_data:/meili_data
    labels:
      - "rivetr.managed=true"

volumes:
  meilisearch_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"meilisearch","secret":false},{"name":"PORT","label":"Port","required":false,"default":"7700","secret":false},{"name":"MEILI_MASTER_KEY","label":"Master Key","required":true,"default":"","secret":true},{"name":"MEILI_ENV","label":"Environment","required":false,"default":"development","secret":false}]"#,
        ),
        (
            "tpl-batch2-typesense",
            "Typesense",
            "Fast, typo-tolerant search engine optimized for instant search-as-you-type experiences.",
            "search",
            "typesense",
            r#"services:
  typesense:
    image: typesense/typesense:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-typesense}
    restart: unless-stopped
    ports:
      - "${PORT:-8108}:8108"
    environment:
      - TYPESENSE_API_KEY=${TYPESENSE_API_KEY:-change-me-to-a-secure-key}
      - TYPESENSE_DATA_DIR=/data
    volumes:
      - typesense_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  typesense_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"typesense","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8108","secret":false},{"name":"TYPESENSE_API_KEY","label":"API Key","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== BATCH 2: PROJECT MANAGEMENT ====================
        (
            "tpl-batch2-plane",
            "Plane",
            "Open-source project tracking tool. A self-hosted alternative to Jira, Linear, and Asana.",
            "project-management",
            "plane",
            r#"services:
  plane:
    image: makeplane/plane-app:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-plane}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgresql://plane:${DB_PASSWORD:-plane}@plane_db:5432/plane
      - REDIS_URL=redis://plane_redis:6379
      - SECRET_KEY=${SECRET_KEY:-change-me-to-a-long-random-string}
    depends_on:
      - plane_db
      - plane_redis
    labels:
      - "rivetr.managed=true"

  plane_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=plane
      - POSTGRES_PASSWORD=${DB_PASSWORD:-plane}
      - POSTGRES_DB=plane
    volumes:
      - plane_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  plane_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  plane_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"plane","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-batch2-vikunja",
            "Vikunja",
            "Open-source to-do and Kanban app. Self-hosted alternative to Todoist and Trello.",
            "project-management",
            "vikunja",
            r#"services:
  vikunja:
    image: vikunja/vikunja:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-vikunja}
    restart: unless-stopped
    ports:
      - "${PORT:-3456}:3456"
    environment:
      - VIKUNJA_SERVICE_JWTSECRET=${JWT_SECRET:-change-me-to-a-random-string}
      - VIKUNJA_SERVICE_FRONTENDURL=${FRONTEND_URL:-http://localhost:3456}
    volumes:
      - vikunja_data:/app/vikunja/files
    labels:
      - "rivetr.managed=true"

volumes:
  vikunja_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"vikunja","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3456","secret":false},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true},{"name":"FRONTEND_URL","label":"Frontend URL","required":false,"default":"http://localhost:3456","secret":false}]"#,
        ),
        (
            "tpl-batch2-leantime",
            "Leantime",
            "Open-source project management system designed for non-project managers.",
            "project-management",
            "leantime",
            r#"services:
  leantime:
    image: leantime/leantime:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-leantime}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - LEAN_DB_HOST=leantime_db
      - LEAN_DB_USER=leantime
      - LEAN_DB_PASSWORD=${DB_PASSWORD:-leantime}
      - LEAN_DB_DATABASE=leantime
      - LEAN_SITENAME=${SITE_NAME:-Leantime}
    depends_on:
      - leantime_db
    labels:
      - "rivetr.managed=true"

  leantime_db:
    image: mysql:8.0
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=leantime
      - MYSQL_USER=leantime
      - MYSQL_PASSWORD=${DB_PASSWORD:-leantime}
      - MYSQL_RANDOM_ROOT_PASSWORD=yes
    volumes:
      - leantime_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  leantime_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"leantime","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"SITE_NAME","label":"Site Name","required":false,"default":"Leantime","secret":false}]"#,
        ),
        (
            "tpl-batch2-calcom",
            "Cal.com",
            "Open-source scheduling platform. Self-hosted alternative to Calendly.",
            "project-management",
            "calcom",
            r#"services:
  calcom:
    image: calcom/cal.com:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-calcom}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgresql://calcom:${DB_PASSWORD:-calcom}@calcom_db:5432/calcom
      - NEXTAUTH_SECRET=${NEXTAUTH_SECRET:-change-me-to-a-random-string}
      - NEXTAUTH_URL=${NEXTAUTH_URL:-http://localhost:3000}
      - CALENDSO_ENCRYPTION_KEY=${ENCRYPTION_KEY:-change-me-to-a-32-char-key}
    depends_on:
      - calcom_db
    labels:
      - "rivetr.managed=true"

  calcom_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=calcom
      - POSTGRES_PASSWORD=${DB_PASSWORD:-calcom}
      - POSTGRES_DB=calcom
    volumes:
      - calcom_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  calcom_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"calcom","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"NEXTAUTH_SECRET","label":"NextAuth Secret","required":true,"default":"","secret":true},{"name":"NEXTAUTH_URL","label":"NextAuth URL","required":true,"default":"http://localhost:3000","secret":false},{"name":"ENCRYPTION_KEY","label":"Encryption Key","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== BATCH 2: OTHER ====================
        (
            "tpl-batch2-paperless-ngx",
            "Paperless-ngx",
            "Document management system that transforms physical documents into a searchable online archive.",
            "other",
            "paperless-ngx",
            r#"services:
  paperless:
    image: ghcr.io/paperless-ngx/paperless-ngx:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-paperless}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - PAPERLESS_REDIS=redis://paperless_redis:6379
      - PAPERLESS_SECRET_KEY=${SECRET_KEY:-change-me-to-a-long-random-string}
      - PAPERLESS_ADMIN_USER=${ADMIN_USER:-admin}
      - PAPERLESS_ADMIN_PASSWORD=${ADMIN_PASSWORD:-admin}
      - PAPERLESS_URL=${URL:-http://localhost:8000}
    volumes:
      - paperless_data:/usr/src/paperless/data
      - paperless_media:/usr/src/paperless/media
      - paperless_consume:/usr/src/paperless/consume
    depends_on:
      - paperless_redis
    labels:
      - "rivetr.managed=true"

  paperless_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  paperless_data:
  paperless_media:
  paperless_consume:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"paperless","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"URL","label":"Public URL","required":false,"default":"http://localhost:8000","secret":false}]"#,
        ),
        (
            "tpl-batch2-trilium",
            "Trilium",
            "Hierarchical note-taking application with focus on building personal knowledge bases.",
            "other",
            "trilium",
            r#"services:
  trilium:
    image: zadam/trilium:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-trilium}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    volumes:
      - trilium_data:/home/node/trilium-data
    labels:
      - "rivetr.managed=true"

volumes:
  trilium_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"trilium","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-batch2-linkwarden",
            "Linkwarden",
            "Self-hosted collaborative bookmark manager to collect, organize, and preserve web content.",
            "other",
            "linkwarden",
            r#"services:
  linkwarden:
    image: ghcr.io/linkwarden/linkwarden:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-linkwarden}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgresql://linkwarden:${DB_PASSWORD:-linkwarden}@linkwarden_db:5432/linkwarden
      - NEXTAUTH_SECRET=${NEXTAUTH_SECRET:-change-me-to-a-random-string}
      - NEXTAUTH_URL=${NEXTAUTH_URL:-http://localhost:3000}
    depends_on:
      - linkwarden_db
    labels:
      - "rivetr.managed=true"

  linkwarden_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=linkwarden
      - POSTGRES_PASSWORD=${DB_PASSWORD:-linkwarden}
      - POSTGRES_DB=linkwarden
    volumes:
      - linkwarden_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  linkwarden_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"linkwarden","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"NEXTAUTH_SECRET","label":"NextAuth Secret","required":true,"default":"","secret":true},{"name":"NEXTAUTH_URL","label":"NextAuth URL","required":false,"default":"http://localhost:3000","secret":false}]"#,
        ),
        (
            "tpl-batch2-tandoor",
            "Tandoor Recipes",
            "Application for managing recipes, meal planning, and shopping lists.",
            "other",
            "tandoor",
            r#"services:
  tandoor:
    image: ghcr.io/tandoorrecipes/recipes:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-tandoor}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DB_ENGINE=django.db.backends.postgresql
      - POSTGRES_HOST=tandoor_db
      - POSTGRES_PORT=5432
      - POSTGRES_USER=tandoor
      - POSTGRES_PASSWORD=${DB_PASSWORD:-tandoor}
      - POSTGRES_DB=tandoor
      - SECRET_KEY=${SECRET_KEY:-change-me-to-a-long-random-string}
    volumes:
      - tandoor_static:/opt/recipes/staticfiles
      - tandoor_media:/opt/recipes/mediafiles
    depends_on:
      - tandoor_db
    labels:
      - "rivetr.managed=true"

  tandoor_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=tandoor
      - POSTGRES_PASSWORD=${DB_PASSWORD:-tandoor}
      - POSTGRES_DB=tandoor
    volumes:
      - tandoor_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  tandoor_static:
  tandoor_media:
  tandoor_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"tandoor","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-batch2-stirling-pdf",
            "Stirling-PDF",
            "Self-hosted web-based PDF manipulation tool. Merge, split, convert, and edit PDF files.",
            "other",
            "stirling-pdf",
            r#"services:
  stirling-pdf:
    image: frooodle/s-pdf:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-stirling-pdf}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DOCKER_ENABLE_SECURITY=${ENABLE_SECURITY:-false}
    volumes:
      - stirling_data:/usr/share/tessdata
      - stirling_config:/configs
    labels:
      - "rivetr.managed=true"

volumes:
  stirling_data:
  stirling_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"stirling-pdf","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"ENABLE_SECURITY","label":"Enable Security","required":false,"default":"false","secret":false}]"#,
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
