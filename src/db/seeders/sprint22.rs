//! Sprint 22 service templates: gaming servers, AI/ML, and monitoring
//!
//! Already present in earlier seeder files (skipped):
//! - Actual Budget (media_productivity.rs)
//! - Firefly III (media_productivity.rs)
//! - Invoice Ninja (media_productivity.rs + sprint18.rs)
//! - Audiobookshelf (media_productivity.rs)
//! - Calibre Web (sprint19.rs as tpl-calibre-web)
//! - Jellyfin (documentation.rs as tpl-batch2-jellyfin)
//! - Navidrome (documentation.rs as tpl-batch2-navidrome)
//! - Plex (extra_services.rs as tpl-plex)
//! - Emby (extra_services.rs as tpl-emby)
//! - Weaviate (databases_tools.rs as tpl-weaviate)
//! - Chroma / ChromaDB (sprint16.rs as tpl-chroma; ai_ml.rs as chromadb)
//! - Uptime Kuma (infrastructure.rs as uptime-kuma)
//!
//! New templates added: Minecraft Java, Palworld, Terraria, Satisfactory,
//! Argilla, Mage AI, Glitchtip

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== GAMING ====================
        (
            "tpl-minecraft-java",
            "Minecraft Java",
            "Java Edition Minecraft server. Supports vanilla, Paper, Spigot, Fabric, Forge, and more via the itzg/minecraft-server image. Set EULA=TRUE to accept the license.",
            "Gaming",
            "minecraft",
            r#"services:
  minecraft:
    image: itzg/minecraft-server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-minecraft}
    restart: unless-stopped
    ports:
      - "${PORT:-25565}:25565"
    environment:
      - EULA=${EULA:-TRUE}
      - TYPE=${SERVER_TYPE:-VANILLA}
      - VERSION=${MC_VERSION:-LATEST}
      - MAX_PLAYERS=${MAX_PLAYERS:-20}
      - MOTD=${MOTD:-A Minecraft Server}
      - MEMORY=${MEMORY:-1G}
      - DIFFICULTY=${DIFFICULTY:-easy}
      - MODE=${MODE:-survival}
      - OPS=${OPS:-}
    volumes:
      - minecraft_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  minecraft_data:
"#,
            r#"[{"name":"EULA","label":"Accept EULA (must be TRUE)","required":true,"default":"TRUE","secret":false},{"name":"MC_VERSION","label":"Minecraft Version","required":false,"default":"LATEST","secret":false},{"name":"SERVER_TYPE","label":"Server Type (VANILLA, PAPER, SPIGOT, FABRIC, FORGE)","required":false,"default":"VANILLA","secret":false},{"name":"MAX_PLAYERS","label":"Max Players","required":false,"default":"20","secret":false},{"name":"MOTD","label":"Server MOTD","required":false,"default":"A Minecraft Server","secret":false},{"name":"MEMORY","label":"JVM Memory (e.g. 1G, 2G)","required":false,"default":"1G","secret":false},{"name":"DIFFICULTY","label":"Difficulty (peaceful, easy, normal, hard)","required":false,"default":"easy","secret":false},{"name":"MODE","label":"Game Mode (survival, creative, adventure, spectator)","required":false,"default":"survival","secret":false},{"name":"OPS","label":"Operator Player Names (comma-separated)","required":false,"default":"","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"25565","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"minecraft","secret":false}]"#,
        ),
        (
            "tpl-palworld",
            "Palworld",
            "Palworld dedicated server. Multiplayer survival crafting game — host your own persistent world. Exposes UDP port 8211.",
            "Gaming",
            "palworld",
            r#"services:
  palworld:
    image: thijsvanloef/palworld-server-docker:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-palworld}
    restart: unless-stopped
    ports:
      - "${PORT:-8211}:8211/udp"
      - "${QUERY_PORT:-27015}:27015/udp"
    environment:
      - PUID=${PUID:-1000}
      - PGID=${PGID:-1000}
      - PORT=${PORT:-8211}
      - PLAYERS=${PLAYERS:-16}
      - SERVER_PASSWORD=${SERVER_PASSWORD:-}
      - MULTITHREADING=${MULTITHREADING:-true}
      - COMMUNITY=${COMMUNITY:-false}
      - SERVER_NAME=${SERVER_NAME:-Palworld Server}
      - SERVER_DESCRIPTION=${SERVER_DESCRIPTION:-A Palworld Server}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD:-}
    volumes:
      - palworld_data:/palworld
    labels:
      - "rivetr.managed=true"

