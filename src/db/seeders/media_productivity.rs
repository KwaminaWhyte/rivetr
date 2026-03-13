//! Media, productivity, finance, and utility service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== MEDIA ====================
        (
            "tpl-immich",
            "Immich",
            "High-performance, self-hosted photo and video management solution. A Google Photos alternative.",
            "storage",
            "immich",
            r#"services:
  immich-server:
    image: ghcr.io/immich-app/immich-server:${VERSION:-release}
    container_name: ${CONTAINER_NAME:-immich-server}
    restart: unless-stopped
    ports:
      - "${PORT:-2283}:2283"
    environment:
      - DB_HOSTNAME=immich_postgres
      - DB_USERNAME=immich
      - DB_PASSWORD=${DB_PASSWORD:-immich}
      - DB_DATABASE_NAME=immich
      - REDIS_HOSTNAME=immich_redis
      - UPLOAD_LOCATION=${UPLOAD_LOCATION:-/usr/src/app/upload}
    volumes:
      - immich_uploads:/usr/src/app/upload
    depends_on:
      - immich_postgres
      - immich_redis
    labels:
      - "rivetr.managed=true"

  immich-machine-learning:
    image: ghcr.io/immich-app/immich-machine-learning:${VERSION:-release}
    restart: unless-stopped
    volumes:
      - immich_model_cache:/cache
    labels:
      - "rivetr.managed=true"

  immich_postgres:
    image: tensorchord/pgvecto-rs:pg14-v0.2.0
    restart: unless-stopped
    environment:
      - POSTGRES_USER=immich
      - POSTGRES_PASSWORD=${DB_PASSWORD:-immich}
      - POSTGRES_DB=immich
    volumes:
      - immich_pgdata:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  immich_redis:
    image: redis:6.2-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  immich_uploads:
  immich_model_cache:
  immich_pgdata:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"release","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"immich-server","secret":false},{"name":"PORT","label":"Port","required":false,"default":"2283","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-jellyfin",
            "Jellyfin",
            "Free, open-source media server. Stream your movies, TV shows, and music from anywhere.",
            "storage",
            "jellyfin",
            r#"services:
  jellyfin:
    image: jellyfin/jellyfin:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-jellyfin}
    restart: unless-stopped
    ports:
      - "${PORT:-8096}:8096"
    environment:
      - JELLYFIN_PublishedServerUrl=${PUBLISHED_URL:-}
    volumes:
      - jellyfin_config:/config
      - jellyfin_cache:/cache
      - ${MEDIA_PATH:-/media}:/media:ro
    labels:
      - "rivetr.managed=true"

volumes:
  jellyfin_config:
  jellyfin_cache:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"jellyfin","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8096","secret":false},{"name":"MEDIA_PATH","label":"Media Path on Host","required":false,"default":"/media","secret":false},{"name":"PUBLISHED_URL","label":"Published Server URL","required":false,"default":"","secret":false}]"#,
        ),
        (
            "tpl-navidrome",
            "Navidrome",
            "Modern self-hosted music server and streamer. Compatible with Subsonic/Airsonic clients.",
            "storage",
            "navidrome",
            r#"services:
  navidrome:
    image: deluan/navidrome:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-navidrome}
    restart: unless-stopped
    ports:
      - "${PORT:-4533}:4533"
    environment:
      - ND_SCANSCHEDULE=${SCAN_SCHEDULE:-1h}
      - ND_LOGLEVEL=${LOG_LEVEL:-info}
      - ND_MUSICFOLDER=/music
    volumes:
      - navidrome_data:/data
      - ${MUSIC_PATH:-/music}:/music:ro
    labels:
      - "rivetr.managed=true"

