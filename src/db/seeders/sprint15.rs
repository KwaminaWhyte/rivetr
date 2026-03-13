//! Sprint 15 service templates: communication, BaaS, AI, monitoring, and more

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== COMMUNICATION ====================
        (
            "tpl-mattermost",
            "Mattermost",
            "Self-hosted Slack alternative for secure team messaging. Open-source, extensible, and enterprise-ready.",
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
      - MM_SQLSETTINGS_DRIVERNAME=postgres
      - MM_SQLSETTINGS_DATASOURCE=postgres://mattermost:${DB_PASSWORD:-mattermost}@mattermost_db:5432/mattermost?sslmode=disable
      - MM_SERVICESETTINGS_SITEURL=${SITE_URL:-http://localhost:8065}
      - MM_SERVICESETTINGS_ENABLELOCALMODE=true
    depends_on:
      - mattermost_db
    volumes:
      - mattermost_data:/mattermost/data
      - mattermost_logs:/mattermost/logs
      - mattermost_plugins:/mattermost/plugins
    labels:
      - "rivetr.managed=true"

  mattermost_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=mattermost
      - POSTGRES_PASSWORD=${DB_PASSWORD:-mattermost}
      - POSTGRES_DB=mattermost
    volumes:
      - mattermost_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  mattermost_data:
  mattermost_logs:
  mattermost_plugins:
  mattermost_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mattermost","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8065","secret":false},{"name":"SITE_URL","label":"Site URL","required":true,"default":"http://localhost:8065","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-rocketchat",
            "Rocket.Chat",
            "Open-source team chat platform with real-time messaging, video conferencing, and marketplace integrations.",
            "communication",
            "rocket-chat",
            r#"services:
  rocketchat:
    image: registry.rocket.chat/rocketchat/rocket.chat:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-rocketchat}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - ROOT_URL=${ROOT_URL:-http://localhost:3000}
      - PORT=3000
      - MONGO_URL=mongodb://rocketchat_mongo:27017/rocketchat
      - MONGO_OPLOG_URL=mongodb://rocketchat_mongo:27017/local?replicaSet=rs0
    depends_on:
      - rocketchat_mongo
    volumes:
      - rocketchat_uploads:/app/uploads
    labels:
      - "rivetr.managed=true"

  rocketchat_mongo:
    image: mongo:6
    restart: unless-stopped
    command: mongod --oplogSize 128 --replSet rs0
    volumes:
      - rocketchat_mongo_data:/data/db
    labels:
      - "rivetr.managed=true"

  rocketchat_mongo_init:
    image: mongo:6
    restart: "no"
    depends_on:
      - rocketchat_mongo
    command: >
      bash -c "sleep 5 && mongosh --host rocketchat_mongo:27017 --eval \"rs.initiate({ _id: 'rs0', members: [{ _id: 0, host: 'rocketchat_mongo:27017' }] })\""
    labels:
      - "rivetr.managed=true"

volumes:
  rocketchat_uploads:
  rocketchat_mongo_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"rocketchat","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"ROOT_URL","label":"Root URL","required":true,"default":"http://localhost:3000","secret":false}]"#,
        ),
        (
            "tpl-jitsi-meet",
            "Jitsi Meet",
            "Fully encrypted, open-source video conferencing. No account required. Host meetings on your own server.",
            "communication",
            "jitsi",
            r#"services:
  jitsi-web:
    image: jitsi/web:${VERSION:-stable-9646}
    container_name: ${CONTAINER_NAME:-jitsi-web}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-8000}:80"
      - "${HTTPS_PORT:-8443}:443"
    environment:
      - PUBLIC_URL=${PUBLIC_URL:-http://localhost:8000}
      - XMPP_SERVER=jitsi-prosody
      - XMPP_DOMAIN=meet.jitsi
      - XMPP_AUTH_DOMAIN=auth.meet.jitsi
      - XMPP_INTERNAL_MUC_DOMAIN=internal-muc.meet.jitsi
      - XMPP_MUC_DOMAIN=muc.meet.jitsi
      - JICOFO_AUTH_USER=focus
      - ENABLE_LETSENCRYPT=0
    volumes:
      - jitsi_web_config:/config
      - jitsi_web_letsencrypt:/etc/letsencrypt
    depends_on:
      - jitsi-prosody
    labels:
      - "rivetr.managed=true"

  jitsi-prosody:
    image: jitsi/prosody:${VERSION:-stable-9646}
    restart: unless-stopped
    expose:
      - "5222"
      - "5347"
      - "5280"
    environment:
      - XMPP_DOMAIN=meet.jitsi
      - XMPP_AUTH_DOMAIN=auth.meet.jitsi
      - XMPP_MUC_DOMAIN=muc.meet.jitsi
      - XMPP_INTERNAL_MUC_DOMAIN=internal-muc.meet.jitsi
      - XMPP_RECORDER_DOMAIN=recorder.meet.jitsi
      - JICOFO_COMPONENT_SECRET=${JICOFO_COMPONENT_SECRET:-s3cr37}
      - JICOFO_AUTH_USER=focus
      - JICOFO_AUTH_PASSWORD=${JICOFO_AUTH_PASSWORD:-passw0rd}
      - JVB_AUTH_USER=jvb
      - JVB_AUTH_PASSWORD=${JVB_AUTH_PASSWORD:-passw0rd}
      - JIBRI_XMPP_USER=jibri
      - JIBRI_XMPP_PASSWORD=${JIBRI_XMPP_PASSWORD:-passw0rd}
      - JIBRI_RECORDER_USER=recorder
      - JIBRI_RECORDER_PASSWORD=${JIBRI_RECORDER_PASSWORD:-passw0rd}
      - TZ=UTC
    volumes:
      - jitsi_prosody_config:/config
      - jitsi_prosody_plugins:/prosody-plugins-custom
    labels:
      - "rivetr.managed=true"

  jitsi-jicofo:
    image: jitsi/jicofo:${VERSION:-stable-9646}
    restart: unless-stopped
    environment:
      - XMPP_SERVER=jitsi-prosody
      - XMPP_DOMAIN=meet.jitsi
      - XMPP_AUTH_DOMAIN=auth.meet.jitsi
      - XMPP_INTERNAL_MUC_DOMAIN=internal-muc.meet.jitsi
      - JICOFO_COMPONENT_SECRET=${JICOFO_COMPONENT_SECRET:-s3cr37}
      - JICOFO_AUTH_USER=focus
      - JICOFO_AUTH_PASSWORD=${JICOFO_AUTH_PASSWORD:-passw0rd}
      - TZ=UTC
    depends_on:
      - jitsi-prosody
    labels:
      - "rivetr.managed=true"

  jitsi-jvb:
    image: jitsi/jvb:${VERSION:-stable-9646}
    restart: unless-stopped
    ports:
      - "${JVB_PORT:-10000}:10000/udp"
    environment:
      - XMPP_SERVER=jitsi-prosody
      - XMPP_DOMAIN=meet.jitsi
      - XMPP_AUTH_DOMAIN=auth.meet.jitsi
      - XMPP_INTERNAL_MUC_DOMAIN=internal-muc.meet.jitsi
      - JVB_AUTH_USER=jvb
      - JVB_AUTH_PASSWORD=${JVB_AUTH_PASSWORD:-passw0rd}
      - JVB_ADVERTISE_IPS=${PUBLIC_IP:-127.0.0.1}
      - TZ=UTC
    depends_on:
      - jitsi-prosody
    volumes:
      - jitsi_jvb_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  jitsi_web_config:
  jitsi_web_letsencrypt:
  jitsi_prosody_config:
  jitsi_prosody_plugins:
  jitsi_jvb_config:
"#,
            r#"[{"name":"VERSION","label":"Version Tag","required":false,"default":"stable-9646","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"jitsi-web","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"8000","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"8443","secret":false},{"name":"PUBLIC_URL","label":"Public URL","required":true,"default":"http://localhost:8000","secret":false},{"name":"PUBLIC_IP","label":"Public IP (for JVB)","required":true,"default":"127.0.0.1","secret":false},{"name":"JVB_PORT","label":"JVB UDP Port","required":false,"default":"10000","secret":false},{"name":"JICOFO_COMPONENT_SECRET","label":"Jicofo Component Secret","required":true,"default":"","secret":true},{"name":"JICOFO_AUTH_PASSWORD","label":"Jicofo Auth Password","required":true,"default":"","secret":true},{"name":"JVB_AUTH_PASSWORD","label":"JVB Auth Password","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== BAAS / BACKEND-AS-A-SERVICE ====================
        (
            "tpl-supabase",
            "Supabase",
            "Open-source Firebase alternative. Postgres database, Auth, Storage, Edge Functions, and Realtime.",
            "infrastructure",
            "supabase",
            r#"services:
  supabase-db:
    image: supabase/postgres:15.1.0.117
    restart: unless-stopped
    environment:
      - POSTGRES_PASSWORD=${DB_PASSWORD:-your-super-secret-db-password}
      - POSTGRES_DB=postgres
    volumes:
      - supabase_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  supabase-auth:
    image: supabase/gotrue:v2.143.0
    restart: unless-stopped
    environment:
      - GOTRUE_DB_DRIVER=postgres
      - GOTRUE_DB_DATABASE_URL=postgres://supabase_auth_admin:${DB_PASSWORD:-your-super-secret-db-password}@supabase-db:5432/postgres
      - GOTRUE_SITE_URL=${SITE_URL:-http://localhost:3000}
      - GOTRUE_JWT_SECRET=${JWT_SECRET:-your-super-secret-jwt-token-with-at-least-32-characters}
      - GOTRUE_JWT_EXP=3600
      - API_EXTERNAL_URL=${SITE_URL:-http://localhost:3000}
    depends_on:
      - supabase-db
    labels:
      - "rivetr.managed=true"

  supabase-rest:
    image: postgrest/postgrest:v12.0.2
    restart: unless-stopped
    environment:
      - PGRST_DB_URI=postgres://authenticator:${DB_PASSWORD:-your-super-secret-db-password}@supabase-db:5432/postgres
      - PGRST_DB_SCHEMAS=public,storage,graphql_public
      - PGRST_DB_ANON_ROLE=anon
      - PGRST_JWT_SECRET=${JWT_SECRET:-your-super-secret-jwt-token-with-at-least-32-characters}
      - PGRST_DB_USE_LEGACY_GUCS=false
    depends_on:
      - supabase-db
    labels:
      - "rivetr.managed=true"

  supabase-studio:
    image: supabase/studio:20240101-ce42139
    restart: unless-stopped
    ports:
      - "${STUDIO_PORT:-3000}:3000"
    environment:
      - SUPABASE_URL=http://supabase-kong:8000
      - STUDIO_PG_META_URL=http://supabase-meta:8080
      - SUPABASE_ANON_KEY=${ANON_KEY:-eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJyb2xlIjoiYW5vbiJ9.ZopqoUt20nEV8rw6HtnRmaikeb7AkcqByFofgPTnEpI}
      - SUPABASE_SERVICE_KEY=${SERVICE_ROLE_KEY:-eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJyb2xlIjoic2VydmljZV9yb2xlIn0.M2d2z4SFn5C7HlJlaSLfrzuYim9nbY_XI40uWFN3hEE}
    labels:
      - "rivetr.managed=true"

  supabase-kong:
    image: kong:2.8.1
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
      - "${KONG_HTTPS_PORT:-8443}:8443"
    environment:
      - KONG_DATABASE=off
      - KONG_DECLARATIVE_CONFIG=/var/lib/kong/kong.yml
      - KONG_DNS_ORDER=LAST,A,CNAME
      - KONG_PLUGINS=request-transformer,cors,key-auth,acl
    volumes:
      - supabase_kong_config:/var/lib/kong
    labels:
      - "rivetr.managed=true"

  supabase-meta:
    image: supabase/postgres-meta:v0.75.0
    restart: unless-stopped
    environment:
      - PG_META_PORT=8080
      - PG_META_DB_HOST=supabase-db
      - PG_META_DB_PORT=5432
      - PG_META_DB_NAME=postgres
      - PG_META_DB_USER=supabase
      - PG_META_DB_PASSWORD=${DB_PASSWORD:-your-super-secret-db-password}
    depends_on:
      - supabase-db
    labels:
      - "rivetr.managed=true"

volumes:
  supabase_db_data:
  supabase_kong_config:
"#,
            r#"[{"name":"STUDIO_PORT","label":"Studio Port","required":false,"default":"3000","secret":false},{"name":"PORT","label":"API Gateway Port","required":false,"default":"8000","secret":false},{"name":"SITE_URL","label":"Site URL","required":true,"default":"http://localhost:3000","secret":false},{"name":"JWT_SECRET","label":"JWT Secret (min 32 chars)","required":true,"default":"","secret":true},{"name":"ANON_KEY","label":"Anon Key (JWT)","required":true,"default":"","secret":true},{"name":"SERVICE_ROLE_KEY","label":"Service Role Key (JWT)","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-appwrite",
            "Appwrite",
            "Secure open-source backend server for web, mobile, and Flutter. Authentication, databases, storage, and functions.",
            "infrastructure",
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
      - _APP_LOCALE=en
      - _APP_OPENSSL_KEY_V1=${OPENSSL_KEY:-your-secret-key-change-this-now}
      - _APP_DOMAIN=${DOMAIN:-localhost}
      - _APP_DOMAIN_TARGET=${DOMAIN:-localhost}
      - _APP_REDIS_HOST=appwrite_redis
      - _APP_REDIS_PORT=6379
      - _APP_DB_HOST=appwrite_db
      - _APP_DB_PORT=3306
      - _APP_DB_SCHEMA=appwrite
      - _APP_DB_USER=appwrite
      - _APP_DB_PASS=${DB_PASSWORD:-appwrite}
      - _APP_INFLUXDB_HOST=appwrite_influxdb
      - _APP_INFLUXDB_PORT=8086
      - _APP_USAGE_STATS=enabled
    depends_on:
      - appwrite_db
      - appwrite_redis
      - appwrite_influxdb
    volumes:
      - appwrite_uploads:/storage/uploads
      - appwrite_cache:/storage/cache
      - appwrite_config:/storage/config
      - appwrite_certificates:/storage/certificates
    labels:
      - "rivetr.managed=true"

  appwrite_db:
    image: mariadb:10.11
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-rootpassword}
      - MYSQL_DATABASE=appwrite
      - MYSQL_USER=appwrite
      - MYSQL_PASSWORD=${DB_PASSWORD:-appwrite}
    volumes:
      - appwrite_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

  appwrite_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

  appwrite_influxdb:
    image: appwrite/influxdb:1.0.0
    restart: unless-stopped
    volumes:
      - appwrite_influxdb_data:/var/lib/influxdb
    labels:
      - "rivetr.managed=true"

volumes:
  appwrite_uploads:
  appwrite_cache:
  appwrite_config:
  appwrite_certificates:
  appwrite_db_data:
  appwrite_influxdb_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"appwrite","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false},{"name":"DOMAIN","label":"Domain","required":true,"default":"localhost","secret":false},{"name":"OPENSSL_KEY","label":"OpenSSL Key","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"DB_ROOT_PASSWORD","label":"DB Root Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-pocketbase",
            "PocketBase",
            "Open-source backend in a single file. Embedded SQLite, realtime subscriptions, auth, and file storage.",
            "infrastructure",
            "pocketbase",
            r#"services:
  pocketbase:
    image: ghcr.io/muchobien/pocketbase:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-pocketbase}
    restart: unless-stopped
    ports:
      - "${PORT:-8090}:8090"
    command: --http=0.0.0.0:8090
    volumes:
      - pocketbase_data:/pb_data
      - pocketbase_public:/pb_public
      - pocketbase_migrations:/pb_migrations
    labels:
      - "rivetr.managed=true"

volumes:
  pocketbase_data:
  pocketbase_public:
  pocketbase_migrations:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"pocketbase","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8090","secret":false}]"#,
        ),
        // ==================== CMS / HEADLESS ====================
        (
            "tpl-directus",
            "Directus",
            "Instant REST and GraphQL API for any SQL database. Visual data studio and extensible headless CMS.",
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
      - KEY=${KEY:-replace-with-a-random-key}
      - SECRET=${SECRET:-replace-with-a-random-secret}
      - DB_CLIENT=pg
      - DB_HOST=directus_db
      - DB_PORT=5432
      - DB_DATABASE=directus
      - DB_USER=directus
      - DB_PASSWORD=${DB_PASSWORD:-directus}
      - CACHE_ENABLED=true
      - CACHE_STORE=redis
      - REDIS=redis://directus_redis:6379
      - ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - PUBLIC_URL=${PUBLIC_URL:-http://localhost:8055}
    depends_on:
      - directus_db
      - directus_redis
    volumes:
      - directus_uploads:/directus/uploads
      - directus_extensions:/directus/extensions
    labels:
      - "rivetr.managed=true"

  directus_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=directus
      - POSTGRES_PASSWORD=${DB_PASSWORD:-directus}
      - POSTGRES_DB=directus
    volumes:
      - directus_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  directus_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  directus_uploads:
  directus_extensions:
  directus_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"directus","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8055","secret":false},{"name":"KEY","label":"App Key","required":true,"default":"","secret":true},{"name":"SECRET","label":"App Secret","required":true,"default":"","secret":true},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"PUBLIC_URL","label":"Public URL","required":false,"default":"http://localhost:8055","secret":false}]"#,
        ),
        (
            "tpl-strapi",
            "Strapi",
            "Leading open-source headless CMS. 100% JavaScript/TypeScript, fully customizable, developer-first.",
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
      - DATABASE_CLIENT=${DATABASE_CLIENT:-postgres}
      - DATABASE_HOST=strapi_db
      - DATABASE_PORT=5432
      - DATABASE_NAME=strapi
      - DATABASE_USERNAME=strapi
      - DATABASE_PASSWORD=${DB_PASSWORD:-strapi}
      - DATABASE_SSL=false
      - JWT_SECRET=${JWT_SECRET:-change-me-to-a-long-random-string}
      - ADMIN_JWT_SECRET=${ADMIN_JWT_SECRET:-change-me-to-another-long-random-string}
      - APP_KEYS=${APP_KEYS:-key1,key2,key3,key4}
      - API_TOKEN_SALT=${API_TOKEN_SALT:-change-me-to-a-random-salt}
      - TRANSFER_TOKEN_SALT=${TRANSFER_TOKEN_SALT:-change-me-to-another-salt}
    depends_on:
      - strapi_db
    volumes:
      - strapi_uploads:/opt/app/public/uploads
      - strapi_config:/opt/app/config
    labels:
      - "rivetr.managed=true"

  strapi_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=strapi
      - POSTGRES_PASSWORD=${DB_PASSWORD:-strapi}
      - POSTGRES_DB=strapi
    volumes:
      - strapi_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  strapi_uploads:
  strapi_config:
  strapi_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"strapi","secret":false},{"name":"PORT","label":"Port","required":false,"default":"1337","secret":false},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true},{"name":"ADMIN_JWT_SECRET","label":"Admin JWT Secret","required":true,"default":"","secret":true},{"name":"APP_KEYS","label":"App Keys (comma-separated)","required":true,"default":"","secret":true},{"name":"API_TOKEN_SALT","label":"API Token Salt","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-outline",
            "Outline",
            "Fast, collaborative wiki and knowledge base. Beautiful editor, team hierarchy, and integrations.",
            "cms",
            "outline",
            r#"services:
  outline:
    image: outlinewiki/outline:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-outline}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - SECRET_KEY=${SECRET_KEY:-change-me-to-a-long-random-string-at-least-32-chars}
      - UTILS_SECRET=${UTILS_SECRET:-change-me-to-another-long-random-string}
      - DATABASE_URL=postgres://outline:${DB_PASSWORD:-outline}@outline_db:5432/outline
      - REDIS_URL=redis://outline_redis:6379
      - URL=${URL:-http://localhost:3000}
      - PORT=3000
      - FILE_STORAGE=local
      - FILE_STORAGE_LOCAL_ROOT_DIR=/var/lib/outline/data
    depends_on:
      - outline_db
      - outline_redis
    volumes:
      - outline_data:/var/lib/outline/data
    labels:
      - "rivetr.managed=true"

  outline_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=outline
      - POSTGRES_PASSWORD=${DB_PASSWORD:-outline}
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
  outline_data:
  outline_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"outline","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"SECRET_KEY","label":"Secret Key (min 32 chars)","required":true,"default":"","secret":true},{"name":"UTILS_SECRET","label":"Utils Secret","required":true,"default":"","secret":true},{"name":"URL","label":"Application URL","required":true,"default":"http://localhost:3000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== DEVTOOLS / CI-CD ====================
        (
            "tpl-drone-ci",
            "Drone CI",
            "Container-native CI/CD platform. Pipelines as code, auto-scaling runners, and deep Git integration.",
            "devtools",
            "drone",
            r#"services:
  drone:
    image: drone/drone:${VERSION:-2}
    container_name: ${CONTAINER_NAME:-drone}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
    environment:
      - DRONE_GITEA_SERVER=${GITEA_SERVER:-http://gitea:3000}
      - DRONE_GITEA_CLIENT_ID=${GITEA_CLIENT_ID:-your-gitea-oauth-client-id}
      - DRONE_GITEA_CLIENT_SECRET=${GITEA_CLIENT_SECRET:-your-gitea-oauth-client-secret}
      - DRONE_RPC_SECRET=${RPC_SECRET:-change-me-to-a-random-secret}
      - DRONE_SERVER_HOST=${SERVER_HOST:-localhost}
      - DRONE_SERVER_PROTO=${SERVER_PROTO:-http}
      - DRONE_DATABASE_DRIVER=sqlite3
      - DRONE_DATABASE_DATASOURCE=/data/drone.db
      - DRONE_USER_CREATE=username:${ADMIN_USER:-admin},admin:true
    volumes:
      - drone_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  drone_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"2","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"drone","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"GITEA_SERVER","label":"Gitea Server URL","required":true,"default":"http://gitea:3000","secret":false},{"name":"GITEA_CLIENT_ID","label":"Gitea OAuth Client ID","required":true,"default":"","secret":false},{"name":"GITEA_CLIENT_SECRET","label":"Gitea OAuth Client Secret","required":true,"default":"","secret":true},{"name":"RPC_SECRET","label":"RPC Secret","required":true,"default":"","secret":true},{"name":"SERVER_HOST","label":"Drone Server Hostname","required":true,"default":"localhost","secret":false},{"name":"SERVER_PROTO","label":"Protocol (http/https)","required":false,"default":"http","secret":false},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false}]"#,
        ),
        (
            "tpl-gitea-runner",
            "Gitea Act Runner",
            "Gitea Actions runner based on act. Execute CI workflows on your own infrastructure.",
            "devtools",
            "gitea",
            r#"services:
  gitea-runner:
    image: gitea/act_runner:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-gitea-runner}
    restart: unless-stopped
    environment:
      - GITEA_INSTANCE_URL=${GITEA_INSTANCE_URL:-http://gitea:3000}
      - GITEA_RUNNER_REGISTRATION_TOKEN=${REGISTRATION_TOKEN:-change-me-to-your-runner-token}
      - GITEA_RUNNER_NAME=${RUNNER_NAME:-rivetr-runner}
      - GITEA_RUNNER_LABELS=${RUNNER_LABELS:-ubuntu-latest:docker://node:16-bullseye,ubuntu-22.04:docker://node:16-bullseye}
      - CONFIG_FILE=/config/config.yaml
    volumes:
      - gitea_runner_data:/data
      - gitea_runner_config:/config
      - /var/run/docker.sock:/var/run/docker.sock
    labels:
      - "rivetr.managed=true"

volumes:
  gitea_runner_data:
  gitea_runner_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"gitea-runner","secret":false},{"name":"GITEA_INSTANCE_URL","label":"Gitea Instance URL","required":true,"default":"http://gitea:3000","secret":false},{"name":"REGISTRATION_TOKEN","label":"Runner Registration Token","required":true,"default":"","secret":true},{"name":"RUNNER_NAME","label":"Runner Name","required":false,"default":"rivetr-runner","secret":false},{"name":"RUNNER_LABELS","label":"Runner Labels","required":false,"default":"ubuntu-latest:docker://node:16-bullseye","secret":false}]"#,
        ),
        (
            "tpl-windmill",
            "Windmill",
            "Open-source developer platform for scripts, workflows, and apps. Alternative to Retool and Airplane.",
            "devtools",
            "windmill",
            r#"services:
  windmill:
    image: ghcr.io/windmill-labs/windmill:${VERSION:-main}
    container_name: ${CONTAINER_NAME:-windmill}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - DATABASE_URL=postgres://windmill:${DB_PASSWORD:-windmill}@windmill_db:5432/windmill
      - BASE_URL=${BASE_URL:-http://localhost:8000}
      - RUST_LOG=info
      - NUM_WORKERS=3
      - DISABLE_SERVER=false
      - METRICS_ADDR=false
    depends_on:
      - windmill_db
    volumes:
      - windmill_data:/tmp/windmill
    labels:
      - "rivetr.managed=true"

  windmill_worker:
    image: ghcr.io/windmill-labs/windmill:${VERSION:-main}
    restart: unless-stopped
    environment:
      - DATABASE_URL=postgres://windmill:${DB_PASSWORD:-windmill}@windmill_db:5432/windmill
      - BASE_URL=${BASE_URL:-http://localhost:8000}
      - DISABLE_SERVER=true
      - RUST_LOG=info
      - NUM_WORKERS=3
      - METRICS_ADDR=false
    depends_on:
      - windmill_db
    volumes:
      - windmill_worker_data:/tmp/windmill
    labels:
      - "rivetr.managed=true"

  windmill_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=windmill
      - POSTGRES_PASSWORD=${DB_PASSWORD:-windmill}
      - POSTGRES_DB=windmill
    volumes:
      - windmill_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  windmill_data:
  windmill_worker_data:
  windmill_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"main","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"windmill","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false},{"name":"BASE_URL","label":"Base URL","required":true,"default":"http://localhost:8000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== AI / ML ====================
        (
            "tpl-ollama",
            "Ollama",
            "Run large language models locally. Supports Llama 3, Mistral, Gemma, and dozens more open models.",
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
      - ollama_data:/root/.ollama
    labels:
      - "rivetr.managed=true"

volumes:
  ollama_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"ollama","secret":false},{"name":"PORT","label":"Port","required":false,"default":"11434","secret":false}]"#,
        ),
        (
            "tpl-open-webui",
            "Open WebUI",
            "Feature-rich web interface for Ollama and OpenAI-compatible APIs. Chat history, RAG, and model management.",
            "ai",
            "open-webui",
            r#"services:
  open-webui:
    image: ghcr.io/open-webui/open-webui:${VERSION:-main}
    container_name: ${CONTAINER_NAME:-open-webui}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:8080"
    environment:
      - OLLAMA_BASE_URL=${OLLAMA_BASE_URL:-http://ollama:11434}
      - WEBUI_SECRET_KEY=${WEBUI_SECRET_KEY:-change-me-to-a-random-secret}
      - ENABLE_RAG_WEB_SEARCH=false
    volumes:
      - open_webui_data:/app/backend/data
    labels:
      - "rivetr.managed=true"

volumes:
  open_webui_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"main","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"open-webui","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"OLLAMA_BASE_URL","label":"Ollama Base URL","required":true,"default":"http://ollama:11434","secret":false},{"name":"WEBUI_SECRET_KEY","label":"WebUI Secret Key","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-flowise",
            "Flowise",
            "Low-code LLM app builder. Drag-and-drop UI for building LLM chains and agents with LangChain.",
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
      - PORT=3000
      - FLOWISE_USERNAME=${FLOWISE_USERNAME:-admin}
      - FLOWISE_PASSWORD=${FLOWISE_PASSWORD:-changeme}
      - FLOWISE_SECRETKEY_OVERWRITE=${SECRET_KEY:-change-me-to-a-random-secret}
      - DATABASE_TYPE=sqlite
      - DATABASE_PATH=/root/.flowise
      - APIKEY_PATH=/root/.flowise
      - SECRETKEY_PATH=/root/.flowise
      - LOG_PATH=/root/.flowise/logs
    volumes:
      - flowise_data:/root/.flowise
    labels:
      - "rivetr.managed=true"

volumes:
  flowise_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"flowise","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"FLOWISE_USERNAME","label":"Username","required":false,"default":"admin","secret":false},{"name":"FLOWISE_PASSWORD","label":"Password","required":true,"default":"","secret":true},{"name":"SECRET_KEY","label":"Encryption Secret Key","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== MONITORING ====================
        (
            "tpl-grafana-prometheus",
            "Grafana + Prometheus",
            "Industry-standard observability stack. Prometheus metrics collection with Grafana dashboards and alerting.",
            "monitoring",
            "grafana",
            r#"services:
  grafana:
    image: grafana/grafana:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-grafana}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - GF_SECURITY_ADMIN_USER=${ADMIN_USER:-admin}
      - GF_SECURITY_ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - GF_USERS_ALLOW_SIGN_UP=false
      - GF_SERVER_ROOT_URL=${ROOT_URL:-http://localhost:3000}
    volumes:
      - grafana_data:/var/lib/grafana
    depends_on:
      - prometheus
    labels:
      - "rivetr.managed=true"

  prometheus:
    image: prom/prometheus:${PROMETHEUS_VERSION:-latest}
    container_name: ${CONTAINER_NAME:-prometheus}
    restart: unless-stopped
    ports:
      - "${PROMETHEUS_PORT:-9090}:9090"
    command:
      - "--config.file=/etc/prometheus/prometheus.yml"
      - "--storage.tsdb.path=/prometheus"
      - "--web.console.libraries=/etc/prometheus/console_libraries"
      - "--web.console.templates=/etc/prometheus/consoles"
      - "--storage.tsdb.retention.time=${RETENTION:-200h}"
      - "--web.enable-lifecycle"
    volumes:
      - prometheus_data:/prometheus
      - prometheus_config:/etc/prometheus
    labels:
      - "rivetr.managed=true"

volumes:
  grafana_data:
  prometheus_data:
  prometheus_config:
"#,
            r#"[{"name":"VERSION","label":"Grafana Version","required":false,"default":"latest","secret":false},{"name":"PROMETHEUS_VERSION","label":"Prometheus Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name Prefix","required":false,"default":"grafana","secret":false},{"name":"PORT","label":"Grafana Port","required":false,"default":"3000","secret":false},{"name":"PROMETHEUS_PORT","label":"Prometheus Port","required":false,"default":"9090","secret":false},{"name":"ADMIN_USER","label":"Grafana Admin User","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Grafana Admin Password","required":true,"default":"","secret":true},{"name":"ROOT_URL","label":"Grafana Root URL","required":false,"default":"http://localhost:3000","secret":false},{"name":"RETENTION","label":"Prometheus Retention Time","required":false,"default":"200h","secret":false}]"#,
        ),
        (
            "tpl-checkmk",
            "Checkmk",
            "Comprehensive IT monitoring for servers, networks, cloud, containers, and applications.",
            "monitoring",
            "checkmk",
            r#"services:
  checkmk:
    image: checkmk/check-mk-raw:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-checkmk}
    restart: unless-stopped
    ports:
      - "${PORT:-5000}:5000"
      - "${AGENT_PORT:-8000}:8000"
    environment:
      - CMK_PASSWORD=${CMK_PASSWORD:-changeme}
      - CMK_SITE_ID=${SITE_ID:-cmk}
    volumes:
      - checkmk_data:/omd/sites
    tmpfs:
      - /opt/omd/sites/cmk/tmp:uid=1000,gid=1000
    labels:
      - "rivetr.managed=true"

volumes:
  checkmk_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"checkmk","secret":false},{"name":"PORT","label":"Web UI Port","required":false,"default":"5000","secret":false},{"name":"AGENT_PORT","label":"Agent Port","required":false,"default":"8000","secret":false},{"name":"CMK_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"SITE_ID","label":"Site ID","required":false,"default":"cmk","secret":false}]"#,
        ),
        // ==================== DATABASES ====================
        (
            "tpl-mariadb",
            "MariaDB",
            "Community-developed, commercially supported fork of MySQL. Drop-in replacement with extra features.",
            "databases",
            "database",
            r#"services:
  mariadb:
    image: mariadb:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-mariadb}
    restart: unless-stopped
    ports:
      - "${PORT:-3306}:3306"
    environment:
      - MARIADB_ROOT_PASSWORD=${ROOT_PASSWORD:-changeme}
      - MARIADB_DATABASE=${DATABASE:-app}
      - MARIADB_USER=${DB_USER:-app}
      - MARIADB_PASSWORD=${DB_PASSWORD:-app}
    volumes:
      - mariadb_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  mariadb_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mariadb","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3306","secret":false},{"name":"ROOT_PASSWORD","label":"Root Password","required":true,"default":"","secret":true},{"name":"DATABASE","label":"Database Name","required":false,"default":"app","secret":false},{"name":"DB_USER","label":"Database User","required":false,"default":"app","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== SECURITY ====================
        (
            "tpl-crowdsec",
            "CrowdSec",
            "Open-source security engine that analyzes logs and blocks attacks. Crowdsourced threat intelligence.",
            "security",
            "crowdsec",
            r#"services:
  crowdsec:
    image: crowdsecurity/crowdsec:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-crowdsec}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
      - "${METRICS_PORT:-6060}:6060"
    environment:
      - COLLECTIONS=crowdsecurity/linux crowdsecurity/sshd
      - BOUNCER_KEY_FIREWALL=${BOUNCER_KEY:-change-me-to-a-random-key}
    volumes:
      - crowdsec_data:/var/lib/crowdsec/data
      - crowdsec_config:/etc/crowdsec
      - /var/log:/var/log:ro
    labels:
      - "rivetr.managed=true"

volumes:
  crowdsec_data:
  crowdsec_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"crowdsec","secret":false},{"name":"PORT","label":"API Port","required":false,"default":"8080","secret":false},{"name":"METRICS_PORT","label":"Metrics Port","required":false,"default":"6060","secret":false},{"name":"BOUNCER_KEY","label":"Firewall Bouncer Key","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-harbor",
            "Harbor",
            "Cloud-native container registry with vulnerability scanning, RBAC, replication, and audit logging.",
            "devtools",
            "harbor",
            r#"services:
  harbor-core:
    image: goharbor/harbor-core:${VERSION:-v2.10.0}
    container_name: ${CONTAINER_NAME:-harbor-core}
    restart: unless-stopped
    environment:
      - CONFIG_PATH=/etc/core/app.conf
      - CORE_SECRET=${CORE_SECRET:-change-me-secret}
      - JOBSERVICE_SECRET=${JOBSERVICE_SECRET:-change-me-jobservice}
      - DATABASE_TYPE=postgresql
      - POSTGRESQL_HOST=harbor_db
      - POSTGRESQL_PORT=5432
      - POSTGRESQL_DATABASE=registry
      - POSTGRESQL_USERNAME=harbor
      - POSTGRESQL_PASSWORD=${DB_PASSWORD:-harbor}
      - REGISTRY_URL=http://harbor-registry:5000
      - TOKEN_SERVICE_URL=http://harbor-core:8080/service/token
      - HARBOR_ADMIN_PASSWORD=${HARBOR_ADMIN_PASSWORD:-Harbor12345}
      - CSRF_KEY=${CSRF_KEY:-change-me-csrf-key-32-chars-min}
      - RELOAD_KEY=${RELOAD_KEY:-reload-key}
    depends_on:
      - harbor_db
      - harbor-registry
    volumes:
      - harbor_core_config:/etc/core
    labels:
      - "rivetr.managed=true"

  harbor-registry:
    image: goharbor/registry-photon:${VERSION:-v2.10.0}
    restart: unless-stopped
    volumes:
      - harbor_registry_data:/storage
      - harbor_registry_config:/etc/registry
    labels:
      - "rivetr.managed=true"

  harbor-registryctl:
    image: goharbor/harbor-registryctl:${VERSION:-v2.10.0}
    restart: unless-stopped
    environment:
      - CORE_SECRET=${CORE_SECRET:-change-me-secret}
      - JOBSERVICE_SECRET=${JOBSERVICE_SECRET:-change-me-jobservice}
    volumes:
      - harbor_registry_data:/storage
      - harbor_registry_config:/etc/registry
      - harbor_registryctl_config:/etc/registryctl
    labels:
      - "rivetr.managed=true"

  harbor-portal:
    image: goharbor/harbor-portal:${VERSION:-v2.10.0}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:8080"
    labels:
      - "rivetr.managed=true"

  harbor_db:
    image: goharbor/harbor-db:${VERSION:-v2.10.0}
    restart: unless-stopped
    environment:
      - POSTGRES_PASSWORD=${DB_PASSWORD:-harbor}
    volumes:
      - harbor_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  harbor_core_config:
  harbor_registry_data:
  harbor_registry_config:
  harbor_registryctl_config:
  harbor_db_data:
"#,
            r#"[{"name":"VERSION","label":"Harbor Version","required":false,"default":"v2.10.0","secret":false},{"name":"CONTAINER_NAME","label":"Core Container Name","required":false,"default":"harbor-core","secret":false},{"name":"PORT","label":"Portal Port","required":false,"default":"80","secret":false},{"name":"HARBOR_ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"CORE_SECRET","label":"Core Secret","required":true,"default":"","secret":true},{"name":"JOBSERVICE_SECRET","label":"Jobservice Secret","required":true,"default":"","secret":true},{"name":"CSRF_KEY","label":"CSRF Key (min 32 chars)","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== ERP / BUSINESS ====================
        (
            "tpl-odoo",
            "Odoo",
            "Complete open-source ERP suite. CRM, sales, inventory, accounting, HR, manufacturing in one platform.",
            "business",
            "odoo",
            r#"services:
  odoo:
    image: odoo:${VERSION:-17}
    container_name: ${CONTAINER_NAME:-odoo}
    restart: unless-stopped
    ports:
      - "${PORT:-8069}:8069"
      - "${CHAT_PORT:-8072}:8072"
    environment:
      - HOST=odoo_db
      - USER=odoo
      - PASSWORD=${DB_PASSWORD:-odoo}
    depends_on:
      - odoo_db
    volumes:
      - odoo_data:/var/lib/odoo
      - odoo_config:/etc/odoo
      - odoo_addons:/mnt/extra-addons
    labels:
      - "rivetr.managed=true"

  odoo_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=odoo
      - POSTGRES_PASSWORD=${DB_PASSWORD:-odoo}
      - POSTGRES_DB=postgres
    volumes:
      - odoo_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  odoo_data:
  odoo_config:
  odoo_addons:
  odoo_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"17","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"odoo","secret":false},{"name":"PORT","label":"Web Port","required":false,"default":"8069","secret":false},{"name":"CHAT_PORT","label":"Live Chat Port","required":false,"default":"8072","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-mautic",
            "Mautic",
            "Open-source marketing automation. Email campaigns, lead management, landing pages, and analytics.",
            "business",
            "mautic",
            r#"services:
  mautic:
    image: mautic/mautic:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-mautic}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - MAUTIC_DB_HOST=mautic_db
      - MAUTIC_DB_USER=mautic
      - MAUTIC_DB_PASSWORD=${DB_PASSWORD:-mautic}
      - MAUTIC_DB_NAME=mautic
      - MAUTIC_TRUSTED_PROXIES=0.0.0.0/0
      - MAUTIC_RUN_CRON_JOBS=true
      - MAUTIC_ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - MAUTIC_ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - MAUTIC_ADMIN_USERNAME=${ADMIN_USERNAME:-admin}
      - MAUTIC_ADMIN_FIRSTNAME=${ADMIN_FIRSTNAME:-Admin}
      - MAUTIC_ADMIN_LASTNAME=${ADMIN_LASTNAME:-User}
    depends_on:
      - mautic_db
    volumes:
      - mautic_data:/var/www/html
    labels:
      - "rivetr.managed=true"

  mautic_db:
    image: mariadb:10.11
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-rootpassword}
      - MYSQL_DATABASE=mautic
      - MYSQL_USER=mautic
      - MYSQL_PASSWORD=${DB_PASSWORD:-mautic}
    volumes:
      - mautic_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  mautic_data:
  mautic_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mautic","secret":false},{"name":"PORT","label":"Port","required":false,"default":"80","secret":false},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_USERNAME","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"DB_ROOT_PASSWORD","label":"DB Root Password","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== FORMS / SURVEYS ====================
        (
            "tpl-limesurvey",
            "LimeSurvey",
            "Professional online survey platform. Advanced question types, branching logic, and statistical analysis.",
            "business",
            "limesurvey",
            r#"services:
  limesurvey:
    image: martialblog/limesurvey:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-limesurvey}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DB_TYPE=mysql
      - DB_HOST=limesurvey_db
      - DB_PORT=3306
      - DB_NAME=limesurvey
      - DB_USERNAME=limesurvey
      - DB_PASSWORD=${DB_PASSWORD:-limesurvey}
      - ADMIN_USER=${ADMIN_USER:-admin}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - ADMIN_NAME=${ADMIN_NAME:-LimeSurvey Admin}
      - ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - PUBLIC_URL=${PUBLIC_URL:-http://localhost:8080}
    depends_on:
      - limesurvey_db
    volumes:
      - limesurvey_data:/var/www/html/upload
    labels:
      - "rivetr.managed=true"

  limesurvey_db:
    image: mariadb:10.11
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-rootpassword}
      - MYSQL_DATABASE=limesurvey
      - MYSQL_USER=limesurvey
      - MYSQL_PASSWORD=${DB_PASSWORD:-limesurvey}
    volumes:
      - limesurvey_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  limesurvey_data:
  limesurvey_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"limesurvey","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"PUBLIC_URL","label":"Public URL","required":false,"default":"http://localhost:8080","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"DB_ROOT_PASSWORD","label":"DB Root Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-formbricks",
            "Formbricks",
            "Open-source survey and experience management platform. In-app surveys, website surveys, and link surveys.",
            "business",
            "formbricks",
            r#"services:
  formbricks:
    image: ghcr.io/formbricks/formbricks:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-formbricks}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - WEBAPP_URL=${WEBAPP_URL:-http://localhost:3000}
      - DATABASE_URL=postgresql://formbricks:${DB_PASSWORD:-formbricks}@formbricks_db:5432/formbricks
      - NEXTAUTH_SECRET=${NEXTAUTH_SECRET:-change-me-to-a-long-random-string}
      - NEXTAUTH_URL=${WEBAPP_URL:-http://localhost:3000}
      - ENCRYPTION_KEY=${ENCRYPTION_KEY:-change-me-to-a-32-char-random-key}
      - TELEMETRY_DISABLED=true
    depends_on:
      - formbricks_db
    labels:
      - "rivetr.managed=true"

  formbricks_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=formbricks
      - POSTGRES_PASSWORD=${DB_PASSWORD:-formbricks}
      - POSTGRES_DB=formbricks
    volumes:
      - formbricks_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  formbricks_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"formbricks","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"WEBAPP_URL","label":"Web App URL","required":true,"default":"http://localhost:3000","secret":false},{"name":"NEXTAUTH_SECRET","label":"NextAuth Secret","required":true,"default":"","secret":true},{"name":"ENCRYPTION_KEY","label":"Encryption Key (32 chars)","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== NO-CODE / LOW-CODE ====================
        (
            "tpl-baserow",
            "Baserow",
            "Open-source no-code database and Airtable alternative. Build collaborative databases without SQL.",
            "infrastructure",
            "baserow",
            r#"services:
  baserow:
    image: baserow/baserow:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-baserow}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
    environment:
      - BASEROW_PUBLIC_URL=${PUBLIC_URL:-http://localhost}
      - SECRET_KEY=${SECRET_KEY:-change-me-to-a-long-random-string}
      - DATABASE_PASSWORD=${DB_PASSWORD:-baserow}
      - DATABASE_USER=baserow
      - DATABASE_NAME=baserow
      - DATABASE_HOST=baserow_db
      - REDIS_HOST=baserow_redis
    depends_on:
      - baserow_db
      - baserow_redis
    volumes:
      - baserow_data:/baserow/data
    labels:
      - "rivetr.managed=true"

  baserow_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=baserow
      - POSTGRES_PASSWORD=${DB_PASSWORD:-baserow}
      - POSTGRES_DB=baserow
    volumes:
      - baserow_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  baserow_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  baserow_data:
  baserow_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"baserow","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false},{"name":"PUBLIC_URL","label":"Public URL","required":true,"default":"http://localhost","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== EXTRA MONITORING / OBSERVABILITY ====================
        (
            "tpl-victoria-metrics",
            "VictoriaMetrics",
            "Fast, cost-effective monitoring solution. Drop-in Prometheus replacement with better performance and compression.",
            "monitoring",
            "victoria-metrics",
            r#"services:
  victoriametrics:
    image: victoriametrics/victoria-metrics:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-victoriametrics}
    restart: unless-stopped
    ports:
      - "${PORT:-8428}:8428"
    command:
      - "--storageDataPath=/storage"
      - "--retentionPeriod=${RETENTION:-1}"
      - "--httpListenAddr=:8428"
    volumes:
      - vm_data:/storage
    labels:
      - "rivetr.managed=true"

  vmagent:
    image: victoriametrics/vmagent:${VERSION:-latest}
    restart: unless-stopped
    ports:
      - "${AGENT_PORT:-8429}:8429"
    command:
      - "--promscrape.config=/etc/prometheus/prometheus.yml"
      - "--remoteWrite.url=http://victoriametrics:8428/api/v1/write"
    volumes:
      - vmagent_config:/etc/prometheus
      - vmagent_data:/vmagentdata
    labels:
      - "rivetr.managed=true"

volumes:
  vm_data:
  vmagent_config:
  vmagent_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"victoriametrics","secret":false},{"name":"PORT","label":"VictoriaMetrics Port","required":false,"default":"8428","secret":false},{"name":"AGENT_PORT","label":"VMAgent Port","required":false,"default":"8429","secret":false},{"name":"RETENTION","label":"Retention Period (months)","required":false,"default":"1","secret":false}]"#,
        ),
        (
            "tpl-signoz",
            "SigNoz",
            "Open-source APM and observability platform. Distributed tracing, metrics, and logs in one unified UI.",
            "monitoring",
            "signoz",
            r#"services:
  signoz-frontend:
    image: signoz/frontend:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-signoz}
    restart: unless-stopped
    ports:
      - "${PORT:-3301}:3301"
    depends_on:
      - signoz-query-service
    labels:
      - "rivetr.managed=true"

  signoz-query-service:
    image: signoz/query-service:${VERSION:-latest}
    restart: unless-stopped
    environment:
      - ClickHouseUrl=tcp://signoz-clickhouse:9000
      - STORAGE=clickhouse
      - GODEBUG=netdns=go
      - TELEMETRY_ENABLED=true
    depends_on:
      - signoz-clickhouse
    volumes:
      - signoz_data:/var/lib/signoz
    labels:
      - "rivetr.managed=true"

  signoz-clickhouse:
    image: clickhouse/clickhouse-server:${CLICKHOUSE_VERSION:-23.11}
    restart: unless-stopped
    environment:
      - CLICKHOUSE_DB=signoz_traces
      - CLICKHOUSE_USER=default
      - CLICKHOUSE_DEFAULT_ACCESS_MANAGEMENT=1
    volumes:
      - signoz_clickhouse_data:/var/lib/clickhouse
    labels:
      - "rivetr.managed=true"

  signoz-otel-collector:
    image: signoz/signoz-otel-collector:${VERSION:-latest}
    restart: unless-stopped
    ports:
      - "${OTLP_GRPC_PORT:-4317}:4317"
      - "${OTLP_HTTP_PORT:-4318}:4318"
    depends_on:
      - signoz-clickhouse
    labels:
      - "rivetr.managed=true"

