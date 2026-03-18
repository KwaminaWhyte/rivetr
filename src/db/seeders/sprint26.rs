//! Sprint 26 service templates: CMS, Auth/SSO, Networking, Productivity,
//! Forms, DevTools, Communication, Media, and Infrastructure
//!
//! Already present in earlier seeder files (skipped):
//! - Mastodon (communication_extra.rs as tpl-mastodon)
//!
//! Net new templates added (11): MediaWiki, Supertokens, Netbird, Affine,
//! Heyform, Opnform, GitHub Actions Runner, Bluesky PDS, PeerTube,
//! Roundcube, Mailserver (docker-mailserver)

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== CMS ====================
        (
            "tpl-mediawiki",
            "MediaWiki",
            "The wiki engine that powers Wikipedia. Host your own collaborative wiki with rich text editing, file uploads, user management, and a vast ecosystem of extensions. Uses MySQL.",
            "CMS",
            "mediawiki",
            r#"services:
  mediawiki:
    image: mediawiki:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-mediawiki}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - MEDIAWIKI_DB_HOST=${DB_HOST:-mediawiki_db}
      - MEDIAWIKI_DB_NAME=${DB_NAME:-mediawiki}
      - MEDIAWIKI_DB_USER=${DB_USER:-mediawiki}
      - MEDIAWIKI_DB_PASSWORD=${DB_PASSWORD}
    volumes:
      - mediawiki_data:/var/www/html/images
      - mediawiki_config:/var/www/html/LocalSettings.php
    depends_on:
      - mediawiki_db
    labels:
      - "rivetr.managed=true"

  mediawiki_db:
    image: mysql:8.0
    container_name: ${CONTAINER_NAME:-mediawiki}-db
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-rootpassword}
      - MYSQL_DATABASE=${DB_NAME:-mediawiki}
      - MYSQL_USER=${DB_USER:-mediawiki}
      - MYSQL_PASSWORD=${DB_PASSWORD}
    volumes:
      - mediawiki_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  mediawiki_data:
  mediawiki_config:
  mediawiki_db_data:
"#,
            r#"[{"name":"DB_PASSWORD","label":"MySQL Password","required":true,"default":"","secret":true},{"name":"DB_ROOT_PASSWORD","label":"MySQL Root Password","required":false,"default":"rootpassword","secret":true},{"name":"DB_HOST","label":"Database Host","required":false,"default":"mediawiki_db","secret":false},{"name":"DB_USER","label":"MySQL Username","required":false,"default":"mediawiki","secret":false},{"name":"DB_NAME","label":"MySQL Database Name","required":false,"default":"mediawiki","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mediawiki","secret":false}]"#,
        ),
        // ==================== AUTH / SSO ====================
        (
            "tpl-supertokens",
            "SuperTokens",
            "Open-source authentication platform — a self-hosted alternative to Auth0 and Firebase Auth. Supports email/password, social login, passwordless, and session management out of the box. Uses PostgreSQL.",
            "Auth/SSO",
            "supertokens",
            r#"services:
  supertokens:
    image: registry.supertokens.io/supertokens/supertokens-postgresql:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-supertokens}
    restart: unless-stopped
    ports:
      - "${PORT:-3567}:3567"
    environment:
      - POSTGRESQL_CONNECTION_URI=postgresql://${DB_USER:-supertokens}:${DB_PASSWORD}@supertokens_db:5432/${DB_NAME:-supertokens}
      - API_KEYS=${API_KEYS:-}
    depends_on:
      - supertokens_db
    labels:
      - "rivetr.managed=true"

  supertokens_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-supertokens}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-supertokens}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=${DB_NAME:-supertokens}
    volumes:
      - supertokens_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  supertokens_db_data:
"#,
            r#"[{"name":"DB_PASSWORD","label":"PostgreSQL Password","required":true,"default":"","secret":true},{"name":"API_KEYS","label":"SuperTokens API Keys (comma-separated, leave empty to disable API key auth)","required":false,"default":"","secret":true},{"name":"DB_USER","label":"PostgreSQL Username","required":false,"default":"supertokens","secret":false},{"name":"DB_NAME","label":"PostgreSQL Database Name","required":false,"default":"supertokens","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3567","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"supertokens","secret":false}]"#,
        ),
        // ==================== NETWORKING ====================
        (
            "tpl-netbird",
            "Netbird",
            "WireGuard-based mesh VPN and overlay network. Connect all your devices and servers into a secure private network without complex firewall rules. Fully self-hosted management plane.",
            "Networking",
            "netbird",
            r#"services:
  netbird:
    image: netbirdio/management:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-netbird}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
      - "${SIGNAL_PORT:-10000}:10000"
      - "${COTURN_PORT:-3478}:3478/udp"
    environment:
      - NETBIRD_DOMAIN=${NETBIRD_DOMAIN:-localhost}
      - NETBIRD_MGMT_API_PORT=${PORT:-80}
    volumes:
      - netbird_mgmt:/var/lib/netbird
    labels:
      - "rivetr.managed=true"

volumes:
  netbird_mgmt:
"#,
            r#"[{"name":"NETBIRD_DOMAIN","label":"Management Domain (e.g. netbird.example.com)","required":true,"default":"localhost","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false},{"name":"SIGNAL_PORT","label":"Signal Port","required":false,"default":"10000","secret":false},{"name":"COTURN_PORT","label":"COTURN/STUN Port (UDP)","required":false,"default":"3478","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"netbird","secret":false}]"#,
        ),
        // ==================== PRODUCTIVITY ====================
        (
            "tpl-affine",
            "AFFiNE",
            "Collaborative knowledge base and workspace — a Notion alternative with a canvas-first editor, database views, and offline-first design. Uses Redis and PostgreSQL.",
            "Productivity",
            "affine",
            r#"services:
  affine:
    image: ghcr.io/toeverything/affine-graphql:${VERSION:-stable}
    container_name: ${CONTAINER_NAME:-affine}
    restart: unless-stopped
    ports:
      - "${PORT:-3010}:3010"
    environment:
      - NODE_ENV=production
      - AFFINE_SERVER_HOST=${AFFINE_SERVER_HOST:-localhost}
      - AFFINE_SERVER_PORT=3010
      - DATABASE_URL=postgresql://${DB_USER:-affine}:${DB_PASSWORD}@affine_db:5432/${DB_NAME:-affine}
      - REDIS_SERVER_HOST=affine_redis
      - AFFINE_ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - AFFINE_ADMIN_PASSWORD=${ADMIN_PASSWORD}
    volumes:
      - affine_uploads:/app/uploads
      - affine_config:/root/.affine
    depends_on:
      - affine_db
      - affine_redis
    labels:
      - "rivetr.managed=true"

  affine_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-affine}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-affine}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=${DB_NAME:-affine}
    volumes:
      - affine_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  affine_redis:
    image: redis:7-alpine
    container_name: ${CONTAINER_NAME:-affine}-redis
    restart: unless-stopped
    volumes:
      - affine_redis_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  affine_uploads:
  affine_config:
  affine_db_data:
  affine_redis_data:
"#,
            r#"[{"name":"DB_PASSWORD","label":"PostgreSQL Password","required":true,"default":"","secret":true},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"AFFINE_SERVER_HOST","label":"Server Host / Domain (e.g. affine.example.com)","required":false,"default":"localhost","secret":false},{"name":"ADMIN_EMAIL","label":"Admin Email","required":false,"default":"admin@example.com","secret":false},{"name":"DB_USER","label":"PostgreSQL Username","required":false,"default":"affine","secret":false},{"name":"DB_NAME","label":"PostgreSQL Database Name","required":false,"default":"affine","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3010","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"stable","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"affine","secret":false}]"#,
        ),
        // ==================== FORMS ====================
        (
            "tpl-heyform",
            "HeyForm",
            "Open-source conversational form builder. Create engaging, interactive forms with conditional logic, file uploads, and integrations. Uses MongoDB and Redis.",
            "Forms",
            "heyform",
            r#"services:
  heyform:
    image: heyform/community-edition:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-heyform}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - APP_HOMEPAGE_URL=${APP_HOMEPAGE_URL:-http://localhost:8000}
      - SESSION_KEY=${SESSION_KEY:-a_long_random_session_secret}
      - MONGO_URI=mongodb://heyform_mongo:27017/${DB_NAME:-heyform}
      - REDIS_HOST=heyform_redis
      - REDIS_PORT=6379
    depends_on:
      - heyform_mongo
      - heyform_redis
    labels:
      - "rivetr.managed=true"

  heyform_mongo:
    image: mongo:6
    container_name: ${CONTAINER_NAME:-heyform}-mongo
    restart: unless-stopped
    environment:
      - MONGO_INITDB_DATABASE=${DB_NAME:-heyform}
    volumes:
      - heyform_mongo_data:/data/db
    labels:
      - "rivetr.managed=true"

  heyform_redis:
    image: redis:7-alpine
    container_name: ${CONTAINER_NAME:-heyform}-redis
    restart: unless-stopped
    volumes:
      - heyform_redis_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  heyform_mongo_data:
  heyform_redis_data:
"#,
            r#"[{"name":"SESSION_KEY","label":"Session Secret Key (long random string)","required":true,"default":"","secret":true},{"name":"APP_HOMEPAGE_URL","label":"App Homepage URL (e.g. https://forms.example.com)","required":false,"default":"http://localhost:8000","secret":false},{"name":"DB_NAME","label":"MongoDB Database Name","required":false,"default":"heyform","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8000","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"heyform","secret":false}]"#,
        ),
        (
            "tpl-opnform",
            "OpnForm",
            "Open-source Typeform alternative. Build beautiful, embeddable forms with conditional logic, file uploads, and native integrations. Uses PostgreSQL and Redis.",
            "Forms",
            "opnform",
            r#"services:
  opnform:
    image: jhumanj/opnform:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-opnform}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - APP_ENV=production
      - APP_URL=${APP_URL:-http://localhost}
      - APP_KEY=${APP_KEY:-base64:change_me_to_a_32_byte_base64_key=}
      - DB_CONNECTION=pgsql
      - DB_HOST=opnform_db
      - DB_PORT=5432
      - DB_DATABASE=${DB_NAME:-opnform}
      - DB_USERNAME=${DB_USER:-opnform}
      - DB_PASSWORD=${DB_PASSWORD}
      - REDIS_HOST=opnform_redis
      - REDIS_PORT=6379
      - CACHE_DRIVER=redis
      - SESSION_DRIVER=redis
      - QUEUE_CONNECTION=redis
    depends_on:
      - opnform_db
      - opnform_redis
    labels:
      - "rivetr.managed=true"

  opnform_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-opnform}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-opnform}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=${DB_NAME:-opnform}
    volumes:
      - opnform_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  opnform_redis:
    image: redis:7-alpine
    container_name: ${CONTAINER_NAME:-opnform}-redis
    restart: unless-stopped
    volumes:
      - opnform_redis_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  opnform_db_data:
  opnform_redis_data:
"#,
            r#"[{"name":"DB_PASSWORD","label":"PostgreSQL Password","required":true,"default":"","secret":true},{"name":"APP_KEY","label":"Laravel App Key (base64:... 32-byte key)","required":true,"default":"","secret":true},{"name":"APP_URL","label":"App URL (e.g. https://forms.example.com)","required":false,"default":"http://localhost","secret":false},{"name":"DB_USER","label":"PostgreSQL Username","required":false,"default":"opnform","secret":false},{"name":"DB_NAME","label":"PostgreSQL Database Name","required":false,"default":"opnform","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"opnform","secret":false}]"#,
        ),
        // ==================== DEVTOOLS ====================
        (
            "tpl-github-runner",
            "GitHub Actions Runner",
            "Self-hosted GitHub Actions runner. Connect a container to your GitHub repository or organization to run CI/CD workflows on your own infrastructure. Requires a runner registration token.",
            "DevTools",
            "github-runner",
            r#"services:
  github-runner:
    image: myoung34/github-runner:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-github-runner}
    restart: unless-stopped
    environment:
      - REPO_URL=${GITHUB_URL}
      - RUNNER_TOKEN=${RUNNER_TOKEN}
      - RUNNER_NAME=${RUNNER_NAME:-rivetr-runner}
      - RUNNER_WORKDIR=${RUNNER_WORKDIR:-/tmp/github-runner}
      - RUNNER_GROUP=${RUNNER_GROUP:-Default}
      - LABELS=${RUNNER_LABELS:-self-hosted,linux,x64}
      - EPHEMERAL=${EPHEMERAL:-false}
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - github_runner_work:/tmp/github-runner
    labels:
      - "rivetr.managed=true"

volumes:
  github_runner_work:
"#,
            r#"[{"name":"GITHUB_URL","label":"GitHub Repository or Organisation URL (e.g. https://github.com/owner/repo)","required":true,"default":"","secret":false},{"name":"RUNNER_TOKEN","label":"Runner Registration Token (from GitHub → Settings → Actions → Runners → New runner)","required":true,"default":"","secret":true},{"name":"RUNNER_NAME","label":"Runner Name","required":false,"default":"rivetr-runner","secret":false},{"name":"RUNNER_LABELS","label":"Runner Labels (comma-separated)","required":false,"default":"self-hosted,linux,x64","secret":false},{"name":"RUNNER_GROUP","label":"Runner Group","required":false,"default":"Default","secret":false},{"name":"EPHEMERAL","label":"Ephemeral Mode (true = runner exits after one job)","required":false,"default":"false","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"github-runner","secret":false}]"#,
        ),
        // ==================== COMMUNICATION ====================
        (
            "tpl-bluesky-pds",
            "Bluesky PDS",
            "Bluesky Personal Data Server — run your own node on the AT Protocol network. Host your own Bluesky accounts and federate with the broader Bluesky social network.",
            "Communication",
            "bluesky",
            r#"services:
  bluesky-pds:
    image: ghcr.io/bluesky-social/pds:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-bluesky-pds}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - PDS_HOSTNAME=${PDS_HOSTNAME:-localhost}
      - PDS_JWT_SECRET=${PDS_JWT_SECRET}
      - PDS_ADMIN_PASSWORD=${PDS_ADMIN_PASSWORD}
      - PDS_PLC_ROTATION_KEY_K256_PRIVATE_KEY_HEX=${PDS_PLC_ROTATION_KEY:-}
      - PDS_DATA_DIRECTORY=/pds
      - PDS_BLOBSTORE_DISK_LOCATION=/pds/blocks
      - PDS_DID_PLC_URL=https://plc.directory
      - PDS_BSKY_APP_VIEW_URL=https://api.bsky.app
      - PDS_BSKY_APP_VIEW_DID=did:web:api.bsky.app
      - PDS_REPORT_SERVICE_URL=https://mod.bsky.app
      - PDS_CRAWLERS=https://bsky.network
      - LOG_ENABLED=true
    volumes:
      - bluesky_pds_data:/pds
    labels:
      - "rivetr.managed=true"

volumes:
  bluesky_pds_data:
"#,
            r#"[{"name":"PDS_JWT_SECRET","label":"JWT Secret (long random string)","required":true,"default":"","secret":true},{"name":"PDS_ADMIN_PASSWORD","label":"PDS Admin Password","required":true,"default":"","secret":true},{"name":"PDS_HOSTNAME","label":"PDS Hostname (e.g. pds.example.com)","required":true,"default":"localhost","secret":false},{"name":"PDS_PLC_ROTATION_KEY","label":"PLC Rotation Key (hex-encoded k256 private key, auto-generated if empty)","required":false,"default":"","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"bluesky-pds","secret":false}]"#,
        ),
        // ==================== MEDIA ====================
        (
            "tpl-peertube",
            "PeerTube",
            "Self-hosted video platform — a federated, ActivityPub-compatible alternative to YouTube. Upload, transcode, and stream video with built-in channel management. Uses PostgreSQL and Redis.",
            "Media",
            "peertube",
            r#"services:
  peertube:
    image: chocobozzz/peertube:${VERSION:-production-bookworm}
    container_name: ${CONTAINER_NAME:-peertube}
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
    environment:
      - PEERTUBE_DB_HOSTNAME=peertube_db
      - PEERTUBE_DB_USERNAME=${DB_USER:-peertube}
      - PEERTUBE_DB_PASSWORD=${DB_PASSWORD}
      - PEERTUBE_DB_NAME=${DB_NAME:-peertube}
      - PEERTUBE_REDIS_HOSTNAME=peertube_redis
      - PEERTUBE_WEBSERVER_HOSTNAME=${PEERTUBE_HOSTNAME:-localhost}
      - PEERTUBE_WEBSERVER_PORT=${PORT:-9000}
      - PEERTUBE_WEBSERVER_HTTPS=${PEERTUBE_HTTPS:-false}
      - PEERTUBE_SMTP_HOSTNAME=${SMTP_HOST:-}
      - PEERTUBE_SMTP_PORT=${SMTP_PORT:-587}
      - PEERTUBE_SMTP_FROM=${SMTP_FROM:-peertube@example.com}
      - PEERTUBE_ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
    volumes:
      - peertube_data:/data
      - peertube_config:/config
    depends_on:
      - peertube_db
      - peertube_redis
    labels:
      - "rivetr.managed=true"

  peertube_db:
    image: postgres:15-alpine
    container_name: ${CONTAINER_NAME:-peertube}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-peertube}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=${DB_NAME:-peertube}
    volumes:
      - peertube_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  peertube_redis:
    image: redis:7-alpine
    container_name: ${CONTAINER_NAME:-peertube}-redis
    restart: unless-stopped
    volumes:
      - peertube_redis_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  peertube_data:
  peertube_config:
  peertube_db_data:
  peertube_redis_data:
"#,
            r#"[{"name":"DB_PASSWORD","label":"PostgreSQL Password","required":true,"default":"","secret":true},{"name":"PEERTUBE_HOSTNAME","label":"PeerTube Hostname (e.g. peertube.example.com)","required":false,"default":"localhost","secret":false},{"name":"ADMIN_EMAIL","label":"Admin Email","required":false,"default":"admin@example.com","secret":false},{"name":"PEERTUBE_HTTPS","label":"Use HTTPS (true/false)","required":false,"default":"false","secret":false},{"name":"DB_USER","label":"PostgreSQL Username","required":false,"default":"peertube","secret":false},{"name":"DB_NAME","label":"PostgreSQL Database Name","required":false,"default":"peertube","secret":false},{"name":"SMTP_HOST","label":"SMTP Hostname","required":false,"default":"","secret":false},{"name":"SMTP_PORT","label":"SMTP Port","required":false,"default":"587","secret":false},{"name":"SMTP_FROM","label":"SMTP From Address","required":false,"default":"peertube@example.com","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"9000","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"production-bookworm","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"peertube","secret":false}]"#,
        ),
        // ==================== PRODUCTIVITY ====================
        (
            "tpl-roundcube",
            "Roundcube",
            "Open-source webmail client with a clean, modern interface. Connect to any IMAP/SMTP server to read and send email from a browser. Supports plugins, address books, and calendar integration.",
            "Productivity",
            "roundcube",
            r#"services:
  roundcube:
    image: roundcube/roundcubemail:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-roundcube}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - ROUNDCUBEMAIL_DB_TYPE=pgsql
      - ROUNDCUBEMAIL_DB_HOST=roundcube_db
      - ROUNDCUBEMAIL_DB_PORT=5432
      - ROUNDCUBEMAIL_DB_USER=${DB_USER:-roundcube}
      - ROUNDCUBEMAIL_DB_PASSWORD=${DB_PASSWORD}
      - ROUNDCUBEMAIL_DB_NAME=${DB_NAME:-roundcubemail}
      - ROUNDCUBEMAIL_DEFAULT_HOST=${IMAP_HOST:-ssl://mail.example.com}
      - ROUNDCUBEMAIL_DEFAULT_PORT=${IMAP_PORT:-993}
      - ROUNDCUBEMAIL_SMTP_SERVER=${SMTP_HOST:-tls://mail.example.com}
      - ROUNDCUBEMAIL_SMTP_PORT=${SMTP_PORT:-587}
      - ROUNDCUBEMAIL_DES_KEY=${DES_KEY:-changeme24charkey!!!!!}
      - ROUNDCUBEMAIL_PLUGINS=${PLUGINS:-archive,zipdownload}
    depends_on:
      - roundcube_db
    labels:
      - "rivetr.managed=true"

  roundcube_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-roundcube}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-roundcube}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=${DB_NAME:-roundcubemail}
    volumes:
      - roundcube_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  roundcube_db_data:
"#,
            r#"[{"name":"DB_PASSWORD","label":"PostgreSQL Password","required":true,"default":"","secret":true},{"name":"DES_KEY","label":"DES Encryption Key (exactly 24 characters)","required":true,"default":"","secret":true},{"name":"IMAP_HOST","label":"IMAP Server (e.g. ssl://mail.example.com)","required":false,"default":"ssl://mail.example.com","secret":false},{"name":"IMAP_PORT","label":"IMAP Port","required":false,"default":"993","secret":false},{"name":"SMTP_HOST","label":"SMTP Server (e.g. tls://mail.example.com)","required":false,"default":"tls://mail.example.com","secret":false},{"name":"SMTP_PORT","label":"SMTP Port","required":false,"default":"587","secret":false},{"name":"PLUGINS","label":"Roundcube Plugins (comma-separated)","required":false,"default":"archive,zipdownload","secret":false},{"name":"DB_USER","label":"PostgreSQL Username","required":false,"default":"roundcube","secret":false},{"name":"DB_NAME","label":"PostgreSQL Database Name","required":false,"default":"roundcubemail","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"roundcube","secret":false}]"#,
        ),
        // ==================== INFRASTRUCTURE ====================
        (
            "tpl-docker-mailserver",
            "Mailserver (docker-mailserver)",
            "Full-featured, production-ready email server in a single container. Handles SMTP, IMAP, anti-spam (Rspamd), DKIM, SPF, DMARC, TLS, and more. Ideal for hosting your own email domain.",
            "Infrastructure",
            "mailserver",
            r#"services:
  mailserver:
    image: mailserver/docker-mailserver:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-mailserver}
    restart: unless-stopped
    hostname: ${MAIL_HOSTNAME:-mail.example.com}
    ports:
      - "${SMTP_PORT:-25}:25"
      - "${SUBMISSION_PORT:-587}:587"
      - "${SMTPS_PORT:-465}:465"
      - "${IMAP_PORT:-143}:143"
      - "${IMAPS_PORT:-993}:993"
    environment:
      - OVERRIDE_HOSTNAME=${MAIL_HOSTNAME:-mail.example.com}
      - DOMAINNAME=${MAIL_DOMAIN:-example.com}
      - POSTMASTER_ADDRESS=${POSTMASTER_EMAIL:-postmaster@example.com}
      - ENABLE_RSPAMD=${ENABLE_RSPAMD:-1}
      - ENABLE_CLAMAV=${ENABLE_CLAMAV:-0}
      - ENABLE_FAIL2BAN=${ENABLE_FAIL2BAN:-1}
      - ENABLE_SPAMASSASSIN=${ENABLE_SPAMASSASSIN:-0}
      - SSL_TYPE=${SSL_TYPE:-letsencrypt}
      - ONE_DIR=1
    volumes:
      - mailserver_data:/var/mail
      - mailserver_state:/var/mail-state
      - mailserver_config:/tmp/docker-mailserver
      - /etc/letsencrypt:/etc/letsencrypt:ro
    cap_add:
      - NET_ADMIN
    labels:
      - "rivetr.managed=true"

volumes:
  mailserver_data:
  mailserver_state:
  mailserver_config:
"#,
            r#"[{"name":"MAIL_HOSTNAME","label":"Mail Server Hostname (e.g. mail.example.com)","required":true,"default":"mail.example.com","secret":false},{"name":"MAIL_DOMAIN","label":"Mail Domain (e.g. example.com)","required":true,"default":"example.com","secret":false},{"name":"POSTMASTER_EMAIL","label":"Postmaster Email Address","required":false,"default":"postmaster@example.com","secret":false},{"name":"SSL_TYPE","label":"SSL Type (letsencrypt, manual, self-signed)","required":false,"default":"letsencrypt","secret":false},{"name":"ENABLE_RSPAMD","label":"Enable Rspamd Anti-Spam (1/0)","required":false,"default":"1","secret":false},{"name":"ENABLE_CLAMAV","label":"Enable ClamAV Anti-Virus (1/0)","required":false,"default":"0","secret":false},{"name":"ENABLE_FAIL2BAN","label":"Enable Fail2Ban (1/0)","required":false,"default":"1","secret":false},{"name":"SMTP_PORT","label":"SMTP Port","required":false,"default":"25","secret":false},{"name":"SUBMISSION_PORT","label":"Submission Port","required":false,"default":"587","secret":false},{"name":"SMTPS_PORT","label":"SMTPS Port","required":false,"default":"465","secret":false},{"name":"IMAP_PORT","label":"IMAP Port","required":false,"default":"143","secret":false},{"name":"IMAPS_PORT","label":"IMAPS Port","required":false,"default":"993","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mailserver","secret":false}]"#,
        ),
    ]
}
