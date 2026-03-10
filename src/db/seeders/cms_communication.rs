//! CMS and communication service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== CMS ====================
        (
            "wordpress",
            "WordPress",
            "The world's most popular content management system. Includes MySQL database.",
            "cms",
            "wordpress",
            r#"services:
  wordpress:
    image: wordpress:${WP_VERSION:-latest}
    container_name: ${CONTAINER_NAME:-wordpress}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - WORDPRESS_DB_HOST=wordpress_db
      - WORDPRESS_DB_USER=wordpress
      - WORDPRESS_DB_PASSWORD=${DB_PASSWORD:-wordpress}
      - WORDPRESS_DB_NAME=wordpress
    volumes:
      - wordpress_data:/var/www/html
    depends_on:
      - wordpress_db
    labels:
      - "rivetr.managed=true"

  wordpress_db:
    image: mysql:8.0
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=wordpress
      - MYSQL_USER=wordpress
      - MYSQL_PASSWORD=${DB_PASSWORD:-wordpress}
      - MYSQL_RANDOM_ROOT_PASSWORD=yes
    volumes:
      - wordpress_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  wordpress_data:
  wordpress_db_data:
"#,
            r#"[{"name":"WP_VERSION","label":"WordPress Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"wordpress","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "ghost",
            "Ghost",
            "Professional publishing platform. Modern alternative to WordPress for blogs and newsletters.",
            "cms",
            "ghost",
            r#"services:
  ghost:
    image: ghost:${GHOST_VERSION:-5-alpine}
    container_name: ${CONTAINER_NAME:-ghost}
    restart: unless-stopped
    ports:
      - "${PORT:-2368}:2368"
    environment:
      - url=${URL:-http://localhost:2368}
      - database__client=sqlite3
      - database__connection__filename=/var/lib/ghost/content/data/ghost.db
    volumes:
      - ghost_data:/var/lib/ghost/content
    labels:
      - "rivetr.managed=true"

volumes:
  ghost_data:
"#,
            r#"[{"name":"GHOST_VERSION","label":"Ghost Version","required":false,"default":"5-alpine","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"ghost","secret":false},{"name":"PORT","label":"Port","required":false,"default":"2368","secret":false},{"name":"URL","label":"Site URL","required":true,"default":"http://localhost:2368","secret":false}]"#,
        ),
        (
            "strapi",
            "Strapi",
            "Leading open-source headless CMS. Build powerful content APIs with a customizable admin panel.",
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
      - DATABASE_CLIENT=sqlite
      - DATABASE_FILENAME=/srv/app/.tmp/data.db
      - APP_KEYS=${APP_KEYS:-key1,key2,key3,key4}
      - API_TOKEN_SALT=${API_TOKEN_SALT:-change-me}
      - ADMIN_JWT_SECRET=${ADMIN_JWT_SECRET:-change-me}
      - JWT_SECRET=${JWT_SECRET:-change-me}
    volumes:
      - strapi_data:/srv/app
    labels:
      - "rivetr.managed=true"

volumes:
  strapi_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"strapi","secret":false},{"name":"PORT","label":"Port","required":false,"default":"1337","secret":false},{"name":"APP_KEYS","label":"App Keys (comma-separated)","required":true,"default":"","secret":true},{"name":"API_TOKEN_SALT","label":"API Token Salt","required":true,"default":"","secret":true},{"name":"ADMIN_JWT_SECRET","label":"Admin JWT Secret","required":true,"default":"","secret":true},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true}]"#,
        ),
        (
            "directus",
            "Directus",
            "Open data platform for managing any SQL database. Instant REST and GraphQL API with a no-code admin app.",
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
      - SECRET=${SECRET:-change-me-to-random-string}
      - DB_CLIENT=pg
      - DB_HOST=directus_db
      - DB_PORT=5432
      - DB_DATABASE=directus
      - DB_USER=directus
      - DB_PASSWORD=directus
      - ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD:-admin}
    volumes:
      - directus_uploads:/directus/uploads
      - directus_extensions:/directus/extensions
    depends_on:
      - directus_db
    labels:
      - "rivetr.managed=true"

  directus_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=directus
      - POSTGRES_USER=directus
      - POSTGRES_PASSWORD=directus
    volumes:
      - directus_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  directus_uploads:
  directus_extensions:
  directus_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"directus","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8055","secret":false},{"name":"SECRET","label":"Secret Key","required":true,"default":"","secret":true},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "payload-cms",
            "Payload CMS",
            "Next-generation TypeScript headless CMS. Code-first with powerful admin panel.",
            "cms",
            "payload-cms",
            r#"services:
  payload:
    image: payloadcms/payload:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-payload}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - PAYLOAD_SECRET=${PAYLOAD_SECRET:-change-me-to-random-string}
      - DATABASE_URI=mongodb://payload_db:27017/payload
    depends_on:
      - payload_db
    labels:
      - "rivetr.managed=true"

  payload_db:
    image: mongo:7
    restart: unless-stopped
    volumes:
      - payload_db_data:/data/db
    labels:
      - "rivetr.managed=true"

