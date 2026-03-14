//! Miscellaneous extra service templates (notes, wikis, utilities, and more)

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== NOTES / KNOWLEDGE ====================
        (
            "tpl-silverbullet",
            "SilverBullet",
            "Markdown-based open-source note-taking and knowledge management app. Extensible with Lua.",
            "project-management",
            "silverbullet",
            r#"services:
  silverbullet:
    image: zefhemel/silverbullet:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-silverbullet}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    volumes:
      - silverbullet_data:/space
    labels:
      - "rivetr.managed=true"

volumes:
  silverbullet_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"silverbullet","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false}]"#,
        ),
        (
            "tpl-obsidian-livesync",
            "Obsidian LiveSync",
            "Self-hosted CouchDB backend for Obsidian LiveSync plugin. Sync your Obsidian vaults.",
            "project-management",
            "obsidian",
            r#"services:
  couchdb:
    image: couchdb:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-obsidian-couchdb}
    restart: unless-stopped
    ports:
      - "${PORT:-5984}:5984"
    environment:
      - COUCHDB_USER=${ADMIN_USER:-admin}
      - COUCHDB_PASSWORD=${ADMIN_PASSWORD:-changeme}
    volumes:
      - couchdb_data:/opt/couchdb/data
    labels:
      - "rivetr.managed=true"

volumes:
  couchdb_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"obsidian-couchdb","secret":false},{"name":"PORT","label":"Port","required":false,"default":"5984","secret":false},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== UTILITIES ====================
        (
            "tpl-it-tools",
            "IT Tools",
            "Collection of handy online tools for developers: encoders, generators, converters, and more.",
            "development",
            "it-tools",
            r#"services:
  it-tools:
    image: corentinth/it-tools:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-it-tools}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"it-tools","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-open-speed-test",
            "OpenSpeedTest",
            "Self-hosted HTML5 internet speed test. No third-party services, no Flash, no Java required.",
            "development",
            "speedtest",
            r#"services:
  openspeedtest:
    image: openspeedtest/latest:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-openspeedtest}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-3000}:3000"
      - "${HTTPS_PORT:-3001}:3001"
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"openspeedtest","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"3000","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"3001","secret":false}]"#,
        ),
        (
            "tpl-drawio",
            "Draw.io (Diagrams.net)",
            "Self-hosted diagramming tool. Create flowcharts, network diagrams, UML, ERDs, and more.",
            "development",
            "drawio",
            r#"services:
  drawio:
    image: jgraph/drawio:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-drawio}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
      - "${HTTPS_PORT:-8443}:8443"
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"drawio","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"8080","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"8443","secret":false}]"#,
        ),

        // ==================== MEDIA / STREAMING ====================
        (
            "tpl-owncast",
            "Owncast",
            "Self-hosted live video streaming server. YouTube Live / Twitch alternative with chat support.",
            "media",
            "owncast",
            r#"services:
  owncast:
    image: gabekangas/owncast:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-owncast}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-8080}:8080"
      - "${RTMP_PORT:-1935}:1935"
    volumes:
      - owncast_data:/app/data
    labels:
      - "rivetr.managed=true"

volumes:
  owncast_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"owncast","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"8080","secret":false},{"name":"RTMP_PORT","label":"RTMP Port","required":false,"default":"1935","secret":false}]"#,
        ),
        (
            "tpl-mediamtx",
            "MediaMTX",
            "Ready-to-use RTSP/RTMP/HLS/WebRTC media server. Forward and record live streams.",
            "media",
            "mediamtx",
            r#"services:
  mediamtx:
    image: bluenviron/mediamtx:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-mediamtx}
    restart: unless-stopped
    ports:
      - "${RTSP_PORT:-8554}:8554"
      - "${RTMP_PORT:-1935}:1935"
      - "${HLS_PORT:-8888}:8888"
      - "${WEBRTC_PORT:-8889}:8889"
      - "${API_PORT:-9997}:9997"
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mediamtx","secret":false},{"name":"RTSP_PORT","label":"RTSP Port","required":false,"default":"8554","secret":false},{"name":"RTMP_PORT","label":"RTMP Port","required":false,"default":"1935","secret":false},{"name":"HLS_PORT","label":"HLS Port","required":false,"default":"8888","secret":false},{"name":"API_PORT","label":"API Port","required":false,"default":"9997","secret":false}]"#,
        ),
        (
            "tpl-photoprism",
            "PhotoPrism",
            "AI-powered photo app for the decentralized web. Browse, organize, and share your photo collection.",
            "media",
            "photoprism",
            r#"services:
  photoprism:
    image: photoprism/photoprism:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-photoprism}
    restart: unless-stopped
    ports:
      - "${PORT:-2342}:2342"
    environment:
      - PHOTOPRISM_ADMIN_USER=${ADMIN_USER:-admin}
      - PHOTOPRISM_ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - PHOTOPRISM_AUTH_MODE=${AUTH_MODE:-password}
      - PHOTOPRISM_SITE_URL=${SITE_URL:-http://localhost:2342/}
      - PHOTOPRISM_ORIGINALS_LIMIT=10000
      - PHOTOPRISM_HTTP_COMPRESSION=gzip
      - PHOTOPRISM_DATABASE_DRIVER=sqlite
    volumes:
      - photoprism_originals:/photoprism/originals
      - photoprism_storage:/photoprism/storage
    labels:
      - "rivetr.managed=true"

volumes:
  photoprism_originals:
  photoprism_storage:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"photoprism","secret":false},{"name":"PORT","label":"Port","required":false,"default":"2342","secret":false},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"SITE_URL","label":"Site URL","required":false,"default":"http://localhost:2342/","secret":false}]"#,
        ),
    ]
}
