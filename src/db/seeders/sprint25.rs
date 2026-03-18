//! Sprint 25 service templates: AI/LLM, CMS, Monitoring, Automation,
//! Auth/SSO, Media, DevTools, Productivity, and Finance
//!
//! Already present in earlier seeder files (skipped):
//! - Langfuse (ai_extras.rs as tpl-langfuse)
//! - Chroma (sprint16.rs as tpl-chroma; ai_ml.rs as chromadb)
//! - Weaviate (databases_tools.rs as tpl-weaviate)
//! - Ollama (ai_ml.rs as ollama)
//! - BookStack (documentation.rs as tpl-batch2-bookstack)
//! - Wiki.js (documentation.rs as tpl-batch2-wikijs)
//! - ClassicPress (sprint21.rs as tpl-classicpress)
//! - Uptime Kuma (infrastructure.rs as uptime-kuma)
//! - Grafana+Prometheus stack (infrastructure.rs as grafana-prometheus)
//! - Glances (media_productivity.rs as tpl-glances)
//! - n8n (infrastructure.rs as n8n; sprint18.rs as tpl-n8n)
//! - Authentik (security_search.rs as tpl-batch2-authentik)
//! - Keycloak (security_search.rs as tpl-batch2-keycloak)
//! - Jellyfin (documentation.rs as tpl-batch2-jellyfin)
//! - Navidrome (documentation.rs as tpl-batch2-navidrome)
//! - Audiobookshelf (media_productivity.rs as tpl-audiobookshelf)
//! - Gitea (infrastructure.rs as gitea)
//! - Forgejo (devtools.rs as forgejo)
//! - Woodpecker CI (devtools_extra.rs as tpl-woodpecker-ci; sprint16.rs as tpl-woodpecker-server/agent)
//! - Memos (media_productivity.rs as tpl-memos)
//! - Vikunja (project_mgmt.rs as tpl-batch2-vikunja)
//! - AppFlowy (media_productivity.rs as tpl-appflowy)
//! - Cal.com (project_mgmt.rs as tpl-batch2-calcom)
//! - Actual Budget (media_productivity.rs as tpl-actual-budget)
//! - Firefly III (media_productivity.rs as tpl-firefly-iii)
//!
//! Also removed as ID/semantic duplicates (existed under different IDs):
//! - Stirling-PDF (media_productivity.rs as tpl-stirling-pdf)
//! - Rallly (sprint18.rs as tpl-rallly)
//! - Portainer (extra_services.rs as tpl-portainer; infrastructure.rs as portainer)
//! - Penpot (media_productivity.rs as tpl-penpot)
//! - Paperless-ngx (media_productivity.rs as tpl-paperless-ngx)
//! - Mealie (sprint19.rs as tpl-mealie)
//! - Listmonk (media_productivity.rs as tpl-listmonk)
//! - Homepage (extra_services.rs as tpl-homepage)
//! - Healthchecks (monitoring_extra.rs as tpl-healthchecks)
//! - Focalboard (business.rs as tpl-focalboard)
//! - Changedetection.io (media_productivity.rs as tpl-changedetection)
//! - Plausible Analytics (infrastructure.rs as plausible)
//! - Umami (analytics_automation.rs as umami)
//! - Leantime (project_mgmt.rs as tpl-batch2-leantime)
//! - Immich (documentation.rs as tpl-batch2-immich)
//! - Ghost (cms_communication.rs as ghost)
//! - NocoDB (infrastructure.rs as nocodb)
//! - Linkwarden (project_mgmt.rs as tpl-batch2-linkwarden)
//!
//! Net new templates added (5): Joomla, Drupal, Grafana (standalone),
//! Etebase, Obsidian Remote

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== CMS / WEB ====================
        (
            "tpl-joomla",
            "Joomla",
            "Award-winning content management system. Build websites and powerful online applications with a flexible, intuitive interface and thousands of extensions. Requires a MySQL or PostgreSQL database.",
            "CMS",
            "joomla",
            r#"services:
  joomla:
    image: joomla:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-joomla}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - JOOMLA_DB_HOST=${DB_HOST:-joomla_db}
      - JOOMLA_DB_NAME=${DB_NAME:-joomla}
      - JOOMLA_DB_USER=${DB_USER:-joomla}
      - JOOMLA_DB_PASSWORD=${DB_PASSWORD}
      - JOOMLA_SITE_NAME=${SITE_NAME:-My Joomla Site}
      - JOOMLA_ADMIN_USER=${ADMIN_USER:-admin}
      - JOOMLA_ADMIN_PASSWORD=${ADMIN_PASSWORD}
      - JOOMLA_ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
    volumes:
      - joomla_data:/var/www/html
    depends_on:
      - joomla_db
    labels:
      - "rivetr.managed=true"

  joomla_db:
    image: mysql:8.0
    container_name: ${CONTAINER_NAME:-joomla}-db
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-rootpassword}
      - MYSQL_DATABASE=${DB_NAME:-joomla}
      - MYSQL_USER=${DB_USER:-joomla}
      - MYSQL_PASSWORD=${DB_PASSWORD}
    volumes:
      - joomla_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  joomla_data:
  joomla_db_data:
"#,
            r#"[{"name":"DB_PASSWORD","label":"MySQL Password","required":true,"default":"","secret":true},{"name":"ADMIN_PASSWORD","label":"Joomla Admin Password","required":true,"default":"","secret":true},{"name":"DB_ROOT_PASSWORD","label":"MySQL Root Password","required":false,"default":"rootpassword","secret":true},{"name":"SITE_NAME","label":"Site Name","required":false,"default":"My Joomla Site","secret":false},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_EMAIL","label":"Admin Email","required":false,"default":"admin@example.com","secret":false},{"name":"DB_HOST","label":"Database Host","required":false,"default":"joomla_db","secret":false},{"name":"DB_USER","label":"MySQL Username","required":false,"default":"joomla","secret":false},{"name":"DB_NAME","label":"MySQL Database Name","required":false,"default":"joomla","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"joomla","secret":false}]"#,
        ),
        (
            "tpl-drupal",
            "Drupal",
            "Open-source CMS and web application framework powering millions of websites. Highly extensible with thousands of modules and themes. Ideal for complex, content-rich sites. Requires a MySQL or PostgreSQL database.",
            "CMS",
            "drupal",
            r#"services:
  drupal:
    image: drupal:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-drupal}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - POSTGRES_USER=${DB_USER:-drupal}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=${DB_NAME:-drupal}
    volumes:
      - drupal_modules:/var/www/html/modules
      - drupal_profiles:/var/www/html/profiles
      - drupal_themes:/var/www/html/themes
      - drupal_sites:/var/www/html/sites
    depends_on:
      - drupal_db
    labels:
      - "rivetr.managed=true"

  drupal_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-drupal}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-drupal}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=${DB_NAME:-drupal}
    volumes:
      - drupal_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  drupal_modules:
  drupal_profiles:
  drupal_themes:
  drupal_sites:
  drupal_db_data:
"#,
            r#"[{"name":"DB_PASSWORD","label":"PostgreSQL Password","required":true,"default":"","secret":true},{"name":"DB_USER","label":"PostgreSQL Username","required":false,"default":"drupal","secret":false},{"name":"DB_NAME","label":"PostgreSQL Database Name","required":false,"default":"drupal","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"drupal","secret":false}]"#,
        ),
        // ==================== MONITORING ====================
        (
            "tpl-grafana",
            "Grafana",
            "Leading open-source analytics and monitoring platform. Create beautiful dashboards to visualize metrics from Prometheus, InfluxDB, Loki, and dozens of other data sources. Includes alerting and team collaboration.",
            "Monitoring",
            "grafana",
            r#"services:
  grafana:
    image: grafana/grafana:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-grafana}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - GF_SECURITY_ADMIN_USER=${GF_SECURITY_ADMIN_USER:-admin}
      - GF_SECURITY_ADMIN_PASSWORD=${GF_SECURITY_ADMIN_PASSWORD}
      - GF_USERS_ALLOW_SIGN_UP=${GF_USERS_ALLOW_SIGN_UP:-false}
      - GF_SERVER_ROOT_URL=${GF_SERVER_ROOT_URL:-http://localhost:3000}
      - GF_SMTP_ENABLED=${GF_SMTP_ENABLED:-false}
      - GF_SMTP_HOST=${GF_SMTP_HOST:-}
      - GF_SMTP_FROM_ADDRESS=${GF_SMTP_FROM_ADDRESS:-}
    volumes:
      - grafana_data:/var/lib/grafana
    labels:
      - "rivetr.managed=true"

volumes:
  grafana_data:
"#,
            r#"[{"name":"GF_SECURITY_ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"GF_SECURITY_ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"GF_USERS_ALLOW_SIGN_UP","label":"Allow User Sign-Up","required":false,"default":"false","secret":false},{"name":"GF_SERVER_ROOT_URL","label":"Root URL (e.g. https://grafana.example.com)","required":false,"default":"http://localhost:3000","secret":false},{"name":"GF_SMTP_ENABLED","label":"Enable SMTP","required":false,"default":"false","secret":false},{"name":"GF_SMTP_HOST","label":"SMTP Host","required":false,"default":"","secret":false},{"name":"GF_SMTP_FROM_ADDRESS","label":"SMTP From Address","required":false,"default":"","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"grafana","secret":false}]"#,
        ),
        // ==================== PRODUCTIVITY ====================
        // ==================== ANALYTICS ====================
        // ==================== DEVTOOLS ====================
        // ==================== INFRASTRUCTURE ====================
        // ==================== BUSINESS ====================
        // ==================== DESIGN ====================
        // ==================== AUTH / IDENTITY ====================
        (
            "tpl-etebase",
            "Etebase",
            "End-to-end encrypted backend for apps. Run your own sync server for EteSync, a self-hosted alternative to Google Calendar and Contacts with zero-knowledge encryption. Simple Python/Django backend.",
            "Auth/SSO",
            "etebase",
            r#"services:
  etebase:
    image: victorrds/etebase:${VERSION:-alpine}
    container_name: ${CONTAINER_NAME:-etebase}
    restart: unless-stopped
    ports:
      - "${PORT:-3735}:3735"
    environment:
      - SERVER=http
      - SUPER_USER=${SUPER_USER:-admin}
      - SUPER_PASS=${SUPER_PASS}
      - SUPER_EMAIL=${SUPER_EMAIL:-admin@example.com}
      - DB_ENGINE=${DB_ENGINE:-sqlite}
      - AUTO_UPDATE=true
    volumes:
      - etebase_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  etebase_data:
"#,
            r#"[{"name":"SUPER_PASS","label":"Superuser Password","required":true,"default":"","secret":true},{"name":"SUPER_USER","label":"Superuser Username","required":false,"default":"admin","secret":false},{"name":"SUPER_EMAIL","label":"Superuser Email","required":false,"default":"admin@example.com","secret":false},{"name":"DB_ENGINE","label":"Database Engine (sqlite or django.db.backends.postgresql)","required":false,"default":"sqlite","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3735","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"alpine","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"etebase","secret":false}]"#,
        ),
        // ==================== MONITORING ====================
        // ==================== MARKETING ====================
        (
            "tpl-obsidian-remote",
            "Obsidian Remote (obsidian-remote)",
            "Run Obsidian, the popular knowledge management app, in a browser via noVNC. Access your vaults remotely without installing the Obsidian desktop app. Useful for server-side note editing.",
            "Productivity",
            "obsidian",
            r#"services:
  obsidian-remote:
    image: sytone/obsidian-remote:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-obsidian-remote}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - PUID=${PUID:-1000}
      - PGID=${PGID:-1000}
      - TZ=${TZ:-UTC}
    volumes:
      - obsidian_vaults:/vaults
      - obsidian_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  obsidian_vaults:
  obsidian_config:
"#,
            r#"[{"name":"PUID","label":"Process UID (user ID for file permissions)","required":false,"default":"1000","secret":false},{"name":"PGID","label":"Process GID (group ID for file permissions)","required":false,"default":"1000","secret":false},{"name":"TZ","label":"Timezone (e.g. America/New_York)","required":false,"default":"UTC","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"obsidian-remote","secret":false}]"#,
        ),
    ]
}
