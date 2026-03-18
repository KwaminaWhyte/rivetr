//! Sprint 21 service templates: productivity, monitoring, utilities, and developer tools
//! NOTE: Duplicates removed — beszel, changedetection, classicpress (cockpit-cms), docmost,
//! excalidraw, listmonk, mealie, ntfy, penpot, rallly, searxng, silverbullet already exist
//! in earlier seeder files.
//!
//! New templates added: beszel-agent, classicpress, cloudbeaver, diun, homebox,
//! karakeep, linkding, pairdrop, readeck, ryot, shlink, slash, wakapi

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== MONITORING ====================
        (
            "tpl-beszel-agent",
            "Beszel Agent",
            "Lightweight monitoring agent for Beszel. Runs on each server you want to monitor and sends metrics to your Beszel hub.",
            "Monitoring",
            "beszel-agent",
            r#"services:
  beszel-agent:
    image: henrygd/beszel-agent:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-beszel-agent}
    restart: unless-stopped
    network_mode: host
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
    environment:
      - PORT=${AGENT_PORT:-45876}
      - KEY=${HUB_PUBLIC_KEY}
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"AGENT_PORT","label":"Agent Port","required":false,"default":"45876","secret":false},{"name":"HUB_PUBLIC_KEY","label":"Beszel Hub Public Key","required":true,"default":"","secret":true},{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"beszel-agent","secret":false}]"#,
        ),
        // ==================== CMS ====================
        (
            "tpl-classicpress",
            "ClassicPress",
            "A fork of WordPress without the Gutenberg block editor. Stable, business-focused CMS with full WordPress plugin compatibility and a classic editing experience.",
            "CMS",
            "classicpress",
            r#"services:
  classicpress:
    image: classicpress/classicpress:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-classicpress}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - WORDPRESS_DB_HOST=classicpress_db
      - WORDPRESS_DB_NAME=${DB_NAME:-classicpress}
      - WORDPRESS_DB_USER=${DB_USER:-classicpress}
      - WORDPRESS_DB_PASSWORD=${DB_PASSWORD:-classicpress}
      - WORDPRESS_TABLE_PREFIX=${TABLE_PREFIX:-cp_}
    volumes:
      - classicpress_data:/var/www/html
    depends_on:
      - classicpress_db
    labels:
      - "rivetr.managed=true"

  classicpress_db:
    image: mysql:8.0
    container_name: ${CONTAINER_NAME:-classicpress}-db
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=${DB_NAME:-classicpress}
      - MYSQL_USER=${DB_USER:-classicpress}
      - MYSQL_PASSWORD=${DB_PASSWORD:-classicpress}
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-rootpassword}
    volumes:
      - classicpress_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  classicpress_data:
  classicpress_db_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false},{"name":"DB_NAME","label":"Database Name","required":false,"default":"classicpress","secret":false},{"name":"DB_USER","label":"Database User","required":false,"default":"classicpress","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"DB_ROOT_PASSWORD","label":"MySQL Root Password","required":true,"default":"","secret":true},{"name":"TABLE_PREFIX","label":"Table Prefix","required":false,"default":"cp_","secret":false},{"name":"VERSION","label":"ClassicPress Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"classicpress","secret":false}]"#,
        ),
        // ==================== DATABASE TOOLS ====================
        (
            "tpl-cloudbeaver",
            "CloudBeaver",
            "Cloud-based database management UI. Browse and edit databases through a web interface — supports PostgreSQL, MySQL, SQLite, and dozens of other databases.",
            "Database",
            "cloudbeaver",
            r#"services:
  cloudbeaver:
    image: dbeaver/cloudbeaver:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-cloudbeaver}
    restart: unless-stopped
    ports:
      - "${PORT:-8978}:8978"
    volumes:
      - cloudbeaver_data:/opt/cloudbeaver/workspace
    labels:
      - "rivetr.managed=true"

