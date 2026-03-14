//! Sprint 20 service templates: development platforms, auth/security, communication, monitoring, storage, media

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== DEVELOPMENT ====================
        (
            "tpl-pocketbase",
            "PocketBase",
            "Open-source backend in a single file: embedded database, auth, file storage, and realtime subscriptions.",
            "development",
            "pocketbase",
            r#"services:
  pocketbase:
    image: ghcr.io/muchobien/pocketbase:latest
    container_name: ${CONTAINER_NAME:-pocketbase}
    restart: unless-stopped
    ports:
      - "${PORT:-8090}:8090"
    volumes:
      - pb_data:/pb_data
    labels:
      - "rivetr.managed=true"

volumes:
  pb_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8090","secret":false}]"#,
        ),
        (
            "tpl-appwrite",
            "Appwrite",
            "End-to-end backend server with auth, database, storage, functions, and real-time subscriptions. Firebase alternative.",
            "development",
            "appwrite",
            r#"services:
  appwrite:
    image: appwrite/appwrite:1.5.7
    container_name: ${CONTAINER_NAME:-appwrite}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    depends_on:
      - appwrite-mariadb
      - appwrite-redis
    environment:
      - _APP_ENV=production
      - _APP_OPENSSL_KEY_V1=${SERVICE_PASSWORD_OPENSSL}
      - _APP_DOMAIN=${APP_DOMAIN:-localhost}
      - _APP_DOMAIN_TARGET=${APP_DOMAIN_TARGET:-localhost}
      - _APP_DB_HOST=appwrite-mariadb
      - _APP_DB_USER=appwrite
      - _APP_DB_PASS=${SERVICE_PASSWORD_DB}
      - _APP_DB_SCHEMA=appwrite
      - _APP_REDIS_HOST=appwrite-redis
      - _APP_REDIS_PORT=6379
    volumes:
      - appwrite_uploads:/storage/uploads
      - appwrite_cache:/storage/cache
      - appwrite_config:/storage/config
      - appwrite_certificates:/storage/certificates
      - appwrite_functions:/storage/functions
    labels:
      - "rivetr.managed=true"

  appwrite-mariadb:
    image: mariadb:10.7
    container_name: ${CONTAINER_NAME:-appwrite}-mariadb
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${SERVICE_PASSWORD_ROOT}
      - MYSQL_DATABASE=appwrite
      - MYSQL_USER=appwrite
      - MYSQL_PASSWORD=${SERVICE_PASSWORD_DB}
    volumes:
      - appwrite_mariadb:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

  appwrite-redis:
    image: redis:7-alpine
    container_name: ${CONTAINER_NAME:-appwrite}-redis
    restart: unless-stopped
    volumes:
      - appwrite_redis:/data
    labels:
      - "rivetr.managed=true"

volumes:
  appwrite_uploads:
  appwrite_cache:
  appwrite_config:
  appwrite_certificates:
  appwrite_functions:
  appwrite_mariadb:
  appwrite_redis:
"#,
            r#"[{"name":"APP_DOMAIN","label":"App Domain","required":true,"default":"localhost","secret":false},{"name":"APP_DOMAIN_TARGET","label":"App Domain Target","required":true,"default":"localhost","secret":false},{"name":"SERVICE_PASSWORD_OPENSSL","label":"OpenSSL Key","required":true,"default":"","secret":true},{"name":"SERVICE_PASSWORD_DB","label":"Database Password","required":true,"default":"","secret":true},{"name":"SERVICE_PASSWORD_ROOT","label":"MariaDB Root Password","required":true,"default":"","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false}]"#,
        ),
        (
            "tpl-directus",
            "Directus",
            "Open-source headless CMS and data platform. Wrap any SQL database with a dynamic REST and GraphQL API.",
            "development",
            "directus",
            r#"services:
  directus:
    image: directus/directus:latest
    container_name: ${CONTAINER_NAME:-directus}
    restart: unless-stopped
    ports:
      - "${PORT:-8055}:8055"
    environment:
      - KEY=${SERVICE_PASSWORD_KEY}
      - SECRET=${SERVICE_PASSWORD_SECRET}
      - DB_CLIENT=sqlite3
      - DB_FILENAME=/directus/database/data.db
      - ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD}
    volumes:
      - directus_database:/directus/database
      - directus_uploads:/directus/uploads
      - directus_extensions:/directus/extensions
    labels:
      - "rivetr.managed=true"