volumes:
  navidrome_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"navidrome","secret":false},{"name":"PORT","label":"Port","required":false,"default":"4533","secret":false},{"name":"MUSIC_PATH","label":"Music Path on Host","required":false,"default":"/music","secret":false},{"name":"SCAN_SCHEDULE","label":"Scan Schedule","required":false,"default":"1h","secret":false}]"#,
        ),
        (
            "tpl-audiobookshelf",
            "Audiobookshelf",
            "Self-hosted audiobook and podcast server. Stream your library anywhere with a beautiful UI.",
            "storage",
            "audiobookshelf",
            r#"services:
  audiobookshelf:
    image: ghcr.io/advplyr/audiobookshelf:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-audiobookshelf}
    restart: unless-stopped
    ports:
      - "${PORT:-13378}:80"
    environment:
      - AUDIOBOOKSHELF_UID=99
      - AUDIOBOOKSHELF_GID=100
    volumes:
      - ${AUDIOBOOKS_PATH:-/audiobooks}:/audiobooks
      - ${PODCASTS_PATH:-/podcasts}:/podcasts
      - audiobookshelf_config:/config
      - audiobookshelf_metadata:/metadata
    labels:
      - "rivetr.managed=true"

volumes:
  audiobookshelf_config:
  audiobookshelf_metadata:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"audiobookshelf","secret":false},{"name":"PORT","label":"Port","required":false,"default":"13378","secret":false},{"name":"AUDIOBOOKS_PATH","label":"Audiobooks Path on Host","required":false,"default":"/audiobooks","secret":false},{"name":"PODCASTS_PATH","label":"Podcasts Path on Host","required":false,"default":"/podcasts","secret":false}]"#,
        ),

        // ==================== RSS ====================
        (
            "tpl-freshrss",
            "FreshRSS",
            "Self-hosted RSS feed aggregator. Lightweight, fast, and supports multi-user mode.",
            "development",
            "freshrss",
            r#"services:
  freshrss:
    image: freshrss/freshrss:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-freshrss}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - TZ=${TZ:-UTC}
      - CRON_MIN=${CRON_MIN:-1,31}
    volumes:
      - freshrss_data:/var/www/FreshRSS/data
      - freshrss_extensions:/var/www/FreshRSS/extensions
    labels:
      - "rivetr.managed=true"

volumes:
  freshrss_data:
  freshrss_extensions:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"freshrss","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false}]"#,
        ),
        (
            "tpl-miniflux",
            "Miniflux",
            "Minimalist and opinionated RSS feed reader. Fast, secure, and distraction-free.",
            "development",
            "miniflux",
            r#"services:
  miniflux:
    image: miniflux/miniflux:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-miniflux}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DATABASE_URL=postgres://miniflux:${DB_PASSWORD:-miniflux}@miniflux_db/miniflux?sslmode=disable
      - RUN_MIGRATIONS=1
      - CREATE_ADMIN=1
      - ADMIN_USERNAME=${ADMIN_USER:-admin}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
    depends_on:
      - miniflux_db
    labels:
      - "rivetr.managed=true"

  miniflux_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=miniflux
      - POSTGRES_PASSWORD=${DB_PASSWORD:-miniflux}
      - POSTGRES_DB=miniflux
    volumes:
      - miniflux_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  miniflux_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"miniflux","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== NOTIFICATIONS ====================
        (
            "tpl-gotify",
            "Gotify",
            "Simple self-hosted notification server. Send push notifications to your devices via REST API.",
            "development",
            "gotify",
            r#"services:
  gotify:
    image: gotify/server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-gotify}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - GOTIFY_DEFAULTUSER_PASS=${ADMIN_PASSWORD:-admin}
    volumes:
      - gotify_data:/app/data
    labels:
      - "rivetr.managed=true"

