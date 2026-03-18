//! Sprint 24 service templates: File Storage, Communication, Security,
//! Monitoring, AI/ML, Productivity, Business, and Automation
//!
//! Already present in earlier seeder files (skipped):
//! - Nextcloud (infrastructure.rs as nextcloud)
//! - Mattermost (cms_communication.rs as mattermost)
//! - Uptime Kuma (infrastructure.rs as uptime-kuma)
//! - Invoice Ninja (business.rs as tpl-invoice-ninja / tpl-invoice-ninja-v5)
//!
//! New templates added: Vaultwarden, LiteLLM, Matrix Synapse, Rocket.Chat,
//! Joplin Server, Hatchet, Zipline, NodeBB, MindsDB, Siyuan Notes,
//! EasyAppointments

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== SECURITY ====================
        (
            "tpl-vaultwarden",
            "Vaultwarden",
            "Lightweight, unofficial Bitwarden-compatible server written in Rust. Store and sync passwords, secure notes, and TOTP codes. Works with all official Bitwarden clients. Fraction of the resource usage of the official server.",
            "Security",
            "vaultwarden",
            r#"services:
  vaultwarden:
    image: vaultwarden/server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-vaultwarden}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - ADMIN_TOKEN=${ADMIN_TOKEN}
      - SIGNUPS_ALLOWED=${SIGNUPS_ALLOWED:-true}
      - INVITATIONS_ALLOWED=${INVITATIONS_ALLOWED:-true}
      - DOMAIN=${DOMAIN:-http://localhost}
      - SMTP_HOST=${SMTP_HOST:-}
      - SMTP_FROM=${SMTP_FROM:-}
      - SMTP_PORT=${SMTP_PORT:-587}
      - SMTP_SECURITY=${SMTP_SECURITY:-starttls}
      - SMTP_USERNAME=${SMTP_USERNAME:-}
      - SMTP_PASSWORD=${SMTP_PASSWORD:-}
      - LOG_LEVEL=${LOG_LEVEL:-warn}
    volumes:
      - vaultwarden_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  vaultwarden_data:
"#,
            r#"[{"name":"ADMIN_TOKEN","label":"Admin Token (access /admin panel — generate with: openssl rand -base64 48)","required":true,"default":"","secret":true},{"name":"DOMAIN","label":"Domain URL (e.g. https://vault.example.com)","required":false,"default":"http://localhost","secret":false},{"name":"SIGNUPS_ALLOWED","label":"Allow User Sign-Ups","required":false,"default":"true","secret":false},{"name":"INVITATIONS_ALLOWED","label":"Allow Invitations","required":false,"default":"true","secret":false},{"name":"SMTP_HOST","label":"SMTP Host (optional, for email)","required":false,"default":"","secret":false},{"name":"SMTP_FROM","label":"SMTP From Address","required":false,"default":"","secret":false},{"name":"SMTP_PORT","label":"SMTP Port","required":false,"default":"587","secret":false},{"name":"SMTP_SECURITY","label":"SMTP Security (starttls, force_tls, off)","required":false,"default":"starttls","secret":false},{"name":"SMTP_USERNAME","label":"SMTP Username","required":false,"default":"","secret":false},{"name":"SMTP_PASSWORD","label":"SMTP Password","required":false,"default":"","secret":true},{"name":"LOG_LEVEL","label":"Log Level","required":false,"default":"warn","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"vaultwarden","secret":false}]"#,
        ),
        // ==================== AI / ML ====================
        (
            "tpl-litellm",
            "LiteLLM",
            "OpenAI-compatible LLM proxy gateway. Route requests to 100+ LLM providers (OpenAI, Anthropic, Mistral, Ollama, etc.) with a unified API. Supports load balancing, rate limiting, budgets, and detailed logging.",
            "AI/ML",
            "litellm",
            r#"services:
  litellm:
    image: ghcr.io/berriai/litellm:${VERSION:-main-latest}
    container_name: ${CONTAINER_NAME:-litellm}
    restart: unless-stopped
    ports:
      - "${PORT:-4000}:4000"
    environment:
      - LITELLM_MASTER_KEY=${LITELLM_MASTER_KEY}
      - LITELLM_SALT_KEY=${LITELLM_SALT_KEY:-}
      - DATABASE_URL=${DATABASE_URL:-}
      - STORE_MODEL_IN_DB=${STORE_MODEL_IN_DB:-True}
      - LITELLM_LOG=${LITELLM_LOG:-INFO}
      - UI_USERNAME=${UI_USERNAME:-admin}
      - UI_PASSWORD=${UI_PASSWORD:-}
    volumes:
      - litellm_data:/app/data
    labels:
      - "rivetr.managed=true"

volumes:
  litellm_data:
"#,
            r#"[{"name":"LITELLM_MASTER_KEY","label":"Master Key (prefix with sk- e.g. sk-my-master-key)","required":true,"default":"","secret":true},{"name":"LITELLM_SALT_KEY","label":"Salt Key for encryption (generate with: openssl rand -hex 32)","required":false,"default":"","secret":true},{"name":"DATABASE_URL","label":"PostgreSQL Database URL (optional, for persistent config)","required":false,"default":"","secret":true},{"name":"STORE_MODEL_IN_DB","label":"Store Model Config in DB","required":false,"default":"True","secret":false},{"name":"UI_USERNAME","label":"UI Username","required":false,"default":"admin","secret":false},{"name":"UI_PASSWORD","label":"UI Password","required":false,"default":"","secret":true},{"name":"LITELLM_LOG","label":"Log Level (DEBUG, INFO, WARNING, ERROR)","required":false,"default":"INFO","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"4000","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"main-latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"litellm","secret":false}]"#,
        ),
        (
            "tpl-mindsdb",
            "MindsDB",
            "AI-powered SQL database layer. Query machine learning models and AI capabilities using standard SQL. Connect to existing databases and add predictions, NLP, and time-series forecasting without leaving SQL.",
            "AI/ML",
            "mindsdb",
            r#"services:
  mindsdb:
    image: mindsdb/mindsdb:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-mindsdb}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-47334}:47334"
      - "${MYSQL_PORT:-47335}:47335"
      - "${MONGO_PORT:-47336}:47336"
    environment:
      - MINDSDB_STORAGE_PATH=/root/mindsdb_storage
    volumes:
      - mindsdb_data:/root/mindsdb_storage
    labels:
      - "rivetr.managed=true"