volumes:
  directus_database:
  directus_uploads:
  directus_extensions:
"#,
            r#"[{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"SERVICE_PASSWORD_KEY","label":"App Key","required":true,"default":"","secret":true},{"name":"SERVICE_PASSWORD_SECRET","label":"App Secret","required":true,"default":"","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8055","secret":false}]"#,
        ),
        (
            "tpl-strapi",
            "Strapi",
            "Leading open-source headless CMS. Fully customizable with a plugin ecosystem. Build APIs fast.",
            "development",
            "strapi",
            r#"services:
  strapi:
    image: strapi/strapi:latest
    container_name: ${CONTAINER_NAME:-strapi}
    restart: unless-stopped
    ports:
      - "${PORT:-1337}:1337"
    environment:
      - DATABASE_CLIENT=sqlite
      - DATABASE_FILENAME=.tmp/data.db
      - APP_KEYS=${SERVICE_BASE64_KEYS}
      - API_TOKEN_SALT=${SERVICE_PASSWORD_API_SALT}
      - ADMIN_JWT_SECRET=${SERVICE_PASSWORD_ADMIN_JWT}
      - JWT_SECRET=${SERVICE_PASSWORD_JWT}
    volumes:
      - strapi_data:/opt/app/.tmp
      - strapi_uploads:/opt/app/public/uploads
    labels:
      - "rivetr.managed=true"

volumes:
  strapi_data:
  strapi_uploads:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"1337","secret":false}]"#,
        ),
        (
            "tpl-outline",
            "Outline",
            "Open-source team wiki and knowledge base. Beautiful markdown editor, intuitive organization, powerful search.",
            "development",
            "outline",
            r#"services:
  outline:
    image: outlinewiki/outline:latest
    container_name: ${CONTAINER_NAME:-outline}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - SECRET_KEY=${SERVICE_BASE64_SECRET}
      - UTILS_SECRET=${SERVICE_BASE64_UTILS}
      - DATABASE_URL=sqlite://db.sqlite
      - FILE_STORAGE=local
      - FILE_STORAGE_LOCAL_ROOT_DIR=/var/lib/outline/data
      - URL=${OUTLINE_URL:-http://localhost:3000}
    volumes:
      - outline_data:/var/lib/outline/data
    labels:
      - "rivetr.managed=true"

volumes:
  outline_data:
"#,
            r#"[{"name":"OUTLINE_URL","label":"Public URL","required":true,"default":"http://localhost:3000","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
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
            "tpl-authelia",
            "Authelia",
            "Open-source authentication and authorization server providing 2FA and SSO for your infrastructure.",
            "security",
            "authelia",
            r#"services:
  authelia:
    image: authelia/authelia:latest
    container_name: ${CONTAINER_NAME:-authelia}
    restart: unless-stopped
    ports:
      - "${PORT:-9091}:9091"
    environment:
      - AUTHELIA_JWT_SECRET=${SERVICE_PASSWORD_JWT}
      - AUTHELIA_SESSION_SECRET=${SERVICE_PASSWORD_SESSION}
      - AUTHELIA_STORAGE_ENCRYPTION_KEY=${SERVICE_PASSWORD_ENCRYPTION}
    volumes:
      - authelia_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  authelia_config:
"#,
            r#"[{"name":"SERVICE_PASSWORD_JWT","label":"JWT Secret","required":true,"default":"","secret":true},{"name":"SERVICE_PASSWORD_SESSION","label":"Session Secret","required":true,"default":"","secret":true},{"name":"SERVICE_PASSWORD_ENCRYPTION","label":"Storage Encryption Key","required":true,"default":"","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"9091","secret":false}]"#,
        ),
        (
            "tpl-zitadel",
            "Zitadel",
            "Cloud-native identity platform: SSO, OIDC, OAuth2, SAML, passkeys, and more. Multi-tenant ready.",
            "security",
            "zitadel",
            r#"services:
  zitadel:
    image: ghcr.io/zitadel/zitadel:latest
    container_name: ${CONTAINER_NAME:-zitadel}
    restart: unless-stopped
    command: start-from-init --masterkey "${SERVICE_PASSWORD_MASTERKEY}" --tlsMode disabled
    ports:
      - "${PORT:-8080}:8080"
    depends_on:
      - zitadel-db
    environment:
      - ZITADEL_DATABASE_POSTGRES_HOST=zitadel-db
      - ZITADEL_DATABASE_POSTGRES_PORT=5432
      - ZITADEL_DATABASE_POSTGRES_DATABASE=zitadel
      - ZITADEL_DATABASE_POSTGRES_USER_USERNAME=zitadel
      - ZITADEL_DATABASE_POSTGRES_USER_PASSWORD=${SERVICE_PASSWORD_DB}
      - ZITADEL_DATABASE_POSTGRES_USER_SSL_MODE=disable
      - ZITADEL_DATABASE_POSTGRES_ADMIN_USERNAME=postgres
      - ZITADEL_DATABASE_POSTGRES_ADMIN_PASSWORD=${SERVICE_PASSWORD_DB}
      - ZITADEL_DATABASE_POSTGRES_ADMIN_SSL_MODE=disable
      - ZITADEL_EXTERNALDOMAIN=${EXTERNAL_DOMAIN:-localhost}
      - ZITADEL_EXTERNALPORT=${PORT:-8080}
      - ZITADEL_EXTERNALSECURE=false
    labels:
      - "rivetr.managed=true"

  zitadel-db:
    image: postgres:15-alpine
    container_name: ${CONTAINER_NAME:-zitadel}-db
    restart: unless-stopped
    environment:
      - POSTGRES_DB=zitadel
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=${SERVICE_PASSWORD_DB}
    volumes:
      - zitadel_postgres:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  zitadel_postgres:
"#,
            r#"[{"name":"SERVICE_PASSWORD_MASTERKEY","label":"Master Key (32 chars)","required":true,"default":"","secret":true},{"name":"SERVICE_PASSWORD_DB","label":"Database Password","required":true,"default":"","secret":true},{"name":"EXTERNAL_DOMAIN","label":"External Domain","required":false,"default":"localhost","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
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
        // ==================== COMMUNICATION ====================
        (
            "tpl-rocketchat",
            "Rocket.Chat",
            "Open-source communications platform for team collaboration: messaging, video, file sharing, and more.",
            "communication",
            "rocketchat",
            r#"services:
  rocketchat:
    image: rocket.chat:latest
    container_name: ${CONTAINER_NAME:-rocketchat}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    depends_on:
      - rocketchat-mongo
    environment:
      - ROOT_URL=${ROOT_URL:-http://localhost:3000}
      - PORT=3000
      - MONGO_URL=mongodb://rocketchat:${SERVICE_PASSWORD_DB}@rocketchat-mongo:27017/rocketchat?authSource=admin
      - MONGO_OPLOG_URL=mongodb://rocketchat:${SERVICE_PASSWORD_DB}@rocketchat-mongo:27017/local?authSource=admin
      - DEPLOY_METHOD=docker
      - DEPLOY_PLATFORM=rivetr
    labels:
      - "rivetr.managed=true"

  rocketchat-mongo:
    image: mongo:6
    container_name: ${CONTAINER_NAME:-rocketchat}-mongo
    restart: unless-stopped
    command: mongod --oplogSize 128 --replSet rs0
    environment:
      - MONGO_INITDB_ROOT_USERNAME=rocketchat
      - MONGO_INITDB_ROOT_PASSWORD=${SERVICE_PASSWORD_DB}
    volumes:
      - rocketchat_mongo:/data/db
    labels:
      - "rivetr.managed=true"

  rocketchat-mongo-init:
    image: mongo:6
    restart: "no"
    depends_on:
      - rocketchat-mongo
    command: >
      bash -c "until mongo --host rocketchat-mongo:27017 -u rocketchat -p ${SERVICE_PASSWORD_DB} --authenticationDatabase admin --eval 'rs.initiate()'; do sleep 2; done"
    labels:
      - "rivetr.managed=true"

volumes:
  rocketchat_mongo:
"#,
            r#"[{"name":"ROOT_URL","label":"Root URL","required":true,"default":"http://localhost:3000","secret":false},{"name":"SERVICE_PASSWORD_DB","label":"MongoDB Password","required":true,"default":"","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
        (
            "tpl-jitsi-meet",
            "Jitsi Meet",
            "Open-source video conferencing platform. Host your own video calls without third-party services.",
            "communication",
            "jitsi",
            r#"services:
  jitsi-web:
    image: jitsi/web:latest
    container_name: ${CONTAINER_NAME:-jitsi}-web
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
      - "${HTTPS_PORT:-8443}:443"
    environment:
      - PUBLIC_URL=${PUBLIC_URL:-http://localhost}
      - ENABLE_AUTH=${ENABLE_AUTH:-0}
      - ENABLE_GUESTS=${ENABLE_GUESTS:-1}
      - JICOFO_AUTH_PASSWORD=${SERVICE_PASSWORD_JICOFO}
      - JVB_AUTH_PASSWORD=${SERVICE_PASSWORD_JVB}
      - JIGASI_XMPP_PASSWORD=${SERVICE_PASSWORD_JIGASI}
      - JIBRI_RECORDER_PASSWORD=${SERVICE_PASSWORD_JIBRI_REC}
      - JIBRI_XMPP_PASSWORD=${SERVICE_PASSWORD_JIBRI}
    volumes:
      - jitsi_web:/config
    labels:
      - "rivetr.managed=true"

  jitsi-prosody:
    image: jitsi/prosody:latest
    container_name: ${CONTAINER_NAME:-jitsi}-prosody
    restart: unless-stopped
    environment:
      - PUBLIC_URL=${PUBLIC_URL:-http://localhost}
      - JICOFO_AUTH_PASSWORD=${SERVICE_PASSWORD_JICOFO}
      - JVB_AUTH_PASSWORD=${SERVICE_PASSWORD_JVB}
      - JIGASI_XMPP_PASSWORD=${SERVICE_PASSWORD_JIGASI}
      - JIBRI_RECORDER_PASSWORD=${SERVICE_PASSWORD_JIBRI_REC}
      - JIBRI_XMPP_PASSWORD=${SERVICE_PASSWORD_JIBRI}
    volumes:
      - jitsi_prosody:/config
    labels:
      - "rivetr.managed=true"

  jitsi-jicofo:
    image: jitsi/jicofo:latest
    container_name: ${CONTAINER_NAME:-jitsi}-jicofo
    restart: unless-stopped
    depends_on:
      - jitsi-prosody
    environment:
      - PUBLIC_URL=${PUBLIC_URL:-http://localhost}
      - JICOFO_AUTH_PASSWORD=${SERVICE_PASSWORD_JICOFO}
    labels:
      - "rivetr.managed=true"

  jitsi-jvb:
    image: jitsi/jvb:latest
    container_name: ${CONTAINER_NAME:-jitsi}-jvb
    restart: unless-stopped
    depends_on:
      - jitsi-prosody
    environment:
      - PUBLIC_URL=${PUBLIC_URL:-http://localhost}
      - JVB_AUTH_PASSWORD=${SERVICE_PASSWORD_JVB}
      - JVB_ADVERTISE_IPS=${JVB_ADVERTISE_IPS:-}
    volumes:
      - jitsi_jvb:/config
    labels:
      - "rivetr.managed=true"

volumes:
  jitsi_web:
  jitsi_prosody:
  jitsi_jvb:
"#,
            r#"[{"name":"PUBLIC_URL","label":"Public URL","required":true,"default":"http://localhost","secret":false},{"name":"ENABLE_AUTH","label":"Enable Authentication (0/1)","required":false,"default":"0","secret":false},{"name":"JVB_ADVERTISE_IPS","label":"JVB Advertise IPs (comma-separated)","required":false,"default":"","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"8443","secret":false}]"#,
        ),
        (
            "tpl-mautic",
            "Mautic",
            "Open-source marketing automation: email campaigns, lead management, drip campaigns, and analytics.",
            "communication",
            "mautic",
            r#"services:
  mautic:
    image: mautic/mautic:latest
    container_name: ${CONTAINER_NAME:-mautic}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    depends_on:
      - mautic-mysql
    environment:
      - MAUTIC_DB_HOST=mautic-mysql
      - MAUTIC_DB_USER=mautic
      - MAUTIC_DB_PASSWORD=${SERVICE_PASSWORD_DB}
      - MAUTIC_DB_NAME=mautic
      - MAUTIC_TRUSTED_HOSTS=.*
    volumes:
      - mautic_data:/var/www/html
    labels:
      - "rivetr.managed=true"

  mautic-mysql:
    image: mysql:8.0
    container_name: ${CONTAINER_NAME:-mautic}-mysql
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${SERVICE_PASSWORD_ROOT}
      - MYSQL_DATABASE=mautic
      - MYSQL_USER=mautic
      - MYSQL_PASSWORD=${SERVICE_PASSWORD_DB}
    volumes:
      - mautic_mysql:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  mautic_data:
  mautic_mysql:
"#,
            r#"[{"name":"SERVICE_PASSWORD_DB","label":"Database Password","required":true,"default":"","secret":true},{"name":"SERVICE_PASSWORD_ROOT","label":"MySQL Root Password","required":true,"default":"","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false}]"#,
        ),
        // ==================== PRODUCTIVITY ====================
        (
            "tpl-limesurvey",
            "LimeSurvey",
            "Professional open-source survey tool. Create complex surveys with skip logic, quotas, and detailed analytics.",
            "cms",
            "limesurvey",
            r#"services:
  limesurvey:
    image: martialblog/limesurvey:latest
    container_name: ${CONTAINER_NAME:-limesurvey}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    depends_on:
      - limesurvey-mysql
    environment:
      - DB_TYPE=mysql
      - DB_HOST=limesurvey-mysql
      - DB_PORT=3306
      - DB_NAME=limesurvey
      - DB_USERNAME=limesurvey
      - DB_PASSWORD=${SERVICE_PASSWORD_DB}
      - ADMIN_USER=${ADMIN_USER:-admin}
      - ADMIN_NAME=${ADMIN_NAME:-Admin}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD}
      - ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
    volumes:
      - limesurvey_data:/var/www/html/upload
    labels:
      - "rivetr.managed=true"

  limesurvey-mysql:
    image: mysql:8.0
    container_name: ${CONTAINER_NAME:-limesurvey}-mysql
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${SERVICE_PASSWORD_ROOT}
      - MYSQL_DATABASE=limesurvey
      - MYSQL_USER=limesurvey
      - MYSQL_PASSWORD=${SERVICE_PASSWORD_DB}
    volumes:
      - limesurvey_mysql:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  limesurvey_data:
  limesurvey_mysql:
"#,
            r#"[{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_NAME","label":"Admin Display Name","required":false,"default":"Admin","secret":false},{"name":"SERVICE_PASSWORD_DB","label":"Database Password","required":true,"default":"","secret":true},{"name":"SERVICE_PASSWORD_ROOT","label":"MySQL Root Password","required":true,"default":"","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-twenty-crm",
            "Twenty CRM",
            "Modern open-source CRM: relationship management, deal tracking, and contact enrichment. Notion-like experience.",
            "cms",
            "twentycrm",
            r#"services:
  twenty:
    image: twentycrm/twenty:latest
    container_name: ${CONTAINER_NAME:-twenty}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    depends_on:
      - twenty-db
      - twenty-redis
    environment:
      - SERVER_URL=${SERVER_URL:-http://localhost:3000}
      - FRONT_AUTH_ENABLED=${FRONT_AUTH_ENABLED:-true}
      - SIGN_IN_PREFILLED=${SIGN_IN_PREFILLED:-false}
      - PG_DATABASE_URL=postgres://twenty:${SERVICE_PASSWORD_DB}@twenty-db:5432/twenty
      - REDIS_URL=redis://twenty-redis:6379
      - APP_SECRET=${SERVICE_PASSWORD_SECRET}
    volumes:
      - twenty_data:/app/.local-storage
    labels:
      - "rivetr.managed=true"

  twenty-db:
    image: postgres:15-alpine
    container_name: ${CONTAINER_NAME:-twenty}-db
    restart: unless-stopped
    environment:
      - POSTGRES_DB=twenty
      - POSTGRES_USER=twenty
      - POSTGRES_PASSWORD=${SERVICE_PASSWORD_DB}
    volumes:
      - twenty_postgres:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  twenty-redis:
    image: redis:7-alpine
    container_name: ${CONTAINER_NAME:-twenty}-redis
    restart: unless-stopped
    volumes:
      - twenty_redis:/data
    labels:
      - "rivetr.managed=true"

volumes:
  twenty_data:
  twenty_postgres:
  twenty_redis:
"#,
            r#"[{"name":"SERVER_URL","label":"Server URL","required":true,"default":"http://localhost:3000","secret":false},{"name":"SERVICE_PASSWORD_DB","label":"Database Password","required":true,"default":"","secret":true},{"name":"SERVICE_PASSWORD_SECRET","label":"App Secret","required":true,"default":"","secret":true},{"name":"FRONT_AUTH_ENABLED","label":"Enable Auth","required":false,"default":"true","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
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
        (
            "tpl-netdata",
            "Netdata",
            "Real-time infrastructure monitoring with 1-second granularity, hundreds of metrics, and anomaly detection.",
            "monitoring",
            "netdata",
            r#"services:
  netdata:
    image: netdata/netdata:latest
    container_name: ${CONTAINER_NAME:-netdata}
    restart: unless-stopped
    pid: host
    ports:
      - "${PORT:-19999}:19999"
    cap_add:
      - SYS_PTRACE
      - SYS_ADMIN
    security_opt:
      - apparmor:unconfined
    volumes:
      - netdata_config:/etc/netdata
      - netdata_lib:/var/lib/netdata
      - netdata_cache:/var/cache/netdata
      - /etc/passwd:/host/etc/passwd:ro
      - /etc/group:/host/etc/group:ro
      - /proc:/host/proc:ro
      - /sys:/host/sys:ro
      - /etc/os-release:/host/etc/os-release:ro
    environment:
      - NETDATA_CLAIM_TOKEN=${CLAIM_TOKEN:-}
      - NETDATA_CLAIM_URL=${CLAIM_URL:-https://app.netdata.cloud}
      - NETDATA_CLAIM_ROOMS=${CLAIM_ROOMS:-}
    labels:
      - "rivetr.managed=true"

volumes:
  netdata_config:
  netdata_lib:
  netdata_cache:
"#,
            r#"[{"name":"CLAIM_TOKEN","label":"Netdata Cloud Claim Token (optional)","required":false,"default":"","secret":true},{"name":"CLAIM_ROOMS","label":"Netdata Cloud Room IDs (optional)","required":false,"default":"","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"19999","secret":false}]"#,
        ),
        (
            "tpl-checkmk",
            "Checkmk",
            "Comprehensive IT monitoring platform: infrastructure, applications, cloud, and network monitoring.",
            "monitoring",
            "checkmk",
            r#"services:
  checkmk:
    image: checkmk/check-mk-raw:latest
    container_name: ${CONTAINER_NAME:-checkmk}
    restart: unless-stopped
    ports:
      - "${PORT:-5000}:5000"
      - "${AGENT_PORT:-8000}:8000"
    environment:
      - CMK_SITE_ID=${SITE_ID:-cmk}
      - CMK_PASSWORD=${ADMIN_PASSWORD}
    volumes:
      - checkmk_data:/omd/sites
    labels:
      - "rivetr.managed=true"

volumes:
  checkmk_data:
"#,
            r#"[{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"SITE_ID","label":"Site ID","required":false,"default":"cmk","secret":false},{"name":"PORT","label":"Web UI Port","required":false,"default":"5000","secret":false}]"#,
        ),
        // ==================== STORAGE ====================
        (
            "tpl-minio",
            "MinIO",
            "High-performance, S3-compatible object storage. Store unstructured data like images, videos, backups, and more.",
            "storage",
            "minio",
            r#"services:
  minio:
    image: minio/minio:latest
    container_name: ${CONTAINER_NAME:-minio}
    restart: unless-stopped
    command: server /data --console-address :9001
    ports:
      - "${PORT:-9000}:9000"
      - "${CONSOLE_PORT:-9001}:9001"
    environment:
      - MINIO_ROOT_USER=${MINIO_ROOT_USER:-admin}
      - MINIO_ROOT_PASSWORD=${SERVICE_PASSWORD_MINIO}
    volumes:
      - minio_data:/data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 30s
      timeout: 20s
      retries: 3
    labels:
      - "rivetr.managed=true"

volumes:
  minio_data:
"#,
            r#"[{"name":"MINIO_ROOT_USER","label":"Root Username","required":false,"default":"admin","secret":false},{"name":"SERVICE_PASSWORD_MINIO","label":"Root Password","required":true,"default":"","secret":true},{"name":"PORT","label":"API Port","required":false,"default":"9000","secret":false},{"name":"CONSOLE_PORT","label":"Console Port","required":false,"default":"9001","secret":false}]"#,
        ),
        // ==================== MEDIA ====================
        (
            "tpl-photoprism",
            "PhotoPrism",
            "AI-powered photo management. Browse, organize, and share your photo collection without giving up privacy.",
            "media",
            "photoprism",
            r#"services:
  photoprism:
    image: photoprism/photoprism:latest
    container_name: ${CONTAINER_NAME:-photoprism}
    restart: unless-stopped
    ports:
      - "${PORT:-2342}:2342"
    environment:
      - PHOTOPRISM_ADMIN_USER=${ADMIN_USER:-admin}
      - PHOTOPRISM_ADMIN_PASSWORD=${SERVICE_PASSWORD_ADMIN}
      - PHOTOPRISM_AUTH_MODE=password
      - PHOTOPRISM_SITE_URL=${SITE_URL:-http://localhost:2342}
      - PHOTOPRISM_DATABASE_DRIVER=sqlite
      - PHOTOPRISM_UPLOAD_NSFW=${UPLOAD_NSFW:-true}
      - PHOTOPRISM_JPEG_QUALITY=${JPEG_QUALITY:-85}
    volumes:
      - photoprism_originals:/photoprism/originals
      - photoprism_storage:/photoprism/storage
    working_dir: /photoprism
    labels:
      - "rivetr.managed=true"

volumes:
  photoprism_originals:
  photoprism_storage:
"#,
            r#"[{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"SERVICE_PASSWORD_ADMIN","label":"Admin Password","required":true,"default":"","secret":true},{"name":"SITE_URL","label":"Site URL","required":false,"default":"http://localhost:2342","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"2342","secret":false}]"#,
        ),
    ]
}