volumes:
  palworld_data:
"#,
            r#"[{"name":"SERVER_NAME","label":"Server Name","required":false,"default":"Palworld Server","secret":false},{"name":"SERVER_DESCRIPTION","label":"Server Description","required":false,"default":"A Palworld Server","secret":false},{"name":"PLAYERS","label":"Max Players","required":false,"default":"16","secret":false},{"name":"SERVER_PASSWORD","label":"Server Password (leave empty for public)","required":false,"default":"","secret":true},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":false,"default":"","secret":true},{"name":"MULTITHREADING","label":"Enable Multithreading","required":false,"default":"true","secret":false},{"name":"COMMUNITY","label":"Show in Community Server List","required":false,"default":"false","secret":false},{"name":"PORT","label":"Game Port (UDP)","required":false,"default":"8211","secret":false},{"name":"QUERY_PORT","label":"Query Port (UDP)","required":false,"default":"27015","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"palworld","secret":false}]"#,
        ),
        (
            "tpl-terraria",
            "Terraria",
            "Terraria dedicated server. Host multiplayer Terraria worlds for you and your friends. Exposes TCP/UDP port 7777.",
            "Gaming",
            "terraria",
            r#"services:
  terraria:
    image: ryshe/terraria:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-terraria}
    restart: unless-stopped
    ports:
      - "${PORT:-7777}:7777"
    environment:
      - WORLD=${WORLD_NAME:-world}
      - MAXPLAYERS=${MAX_PLAYERS:-8}
      - PASSWORD=${SERVER_PASSWORD:-}
      - AUTOCREATE=${AUTOCREATE_SIZE:-2}
      - WORLDNAME=${WORLD_NAME:-world}
    volumes:
      - terraria_worlds:/root/.local/share/Terraria/Worlds
    labels:
      - "rivetr.managed=true"

volumes:
  terraria_worlds:
"#,
            r#"[{"name":"WORLD_NAME","label":"World Name","required":false,"default":"world","secret":false},{"name":"MAX_PLAYERS","label":"Max Players","required":false,"default":"8","secret":false},{"name":"SERVER_PASSWORD","label":"Server Password (leave empty for public)","required":false,"default":"","secret":true},{"name":"AUTOCREATE_SIZE","label":"Auto-Create World Size (1=small, 2=medium, 3=large)","required":false,"default":"2","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"7777","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"terraria","secret":false}]"#,
        ),
        (
            "tpl-satisfactory",
            "Satisfactory",
            "Satisfactory dedicated server. Factory-building and exploration game — host a persistent multiplayer world. Exposes UDP port 7777.",
            "Gaming",
            "satisfactory",
            r#"services:
  satisfactory:
    image: wolveix/satisfactory-server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-satisfactory}
    restart: unless-stopped
    ports:
      - "${PORT:-7777}:7777/udp"
      - "${BEACON_PORT:-15000}:15000/udp"
      - "${QUERY_PORT:-15777}:15777/udp"
    environment:
      - ROOTLESS=${ROOTLESS:-false}
      - AUTOPAUSE=${AUTOPAUSE:-true}
      - AUTOSAVENUM=${AUTOSAVE_COUNT:-5}
      - AUTOSAVEONDISCONNECT=${AUTOSAVE_ON_DISCONNECT:-true}
      - CRASHREPORT=${CRASH_REPORT:-true}
      - DEBUG=${DEBUG:-false}
      - DISABLESEASONALEVENTS=${DISABLE_SEASONAL:-false}
      - MAXPLAYERS=${MAX_PLAYERS:-4}
      - PGID=${PGID:-1000}
      - PUID=${PUID:-1000}
      - SKIPUPDATE=${SKIP_UPDATE:-false}
      - STEAMBETA=${STEAM_BETA:-false}
    volumes:
      - satisfactory_config:/config
      - satisfactory_gamefiles:/home/steam/SatisfactoryDedicatedServer
    labels:
      - "rivetr.managed=true"