volumes:
  cloudbeaver_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8978","secret":false},{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"cloudbeaver","secret":false}]"#,
        ),
        // ==================== INFRASTRUCTURE ====================
        (
            "tpl-diun",
            "Diun",
            "Docker Image Update Notifier. Receives notifications when a Docker image is updated on a registry. Supports Slack, Discord, email, Telegram, and more.",
            "Infrastructure",
            "diun",
            r#"services:
  diun:
    image: crazymax/diun:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-diun}
    restart: unless-stopped
    volumes:
      - diun_data:/data
      - /var/run/docker.sock:/var/run/docker.sock
    environment:
      - LOG_LEVEL=${LOG_LEVEL:-info}
      - LOG_JSON=false
      - DIUN_WATCH_WORKERS=${WATCH_WORKERS:-20}
      - DIUN_WATCH_SCHEDULE=${WATCH_SCHEDULE:-0 */6 * * *}
      - DIUN_PROVIDERS_DOCKER=true
      - DIUN_PROVIDERS_DOCKER_WATCHBYDEFAULT=${WATCH_BY_DEFAULT:-false}
    labels:
      - "rivetr.managed=true"

volumes:
  diun_data:
"#,
            r#"[{"name":"WATCH_SCHEDULE","label":"Watch Schedule (cron)","required":false,"default":"0 */6 * * *","secret":false},{"name":"WATCH_WORKERS","label":"Watch Workers","required":false,"default":"20","secret":false},{"name":"WATCH_BY_DEFAULT","label":"Watch All Containers by Default","required":false,"default":"false","secret":false},{"name":"LOG_LEVEL","label":"Log Level","required":false,"default":"info","secret":false},{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"diun","secret":false}]"#,
        ),
        // ==================== PRODUCTIVITY ====================
        (
            "tpl-homebox",
            "Homebox",
            "Inventory and organization system for the home. Track your belongings, create labels, and manage warranties — simple and focused on the home use case.",
            "Productivity",
            "homebox",
            r#"services:
  homebox:
    image: ghcr.io/hay-kot/homebox:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-homebox}
    restart: unless-stopped
    ports:
      - "${PORT:-7745}:7745"
    environment:
      - HBOX_LOG_LEVEL=${LOG_LEVEL:-info}
      - HBOX_LOG_FORMAT=text
      - HBOX_WEB_MAX_UPLOAD_SIZE=${MAX_UPLOAD_MB:-10}
    volumes:
      - homebox_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  homebox_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"7745","secret":false},{"name":"LOG_LEVEL","label":"Log Level","required":false,"default":"info","secret":false},{"name":"MAX_UPLOAD_MB","label":"Max Upload Size (MB)","required":false,"default":"10","secret":false},{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"homebox","secret":false}]"#,
        ),
        (
            "tpl-karakeep",
            "Karakeep",
            "A self-hostable bookmark manager (formerly Hoarder) with AI-powered automatic tagging. Save links, notes, and images with full-text search and mobile apps.",
            "Productivity",
            "karakeep",
            r#"services:
  karakeep:
    image: ghcr.io/karakeep-app/karakeep:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-karakeep}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - NEXTAUTH_SECRET=${NEXTAUTH_SECRET}
      - NEXTAUTH_URL=${NEXTAUTH_URL:-http://localhost:3000}
      - DATA_DIR=/data
      - MEILI_ADDR=http://karakeep_meilisearch:7700
      - MEILI_MASTER_KEY=${MEILI_MASTER_KEY}
      - OPENAI_API_KEY=${OPENAI_API_KEY:-}
      - INFERENCE_LANG=${INFERENCE_LANG:-english}
    volumes:
      - karakeep_data:/data
    depends_on:
      - karakeep_meilisearch
    labels:
      - "rivetr.managed=true"

  karakeep_meilisearch:
    image: getmeili/meilisearch:latest
    container_name: ${CONTAINER_NAME:-karakeep}-meilisearch
    restart: unless-stopped
    environment:
      - MEILI_MASTER_KEY=${MEILI_MASTER_KEY}
      - MEILI_NO_ANALYTICS=true
    volumes:
      - karakeep_meilisearch_data:/meili_data
    labels:
      - "rivetr.managed=true"

volumes:
  karakeep_data:
  karakeep_meilisearch_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false},{"name":"NEXTAUTH_SECRET","label":"NextAuth Secret","required":true,"default":"","secret":true},{"name":"NEXTAUTH_URL","label":"NextAuth URL","required":false,"default":"http://localhost:3000","secret":false},{"name":"MEILI_MASTER_KEY","label":"MeiliSearch Master Key","required":true,"default":"","secret":true},{"name":"OPENAI_API_KEY","label":"OpenAI API Key (optional, for AI tagging)","required":false,"default":"","secret":true},{"name":"INFERENCE_LANG","label":"Inference Language","required":false,"default":"english","secret":false},{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"karakeep","secret":false}]"#,
        ),
        (
            "tpl-linkding",
            "Linkding",
            "A minimal self-hosted bookmark manager. Clean, fast, and simple — tag bookmarks, full-text search, browser extensions, import/export, and a REST API.",
            "Productivity",
            "linkding",
            r#"services:
  linkding:
    image: sissbruecker/linkding:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-linkding}
    restart: unless-stopped
    ports:
      - "${PORT:-9090}:9090"
    environment:
      - LD_SUPERUSER_NAME=${ADMIN_USER:-admin}
      - LD_SUPERUSER_PASSWORD=${ADMIN_PASSWORD}
      - LD_ENABLE_AUTH_PROXY=${ENABLE_AUTH_PROXY:-False}
      - LD_DISABLE_BACKGROUND_TASKS=${DISABLE_BACKGROUND_TASKS:-False}
      - LD_DISABLE_URL_VALIDATION=${DISABLE_URL_VALIDATION:-False}
    volumes:
      - linkding_data:/etc/linkding/data
    labels:
      - "rivetr.managed=true"