volumes:
  payload_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"payload","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"PAYLOAD_SECRET","label":"Payload Secret","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== COMMUNICATION ====================
        (
            "rocketchat",
            "Rocket.Chat",
            "Open-source team communication platform. Chat, video, file sharing, and integrations.",
            "communication",
            "rocketchat",
            r#"services:
  rocketchat:
    image: rocket.chat:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-rocketchat}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - ROOT_URL=${ROOT_URL:-http://localhost:3000}
      - MONGO_URL=mongodb://rocketchat_db:27017/rocketchat?replicaSet=rs0
      - MONGO_OPLOG_URL=mongodb://rocketchat_db:27017/local?replicaSet=rs0
    depends_on:
      - rocketchat_db
    labels:
      - "rivetr.managed=true"

  rocketchat_db:
    image: mongo:6
    restart: unless-stopped
    command: mongod --oplogSize 128 --replSet rs0
    volumes:
      - rocketchat_db_data:/data/db
    labels:
      - "rivetr.managed=true"

volumes:
  rocketchat_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"rocketchat","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"ROOT_URL","label":"Root URL","required":false,"default":"http://localhost:3000","secret":false}]"#,
        ),
        (
            "mattermost",
            "Mattermost",
            "Open-source platform for secure team collaboration. Self-hosted Slack alternative.",
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
      - TZ=UTC
      - MM_SQLSETTINGS_DRIVERNAME=postgres
      - MM_SQLSETTINGS_DATASOURCE=postgres://mattermost:mattermost@mattermost_db:5432/mattermost?sslmode=disable&connect_timeout=10
      - MM_SERVICESETTINGS_SITEURL=${SITE_URL:-http://localhost:8065}
    volumes:
      - mattermost_data:/mattermost/data
      - mattermost_logs:/mattermost/logs
      - mattermost_config:/mattermost/config
      - mattermost_plugins:/mattermost/plugins
    depends_on:
      - mattermost_db
    labels:
      - "rivetr.managed=true"

  mattermost_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=mattermost
      - POSTGRES_USER=mattermost
      - POSTGRES_PASSWORD=mattermost
    volumes:
      - mattermost_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  mattermost_data:
  mattermost_logs:
  mattermost_config:
  mattermost_plugins:
  mattermost_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mattermost","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8065","secret":false},{"name":"SITE_URL","label":"Site URL","required":false,"default":"http://localhost:8065","secret":false}]"#,
        ),
        (
            "matrix-synapse",
            "Matrix Synapse",
            "Decentralized communication server implementing the Matrix protocol. Federated chat and VoIP.",
            "communication",
            "matrix-synapse",
            r#"services:
  synapse:
    image: matrixdotorg/synapse:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-synapse}
    restart: unless-stopped
    ports:
      - "${PORT:-8008}:8008"
    environment:
      - SYNAPSE_SERVER_NAME=${SERVER_NAME:-localhost}
      - SYNAPSE_REPORT_STATS=${REPORT_STATS:-no}
      - SYNAPSE_NO_TLS=true
    volumes:
      - synapse_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  synapse_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"synapse","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8008","secret":false},{"name":"SERVER_NAME","label":"Server Name","required":true,"default":"localhost","secret":false},{"name":"REPORT_STATS","label":"Report Stats","required":false,"default":"no","secret":false}]"#,
        ),
    ]
}
