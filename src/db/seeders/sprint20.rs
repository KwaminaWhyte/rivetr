//! Sprint 20 service templates: auth/security, monitoring (unique templates only)
//! NOTE: Duplicates removed — tpl-pocketbase, tpl-appwrite, tpl-directus, tpl-strapi,
//! tpl-outline, tpl-authelia, tpl-zitadel, tpl-rocketchat, tpl-jitsi-meet, tpl-mautic,
//! tpl-limesurvey, tpl-twenty-crm, tpl-netdata, tpl-checkmk, tpl-minio, tpl-photoprism
//! already exist in earlier seeder files.

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== SECURITY / AUTH ====================
        (
            "tpl-authentik",
            "Authentik",
            "Open-source identity provider supporting SSO, OAuth2, SAML, LDAP, and more. Drop-in replacement for many IdPs.",
            "security",
            "authentik",
            r#"services:
  authentik-server:
    image: ghcr.io/goauthentik/server:2024.2
    container_name: ${CONTAINER_NAME:-authentik}-server
    restart: unless-stopped
    command: server
    ports:
      - "${PORT:-9000}:9000"
      - "${HTTPS_PORT:-9443}:9443"
    depends_on:
      - authentik-postgresql
      - authentik-redis
    environment:
      - AUTHENTIK_SECRET_KEY=${SERVICE_PASSWORD_SECRET}
      - AUTHENTIK_REDIS__HOST=authentik-redis
      - AUTHENTIK_POSTGRESQL__HOST=authentik-postgresql
      - AUTHENTIK_POSTGRESQL__USER=authentik
      - AUTHENTIK_POSTGRESQL__PASSWORD=${SERVICE_PASSWORD_DB}
      - AUTHENTIK_POSTGRESQL__NAME=authentik
    volumes:
      - authentik_media:/media
      - authentik_templates:/templates
    labels:
      - "rivetr.managed=true"

  authentik-worker:
    image: ghcr.io/goauthentik/server:2024.2
    container_name: ${CONTAINER_NAME:-authentik}-worker
    restart: unless-stopped
    command: worker
    depends_on:
      - authentik-postgresql
      - authentik-redis
    environment:
      - AUTHENTIK_SECRET_KEY=${SERVICE_PASSWORD_SECRET}
      - AUTHENTIK_REDIS__HOST=authentik-redis
      - AUTHENTIK_POSTGRESQL__HOST=authentik-postgresql
      - AUTHENTIK_POSTGRESQL__USER=authentik
      - AUTHENTIK_POSTGRESQL__PASSWORD=${SERVICE_PASSWORD_DB}
      - AUTHENTIK_POSTGRESQL__NAME=authentik
    volumes:
      - authentik_media:/media
      - authentik_templates:/templates
      - /var/run/docker.sock:/var/run/docker.sock
    labels:
      - "rivetr.managed=true"

  authentik-postgresql:
    image: postgres:15-alpine
    container_name: ${CONTAINER_NAME:-authentik}-postgresql
    restart: unless-stopped
    environment:
      - POSTGRES_DB=authentik
      - POSTGRES_USER=authentik
      - POSTGRES_PASSWORD=${SERVICE_PASSWORD_DB}
    volumes:
      - authentik_postgres:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  authentik-redis:
    image: redis:7-alpine
    container_name: ${CONTAINER_NAME:-authentik}-redis
    restart: unless-stopped
    volumes:
      - authentik_redis:/data
    labels:
      - "rivetr.managed=true"

volumes:
  authentik_media:
  authentik_templates:
  authentik_postgres:
  authentik_redis:
"#,
            r#"[{"name":"SERVICE_PASSWORD_SECRET","label":"Secret Key","required":true,"default":"","secret":true},{"name":"SERVICE_PASSWORD_DB","label":"Database Password","required":true,"default":"","secret":true},{"name":"PORT","label":"HTTP Port","required":false,"default":"9000","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"9443","secret":false}]"#,
        ),
        (
            "tpl-infisical",
            "Infisical",
            "Open-source secrets manager: store, sync, and distribute API keys, database credentials, and configs across your team.",
            "security",
            "infisical",
            r#"services:
  infisical:
    image: infisical/infisical:latest
    container_name: ${CONTAINER_NAME:-infisical}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:8080"
    depends_on:
      - infisical-mongo
    environment:
      - ENCRYPTION_KEY=${SERVICE_PASSWORD_ENC}
      - JWT_SIGNUP_SECRET=${SERVICE_BASE64_JWT}
      - JWT_REFRESH_SECRET=${SERVICE_BASE64_REFRESH}
      - JWT_AUTH_SECRET=${SERVICE_BASE64_AUTH}
      - JWT_SERVICE_SECRET=${SERVICE_BASE64_SERVICE}
      - MONGO_URL=mongodb://infisical:${SERVICE_PASSWORD_DB}@infisical-mongo:27017/infisical
      - SITE_URL=${SITE_URL:-http://localhost}
    labels:
      - "rivetr.managed=true"

  infisical-mongo:
    image: mongo:6
    container_name: ${CONTAINER_NAME:-infisical}-mongo
    restart: unless-stopped
    environment:
      - MONGO_INITDB_ROOT_USERNAME=infisical
      - MONGO_INITDB_ROOT_PASSWORD=${SERVICE_PASSWORD_DB}
      - MONGO_INITDB_DATABASE=infisical
    volumes:
      - infisical_mongo:/data/db
    labels:
      - "rivetr.managed=true"

volumes:
  infisical_mongo:
"#,
            r#"[{"name":"SERVICE_PASSWORD_ENC","label":"Encryption Key","required":true,"default":"","secret":true},{"name":"SERVICE_PASSWORD_DB","label":"MongoDB Password","required":true,"default":"","secret":true},{"name":"SERVICE_BASE64_JWT","label":"JWT Signup Secret","required":true,"default":"","secret":true},{"name":"SERVICE_BASE64_REFRESH","label":"JWT Refresh Secret","required":true,"default":"","secret":true},{"name":"SERVICE_BASE64_AUTH","label":"JWT Auth Secret","required":true,"default":"","secret":true},{"name":"SERVICE_BASE64_SERVICE","label":"JWT Service Secret","required":true,"default":"","secret":true},{"name":"SITE_URL","label":"Site URL","required":false,"default":"http://localhost","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false}]"#,
        ),
        // ==================== MONITORING ====================
        (
            "tpl-victoriametrics",
            "VictoriaMetrics",
            "Fast, cost-effective time series database and monitoring solution. Drop-in replacement for Prometheus.",
            "monitoring",
            "victoriametrics",
            r#"services:
  victoriametrics:
    image: victoriametrics/victoria-metrics:latest
    container_name: ${CONTAINER_NAME:-victoriametrics}
    restart: unless-stopped
    ports:
      - "${PORT:-8428}:8428"
    command:
      - "--retentionPeriod=${RETENTION_PERIOD:-1}"
      - "--storageDataPath=/victoria-metrics-data"
    volumes:
      - vm_data:/victoria-metrics-data
    labels:
      - "rivetr.managed=true"

volumes:
  vm_data:
"#,
            r#"[{"name":"RETENTION_PERIOD","label":"Retention Period (months)","required":false,"default":"1","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8428","secret":false}]"#,
        ),
    ]
}
