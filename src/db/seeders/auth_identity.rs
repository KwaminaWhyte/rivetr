//! Identity, authentication, and authorization service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== IDENTITY / AUTH ====================
        (
            "tpl-authelia",
            "Authelia",
            "Open-source authentication and authorization server. Two-factor auth, SSO, and access control proxy.",
            "security",
            "authelia",
            r#"services:
  authelia:
    image: authelia/authelia:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-authelia}
    restart: unless-stopped
    ports:
      - "${PORT:-9091}:9091"
    environment:
      - AUTHELIA_JWT_SECRET=${JWT_SECRET:-change-me-to-a-random-string}
      - AUTHELIA_SESSION_SECRET=${SESSION_SECRET:-change-me-to-a-random-string}
      - AUTHELIA_STORAGE_ENCRYPTION_KEY=${STORAGE_KEY:-change-me-to-a-32-char-key}
      - AUTHELIA_STORAGE_LOCAL_PATH=/config/db.sqlite3
    volumes:
      - authelia_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  authelia_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"authelia","secret":false},{"name":"PORT","label":"Port","required":false,"default":"9091","secret":false},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true},{"name":"SESSION_SECRET","label":"Session Secret","required":true,"default":"","secret":true},{"name":"STORAGE_KEY","label":"Storage Encryption Key (32+ chars)","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-zitadel",
            "ZITADEL",
            "Cloud-native identity and access management platform. OIDC, OAuth 2, SAML, and passwordless auth.",
            "security",
            "zitadel",
            r#"services:
  zitadel:
    image: ghcr.io/zitadel/zitadel:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-zitadel}
    restart: unless-stopped
    command: start-from-init --masterkey "${MASTER_KEY:-MasterkeyNeedsToHave32Characters}" --tlsMode disabled
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - ZITADEL_DATABASE_POSTGRES_HOST=zitadel_db
      - ZITADEL_DATABASE_POSTGRES_PORT=5432
      - ZITADEL_DATABASE_POSTGRES_DATABASE=zitadel
      - ZITADEL_DATABASE_POSTGRES_USER_USERNAME=zitadel
      - ZITADEL_DATABASE_POSTGRES_USER_PASSWORD=${DB_PASSWORD:-zitadel}
      - ZITADEL_DATABASE_POSTGRES_USER_SSL_MODE=disable
      - ZITADEL_DATABASE_POSTGRES_ADMIN_USERNAME=postgres
      - ZITADEL_DATABASE_POSTGRES_ADMIN_PASSWORD=${DB_PASSWORD:-zitadel}
      - ZITADEL_DATABASE_POSTGRES_ADMIN_SSL_MODE=disable
      - ZITADEL_EXTERNALSECURE=false
      - ZITADEL_EXTERNALPORT=${PORT:-8080}
      - ZITADEL_EXTERNALDOMAIN=${EXTERNAL_DOMAIN:-localhost}
    depends_on:
      - zitadel_db
    labels:
      - "rivetr.managed=true"

  zitadel_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=${DB_PASSWORD:-zitadel}
      - POSTGRES_DB=zitadel
    volumes:
      - zitadel_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  zitadel_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"zitadel","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"MASTER_KEY","label":"Master Key (32 chars)","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"EXTERNAL_DOMAIN","label":"External Domain","required":false,"default":"localhost","secret":false}]"#,
        ),
        (
            "tpl-casdoor",
            "Casdoor",
            "Open-source Identity and Access Management (IAM) / Single Sign-On (SSO) platform.",
            "security",
            "casdoor",
            r#"services:
  casdoor:
    image: casbin/casdoor:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-casdoor}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - httpport=8000
      - driverName=postgres
      - dataSourceName=user=casdoor password=${DB_PASSWORD:-casdoor} host=casdoor_db port=5432 sslmode=disable dbname=casdoor
      - dbName=casdoor
      - runmode=dev
    depends_on:
      - casdoor_db
    labels:
      - "rivetr.managed=true"

  casdoor_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=casdoor
      - POSTGRES_PASSWORD=${DB_PASSWORD:-casdoor}
      - POSTGRES_DB=casdoor
    volumes:
      - casdoor_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  casdoor_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"casdoor","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-logto",
            "Logto",
            "Modern identity infrastructure for developers. OIDC-based auth with intuitive admin console.",
            "security",
            "logto",
            r#"services:
  logto:
    image: svhd/logto:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-logto}
    restart: unless-stopped
    ports:
      - "${PORT:-3001}:3001"
      - "${ADMIN_PORT:-3002}:3002"
    environment:
      - DB_URL=postgresql://logto:${DB_PASSWORD:-logto}@logto_db:5432/logto
      - TRUST_PROXY_HEADER=1
      - ENDPOINT=${ENDPOINT:-http://localhost:3001}
      - ADMIN_ENDPOINT=${ADMIN_ENDPOINT:-http://localhost:3002}
    depends_on:
      - logto_db
    labels:
      - "rivetr.managed=true"

  logto_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=logto
      - POSTGRES_PASSWORD=${DB_PASSWORD:-logto}
      - POSTGRES_DB=logto
    volumes:
      - logto_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  logto_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"logto","secret":false},{"name":"PORT","label":"App Port","required":false,"default":"3001","secret":false},{"name":"ADMIN_PORT","label":"Admin Port","required":false,"default":"3002","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"ENDPOINT","label":"App Endpoint URL","required":false,"default":"http://localhost:3001","secret":false},{"name":"ADMIN_ENDPOINT","label":"Admin Endpoint URL","required":false,"default":"http://localhost:3002","secret":false}]"#,
        ),
        (
            "tpl-ory-kratos",
            "Ory Kratos",
            "Cloud-native, headless user management and identity server. Handles registration, login, and MFA.",
            "security",
            "ory-kratos",
            r#"services:
  kratos:
    image: oryd/kratos:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-kratos}
    restart: unless-stopped
    ports:
      - "${PUBLIC_PORT:-4433}:4433"
      - "${ADMIN_PORT:-4434}:4434"
    environment:
      - DSN=postgres://kratos:${DB_PASSWORD:-kratos}@kratos_db:5432/kratos?sslmode=disable
      - KRATOS_ADMIN_BASE_URL=http://kratos:4434
    command: serve --config /etc/config/kratos/kratos.yml --dev --watch-courier
    volumes:
      - kratos_config:/etc/config/kratos
    depends_on:
      - kratos_db
    labels:
      - "rivetr.managed=true"

  kratos_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=kratos
      - POSTGRES_PASSWORD=${DB_PASSWORD:-kratos}
      - POSTGRES_DB=kratos
    volumes:
      - kratos_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  kratos_config:
  kratos_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"kratos","secret":false},{"name":"PUBLIC_PORT","label":"Public API Port","required":false,"default":"4433","secret":false},{"name":"ADMIN_PORT","label":"Admin API Port","required":false,"default":"4434","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
    ]
}
