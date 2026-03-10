//! Batch 2 security and search service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
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
    ]
}