volumes:
  linkding_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"9090","secret":false},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"linkding","secret":false}]"#,
        ),
        (
            "tpl-pairdrop",
            "PairDrop",
            "Local file sharing in your browser. AirDrop-like experience for all operating systems on the local network. No account, no cloud, no limits.",
            "Productivity",
            "pairdrop",
            r#"services:
  pairdrop:
    image: lscr.io/linuxserver/pairdrop:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-pairdrop}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - PUID=${PUID:-1000}
      - PGID=${PGID:-1000}
      - TZ=${TZ:-UTC}
      - RATE_LIMIT=${RATE_LIMIT:-false}
      - WS_FALLBACK=${WS_FALLBACK:-false}
      - RTC_CONFIG=${RTC_CONFIG:-}
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"RATE_LIMIT","label":"Enable Rate Limiting","required":false,"default":"false","secret":false},{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"pairdrop","secret":false}]"#,
        ),
        (
            "tpl-readeck",
            "Readeck",
            "A read-it-later app that saves web articles as clean, readable versions. Supports full-text search, tags, and a browser extension for quick saves.",
            "Productivity",
            "readeck",
            r#"services:
  readeck:
    image: codeberg.org/readeck/readeck:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-readeck}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - READECK_SERVER_HOST=0.0.0.0
      - READECK_SERVER_PORT=8000
      - READECK_SERVER_PREFIX=${SERVER_PREFIX:-/}
      - READECK_LOG_LEVEL=${LOG_LEVEL:-info}
      - READECK_DATABASE_SOURCE=/readeck/readeck.db
    volumes:
      - readeck_data:/readeck
    labels:
      - "rivetr.managed=true"