volumes:
  signoz_data:
  signoz_clickhouse_data:
"#,
            r#"[{"name":"VERSION","label":"SigNoz Version","required":false,"default":"latest","secret":false},{"name":"CLICKHOUSE_VERSION","label":"ClickHouse Version","required":false,"default":"23.11","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"signoz","secret":false},{"name":"PORT","label":"Frontend Port","required":false,"default":"3301","secret":false},{"name":"OTLP_GRPC_PORT","label":"OTLP gRPC Port","required":false,"default":"4317","secret":false},{"name":"OTLP_HTTP_PORT","label":"OTLP HTTP Port","required":false,"default":"4318","secret":false}]"#,
        ),
        (
            "tpl-healthchecks",
            "Healthchecks.io",
            "Cron job and scheduled task monitoring. Get alerted when your cron jobs fail to run on time.",
            "monitoring",
            "healthchecks",
            r#"services:
  healthchecks:
    image: healthchecks/healthchecks:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-healthchecks}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - DEBUG=False
      - SECRET_KEY=${SECRET_KEY:-change-me-to-a-long-random-string}
      - SITE_ROOT=${SITE_ROOT:-http://localhost:8000}
      - SITE_NAME=${SITE_NAME:-Healthchecks}
      - DEFAULT_FROM_EMAIL=${FROM_EMAIL:-healthchecks@example.com}
      - EMAIL_HOST=${EMAIL_HOST:-smtp.example.com}
      - EMAIL_PORT=${EMAIL_PORT:-587}
      - EMAIL_USE_TLS=${EMAIL_TLS:-True}
      - DB=postgres
      - DB_HOST=healthchecks_db
      - DB_PORT=5432
      - DB_NAME=healthchecks
      - DB_USER=healthchecks
      - DB_PASSWORD=${DB_PASSWORD:-healthchecks}
      - REGISTRATION_OPEN=False
    depends_on:
      - healthchecks_db
    labels:
      - "rivetr.managed=true"

  healthchecks_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=healthchecks
      - POSTGRES_PASSWORD=${DB_PASSWORD:-healthchecks}
      - POSTGRES_DB=healthchecks
    volumes:
      - healthchecks_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  healthchecks_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"healthchecks","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"SITE_ROOT","label":"Site Root URL","required":true,"default":"http://localhost:8000","secret":false},{"name":"SITE_NAME","label":"Site Name","required":false,"default":"Healthchecks","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"FROM_EMAIL","label":"From Email","required":false,"default":"healthchecks@example.com","secret":false},{"name":"EMAIL_HOST","label":"SMTP Host","required":false,"default":"smtp.example.com","secret":false},{"name":"EMAIL_PORT","label":"SMTP Port","required":false,"default":"587","secret":false}]"#,
        ),
    ]
}