volumes:
  satisfactory_config:
  satisfactory_gamefiles:
"#,
            r#"[{"name":"MAX_PLAYERS","label":"Max Players","required":false,"default":"4","secret":false},{"name":"AUTOPAUSE","label":"Pause When Empty","required":false,"default":"true","secret":false},{"name":"AUTOSAVE_COUNT","label":"Number of Auto-Save Slots","required":false,"default":"5","secret":false},{"name":"AUTOSAVE_ON_DISCONNECT","label":"Auto-Save on Disconnect","required":false,"default":"true","secret":false},{"name":"SKIP_UPDATE","label":"Skip Game Updates on Start","required":false,"default":"false","secret":false},{"name":"PORT","label":"Game Port (UDP)","required":false,"default":"7777","secret":false},{"name":"BEACON_PORT","label":"Beacon Port (UDP)","required":false,"default":"15000","secret":false},{"name":"QUERY_PORT","label":"Query Port (UDP)","required":false,"default":"15777","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"satisfactory","secret":false}]"#,
        ),
        // ==================== AI / VECTOR DBs ====================
        (
            "tpl-argilla",
            "Argilla",
            "Open-source data labeling and RLHF platform for LLMs. Create high-quality training datasets through human feedback, annotation workflows, and active learning.",
            "AI",
            "argilla",
            r#"services:
  argilla:
    image: argilla/argilla-server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-argilla}
    restart: unless-stopped
    ports:
      - "${PORT:-6900}:6900"
    environment:
      - DATABASE_URL=postgresql+asyncpg://${DB_USER:-argilla}:${DB_PASSWORD:-argilla}@argilla_db:5432/${DB_NAME:-argilla}
      - SECRET_KEY=${SECRET_KEY}
      - USERNAME=${ADMIN_USER:-admin}
      - PASSWORD=${ADMIN_PASSWORD}
      - API_KEY=${API_KEY:-argilla.apikey}
      - WORKSPACE=${WORKSPACE:-argilla}
    volumes:
      - argilla_data:/var/lib/argilla
    depends_on:
      - argilla_db
    labels:
      - "rivetr.managed=true"

  argilla_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-argilla}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-argilla}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-argilla}
      - POSTGRES_DB=${DB_NAME:-argilla}
    volumes:
      - argilla_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  argilla_data:
  argilla_db_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"6900","secret":false},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"API_KEY","label":"API Key","required":false,"default":"argilla.apikey","secret":true},{"name":"WORKSPACE","label":"Default Workspace Name","required":false,"default":"argilla","secret":false},{"name":"DB_USER","label":"Database User","required":false,"default":"argilla","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"DB_NAME","label":"Database Name","required":false,"default":"argilla","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"argilla","secret":false}]"#,
        ),
        (
            "tpl-mage-ai",
            "Mage AI",
            "Open-source AI pipeline tool for transforming and integrating data. Build, run, and manage data pipelines with a modern UI. Alternative to Airflow with an AI-first workflow.",
            "AI",
            "mage-ai",
            r#"services:
  mage-ai:
    image: mageai/mageai:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-mage-ai}
    restart: unless-stopped
    ports:
      - "${PORT:-6789}:6789"
    environment:
      - USER_CODE_PATH=${PROJECT_PATH:-/home/src/default_repo}
      - MAGE_DATABASE_CONNECTION_URL=${DATABASE_URL:-}
      - PROJECT_TYPE=${PROJECT_TYPE:-standalone}
      - REQUIRE_USER_AUTHENTICATION=${REQUIRE_AUTH:-0}
      - MAGE_ACCESS_TOKEN=${ACCESS_TOKEN:-}
    volumes:
      - mage_ai_data:/home/src
    labels:
      - "rivetr.managed=true"