volumes:
  readeck_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8000","secret":false},{"name":"SERVER_PREFIX","label":"URL Prefix","required":false,"default":"/","secret":false},{"name":"LOG_LEVEL","label":"Log Level","required":false,"default":"info","secret":false},{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"readeck","secret":false}]"#,
        ),
        (
            "tpl-ryot",
            "Ryot",
            "Track everything in your life — anime, books, movies, shows, video games, podcasts, and more. Open-source alternative to Goodreads and Trakt.",
            "Productivity",
            "ryot",
            r#"services:
  ryot:
    image: ghcr.io/ignisda/ryot:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-ryot}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - DATABASE_URL=postgres://${DB_USER:-ryot}:${DB_PASSWORD:-ryot}@ryot_db:5432/${DB_NAME:-ryot}
      - SERVER_INSECURE_COOKIE=${INSECURE_COOKIE:-true}
    depends_on:
      - ryot_db
    labels:
      - "rivetr.managed=true"

  ryot_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-ryot}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-ryot}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-ryot}
      - POSTGRES_DB=${DB_NAME:-ryot}
    volumes:
      - ryot_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  ryot_db_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8000","secret":false},{"name":"DB_USER","label":"Database User","required":false,"default":"ryot","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"DB_NAME","label":"Database Name","required":false,"default":"ryot","secret":false},{"name":"INSECURE_COOKIE","label":"Insecure Cookie (set false if using HTTPS)","required":false,"default":"true","secret":false},{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"ryot","secret":false}]"#,
        ),
        // ==================== DEVELOPER TOOLS ====================
        (
            "tpl-shlink",
            "Shlink",
            "Self-hosted URL shortener with analytics. Features QR codes, tag management, visit statistics, REST API, and a separate web UI (Shlink Web Client).",
            "Development",
            "shlink",
            r#"services:
  shlink:
    image: shlinkio/shlink:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-shlink}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DEFAULT_DOMAIN=${DEFAULT_DOMAIN:-localhost:8080}
      - IS_HTTPS_ENABLED=${IS_HTTPS:-false}
      - DB_DRIVER=postgres
      - DB_HOST=shlink_db
      - DB_PORT=5432
      - DB_NAME=${DB_NAME:-shlink}
      - DB_USER=${DB_USER:-shlink}
      - DB_PASSWORD=${DB_PASSWORD:-shlink}
      - INITIAL_API_KEY=${INITIAL_API_KEY}
    depends_on:
      - shlink_db
    labels:
      - "rivetr.managed=true"

  shlink_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-shlink}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-shlink}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-shlink}
      - POSTGRES_DB=${DB_NAME:-shlink}
    volumes:
      - shlink_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  shlink_db_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false},{"name":"DEFAULT_DOMAIN","label":"Default Domain (e.g. s.example.com)","required":true,"default":"localhost:8080","secret":false},{"name":"IS_HTTPS","label":"HTTPS Enabled","required":false,"default":"false","secret":false},{"name":"INITIAL_API_KEY","label":"Initial API Key","required":true,"default":"","secret":true},{"name":"DB_USER","label":"Database User","required":false,"default":"shlink","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"DB_NAME","label":"Database Name","required":false,"default":"shlink","secret":false},{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"shlink","secret":false}]"#,
        ),
        (
            "tpl-slash",
            "Slash",
            "An open source, self-hosted URL shortener and bookmark manager. Create short links, organize bookmarks with tags, and share collections with teams.",
            "Development",
            "slash",
            r#"services:
  slash:
    image: yourselfhosted/slash:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-slash}
    restart: unless-stopped
    ports:
      - "${PORT:-5231}:5231"
    environment:
      - SLASH_SECRET_KEY=${SECRET_KEY}
    volumes:
      - slash_data:/var/opt/slash
    labels:
      - "rivetr.managed=true"

volumes:
  slash_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"5231","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"slash","secret":false}]"#,
        ),
        (
            "tpl-wakapi",
            "Wakapi",
            "Self-hosted WakaTime-compatible coding time tracker. Collects statistics from your editor plugins (VS Code, JetBrains, Vim, etc.) and shows detailed dashboards.",
            "Development",
            "wakapi",
            r#"services:
  wakapi:
    image: ghcr.io/muety/wakapi:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-wakapi}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - WAKAPI_DB_TYPE=postgres
      - WAKAPI_DB_HOST=wakapi_db
      - WAKAPI_DB_PORT=5432
      - WAKAPI_DB_NAME=${DB_NAME:-wakapi}
      - WAKAPI_DB_USER=${DB_USER:-wakapi}
      - WAKAPI_DB_PASSWORD=${DB_PASSWORD:-wakapi}
      - WAKAPI_PASSWORD_SALT=${PASSWORD_SALT}
      - WAKAPI_ALLOW_SIGNUP=${ALLOW_SIGNUP:-true}
      - WAKAPI_PUBLIC_URL=${PUBLIC_URL:-http://localhost:3000}
    depends_on:
      - wakapi_db
    labels:
      - "rivetr.managed=true"

  wakapi_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-wakapi}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-wakapi}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-wakapi}
      - POSTGRES_DB=${DB_NAME:-wakapi}
    volumes:
      - wakapi_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  wakapi_db_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false},{"name":"PUBLIC_URL","label":"Public URL","required":false,"default":"http://localhost:3000","secret":false},{"name":"PASSWORD_SALT","label":"Password Salt","required":true,"default":"","secret":true},{"name":"ALLOW_SIGNUP","label":"Allow Sign Up","required":false,"default":"true","secret":false},{"name":"DB_USER","label":"Database User","required":false,"default":"wakapi","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"DB_NAME","label":"Database Name","required":false,"default":"wakapi","secret":false},{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"wakapi","secret":false}]"#,
        ),
    ]
}