volumes:
  mindsdb_data:
"#,
            r#"[{"name":"HTTP_PORT","label":"HTTP API Port","required":false,"default":"47334","secret":false},{"name":"MYSQL_PORT","label":"MySQL Protocol Port","required":false,"default":"47335","secret":false},{"name":"MONGO_PORT","label":"MongoDB Protocol Port","required":false,"default":"47336","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mindsdb","secret":false}]"#,
        ),
        // ==================== COMMUNICATION ====================
        (
            "tpl-matrix-synapse",
            "Matrix Synapse",
            "Reference homeserver for the Matrix open communication protocol. Powers decentralized, end-to-end encrypted messaging. Compatible with Element and other Matrix clients. Supports federation with other homeservers.",
            "Communication",
            "matrix-synapse",
            r#"services:
  matrix-synapse:
    image: matrixdotorg/synapse:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-matrix-synapse}
    restart: unless-stopped
    ports:
      - "${PORT:-8008}:8008"
    environment:
      - SYNAPSE_SERVER_NAME=${SYNAPSE_SERVER_NAME}
      - SYNAPSE_REPORT_STATS=${SYNAPSE_REPORT_STATS:-no}
      - SYNAPSE_HTTP_PORT=8008
      - UID=991
      - GID=991
    volumes:
      - synapse_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  synapse_data:
"#,
            r#"[{"name":"SYNAPSE_SERVER_NAME","label":"Server Name (your domain, e.g. example.com)","required":true,"default":"","secret":false},{"name":"SYNAPSE_REPORT_STATS","label":"Report Anonymous Stats to Matrix.org (yes/no)","required":false,"default":"no","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8008","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"matrix-synapse","secret":false}]"#,
        ),
        (
            "tpl-rocketchat",
            "Rocket.Chat",
            "Open-source team messaging and collaboration platform. Supports channels, direct messages, video conferencing, file sharing, and integrations. Self-hosted alternative to Slack. Requires MongoDB.",
            "Communication",
            "rocketchat",
            r#"services:
  rocketchat:
    image: rocket.chat:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-rocketchat}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - MONGO_URL=mongodb://rocketchat_db:27017/rocketchat
      - MONGO_OPLOG_URL=mongodb://rocketchat_db:27017/local
      - ROOT_URL=${ROOT_URL:-http://localhost:3000}
      - PORT=3000
      - DEPLOY_PLATFORM=docker
      - OVERWRITE_SETTING_Show_Setup_Wizard=${SHOW_SETUP_WIZARD:-pending}
    depends_on:
      - rocketchat_db
    labels:
      - "rivetr.managed=true"

  rocketchat_db:
    image: mongo:${MONGO_VERSION:-6.0}
    container_name: ${CONTAINER_NAME:-rocketchat}-db
    restart: unless-stopped
    command: mongod --oplogSize 128 --replSet rs0
    volumes:
      - rocketchat_db_data:/data/db
    labels:
      - "rivetr.managed=true"

  rocketchat_db_init:
    image: mongo:${MONGO_VERSION:-6.0}
    command: >
      bash -c "sleep 10 && mongosh mongodb://rocketchat_db:27017 --eval \"rs.initiate({_id: 'rs0', members: [{_id: 0, host: 'rocketchat_db:27017'}]})\""
    depends_on:
      - rocketchat_db
    labels:
      - "rivetr.managed=true"

volumes:
  rocketchat_db_data:
"#,
            r#"[{"name":"ROOT_URL","label":"Root URL (e.g. https://chat.example.com)","required":false,"default":"http://localhost:3000","secret":false},{"name":"SHOW_SETUP_WIZARD","label":"Setup Wizard State (pending, in_progress, completed)","required":false,"default":"pending","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false},{"name":"VERSION","label":"Rocket.Chat Image Version","required":false,"default":"latest","secret":false},{"name":"MONGO_VERSION","label":"MongoDB Image Version","required":false,"default":"6.0","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"rocketchat","secret":false}]"#,
        ),
        (
            "tpl-nodebb",
            "NodeBB",
            "Modern, high-performance forum software built on Node.js. Features real-time notifications, mobile-responsive UI, rich text editor, and plugin ecosystem. Requires Redis or MongoDB as data store.",
            "Communication",
            "nodebb",
            r#"services:
  nodebb:
    image: nodebb/docker:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-nodebb}
    restart: unless-stopped
    ports:
      - "${PORT:-4567}:4567"
    environment:
      - REDIS_HOST=nodebb_redis
      - REDIS_PORT=6379
      - REDIS_PASSWORD=${REDIS_PASSWORD:-}
      - NODE_ENV=${NODE_ENV:-production}
      - SETUP=${NODEBB_SETUP:-true}
    volumes:
      - nodebb_data:/usr/src/app
      - nodebb_uploads:/usr/src/app/public/uploads
    depends_on:
      - nodebb_redis
    labels:
      - "rivetr.managed=true"

  nodebb_redis:
    image: redis:7-alpine
    container_name: ${CONTAINER_NAME:-nodebb}-redis
    restart: unless-stopped
    command: >
      redis-server ${REDIS_PASSWORD:+--requirepass $REDIS_PASSWORD}
    volumes:
      - nodebb_redis_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  nodebb_data:
  nodebb_uploads:
  nodebb_redis_data:
"#,
            r#"[{"name":"REDIS_PASSWORD","label":"Redis Password (optional)","required":false,"default":"","secret":true},{"name":"NODE_ENV","label":"Node Environment","required":false,"default":"production","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"4567","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"nodebb","secret":false}]"#,
        ),
        // ==================== FILE STORAGE ====================
        (
            "tpl-zipline",
            "Zipline",
            "Fast, feature-rich file/image upload server compatible with ShareX and other upload tools. Supports OAuth, custom domains, URL shortening, and image transformations. PostgreSQL-backed storage.",
            "File Storage",
            "zipline",
            r#"services:
  zipline:
    image: ghcr.io/diced/zipline:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-zipline}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - CORE_SECRET=${CORE_SECRET}
      - CORE_DATABASE_URL=postgresql://${DB_USER:-zipline}:${DB_PASSWORD}@zipline_db:5432/${DB_NAME:-zipline}
      - CORE_HOST=0.0.0.0
      - CORE_PORT=3000
      - DATASOURCE_TYPE=${DATASOURCE_TYPE:-local}
      - DATASOURCE_LOCAL_DIRECTORY=/zipline/uploads
    volumes:
      - zipline_uploads:/zipline/uploads
    depends_on:
      - zipline_db
    labels:
      - "rivetr.managed=true"

  zipline_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-zipline}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-zipline}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=${DB_NAME:-zipline}
    volumes:
      - zipline_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  zipline_uploads:
  zipline_db_data:
"#,
            r#"[{"name":"CORE_SECRET","label":"Core Secret (generate with: openssl rand -hex 32)","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"PostgreSQL Password","required":true,"default":"","secret":true},{"name":"DB_USER","label":"PostgreSQL Username","required":false,"default":"zipline","secret":false},{"name":"DB_NAME","label":"PostgreSQL Database Name","required":false,"default":"zipline","secret":false},{"name":"DATASOURCE_TYPE","label":"Storage Type (local, s3)","required":false,"default":"local","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"zipline","secret":false}]"#,
        ),
        // ==================== PRODUCTIVITY ====================
        (
            "tpl-joplin-server",
            "Joplin Server",
            "Sync server for the Joplin note-taking application. Sync notes, notebooks, and attachments across all your devices. Supports end-to-end encryption. PostgreSQL-backed.",
            "Productivity",
            "joplin-server",
            r#"services:
  joplin-server:
    image: joplin/server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-joplin-server}
    restart: unless-stopped
    ports:
      - "${PORT:-22300}:22300"
    environment:
      - APP_BASE_URL=${APP_BASE_URL:-http://localhost:22300}
      - APP_PORT=22300
      - DB_CLIENT=pg
      - POSTGRES_HOST=joplin_db
      - POSTGRES_PORT=5432
      - POSTGRES_USER=${DB_USER:-joplin}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DATABASE=${DB_NAME:-joplin}
      - MAX_TIME_DRIFT=0
    depends_on:
      - joplin_db
    labels:
      - "rivetr.managed=true"

  joplin_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-joplin}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-joplin}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=${DB_NAME:-joplin}
    volumes:
      - joplin_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  joplin_db_data:
"#,
            r#"[{"name":"APP_BASE_URL","label":"Base URL (e.g. https://joplin.example.com:22300)","required":true,"default":"http://localhost:22300","secret":false},{"name":"DB_PASSWORD","label":"PostgreSQL Password","required":true,"default":"","secret":true},{"name":"DB_USER","label":"PostgreSQL Username","required":false,"default":"joplin","secret":false},{"name":"DB_NAME","label":"PostgreSQL Database Name","required":false,"default":"joplin","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"22300","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"joplin-server","secret":false}]"#,
        ),
        (
            "tpl-siyuan",
            "Siyuan Notes",
            "Privacy-first, local-first personal knowledge management system. Features bidirectional links, block-level references, full-text search, and an export-friendly data format. Can self-host for multi-device sync.",
            "Productivity",
            "siyuan",
            r#"services:
  siyuan:
    image: b3log/siyuan:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-siyuan}
    restart: unless-stopped
    ports:
      - "${PORT:-6806}:6806"
    command:
      - "--workspace=/siyuan/workspace"
      - "--accessAuthCode=${ACCESS_AUTH_CODE}"
    volumes:
      - siyuan_workspace:/siyuan/workspace
    labels:
      - "rivetr.managed=true"

volumes:
  siyuan_workspace:
"#,
            r#"[{"name":"ACCESS_AUTH_CODE","label":"Access Auth Code (password to access the web UI)","required":true,"default":"","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"6806","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"siyuan","secret":false}]"#,
        ),
        // ==================== AUTOMATION ====================
        (
            "tpl-hatchet",
            "Hatchet",
            "Developer-first workflow orchestration engine. Run durable, distributed background jobs with retries, scheduling, and real-time visibility. TypeScript SDK with fan-out/fan-in, concurrency controls, and event-driven triggers.",
            "Automation",
            "hatchet",
            r#"services:
  hatchet-engine:
    image: ghcr.io/hatchet-dev/hatchet/hatchet-engine:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-hatchet-engine}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-8080}:8080"
      - "${GRPC_PORT:-7070}:7070"
    environment:
      - DATABASE_URL=postgresql://${DB_USER:-hatchet}:${DB_PASSWORD}@hatchet_db:5432/${DB_NAME:-hatchet}?sslmode=disable
      - SERVER_AUTH_COOKIE_SECRETS=${SERVER_AUTH_COOKIE_SECRETS}
      - SERVER_AUTH_COOKIE_DOMAIN=${SERVER_AUTH_COOKIE_DOMAIN:-localhost}
      - SERVER_URL=${SERVER_URL:-http://localhost:8080}
      - SERVER_GRPC_BIND_ADDRESS=0.0.0.0
      - SERVER_GRPC_PORT=7070
      - SERVER_GRPC_BROADCAST_ADDRESS=${SERVER_GRPC_BROADCAST_ADDRESS:-localhost:7070}
      - SERVER_TASKQUEUE_RABBITMQ_URL=amqp://hatchet_rabbitmq:5672
      - CACHE_URL=redis://hatchet_redis:6379
    depends_on:
      - hatchet_db
      - hatchet_redis
      - hatchet_rabbitmq
    labels:
      - "rivetr.managed=true"

  hatchet_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-hatchet}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-hatchet}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=${DB_NAME:-hatchet}
    volumes:
      - hatchet_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  hatchet_redis:
    image: redis:7-alpine
    container_name: ${CONTAINER_NAME:-hatchet}-redis
    restart: unless-stopped
    volumes:
      - hatchet_redis_data:/data
    labels:
      - "rivetr.managed=true"

  hatchet_rabbitmq:
    image: rabbitmq:3-management-alpine
    container_name: ${CONTAINER_NAME:-hatchet}-rabbitmq
    restart: unless-stopped
    ports:
      - "${RABBITMQ_MGMT_PORT:-15672}:15672"
    volumes:
      - hatchet_rabbitmq_data:/var/lib/rabbitmq
    labels:
      - "rivetr.managed=true"