volumes:
  gotify_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"gotify","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-ntfy",
            "Ntfy",
            "Simple HTTP-based pub-sub notification service. Send notifications to your phone or desktop via curl.",
            "development",
            "ntfy",
            r#"services:
  ntfy:
    image: binwiederhier/ntfy:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-ntfy}
    restart: unless-stopped
    command: serve
    ports:
      - "${PORT:-8080}:80"
    environment:
      - NTFY_BASE_URL=${BASE_URL:-http://localhost}
      - NTFY_UPSTREAM_BASE_URL=https://ntfy.sh
      - NTFY_CACHE_FILE=/var/lib/ntfy/cache.db
      - NTFY_AUTH_FILE=/var/lib/ntfy/auth.db
      - NTFY_AUTH_DEFAULT_ACCESS=${DEFAULT_ACCESS:-deny-all}
    volumes:
      - ntfy_data:/var/lib/ntfy
    labels:
      - "rivetr.managed=true"

volumes:
  ntfy_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"ntfy","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"BASE_URL","label":"Base URL","required":false,"default":"http://localhost","secret":false},{"name":"DEFAULT_ACCESS","label":"Default Access","required":false,"default":"deny-all","secret":false}]"#,
        ),

        // ==================== DOCUMENT MANAGEMENT ====================
        (
            "tpl-paperless-ngx",
            "Paperless-ngx",
            "Community-maintained fork of Paperless. Scan, index, and archive your physical documents.",
            "development",
            "paperless-ngx",
            r#"services:
  paperless-ngx:
    image: ghcr.io/paperless-ngx/paperless-ngx:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-paperless-ngx}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - PAPERLESS_REDIS=redis://paperless_redis:6379
      - PAPERLESS_DBHOST=paperless_db
      - PAPERLESS_DBUSER=paperless
      - PAPERLESS_DBPASS=${DB_PASSWORD:-paperless}
      - PAPERLESS_DBNAME=paperless
      - PAPERLESS_SECRET_KEY=${SECRET_KEY:-change-me-to-a-long-random-string}
      - PAPERLESS_TIME_ZONE=${TZ:-UTC}
      - USERMAP_UID=1000
      - USERMAP_GID=1000
    volumes:
      - paperless_data:/usr/src/paperless/data
      - paperless_media:/usr/src/paperless/media
      - paperless_export:/usr/src/paperless/export
      - paperless_consume:/usr/src/paperless/consume
    depends_on:
      - paperless_db
      - paperless_redis
    labels:
      - "rivetr.managed=true"

  paperless_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=paperless
      - POSTGRES_PASSWORD=${DB_PASSWORD:-paperless}
      - POSTGRES_DB=paperless
    volumes:
      - paperless_pgdata:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  paperless_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  paperless_data:
  paperless_media:
  paperless_export:
  paperless_consume:
  paperless_pgdata:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"paperless-ngx","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false}]"#,
        ),
        (
            "tpl-stirling-pdf",
            "Stirling PDF",
            "Locally hosted web application for PDF manipulation — merge, split, compress, convert, and more.",
            "development",
            "stirling-pdf",
            r#"services:
  stirling-pdf:
    image: frooodle/s-pdf:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-stirling-pdf}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DOCKER_ENABLE_SECURITY=false
      - INSTALL_BOOK_AND_ADVANCED_HTML_OPS=${INSTALL_OCR:-false}
      - LANGS=${LANGS:-en_GB}
    volumes:
      - stirling_training_data:/usr/share/tessdata
      - stirling_extra_configs:/configs
    labels:
      - "rivetr.managed=true"

volumes:
  stirling_training_data:
  stirling_extra_configs:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"stirling-pdf","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"INSTALL_OCR","label":"Install OCR (slower start)","required":false,"default":"false","secret":false}]"#,
        ),

        // ==================== UTILITIES ====================
        (
            "tpl-changedetection",
            "Changedetection.io",
            "Self-hosted website change monitoring. Get notified when any website changes.",
            "monitoring",
            "changedetection",
            r#"services:
  changedetection:
    image: ghcr.io/dgtlmoon/changedetection.io:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-changedetection}
    restart: unless-stopped
    ports:
      - "${PORT:-5000}:5000"
    environment:
      - BASE_URL=${BASE_URL:-}
      - PLAYWRIGHT_DRIVER_URL=ws://playwright:3000/?stealth=1&--disable-web-security=true
    volumes:
      - changedetection_data:/datastore
    depends_on:
      - playwright
    labels:
      - "rivetr.managed=true"

  playwright:
    image: browserless/chrome:${PLAYWRIGHT_VERSION:-1-chrome-stable}
    restart: unless-stopped
    environment:
      - SCREEN_WIDTH=1920
      - SCREEN_HEIGHT=1024
      - SCREEN_DEPTH=16
      - ENABLE_DEBUGGER=false
      - PREBOOT_CHROME=true
      - CONNECTION_TIMEOUT=300000
      - MAX_CONCURRENT_SESSIONS=10
    labels:
      - "rivetr.managed=true"

