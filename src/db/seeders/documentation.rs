//! Documentation and file/media service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== BATCH 2: DOCUMENTATION ====================
        (
            "tpl-batch2-bookstack",
            "BookStack",
            "A simple, self-hosted wiki platform for organising and storing information.",
            "documentation",
            "bookstack",
            r#"services:
  bookstack:
    image: lscr.io/linuxserver/bookstack:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-bookstack}
    restart: unless-stopped
    ports:
      - "${PORT:-6875}:80"
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=${TZ:-UTC}
      - APP_URL=${APP_URL:-http://localhost:6875}
      - DB_HOST=bookstack_db
      - DB_PORT=3306
      - DB_USER=bookstack
      - DB_PASS=${DB_PASSWORD:-bookstack}
      - DB_DATABASE=bookstack
    volumes:
      - bookstack_data:/config
    depends_on:
      - bookstack_db
    labels:
      - "rivetr.managed=true"

  bookstack_db:
    image: mysql:8.0
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=bookstack
      - MYSQL_USER=bookstack
      - MYSQL_PASSWORD=${DB_PASSWORD:-bookstack}
      - MYSQL_RANDOM_ROOT_PASSWORD=yes
    volumes:
      - bookstack_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  bookstack_data:
  bookstack_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"bookstack","secret":false},{"name":"PORT","label":"Port","required":false,"default":"6875","secret":false},{"name":"APP_URL","label":"Application URL","required":true,"default":"http://localhost:6875","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false}]"#,
        ),
        (
            "tpl-batch2-wikijs",
            "Wiki.js",
            "A modern, powerful wiki app built on Node.js with beautiful interface and extensive features.",
            "documentation",
            "wikijs",
            r#"services:
  wikijs:
    image: ghcr.io/requarks/wiki:${VERSION:-2}
    container_name: ${CONTAINER_NAME:-wikijs}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DB_TYPE=postgres
      - DB_HOST=wikijs_db
      - DB_PORT=5432
      - DB_USER=wikijs
      - DB_PASS=${DB_PASSWORD:-wikijs}
      - DB_NAME=wikijs
    depends_on:
      - wikijs_db
    labels:
      - "rivetr.managed=true"

  wikijs_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=wikijs
      - POSTGRES_PASSWORD=${DB_PASSWORD:-wikijs}
      - POSTGRES_DB=wikijs
    volumes:
      - wikijs_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  wikijs_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"2","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"wikijs","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-batch2-docmost",
            "Docmost",
            "An open-source collaborative wiki and documentation software. Alternative to Notion and Confluence.",
            "documentation",
            "docmost",
            r#"services:
  docmost:
    image: docmost/docmost:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-docmost}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - APP_URL=${APP_URL:-http://localhost:3000}
      - APP_SECRET=${APP_SECRET:-change-me-to-a-long-random-string}
      - DATABASE_URL=postgresql://docmost:${DB_PASSWORD:-docmost}@docmost_db:5432/docmost
      - REDIS_URL=redis://docmost_redis:6379
    depends_on:
      - docmost_db
      - docmost_redis
    labels:
      - "rivetr.managed=true"

  docmost_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=docmost
      - POSTGRES_PASSWORD=${DB_PASSWORD:-docmost}
      - POSTGRES_DB=docmost
    volumes:
      - docmost_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  docmost_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  docmost_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"docmost","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"APP_URL","label":"Application URL","required":true,"default":"http://localhost:3000","secret":false},{"name":"APP_SECRET","label":"App Secret","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== BATCH 2: FILE & MEDIA ====================
        (
            "tpl-batch2-immich",
            "Immich",
            "High-performance self-hosted photo and video management solution with ML-powered features.",
            "media",
            "immich",
            r#"services:
  immich-server:
    image: ghcr.io/immich-app/immich-server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-immich-server}
    restart: unless-stopped
    ports:
      - "${PORT:-2283}:2283"
    environment:
      - DB_HOSTNAME=immich_db
      - DB_USERNAME=immich
      - DB_PASSWORD=${DB_PASSWORD:-immich}
      - DB_DATABASE_NAME=immich
      - REDIS_HOSTNAME=immich_redis
    volumes:
      - immich_upload:/usr/src/app/upload
    depends_on:
      - immich_db
      - immich_redis
    labels:
      - "rivetr.managed=true"

  immich-machine-learning:
    image: ghcr.io/immich-app/immich-machine-learning:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-immich-ml}
    restart: unless-stopped
    volumes:
      - immich_ml_cache:/cache
    labels:
      - "rivetr.managed=true"

  immich_db:
    image: tensorchord/pgvecto-rs:pg14-v0.2.0
    restart: unless-stopped
    environment:
      - POSTGRES_USER=immich
      - POSTGRES_PASSWORD=${DB_PASSWORD:-immich}
      - POSTGRES_DB=immich
      - POSTGRES_INITDB_ARGS=--data-checksums
    volumes:
      - immich_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  immich_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  immich_upload:
  immich_ml_cache:
  immich_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"immich-server","secret":false},{"name":"PORT","label":"Port","required":false,"default":"2283","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-batch2-jellyfin",
            "Jellyfin",
            "Free software media system for streaming movies, TV shows, music, and more.",
            "media",
            "jellyfin",
            r#"services:
  jellyfin:
    image: jellyfin/jellyfin:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-jellyfin}
    restart: unless-stopped
    ports:
      - "${PORT:-8096}:8096"
    volumes:
      - jellyfin_config:/config
      - jellyfin_cache:/cache
      - jellyfin_media:/media
    labels:
      - "rivetr.managed=true"

