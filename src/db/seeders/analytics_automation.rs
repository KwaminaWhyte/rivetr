//! Analytics (additional) and automation service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
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
    ]
}