volumes:
  hatchet_db_data:
  hatchet_redis_data:
  hatchet_rabbitmq_data:
"#,
            r#"[{"name":"SERVER_AUTH_COOKIE_SECRETS","label":"Cookie Secrets (comma-separated, generate with: openssl rand -hex 16,openssl rand -hex 16)","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"PostgreSQL Password","required":true,"default":"","secret":true},{"name":"SERVER_URL","label":"Server URL (e.g. https://hatchet.example.com)","required":false,"default":"http://localhost:8080","secret":false},{"name":"SERVER_AUTH_COOKIE_DOMAIN","label":"Cookie Domain (e.g. example.com)","required":false,"default":"localhost","secret":false},{"name":"SERVER_GRPC_BROADCAST_ADDRESS","label":"gRPC Broadcast Address (host:port for workers to connect)","required":false,"default":"localhost:7070","secret":false},{"name":"DB_USER","label":"PostgreSQL Username","required":false,"default":"hatchet","secret":false},{"name":"DB_NAME","label":"PostgreSQL Database Name","required":false,"default":"hatchet","secret":false},{"name":"HTTP_PORT","label":"HTTP Host Port","required":false,"default":"8080","secret":false},{"name":"GRPC_PORT","label":"gRPC Host Port","required":false,"default":"7070","secret":false},{"name":"RABBITMQ_MGMT_PORT","label":"RabbitMQ Management UI Port","required":false,"default":"15672","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"hatchet","secret":false}]"#,
        ),
        // ==================== BUSINESS ====================
        (
            "tpl-easyappointments",
            "EasyAppointments",
            "Open-source online appointment scheduling application. Customers book appointments through a clean web interface while admins manage services, staff, and working hours. MySQL-backed.",
            "Business",
            "easyappointments",
            r#"services:
  easyappointments:
    image: alextselegidis/easyappointments:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-easyappointments}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - BASE_URL=${BASE_URL:-http://localhost}
      - DB_HOST=easyappointments_db
      - DB_PORT=3306
      - DB_NAME=${DB_NAME:-easyappointments}
      - DB_USERNAME=${DB_USER:-easyappointments}
      - DB_PASSWORD=${DB_PASSWORD}
      - DEBUG_MODE=${DEBUG_MODE:-FALSE}
    depends_on:
      - easyappointments_db
    labels:
      - "rivetr.managed=true"

  easyappointments_db:
    image: mysql:8.0
    container_name: ${CONTAINER_NAME:-easyappointments}-db
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-rootpassword}
      - MYSQL_DATABASE=${DB_NAME:-easyappointments}
      - MYSQL_USER=${DB_USER:-easyappointments}
      - MYSQL_PASSWORD=${DB_PASSWORD}
    volumes:
      - easyappointments_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  easyappointments_db_data:
"#,
            r#"[{"name":"DB_PASSWORD","label":"MySQL Password","required":true,"default":"","secret":true},{"name":"DB_ROOT_PASSWORD","label":"MySQL Root Password","required":false,"default":"rootpassword","secret":true},{"name":"BASE_URL","label":"Base URL (e.g. https://appointments.example.com)","required":false,"default":"http://localhost","secret":false},{"name":"DB_USER","label":"MySQL Username","required":false,"default":"easyappointments","secret":false},{"name":"DB_NAME","label":"MySQL Database Name","required":false,"default":"easyappointments","secret":false},{"name":"DEBUG_MODE","label":"Debug Mode (TRUE/FALSE)","required":false,"default":"FALSE","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"easyappointments","secret":false}]"#,
        ),
    ]
}
