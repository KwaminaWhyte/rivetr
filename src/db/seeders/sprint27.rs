//! Sprint 27 service templates: popular self-hosted apps that were still missing
//! from the catalog.
//!
//! Conventions for this file:
//! - Upstream images pinned to a major, major.minor or exact release tag (never `latest`).
//! - Secrets come from magic variables (`${SERVICE_PASSWORD_*}`, `${SERVICE_BASE64_*}`),
//!   public URLs from `${SERVICE_URL_*}` / `${SERVICE_FQDN_*}`. Nothing is hardcoded.
//! - Databases are internal only (no host ports) and gated with healthchecks;
//!   apps wait on them with `condition: service_healthy`.
//! - The web-facing service is always listed first (Rivetr publishes its first port).
//! - Supporting service names are prefixed with the app name because every service
//!   also joins the shared `rivetr` network, where bare names like `postgres` collide.
//!
//! Net new templates (30): Kestra, Apache Superset, Grafana OTel LGTM, Coder, DocuSeal,
//! Komga, Wallabag, Kutt, Planka, Gatus, Semaphore UI, Mailpit, Paperless-AI, Ghostfolio,
//! Seerr, CyberChef, Redis Insight, pgweb, ownCloud Infinite Scale, RustDesk Server,
//! LibreSpeed, ConvertX, Unleash, Snipe-IT, Centrifugo, Bugsink, Gotenberg, Tautulli,
//! Technitium DNS, Whisper ASR.

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== AUTOMATION ====================
        (
            "tpl-kestra",
            "Kestra",
            "Event-driven orchestration and scheduling platform. Build data pipelines and automations as declarative YAML flows with a web UI, hundreds of plugins and a PostgreSQL backend. Script tasks use the process runner (no Docker socket is mounted).",
            "automation",
            "kestra",
            r#"services:
  kestra:
    image: kestra/kestra:v1.3
    restart: unless-stopped
    user: "root"
    command: server standalone
    ports:
      - "${PORT:-8080}:8080"
    environment:
      KESTRA_CONFIGURATION: |
        datasources:
          postgres:
            url: jdbc:postgresql://kestra-db:5432/kestra
            driverClassName: org.postgresql.Driver
            username: kestra
            password: ${SERVICE_PASSWORD_POSTGRES}
        kestra:
          server:
            basicAuth:
              username: ${ADMIN_EMAIL:-admin@example.com}
              password: ${SERVICE_PASSWORD_ADMIN}
          repository:
            type: postgres
          queue:
            type: postgres
          storage:
            type: local
            local:
              basePath: /app/storage
          tasks:
            tmpDir:
              path: /tmp/kestra-wd/tmp
          url: ${SERVICE_URL_KESTRA}/
    volumes:
      - kestra_storage:/app/storage
      - kestra_tmp:/tmp/kestra-wd
    depends_on:
      kestra-db:
        condition: service_healthy
    labels:
      - "rivetr.managed=true"

  kestra-db:
    image: postgres:17-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=kestra
      - POSTGRES_PASSWORD=${SERVICE_PASSWORD_POSTGRES}
      - POSTGRES_DB=kestra
    volumes:
      - kestra_db_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U kestra -d kestra"]
      interval: 10s
      timeout: 5s
      retries: 10
    labels:
      - "rivetr.managed=true"

volumes:
  kestra_storage:
  kestra_tmp:
  kestra_db_data:
"#,
            r#"[{"name":"ADMIN_EMAIL","label":"Admin Login Email (password is auto-generated, see service env)","required":true,"default":"admin@example.com","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        // ==================== ANALYTICS ====================
        (
            "tpl-superset",
            "Apache Superset",
            "Modern data exploration and visualization platform. Connect SQL databases, build charts and dashboards, and explore data with a no-code builder or SQL Lab. Metadata is stored in SQLite on a persistent volume.",
            "analytics",
            "superset",
            r#"services:
  superset:
    image: apache/superset:6.1.0
    restart: unless-stopped
    ports:
      - "${PORT:-8088}:8088"
    environment:
      - SUPERSET_SECRET_KEY=${SERVICE_BASE64_64_SUPERSET}
      - SUPERSET_LOAD_EXAMPLES=no
    command:
      - sh
      - -c
      - |
        superset db upgrade &&
        (superset fab create-admin --username ${ADMIN_USERNAME:-admin} --firstname Admin --lastname User --email ${ADMIN_EMAIL:-admin@example.com} --password ${SERVICE_PASSWORD_ADMIN} || true) &&
        superset init &&
        /app/docker/entrypoints/run-server.sh
    volumes:
      - superset_home:/app/superset_home
    healthcheck:
      test: ["CMD-SHELL", "/app/docker/docker-healthcheck.sh"]
      interval: 30s
      timeout: 10s
      retries: 5
      start_period: 120s
    labels:
      - "rivetr.managed=true"

volumes:
  superset_home:
"#,
            r#"[{"name":"ADMIN_USERNAME","label":"Admin Username (password is auto-generated, see service env)","required":false,"default":"admin","secret":false},{"name":"ADMIN_EMAIL","label":"Admin Email","required":false,"default":"admin@example.com","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8088","secret":false}]"#,
        ),
        // ==================== MONITORING ====================
        (
            "tpl-grafana-otel-lgtm",
            "Grafana OTel LGTM",
            "All-in-one OpenTelemetry backend from Grafana Labs: an OTLP collector plus Loki (logs), Grafana, Tempo (traces) and Mimir/Prometheus (metrics) in a single container. Point apps at port 4317 (gRPC) or 4318 (HTTP) over the rivetr network. Best suited to development and small setups.",
            "monitoring",
            "grafana",
            r#"services:
  otel-lgtm:
    image: grafana/otel-lgtm:0.39.0
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
      - "127.0.0.1:${OTLP_GRPC_PORT:-4317}:4317"
      - "127.0.0.1:${OTLP_HTTP_PORT:-4318}:4318"
    environment:
      - GF_SECURITY_ADMIN_USER=admin
      - GF_SECURITY_ADMIN_PASSWORD=${SERVICE_PASSWORD_GRAFANA}
      - GF_AUTH_ANONYMOUS_ENABLED=false
      - GF_AUTH_DISABLE_LOGIN_FORM=false
    volumes:
      - otel_lgtm_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  otel_lgtm_data:
"#,
            r#"[{"name":"PORT","label":"Grafana Host Port","required":false,"default":"3000","secret":false},{"name":"OTLP_GRPC_PORT","label":"OTLP gRPC Port (bound to 127.0.0.1)","required":false,"default":"4317","secret":false},{"name":"OTLP_HTTP_PORT","label":"OTLP HTTP Port (bound to 127.0.0.1)","required":false,"default":"4318","secret":false}]"#,
        ),
        // ==================== DEVELOPMENT ====================
        (
            "tpl-coder",
            "Coder",
            "Self-hosted cloud development environments. Provision remote workspaces from Terraform templates and connect with VS Code, JetBrains or the browser. Uses PostgreSQL. The first account created in the UI becomes the owner. Mount the Docker socket yourself if you want Docker-based workspaces.",
            "development",
            "coder",
            r#"services:
  coder:
    image: ghcr.io/coder/coder:v2.38.6
    restart: unless-stopped
    ports:
      - "${PORT:-7080}:7080"
    environment:
      - CODER_PG_CONNECTION_URL=postgresql://coder:${SERVICE_PASSWORD_POSTGRES}@coder-db:5432/coder?sslmode=disable
      - CODER_HTTP_ADDRESS=0.0.0.0:7080
      - CODER_ACCESS_URL=${SERVICE_URL_CODER}
      - CODER_TELEMETRY_ENABLE=false
    volumes:
      - coder_home:/home/coder
    depends_on:
      coder-db:
        condition: service_healthy
    labels:
      - "rivetr.managed=true"

  coder-db:
    image: postgres:17-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=coder
      - POSTGRES_PASSWORD=${SERVICE_PASSWORD_POSTGRES}
      - POSTGRES_DB=coder
    volumes:
      - coder_db_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U coder -d coder"]
      interval: 10s
      timeout: 5s
      retries: 10
    labels:
      - "rivetr.managed=true"

volumes:
  coder_home:
  coder_db_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"7080","secret":false}]"#,
        ),
        // ==================== BUSINESS ====================
        (
            "tpl-docuseal",
            "DocuSeal",
            "Open-source document signing platform, a DocuSign alternative. Create fillable PDF forms, send them for signature, and track completion. Uses PostgreSQL. Create the admin account on first visit.",
            "business",
            "docuseal",
            r#"services:
  docuseal:
    image: docuseal/docuseal:3.3.5
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - HOST=${SERVICE_FQDN_DOCUSEAL}
      - SECRET_KEY_BASE=${SERVICE_BASE64_64_DOCUSEAL}
      - DATABASE_URL=postgresql://docuseal:${SERVICE_PASSWORD_POSTGRES}@docuseal-db:5432/docuseal
    volumes:
      - docuseal_data:/data/docuseal
    depends_on:
      docuseal-db:
        condition: service_healthy
    labels:
      - "rivetr.managed=true"

  docuseal-db:
    image: postgres:17-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=docuseal
      - POSTGRES_PASSWORD=${SERVICE_PASSWORD_POSTGRES}
      - POSTGRES_DB=docuseal
    volumes:
      - docuseal_db_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U docuseal -d docuseal"]
      interval: 10s
      timeout: 5s
      retries: 10
    labels:
      - "rivetr.managed=true"

volumes:
  docuseal_data:
  docuseal_db_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
        // ==================== MEDIA ====================
        (
            "tpl-komga",
            "Komga",
            "Media server for comics, manga, BDs and magazines. Organize CBZ/CBR/PDF/EPUB libraries, read in the browser, and sync with OPDS and Tachiyomi/Mihon clients. Create the admin account on first visit.",
            "media",
            "komga",
            r#"services:
  komga:
    image: gotson/komga:1.x
    restart: unless-stopped
    ports:
      - "${PORT:-25600}:25600"
    environment:
      - TZ=${TZ:-UTC}
    volumes:
      - komga_config:/config
      - komga_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  komga_config:
  komga_data:
"#,
            r#"[{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"25600","secret":false}]"#,
        ),
        // ==================== PRODUCTIVITY ====================
        (
            "tpl-wallabag",
            "Wallabag",
            "Read-it-later app: save web articles, read them distraction-free, tag and annotate, with browser extensions and mobile apps. Uses SQLite on a persistent volume. The image creates a default wallabag/wallabag account: change its password right after the first login.",
            "productivity",
            "wallabag",
            r#"services:
  wallabag:
    image: wallabag/wallabag:2.6.15.1
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - SYMFONY__ENV__DOMAIN_NAME=${SERVICE_URL_WALLABAG}
      - SYMFONY__ENV__SERVER_NAME=${SERVER_NAME:-Wallabag}
      - SYMFONY__ENV__SECRET=${SERVICE_PASSWORD_SECRET}
      - SYMFONY__ENV__FOSUSER_REGISTRATION=false
      - SYMFONY__ENV__FOSUSER_CONFIRMATION=false
    volumes:
      - wallabag_data:/var/www/wallabag/data
      - wallabag_images:/var/www/wallabag/web/assets/images
    labels:
      - "rivetr.managed=true"

volumes:
  wallabag_data:
  wallabag_images:
"#,
            r#"[{"name":"SERVER_NAME","label":"Instance Name","required":false,"default":"Wallabag","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-kutt",
            "Kutt",
            "Modern URL shortener with custom domains, link expiration, password-protected links and per-link statistics. Uses SQLite on a persistent volume. The first account to sign up becomes the admin.",
            "productivity",
            "kutt",
            r#"services:
  kutt:
    image: kutt/kutt:v3.2.6
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - JWT_SECRET=${SERVICE_PASSWORD_JWT}
      - DEFAULT_DOMAIN=${SERVICE_FQDN_KUTT}
      - DB_FILENAME=/var/lib/kutt/data.sqlite
      - TRUST_PROXY=true
      - DISALLOW_REGISTRATION=${DISALLOW_REGISTRATION:-false}
    volumes:
      - kutt_data:/var/lib/kutt
    labels:
      - "rivetr.managed=true"

volumes:
  kutt_data:
"#,
            r#"[{"name":"DISALLOW_REGISTRATION","label":"Disallow Public Registration (true/false)","required":false,"default":"false","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
        (
            "tpl-planka",
            "Planka",
            "Real-time Kanban board for teams, a Trello alternative with projects, boards, labels, due dates, attachments and notifications. Uses PostgreSQL. An admin account is created on first start (password auto-generated, see service env).",
            "productivity",
            "planka",
            r#"services:
  planka:
    image: ghcr.io/plankanban/planka:2.3.1
    restart: unless-stopped
    ports:
      - "${PORT:-1337}:1337"
    environment:
      - BASE_URL=${SERVICE_URL_PLANKA}
      - DATABASE_URL=postgresql://planka:${SERVICE_PASSWORD_POSTGRES}@planka-db:5432/planka
      - SECRET_KEY=${SERVICE_PASSWORD_SECRET}
      - TRUST_PROXY=true
      - DEFAULT_ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - DEFAULT_ADMIN_USERNAME=${ADMIN_USERNAME:-admin}
      - DEFAULT_ADMIN_NAME=${ADMIN_NAME:-Admin}
      - DEFAULT_ADMIN_PASSWORD=${SERVICE_PASSWORD_ADMIN}
    volumes:
      - planka_data:/app/data
    depends_on:
      planka-db:
        condition: service_healthy
    labels:
      - "rivetr.managed=true"

  planka-db:
    image: postgres:17-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=planka
      - POSTGRES_PASSWORD=${SERVICE_PASSWORD_POSTGRES}
      - POSTGRES_DB=planka
    volumes:
      - planka_db_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U planka -d planka"]
      interval: 10s
      timeout: 5s
      retries: 10
    labels:
      - "rivetr.managed=true"

volumes:
  planka_data:
  planka_db_data:
"#,
            r#"[{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_USERNAME","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_NAME","label":"Admin Display Name","required":false,"default":"Admin","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"1337","secret":false}]"#,
        ),
        (
            "tpl-gatus",
            "Gatus",
            "Developer-oriented health dashboard and status page. Monitor HTTP, TCP, ICMP and DNS endpoints with conditions on status, body and latency, and alert to Slack, Discord, PagerDuty and more. Edit config.yaml in the config volume to add endpoints.",
            "monitoring",
            "gatus",
            r#"services:
  gatus:
    image: twinproduction/gatus:v5.38.0
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    volumes:
      - gatus_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  gatus_config:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-semaphore",
            "Semaphore UI",
            "Modern web UI for Ansible, Terraform/OpenTofu, PowerShell and shell scripts. Run playbooks from Git repositories with schedules, inventories, secrets and per-project access control. Uses PostgreSQL. The admin password is auto-generated (see service env).",
            "devops",
            "semaphore",
            r#"services:
  semaphore:
    image: semaphoreui/semaphore:v2.19.17
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - SEMAPHORE_DB_DIALECT=postgres
      - SEMAPHORE_DB_HOST=semaphore-db
      - SEMAPHORE_DB_PORT=5432
      - SEMAPHORE_DB_USER=semaphore
      - SEMAPHORE_DB_PASS=${SERVICE_PASSWORD_POSTGRES}
      - SEMAPHORE_DB=semaphore
      - 'SEMAPHORE_DB_OPTIONS={"sslmode":"disable"}'
      - SEMAPHORE_PLAYBOOK_PATH=/tmp/semaphore/
      - SEMAPHORE_ADMIN=${ADMIN_USERNAME:-admin}
      - SEMAPHORE_ADMIN_NAME=${ADMIN_NAME:-Admin}
      - SEMAPHORE_ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - SEMAPHORE_ADMIN_PASSWORD=${SERVICE_PASSWORD_ADMIN}
      - SEMAPHORE_ACCESS_KEY_ENCRYPTION=${SERVICE_BASE64_SEMAPHORE}
      - SEMAPHORE_WEB_ROOT=${SERVICE_URL_SEMAPHORE}
      - TZ=${TZ:-UTC}
    volumes:
      - semaphore_data:/var/lib/semaphore
      - semaphore_config:/etc/semaphore
      - semaphore_tmp:/tmp/semaphore
    depends_on:
      semaphore-db:
        condition: service_healthy
    labels:
      - "rivetr.managed=true"

  semaphore-db:
    image: postgres:17-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=semaphore
      - POSTGRES_PASSWORD=${SERVICE_PASSWORD_POSTGRES}
      - POSTGRES_DB=semaphore
    volumes:
      - semaphore_db_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U semaphore -d semaphore"]
      interval: 10s
      timeout: 5s
      retries: 10
    labels:
      - "rivetr.managed=true"

volumes:
  semaphore_data:
  semaphore_config:
  semaphore_tmp:
  semaphore_db_data:
"#,
            r#"[{"name":"ADMIN_USERNAME","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_NAME","label":"Admin Display Name","required":false,"default":"Admin","secret":false},{"name":"ADMIN_EMAIL","label":"Admin Email","required":false,"default":"admin@example.com","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
        (
            "tpl-mailpit",
            "Mailpit",
            "Email and SMTP testing tool for developers. Catches all outgoing mail from your apps and shows it in a fast web UI with HTML preview, spam and link checks, and a REST API. The web UI is protected with basic auth (user admin, password auto-generated, see service env).",
            "development",
            "mailpit",
            r#"services:
  mailpit:
    image: axllent/mailpit:v1.33
    restart: unless-stopped
    ports:
      - "${PORT:-8025}:8025"
      - "${SMTP_PORT:-1025}:1025"
    environment:
      - MP_DATABASE=/data/mailpit.db
      - MP_MAX_MESSAGES=${MAX_MESSAGES:-5000}
      - MP_UI_AUTH=admin:${SERVICE_PASSWORD_UI}
      - MP_SMTP_AUTH_ACCEPT_ANY=true
      - MP_SMTP_AUTH_ALLOW_INSECURE=true
      - TZ=${TZ:-UTC}
    volumes:
      - mailpit_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  mailpit_data:
"#,
            r#"[{"name":"MAX_MESSAGES","label":"Max Stored Messages","required":false,"default":"5000","secret":false},{"name":"SMTP_PORT","label":"SMTP Host Port","required":false,"default":"1025","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"PORT","label":"Web UI Host Port","required":false,"default":"8025","secret":false}]"#,
        ),
        // ==================== AI ====================
        (
            "tpl-paperless-ai",
            "Paperless-AI",
            "AI companion for Paperless-ngx. Automatically classifies, tags and titles new documents with OpenAI, Ollama or other LLM providers, and adds a RAG chat over your archive. Connect it to Paperless-ngx in the setup wizard.",
            "ai",
            "paperless",
            r#"services:
  paperless-ai:
    image: clusterzx/paperless-ai:3.0.9
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - PUID=${PUID:-1000}
      - PGID=${PGID:-1000}
      - PAPERLESS_AI_PORT=3000
    volumes:
      - paperless_ai_data:/app/data
    labels:
      - "rivetr.managed=true"

volumes:
  paperless_ai_data:
"#,
            r#"[{"name":"PUID","label":"User ID","required":false,"default":"1000","secret":false},{"name":"PGID","label":"Group ID","required":false,"default":"1000","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
        // ==================== BUSINESS ====================
        (
            "tpl-ghostfolio",
            "Ghostfolio",
            "Open-source wealth management software. Track stocks, ETFs and crypto across accounts, see performance, allocation and dividends, with market data from Yahoo Finance and others. Uses PostgreSQL and Redis. The first account created becomes the admin.",
            "business",
            "ghostfolio",
            r#"services:
  ghostfolio:
    image: ghostfolio/ghostfolio:3
    restart: unless-stopped
    init: true
    ports:
      - "${PORT:-3333}:3333"
    environment:
      - DATABASE_URL=postgresql://ghostfolio:${SERVICE_PASSWORD_POSTGRES}@ghostfolio-db:5432/ghostfolio?connect_timeout=300&sslmode=prefer
      - REDIS_HOST=ghostfolio-redis
      - REDIS_PORT=6379
      - REDIS_PASSWORD=${SERVICE_PASSWORD_REDIS}
      - ACCESS_TOKEN_SALT=${SERVICE_PASSWORD_SALT}
      - JWT_SECRET_KEY=${SERVICE_PASSWORD_JWT}
    depends_on:
      ghostfolio-db:
        condition: service_healthy
      ghostfolio-redis:
        condition: service_healthy
    healthcheck:
      test: ["CMD-SHELL", "curl -f http://localhost:3333/api/v1/health"]
      interval: 10s
      timeout: 5s
      retries: 5
    labels:
      - "rivetr.managed=true"

  ghostfolio-db:
    image: postgres:17-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=ghostfolio
      - POSTGRES_PASSWORD=${SERVICE_PASSWORD_POSTGRES}
      - POSTGRES_DB=ghostfolio
    volumes:
      - ghostfolio_db_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ghostfolio -d ghostfolio"]
      interval: 10s
      timeout: 5s
      retries: 10
    labels:
      - "rivetr.managed=true"

  ghostfolio-redis:
    image: redis:7-alpine
    restart: unless-stopped
    command: redis-server --requirepass ${SERVICE_PASSWORD_REDIS}
    healthcheck:
      test: ["CMD-SHELL", "redis-cli -a ${SERVICE_PASSWORD_REDIS} --no-auth-warning ping | grep PONG"]
      interval: 10s
      timeout: 5s
      retries: 5
    labels:
      - "rivetr.managed=true"

volumes:
  ghostfolio_db_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"3333","secret":false}]"#,
        ),
        // ==================== MEDIA ====================
        (
            "tpl-seerr",
            "Seerr",
            "Media request manager for Jellyfin, Plex and Emby, the merged successor of Overseerr and Jellyseerr. Users discover and request movies and shows, which are sent to Sonarr and Radarr automatically.",
            "media",
            "seerr",
            r#"services:
  seerr:
    image: ghcr.io/seerr-team/seerr:v3
    restart: unless-stopped
    init: true
    ports:
      - "${PORT:-5055}:5055"
    environment:
      - LOG_LEVEL=${LOG_LEVEL:-info}
      - TZ=${TZ:-UTC}
      - PORT=5055
    volumes:
      - seerr_config:/app/config
    healthcheck:
      test: ["CMD-SHELL", "wget --no-verbose --tries=1 --spider http://localhost:5055/api/v1/status || exit 1"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 30s
    labels:
      - "rivetr.managed=true"

volumes:
  seerr_config:
"#,
            r#"[{"name":"LOG_LEVEL","label":"Log Level","required":false,"default":"info","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"5055","secret":false}]"#,
        ),
        (
            "tpl-tautulli",
            "Tautulli",
            "Monitoring and statistics companion for Plex Media Server. See who is watching what, playback history, library stats and graphs, and send notifications on playback events.",
            "media",
            "tautulli",
            r#"services:
  tautulli:
    image: ghcr.io/tautulli/tautulli:v2.18.2
    restart: unless-stopped
    ports:
      - "${PORT:-8181}:8181"
    environment:
      - PUID=${PUID:-1000}
      - PGID=${PGID:-1000}
      - TZ=${TZ:-UTC}
    volumes:
      - tautulli_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  tautulli_config:
"#,
            r#"[{"name":"PUID","label":"User ID","required":false,"default":"1000","secret":false},{"name":"PGID","label":"Group ID","required":false,"default":"1000","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8181","secret":false}]"#,
        ),
        // ==================== DEVELOPMENT ====================
        (
            "tpl-cyberchef",
            "CyberChef",
            "The Cyber Swiss Army Knife from GCHQ. Encode, decode, encrypt, compress, parse and analyse data with hundreds of chainable operations, all in the browser. Stateless, no data leaves the client.",
            "development",
            "cyberchef",
            r#"services:
  cyberchef:
    image: ghcr.io/gchq/cyberchef:11
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-redisinsight",
            "Redis Insight",
            "Official Redis GUI. Browse keys, run commands in a workbench, profile and analyse memory, and manage Redis, Valkey and Redis Stack databases. Add connections in the UI using container hostnames on the rivetr network.",
            "database",
            "redis",
            r#"services:
  redisinsight:
    image: redis/redisinsight:3.8
    restart: unless-stopped
    ports:
      - "${PORT:-5540}:5540"
    volumes:
      - redisinsight_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  redisinsight_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"5540","secret":false}]"#,
        ),
        (
            "tpl-pgweb",
            "pgweb",
            "Lightweight web-based PostgreSQL browser written in Go. Browse tables, run queries, export results and inspect schemas. Runs in session mode behind basic auth (user admin, password auto-generated, see service env); connect to any database from the UI.",
            "database",
            "postgresql",
            r#"services:
  pgweb:
    image: sosedoff/pgweb:0.17.0
    restart: unless-stopped
    ports:
      - "${PORT:-8081}:8081"
    command:
      - --sessions
      - --auth-user=admin
      - --auth-pass=${SERVICE_PASSWORD_PGWEB}
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8081","secret":false}]"#,
        ),
        // ==================== STORAGE ====================
        (
            "tpl-ocis",
            "ownCloud Infinite Scale",
            "Modern file sync and share platform from ownCloud, rewritten in Go with no external database. Spaces, sharing, versioning and desktop/mobile clients. Requires a domain; log in as admin with the auto-generated password (see service env).",
            "storage",
            "owncloud",
            r#"services:
  ocis:
    image: owncloud/ocis:8
    restart: unless-stopped
    entrypoint: /bin/sh
    command: ["-c", "ocis init || true; ocis server"]
    ports:
      - "${PORT:-9200}:9200"
    environment:
      - OCIS_URL=${SERVICE_URL_OCIS}
      - OCIS_INSECURE=true
      - PROXY_TLS=false
      - PROXY_HTTP_ADDR=0.0.0.0:9200
      - IDM_ADMIN_PASSWORD=${SERVICE_PASSWORD_ADMIN}
      - OCIS_LOG_LEVEL=${LOG_LEVEL:-info}
    volumes:
      - ocis_config:/etc/ocis
      - ocis_data:/var/lib/ocis
    labels:
      - "rivetr.managed=true"

volumes:
  ocis_config:
  ocis_data:
"#,
            r#"[{"name":"LOG_LEVEL","label":"Log Level","required":false,"default":"info","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"9200","secret":false}]"#,
        ),
        // ==================== NETWORKING ====================
        (
            "tpl-rustdesk-server",
            "RustDesk Server",
            "Self-hosted ID/rendezvous (hbbs) and relay (hbbr) server for RustDesk, the open-source TeamViewer alternative. Not a web app: point RustDesk clients at this host and paste the public key from id_ed25519.pub in the data volume.",
            "networking",
            "rustdesk",
            r#"services:
  hbbs:
    image: rustdesk/rustdesk-server:1
    restart: unless-stopped
    command: hbbs
    ports:
      - "${PORT:-21115}:21115"
      - "21116:21116"
      - "21116:21116/udp"
      - "21118:21118"
    volumes:
      - rustdesk_data:/root
    depends_on:
      - hbbr
    labels:
      - "rivetr.managed=true"

  hbbr:
    image: rustdesk/rustdesk-server:1
    restart: unless-stopped
    command: hbbr
    ports:
      - "21117:21117"
      - "21119:21119"
    volumes:
      - rustdesk_data:/root
    labels:
      - "rivetr.managed=true"

volumes:
  rustdesk_data:
"#,
            r#"[{"name":"PORT","label":"NAT Test Host Port","required":false,"default":"21115","secret":false}]"#,
        ),
        (
            "tpl-librespeed",
            "LibreSpeed",
            "Lightweight self-hosted speed test. Measures download, upload, ping and jitter from the browser with no Flash or Java. Telemetry is off by default.",
            "networking",
            "speedtest",
            r#"services:
  librespeed:
    image: ghcr.io/librespeed/speedtest:6
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - MODE=standalone
      - WEBPORT=8080
      - TITLE=${TITLE:-LibreSpeed}
      - TELEMETRY=false
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"TITLE","label":"Page Title","required":false,"default":"LibreSpeed","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-technitium-dns",
            "Technitium DNS Server",
            "Authoritative and recursive DNS server with a web console. Block ads and malware with blocklists, host zones, use DNS-over-HTTPS/TLS/QUIC, and run a DHCP server. Admin password auto-generated (see service env). Port 53 may clash with systemd-resolved on the host.",
            "networking",
            "dns",
            r#"services:
  technitium:
    image: technitium/dns-server:15.6.2
    restart: unless-stopped
    hostname: ${DNS_SERVER_DOMAIN:-dns-server}
    ports:
      - "${PORT:-5380}:5380"
      - "${DNS_PORT:-53}:53/udp"
      - "${DNS_PORT:-53}:53/tcp"
    environment:
      - DNS_SERVER_DOMAIN=${DNS_SERVER_DOMAIN:-dns-server}
      - DNS_SERVER_ADMIN_PASSWORD=${SERVICE_PASSWORD_ADMIN}
    volumes:
      - technitium_config:/etc/dns
    sysctls:
      - net.ipv4.ip_local_port_range=1024 65000
    labels:
      - "rivetr.managed=true"

volumes:
  technitium_config:
"#,
            r#"[{"name":"DNS_SERVER_DOMAIN","label":"Server Domain Name","required":false,"default":"dns-server","secret":false},{"name":"DNS_PORT","label":"DNS Host Port","required":false,"default":"53","secret":false},{"name":"PORT","label":"Web Console Host Port","required":false,"default":"5380","secret":false}]"#,
        ),
        // ==================== PRODUCTIVITY ====================
        (
            "tpl-convertx",
            "ConvertX",
            "Self-hosted online file converter supporting 1000+ formats (images, video, audio, documents, ebooks) through FFmpeg, ImageMagick, LibreOffice, Pandoc and more. Create the account on first visit.",
            "productivity",
            "convertx",
            r#"services:
  convertx:
    image: ghcr.io/c4illin/convertx:v0.20.0
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - JWT_SECRET=${SERVICE_PASSWORD_JWT}
      - ACCOUNT_REGISTRATION=${ACCOUNT_REGISTRATION:-false}
      - HTTP_ALLOWED=${HTTP_ALLOWED:-false}
      - AUTO_DELETE_EVERY_N_HOURS=${AUTO_DELETE_EVERY_N_HOURS:-24}
      - TZ=${TZ:-UTC}
    volumes:
      - convertx_data:/app/data
    labels:
      - "rivetr.managed=true"

volumes:
  convertx_data:
"#,
            r#"[{"name":"ACCOUNT_REGISTRATION","label":"Allow Registration After First Account (true/false)","required":false,"default":"false","secret":false},{"name":"HTTP_ALLOWED","label":"Allow Plain HTTP Logins (true/false, only without a domain)","required":false,"default":"false","secret":false},{"name":"AUTO_DELETE_EVERY_N_HOURS","label":"Delete Converted Files After N Hours","required":false,"default":"24","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
        // ==================== DEVELOPMENT ====================
        (
            "tpl-unleash",
            "Unleash",
            "Open-source feature flag and feature management platform. Gradual rollouts, A/B tests, kill switches and targeting strategies with SDKs for every major language. Uses PostgreSQL. Log in as admin with the auto-generated password (see service env).",
            "development",
            "unleash",
            r#"services:
  unleash:
    image: unleashorg/unleash-server:8
    restart: unless-stopped
    ports:
      - "${PORT:-4242}:4242"
    environment:
      - DATABASE_URL=postgres://unleash:${SERVICE_PASSWORD_POSTGRES}@unleash-db:5432/unleash
      - DATABASE_SSL=false
      - LOG_LEVEL=${LOG_LEVEL:-warn}
      - UNLEASH_URL=${SERVICE_URL_UNLEASH}
      - UNLEASH_DEFAULT_ADMIN_USERNAME=admin
      - UNLEASH_DEFAULT_ADMIN_PASSWORD=${SERVICE_PASSWORD_ADMIN}
    depends_on:
      unleash-db:
        condition: service_healthy
    healthcheck:
      test: ["CMD-SHELL", "wget --no-verbose --tries=1 --spider http://localhost:4242/health || exit 1"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 30s
    labels:
      - "rivetr.managed=true"

  unleash-db:
    image: postgres:17-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=unleash
      - POSTGRES_PASSWORD=${SERVICE_PASSWORD_POSTGRES}
      - POSTGRES_DB=unleash
    volumes:
      - unleash_db_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U unleash -d unleash"]
      interval: 10s
      timeout: 5s
      retries: 10
    labels:
      - "rivetr.managed=true"

volumes:
  unleash_db_data:
"#,
            r#"[{"name":"LOG_LEVEL","label":"Log Level","required":false,"default":"warn","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"4242","secret":false}]"#,
        ),
        // ==================== BUSINESS ====================
        (
            "tpl-snipe-it",
            "Snipe-IT",
            "IT asset and license management. Track hardware, software licenses, accessories and consumables, check items in and out to users, and keep a full audit history. Uses MariaDB. Finish setup and create the admin in the web installer.",
            "business",
            "snipeit",
            r#"services:
  snipeit:
    image: snipe/snipe-it:v8.8.3
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - APP_ENV=production
      - APP_DEBUG=false
      - APP_KEY=base64:${SERVICE_BASE64_APPKEY}
      - APP_URL=${SERVICE_URL_SNIPEIT}
      - APP_TIMEZONE=${TZ:-UTC}
      - APP_LOCALE=${APP_LOCALE:-en-US}
      - APP_TRUSTED_PROXIES=*
      - DB_CONNECTION=mysql
      - DB_HOST=snipeit-db
      - DB_PORT=3306
      - DB_DATABASE=snipeit
      - DB_USERNAME=snipeit
      - DB_PASSWORD=${SERVICE_PASSWORD_MARIADB}
    volumes:
      - snipeit_data:/var/lib/snipeit
    depends_on:
      snipeit-db:
        condition: service_healthy
    labels:
      - "rivetr.managed=true"

  snipeit-db:
    image: mariadb:11.4
    restart: unless-stopped
    environment:
      - MARIADB_DATABASE=snipeit
      - MARIADB_USER=snipeit
      - MARIADB_PASSWORD=${SERVICE_PASSWORD_MARIADB}
      - MARIADB_RANDOM_ROOT_PASSWORD=1
    volumes:
      - snipeit_db_data:/var/lib/mysql
    healthcheck:
      test: ["CMD", "healthcheck.sh", "--connect", "--innodb_initialized"]
      interval: 10s
      timeout: 5s
      retries: 10
      start_period: 20s
    labels:
      - "rivetr.managed=true"

volumes:
  snipeit_data:
  snipeit_db_data:
"#,
            r#"[{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"APP_LOCALE","label":"Locale","required":false,"default":"en-US","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        // ==================== COMMUNICATION ====================
        (
            "tpl-centrifugo",
            "Centrifugo",
            "Scalable real-time messaging server. Push messages to WebSocket, SSE, HTTP-streaming and gRPC clients with channels, presence and history, published from your backend over the HTTP API. Admin UI password, API key and token secret are auto-generated (see service env).",
            "communication",
            "centrifugo",
            r#"services:
  centrifugo:
    image: centrifugo/centrifugo:v6
    restart: unless-stopped
    command: centrifugo
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - CENTRIFUGO_CLIENT_TOKEN_HMAC_SECRET_KEY=${SERVICE_PASSWORD_HMAC}
      - CENTRIFUGO_CLIENT_ALLOWED_ORIGINS=${SERVICE_URL_CENTRIFUGO}
      - CENTRIFUGO_HTTP_API_KEY=${SERVICE_PASSWORD_APIKEY}
      - CENTRIFUGO_ADMIN_ENABLED=true
      - CENTRIFUGO_ADMIN_PASSWORD=${SERVICE_PASSWORD_ADMIN}
      - CENTRIFUGO_ADMIN_SECRET=${SERVICE_PASSWORD_ADMINSECRET}
      - CENTRIFUGO_LOG_LEVEL=${LOG_LEVEL:-info}
    ulimits:
      nofile:
        soft: 65535
        hard: 65535
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"LOG_LEVEL","label":"Log Level","required":false,"default":"info","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8000","secret":false}]"#,
        ),
        // ==================== MONITORING ====================
        (
            "tpl-bugsink",
            "Bugsink",
            "Lightweight self-hosted error tracking that is compatible with Sentry SDKs. Groups exceptions into issues, shows stack traces and notifies by email or Slack, at a fraction of Sentry's footprint. Uses PostgreSQL. Log in as admin with the auto-generated password (see service env).",
            "monitoring",
            "bugsink",
            r#"services:
  bugsink:
    image: bugsink/bugsink:2
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - SECRET_KEY=${SERVICE_BASE64_64_BUGSINK}
      - CREATE_SUPERUSER=admin:${SERVICE_PASSWORD_ADMIN}
      - PORT=8000
      - BASE_URL=${SERVICE_URL_BUGSINK}
      - BEHIND_HTTPS_PROXY=true
      - DATABASE_URL=postgresql://bugsink:${SERVICE_PASSWORD_POSTGRES}@bugsink-db:5432/bugsink
    depends_on:
      bugsink-db:
        condition: service_healthy
    labels:
      - "rivetr.managed=true"

  bugsink-db:
    image: postgres:17-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=bugsink
      - POSTGRES_PASSWORD=${SERVICE_PASSWORD_POSTGRES}
      - POSTGRES_DB=bugsink
    volumes:
      - bugsink_db_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U bugsink -d bugsink"]
      interval: 10s
      timeout: 5s
      retries: 10
    labels:
      - "rivetr.managed=true"

volumes:
  bugsink_db_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8000","secret":false}]"#,
        ),
        // ==================== DEVELOPMENT ====================
        (
            "tpl-gotenberg",
            "Gotenberg",
            "Docker-powered API for converting HTML, Markdown, URLs and Office documents to PDF using Chromium and LibreOffice, plus PDF merge, split and metadata. Protected with basic auth (user api, password auto-generated, see service env).",
            "development",
            "pdf",
            r#"services:
  gotenberg:
    image: gotenberg/gotenberg:8
    restart: unless-stopped
    command:
      - gotenberg
      - --api-enable-basic-auth
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - GOTENBERG_API_BASIC_AUTH_USERNAME=api
      - GOTENBERG_API_BASIC_AUTH_PASSWORD=${SERVICE_PASSWORD_GOTENBERG}
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
        // ==================== AI ====================
        (
            "tpl-whisper-asr",
            "Whisper ASR Webservice",
            "Speech-to-text API powered by OpenAI Whisper and faster-whisper. Transcribe and translate audio in 90+ languages over a simple HTTP API with Swagger docs. CPU image; models are cached in a volume.",
            "ai",
            "whisper",
            r#"services:
  whisper-asr:
    image: onerahmet/openai-whisper-asr-webservice:v1.10.0
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
    environment:
      - ASR_MODEL=${ASR_MODEL:-base}
      - ASR_ENGINE=${ASR_ENGINE:-faster_whisper}
    volumes:
      - whisper_cache:/root/.cache
    labels:
      - "rivetr.managed=true"

volumes:
  whisper_cache:
"#,
            r#"[{"name":"ASR_MODEL","label":"Model (tiny, base, small, medium, large-v3, turbo)","required":false,"default":"base","secret":false},{"name":"ASR_ENGINE","label":"Engine (openai_whisper, faster_whisper, whisperx)","required":false,"default":"faster_whisper","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"9000","secret":false}]"#,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::super::TemplateEntry;
    use super::templates;
    use std::collections::HashSet;

    /// Every other seeder module, so ids can be checked for global uniqueness.
    fn other_templates() -> Vec<TemplateEntry> {
        use super::super::*;
        let mut all = Vec::new();
        all.extend(infrastructure::templates());
        all.extend(ai_ml::templates());
        all.extend(analytics_automation::templates());
        all.extend(cms_communication::templates());
        all.extend(devtools::templates());
        all.extend(documentation::templates());
        all.extend(media_monitoring::templates());
        all.extend(media_productivity::templates());
        all.extend(security_search::templates());
        all.extend(project_mgmt::templates());
        all.extend(extra_services::templates());
        all.extend(ai_extras::templates());
        all.extend(auth_identity::templates());
        all.extend(business::templates());
        all.extend(cms_extra::templates());
        all.extend(communication_extra::templates());
        all.extend(databases_tools::templates());
        all.extend(devtools_extra::templates());
        all.extend(misc_extras::templates());
        all.extend(monitoring_extra::templates());
        all.extend(networking_extra::templates());
        all.extend(sprint15::templates());
        all.extend(sprint16::templates());
        all.extend(sprint18::templates());
        all.extend(sprint19::templates());
        all.extend(sprint20::templates());
        all.extend(sprint21::templates());
        all.extend(sprint22::templates());
        all.extend(sprint23::templates());
        all.extend(sprint24::templates());
        all.extend(sprint25::templates());
        all.extend(sprint26::templates());
        all
    }

    fn normalize(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    }

    #[test]
    fn sprint27_has_at_least_25_templates() {
        assert!(templates().len() >= 25);
    }

    #[test]
    fn sprint27_ids_and_names_are_unique_globally() {
        let others = other_templates();
        let other_ids: HashSet<&str> = others.iter().map(|t| t.0).collect();
        let other_names: HashSet<String> = others.iter().map(|t| normalize(t.1)).collect();

        let mut seen_ids = HashSet::new();
        let mut seen_names = HashSet::new();
        for (id, name, ..) in templates() {
            assert!(id.starts_with("tpl-"), "{id}: id must start with tpl-");
            assert!(seen_ids.insert(id), "{id}: duplicate id within sprint27");
            assert!(
                seen_names.insert(normalize(name)),
                "{name}: duplicate name within sprint27"
            );
            assert!(
                !other_ids.contains(id),
                "{id}: id already used by another seeder"
            );
            assert!(
                !other_names.contains(&normalize(name)),
                "{name}: template name already used by another seeder"
            );
        }
    }

    #[test]
    fn sprint27_compose_and_env_schema_are_valid() {
        for (id, name, description, category, icon, compose, env_schema) in templates() {
            assert!(!name.is_empty(), "{id}: empty name");
            assert!(!description.is_empty(), "{id}: empty description");
            assert!(!category.is_empty(), "{id}: empty category");
            assert!(!icon.is_empty(), "{id}: empty icon");
            assert!(
                !description.contains('\u{2014}') && !description.contains('\u{2013}'),
                "{id}: description must not use em or en dashes"
            );

            let yaml: serde_yaml::Value = serde_yaml::from_str(compose)
                .unwrap_or_else(|e| panic!("{id}: invalid compose YAML: {e}"));
            let services = yaml
                .get("services")
                .and_then(|s| s.as_mapping())
                .unwrap_or_else(|| panic!("{id}: missing services mapping"));
            assert!(!services.is_empty(), "{id}: services mapping is empty");

            for (svc_name, svc) in services {
                let svc_name = svc_name.as_str().unwrap_or("?");
                assert!(
                    svc.get("container_name").is_none(),
                    "{id}/{svc_name}: container_name must not be set"
                );
                let image = svc
                    .get("image")
                    .and_then(|i| i.as_str())
                    .unwrap_or_else(|| panic!("{id}/{svc_name}: missing image"));
                let last_segment = image.rsplit('/').next().unwrap_or(image);
                let tag = last_segment
                    .split_once(':')
                    .map(|(_, t)| t)
                    .unwrap_or_else(|| panic!("{id}/{svc_name}: image {image} has no tag"));
                assert!(
                    !tag.is_empty() && tag != "latest" && !tag.contains("${"),
                    "{id}/{svc_name}: image {image} must use a pinned tag"
                );
            }

            let schema: serde_json::Value = serde_json::from_str(env_schema)
                .unwrap_or_else(|e| panic!("{id}: invalid env_schema JSON: {e}"));
            let entries = schema
                .as_array()
                .unwrap_or_else(|| panic!("{id}: env_schema must be a JSON array"));
            for entry in entries {
                let var = entry
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_else(|| panic!("{id}: env_schema entry without name"));
                assert!(
                    entry.get("label").and_then(|l| l.as_str()).is_some(),
                    "{id}: env_schema entry {var} without label"
                );
                assert!(
                    compose.contains(&format!("${{{var}")),
                    "{id}: env_schema var {var} is not used in the compose template"
                );
                // The deploy path substitutes `${VAR:-default}` only when the default in
                // the compose file matches the schema default exactly.
                let default = entry.get("default").and_then(|d| d.as_str()).unwrap_or("");
                let prefix = format!("${{{var}:-");
                let mut rest = compose;
                while let Some(pos) = rest.find(&prefix) {
                    let after = &rest[pos + prefix.len()..];
                    let end = after.find('}').expect("unterminated variable");
                    assert_eq!(
                        &after[..end],
                        default,
                        "{id}: compose default for {var} differs from env_schema default"
                    );
                    rest = &after[end..];
                }
            }
        }
    }
}