volumes:
  changedetection_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"changedetection","secret":false},{"name":"PORT","label":"Port","required":false,"default":"5000","secret":false},{"name":"BASE_URL","label":"Base URL (optional)","required":false,"default":"","secret":false}]"#,
        ),
        (
            "tpl-syncthing",
            "Syncthing",
            "Continuous file synchronization between devices. Decentralized, private, and open source.",
            "storage",
            "syncthing",
            r#"services:
  syncthing:
    image: syncthing/syncthing:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-syncthing}
    restart: unless-stopped
    hostname: ${HOSTNAME:-syncthing}
    ports:
      - "${PORT:-8384}:8384"
      - "22000:22000/tcp"
      - "22000:22000/udp"
      - "21027:21027/udp"
    volumes:
      - syncthing_data:/var/syncthing
    labels:
      - "rivetr.managed=true"

volumes:
  syncthing_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"syncthing","secret":false},{"name":"PORT","label":"Web UI Port","required":false,"default":"8384","secret":false},{"name":"HOSTNAME","label":"Device Hostname","required":false,"default":"syncthing","secret":false}]"#,
        ),
        (
            "tpl-glances",
            "Glances",
            "Cross-platform system monitoring tool. Monitor CPU, memory, disk, network, and more via web UI.",
            "monitoring",
            "glances",
            r#"services:
  glances:
    image: nicolargo/glances:${VERSION:-latest-full}
    container_name: ${CONTAINER_NAME:-glances}
    restart: unless-stopped
    pid: host
    ports:
      - "${PORT:-61208}:61208"
    environment:
      - GLANCES_OPT=-w
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - /run/user/1000/podman/podman.sock:/run/user/1000/podman/podman.sock:ro
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest-full","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"glances","secret":false},{"name":"PORT","label":"Port","required":false,"default":"61208","secret":false}]"#,
        ),

        // ==================== EMAIL MARKETING ====================
        (
            "tpl-listmonk",
            "Listmonk",
            "High-performance self-hosted newsletter and mailing list manager. Powerful, yet simple.",
            "communication",
            "listmonk",
            r#"services:
  listmonk:
    image: listmonk/listmonk:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-listmonk}
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
    environment:
      - LISTMONK_app__address=0.0.0.0:9000
      - LISTMONK_db__host=listmonk_db
      - LISTMONK_db__port=5432
      - LISTMONK_db__user=listmonk
      - LISTMONK_db__password=${DB_PASSWORD:-listmonk}
      - LISTMONK_db__database=listmonk
    depends_on:
      - listmonk_db
    labels:
      - "rivetr.managed=true"

  listmonk_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=listmonk
      - POSTGRES_PASSWORD=${DB_PASSWORD:-listmonk}
      - POSTGRES_DB=listmonk
    volumes:
      - listmonk_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  listmonk_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"listmonk","secret":false},{"name":"PORT","label":"Port","required":false,"default":"9000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== FINANCE ====================
        (
            "tpl-actual-budget",
            "Actual Budget",
            "Local-first personal finance app. Zero-based budgeting with bank syncing and rich reports.",
            "analytics",
            "actual-budget",
            r#"services:
  actual:
    image: actualbudget/actual-server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-actual-budget}
    restart: unless-stopped
    ports:
      - "${PORT:-5006}:5006"
    volumes:
      - actual_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  actual_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"actual-budget","secret":false},{"name":"PORT","label":"Port","required":false,"default":"5006","secret":false}]"#,
        ),
        (
            "tpl-firefly-iii",
            "Firefly III",
            "Self-hosted personal finance manager. Track income, expenses, budgets, and financial goals.",
            "analytics",
            "firefly-iii",
            r#"services:
  firefly-iii:
    image: fireflyiii/core:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-firefly-iii}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - APP_KEY=${APP_KEY:-SomeRandomStringOf32CharsExactly!!}
      - APP_URL=${APP_URL:-http://localhost}
      - DB_CONNECTION=pgsql
      - DB_HOST=firefly_db
      - DB_PORT=5432
      - DB_DATABASE=firefly
      - DB_USERNAME=firefly
      - DB_PASSWORD=${DB_PASSWORD:-firefly}
      - TRUSTED_PROXIES=**
    volumes:
      - firefly_upload:/var/www/html/storage/upload
    depends_on:
      - firefly_db
    labels:
      - "rivetr.managed=true"

  firefly_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=firefly
      - POSTGRES_PASSWORD=${DB_PASSWORD:-firefly}
      - POSTGRES_DB=firefly
    volumes:
      - firefly_pgdata:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  firefly_upload:
  firefly_pgdata:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"firefly-iii","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"APP_KEY","label":"App Key (32 chars)","required":true,"default":"","secret":true},{"name":"APP_URL","label":"App URL","required":false,"default":"http://localhost","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== NETWORKING ====================
        (
            "tpl-wireguard-easy",
            "WireGuard Easy",
            "The easiest way to run WireGuard VPN + Web UI. Manage clients, QR codes, and connections.",
            "networking",
            "wireguard-easy",
            r#"services:
  wg-easy:
    image: ghcr.io/wg-easy/wg-easy:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-wg-easy}
    restart: unless-stopped
    cap_add:
      - NET_ADMIN
      - SYS_MODULE
    sysctls:
      - net.ipv4.conf.all.src_valid_mark=1
      - net.ipv4.ip_forward=1
    ports:
      - "${WG_PORT:-51820}:51820/udp"
      - "${UI_PORT:-51821}:51821/tcp"
    environment:
      - WG_HOST=${WG_HOST:-vpn.example.com}
      - PASSWORD_HASH=${PASSWORD_HASH:-}
      - WG_PORT=${WG_PORT:-51820}
      - WG_DEFAULT_ADDRESS=10.8.0.x
      - WG_DEFAULT_DNS=1.1.1.1
    volumes:
      - wg_easy_data:/etc/wireguard
    labels:
      - "rivetr.managed=true"

volumes:
  wg_easy_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"wg-easy","secret":false},{"name":"WG_HOST","label":"VPN Hostname/IP","required":true,"default":"","secret":false},{"name":"PASSWORD_HASH","label":"Web UI Password Hash (bcrypt)","required":true,"default":"","secret":true},{"name":"WG_PORT","label":"WireGuard UDP Port","required":false,"default":"51820","secret":false},{"name":"UI_PORT","label":"Web UI Port","required":false,"default":"51821","secret":false}]"#,
        ),

        // ==================== PRODUCTIVITY ====================
        (
            "tpl-excalidraw",
            "Excalidraw",
            "Virtual whiteboard for sketching hand-drawn diagrams. Collaborative, simple, and privacy-first.",
            "development",
            "excalidraw",
            r#"services:
  excalidraw:
    image: excalidraw/excalidraw:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-excalidraw}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"excalidraw","secret":false},{"name":"PORT","label":"Port","required":false,"default":"80","secret":false}]"#,
        ),
        (
            "tpl-memos",
            "Memos",
            "Privacy-first, lightweight note-taking service. Capture thoughts and ideas in plain text or Markdown.",
            "development",
            "memos",
            r#"services:
  memos:
    image: neosmemo/memos:${VERSION:-stable}
    container_name: ${CONTAINER_NAME:-memos}
    restart: unless-stopped
    ports:
      - "${PORT:-5230}:5230"
    volumes:
      - memos_data:/.memos
    labels:
      - "rivetr.managed=true"

volumes:
  memos_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"stable","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"memos","secret":false},{"name":"PORT","label":"Port","required":false,"default":"5230","secret":false}]"#,
        ),
        (
            "tpl-appflowy",
            "AppFlowy",
            "Open-source Notion alternative. AI-powered workspace for notes, tasks, and databases.",
            "development",
            "appflowy",
            r#"services:
  appflowy-cloud:
    image: appflowyinc/appflowy_cloud:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-appflowy-cloud}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - APPFLOWY_DATABASE_URL=postgres://appflowy:${DB_PASSWORD:-appflowy}@appflowy_db/appflowy
      - APPFLOWY_REDIS_URI=redis://appflowy_redis:6379
      - APPFLOWY_JWT_SECRET=${JWT_SECRET:-change-me-to-a-random-string}
      - APPFLOWY_GOTRUE_JWT_SECRET=${JWT_SECRET:-change-me-to-a-random-string}
    depends_on:
      - appflowy_db
      - appflowy_redis
    labels:
      - "rivetr.managed=true"

  appflowy_db:
    image: pgvector/pgvector:pg16
    restart: unless-stopped
    environment:
      - POSTGRES_USER=appflowy
      - POSTGRES_PASSWORD=${DB_PASSWORD:-appflowy}
      - POSTGRES_DB=appflowy
    volumes:
      - appflowy_pgdata:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  appflowy_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  appflowy_pgdata:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"appflowy-cloud","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== DESIGN ====================
        (
            "tpl-penpot",
            "Penpot",
            "Open-source design and prototyping tool. The Figma alternative that works in the browser.",
            "development",
            "penpot",
            r#"services:
  penpot-frontend:
    image: penpotapp/frontend:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-penpot-frontend}
    restart: unless-stopped
    ports:
      - "${PORT:-9001}:80"
    volumes:
      - penpot_assets:/opt/data/assets
    labels:
      - "rivetr.managed=true"

  penpot-backend:
    image: penpotapp/backend:${VERSION:-latest}
    restart: unless-stopped
    volumes:
      - penpot_assets:/opt/data/assets
    environment:
      - PENPOT_FLAGS=enable-registration enable-login-with-password
      - PENPOT_SECRET_KEY=${SECRET_KEY:-change-me-to-a-random-string}
      - PENPOT_DATABASE_URI=postgresql://penpot_db/penpot
      - PENPOT_DATABASE_USERNAME=penpot
      - PENPOT_DATABASE_PASSWORD=${DB_PASSWORD:-penpot}
      - PENPOT_REDIS_URI=redis://penpot_redis/0
      - PENPOT_ASSETS_STORAGE_BACKEND=assets-fs
      - PENPOT_STORAGE_ASSETS_FS_DIRECTORY=/opt/data/assets
      - PENPOT_TELEMETRY_ENABLED=false
    depends_on:
      - penpot_db
      - penpot_redis
    labels:
      - "rivetr.managed=true"

  penpot_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=penpot
      - POSTGRES_PASSWORD=${DB_PASSWORD:-penpot}
      - POSTGRES_DB=penpot
    volumes:
      - penpot_pgdata:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  penpot_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  penpot_assets:
  penpot_pgdata:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"penpot-frontend","secret":false},{"name":"PORT","label":"Port","required":false,"default":"9001","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== INVOICING ====================
        (
            "tpl-invoice-ninja",
            "Invoice Ninja",
            "Open-source invoicing, billing, and time-tracking platform for freelancers and businesses.",
            "analytics",
            "invoice-ninja",
            r#"services:
  invoice-ninja:
    image: invoiceninja/invoiceninja:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-invoice-ninja}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - APP_URL=${APP_URL:-http://localhost}
      - APP_KEY=${APP_KEY:-SomeRandomStringOf32CharsExactly!!}
      - DB_HOST=invoice_ninja_db
      - DB_DATABASE=ninja
      - DB_USERNAME=ninja
      - DB_PASSWORD=${DB_PASSWORD:-ninja}
      - REQUIRE_HTTPS=false
      - PHANTOMJS_PDF_GENERATION=false
    volumes:
      - invoice_ninja_public:/var/www/app/public
      - invoice_ninja_storage:/var/www/app/storage
    depends_on:
      - invoice_ninja_db
    labels:
      - "rivetr.managed=true"

  invoice_ninja_db:
    image: mysql:8.0
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-root}
      - MYSQL_DATABASE=ninja
      - MYSQL_USER=ninja
      - MYSQL_PASSWORD=${DB_PASSWORD:-ninja}
    volumes:
      - invoice_ninja_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  invoice_ninja_public:
  invoice_ninja_storage:
  invoice_ninja_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"invoice-ninja","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"APP_URL","label":"App URL","required":false,"default":"http://localhost","secret":false},{"name":"APP_KEY","label":"App Key (32 chars)","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"DB_ROOT_PASSWORD","label":"MySQL Root Password","required":true,"default":"","secret":true}]"#,
        ),
    ]
}