volumes:
  jellyfin_config:
  jellyfin_cache:
  jellyfin_media:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"jellyfin","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8096","secret":false}]"#,
        ),
        (
            "tpl-batch2-navidrome",
            "Navidrome",
            "Modern music server and streamer compatible with Subsonic/Airsonic clients.",
            "media",
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
      - ND_BASEURL=${BASE_URL:-}
    volumes:
      - navidrome_data:/data
      - navidrome_music:/music:ro
    labels:
      - "rivetr.managed=true"

volumes:
  navidrome_data:
  navidrome_music:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"navidrome","secret":false},{"name":"PORT","label":"Port","required":false,"default":"4533","secret":false},{"name":"SCAN_SCHEDULE","label":"Scan Schedule","required":false,"default":"1h","secret":false},{"name":"LOG_LEVEL","label":"Log Level","required":false,"default":"info","secret":false},{"name":"BASE_URL","label":"Base URL","required":false,"default":"","secret":false}]"#,
        ),
        (
            "tpl-batch2-seafile",
            "Seafile",
            "Open-source file sync and share solution with high performance and reliability.",
            "media",
            "seafile",
            r#"services:
  seafile:
    image: seafileltd/seafile-mc:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-seafile}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - DB_HOST=seafile_db
      - DB_ROOT_PASSWD=${DB_ROOT_PASSWORD:-seafile}
      - SEAFILE_ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - SEAFILE_ADMIN_PASSWORD=${ADMIN_PASSWORD:-admin}
      - SEAFILE_SERVER_HOSTNAME=${SERVER_HOSTNAME:-localhost}
    volumes:
      - seafile_data:/shared
    depends_on:
      - seafile_db
      - seafile_memcached
    labels:
      - "rivetr.managed=true"

  seafile_db:
    image: mysql:8.0
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-seafile}
      - MYSQL_LOG_CONSOLE=true
    volumes:
      - seafile_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

  seafile_memcached:
    image: memcached:1.6-alpine
    restart: unless-stopped
    entrypoint: memcached -m 256
    labels:
      - "rivetr.managed=true"

volumes:
  seafile_data:
  seafile_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"seafile","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"DB_ROOT_PASSWORD","label":"DB Root Password","required":true,"default":"","secret":true},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"SERVER_HOSTNAME","label":"Server Hostname","required":false,"default":"localhost","secret":false}]"#,
        ),
    ]
}