volumes:
  mage_ai_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"6789","secret":false},{"name":"PROJECT_PATH","label":"Project Path (inside container)","required":false,"default":"/home/src/default_repo","secret":false},{"name":"REQUIRE_AUTH","label":"Require Authentication (0=no, 1=yes)","required":false,"default":"0","secret":false},{"name":"ACCESS_TOKEN","label":"Access Token (if auth enabled)","required":false,"default":"","secret":true},{"name":"DATABASE_URL","label":"External Database URL (leave empty for SQLite)","required":false,"default":"","secret":true},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mage-ai","secret":false}]"#,
        ),
        // ==================== MONITORING ====================
        (
            "tpl-glitchtip",
            "Glitchtip",
            "Open-source error tracking — a simpler, cheaper alternative to Sentry. Collects exceptions from your apps, aggregates them into issues, and sends alerts. Compatible with the Sentry SDK.",
            "Monitoring",
            "glitchtip",
            r#"services:
  glitchtip:
    image: glitchtip/glitchtip:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-glitchtip}
    restart: unless-stopped
    depends_on:
      - glitchtip_db
      - glitchtip_redis
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DATABASE_URL=postgres://${DB_USER:-glitchtip}:${DB_PASSWORD:-glitchtip}@glitchtip_db:5432/${DB_NAME:-glitchtip}
      - REDIS_URL=redis://glitchtip_redis:6379
      - SECRET_KEY=${SECRET_KEY}
      - PORT=8080
      - EMAIL_URL=${EMAIL_URL:-consolemail://}
      - GLITCHTIP_DOMAIN=${GLITCHTIP_DOMAIN:-http://localhost:8080}
      - DEFAULT_FROM_EMAIL=${DEFAULT_FROM_EMAIL:-admin@example.com}
      - CELERY_WORKER_CONCURRENCY=${WORKER_CONCURRENCY:-2}
    volumes:
      - glitchtip_uploads:/code/uploads
    labels:
      - "rivetr.managed=true"

  glitchtip_worker:
    image: glitchtip/glitchtip:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-glitchtip}-worker
    restart: unless-stopped
    command: ./bin/run-celery-with-beat.sh
    depends_on:
      - glitchtip_db
      - glitchtip_redis
    environment:
      - DATABASE_URL=postgres://${DB_USER:-glitchtip}:${DB_PASSWORD:-glitchtip}@glitchtip_db:5432/${DB_NAME:-glitchtip}
      - REDIS_URL=redis://glitchtip_redis:6379
      - SECRET_KEY=${SECRET_KEY}
      - EMAIL_URL=${EMAIL_URL:-consolemail://}
      - GLITCHTIP_DOMAIN=${GLITCHTIP_DOMAIN:-http://localhost:8080}
      - CELERY_WORKER_CONCURRENCY=${WORKER_CONCURRENCY:-2}
    volumes:
      - glitchtip_uploads:/code/uploads
    labels:
      - "rivetr.managed=true"

  glitchtip_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-glitchtip}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-glitchtip}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-glitchtip}
      - POSTGRES_DB=${DB_NAME:-glitchtip}
    volumes:
      - glitchtip_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  glitchtip_redis:
    image: redis:7-alpine
    container_name: ${CONTAINER_NAME:-glitchtip}-redis
    restart: unless-stopped
    volumes:
      - glitchtip_redis_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  glitchtip_uploads:
  glitchtip_db_data:
  glitchtip_redis_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false},{"name":"GLITCHTIP_DOMAIN","label":"Public URL (e.g. https://errors.example.com)","required":false,"default":"http://localhost:8080","secret":false},{"name":"SECRET_KEY","label":"Secret Key (generate with: openssl rand -hex 32)","required":true,"default":"","secret":true},{"name":"EMAIL_URL","label":"Email URL (e.g. smtp://user:pass@host:587)","required":false,"default":"consolemail://","secret":true},{"name":"DEFAULT_FROM_EMAIL","label":"From Email Address","required":false,"default":"admin@example.com","secret":false},{"name":"WORKER_CONCURRENCY","label":"Celery Worker Concurrency","required":false,"default":"2","secret":false},{"name":"DB_USER","label":"Database User","required":false,"default":"glitchtip","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"DB_NAME","label":"Database Name","required":false,"default":"glitchtip","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"glitchtip","secret":false}]"#,
        ),
    ]
}
