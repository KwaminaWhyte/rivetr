//! Sprint 19 service templates: media, developer tools, monitoring, communication, security, AI/ML, business

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== MEDIA & CONTENT ====================
        (
            "tpl-kavita",
            "Kavita",
            "Fast, feature rich, self-hosted digital library for manga, comics, and books. Supports CBZ, CBR, PDF, and EPUB formats.",
            "media",
            "kavita",
            r#"services:
  kavita:
    image: jvmilazz0/kavita:latest
    container_name: ${CONTAINER_NAME:-kavita}
    restart: unless-stopped
    ports:
      - "${PORT:-5000}:5000"
    volumes:
      - kavita_data:/kavita/config
      - ${LIBRARY_PATH:-/data/books}:/books
    labels:
      - "rivetr.managed=true"

volumes:
  kavita_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"5000","secret":false},{"name":"LIBRARY_PATH","label":"Library Path on Host","required":false,"default":"/data/books","secret":false}]"#,
        ),
        (
            "tpl-calibre-web",
            "Calibre-Web",
            "Clean web app for browsing, reading and downloading eBooks stored in a Calibre database. Supports OPDS, Kindle send-to.",
            "media",
            "calibreweb",
            r#"services:
  calibre-web:
    image: lscr.io/linuxserver/calibre-web:latest
    container_name: ${CONTAINER_NAME:-calibre-web}
    restart: unless-stopped
    ports:
      - "${PORT:-8083}:8083"
    environment:
      - PUID=${PUID:-1000}
      - PGID=${PGID:-1000}
      - TZ=${TZ:-UTC}
      - DOCKER_MODS=linuxserver/mods:universal-calibre
    volumes:
      - calibre_config:/config
      - ${BOOKS_PATH:-/data/books}:/books
    labels:
      - "rivetr.managed=true"

volumes:
  calibre_config:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8083","secret":false},{"name":"BOOKS_PATH","label":"Calibre Library Path","required":false,"default":"/data/books","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false}]"#,
        ),
        (
            "tpl-frigate",
            "Frigate",
            "NVR with realtime local object detection for IP cameras. Uses AI to detect people, cars, animals and more.",
            "media",
            "frigate",
            r#"services:
  frigate:
    image: ghcr.io/blakeblackshear/frigate:stable
    container_name: ${CONTAINER_NAME:-frigate}
    restart: unless-stopped
    privileged: true
    ports:
      - "${PORT:-5000}:5000"
      - "${RTSP_PORT:-8554}:8554"
      - "8555:8555/tcp"
      - "8555:8555/udp"
    environment:
      - FRIGATE_RTSP_PASSWORD=${RTSP_PASSWORD:-changeme}
    volumes:
      - frigate_config:/config
      - frigate_media:/media/frigate
      - /etc/localtime:/etc/localtime:ro
    shm_size: "64mb"
    labels:
      - "rivetr.managed=true"

volumes:
  frigate_config:
  frigate_media:
"#,
            r#"[{"name":"RTSP_PASSWORD","label":"RTSP Password","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"Web UI Port","required":false,"default":"5000","secret":false}]"#,
        ),
        (
            "tpl-fireshare",
            "Fireshare",
            "Self-hosted service to share your game clips, videos, and screenshots. Upload once, share with a simple link.",
            "media",
            "fireshare",
            r#"services:
  fireshare:
    image: shaneisrael9/fireshare:latest
    container_name: ${CONTAINER_NAME:-fireshare}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - APP_KEY=${APP_KEY:-changeme32characterslongatleast}
      - ADMIN_USERNAME=${ADMIN_USERNAME:-admin}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
    volumes:
      - fireshare_data:/data
      - fireshare_videos:/videos
    labels:
      - "rivetr.managed=true"

volumes:
  fireshare_data:
  fireshare_videos:
"#,
            r#"[{"name":"APP_KEY","label":"App Secret Key","required":true,"default":"changeme32characterslongatleast","secret":true},{"name":"ADMIN_USERNAME","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        // ==================== DEVELOPER TOOLS ====================
        (
            "tpl-forgejo",
            "Forgejo",
            "Self-hosted lightweight software forge. Community-driven fork of Gitea with focus on privacy and federation.",
            "devops",
            "forgejo",
            r#"services:
  forgejo:
    image: forgejo/forgejo:latest
    container_name: ${CONTAINER_NAME:-forgejo}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
      - "${SSH_PORT:-22}:22"
    environment:
      - USER_UID=1000
      - USER_GID=1000
      - FORGEJO__database__DB_TYPE=sqlite3
      - FORGEJO__database__PATH=/data/forgejo.db
      - FORGEJO__server__DOMAIN=${DOMAIN:-localhost}
      - FORGEJO__server__ROOT_URL=${ROOT_URL:-http://localhost:3000}
    volumes:
      - forgejo_data:/data
      - /etc/timezone:/etc/timezone:ro
      - /etc/localtime:/etc/localtime:ro
    labels:
      - "rivetr.managed=true"

volumes:
  forgejo_data:
"#,
            r#"[{"name":"DOMAIN","label":"Domain Name","required":false,"default":"localhost","secret":false},{"name":"ROOT_URL","label":"Root URL","required":false,"default":"http://localhost:3000","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"3000","secret":false},{"name":"SSH_PORT","label":"SSH Port","required":false,"default":"22","secret":false}]"#,
        ),
        (
            "tpl-code-server",
            "Code-Server",
            "VS Code in the browser. Run Visual Studio Code on any machine and access it from the browser anywhere.",
            "devops",
            "code-server",
            r#"services:
  code-server:
    image: lscr.io/linuxserver/code-server:latest
    container_name: ${CONTAINER_NAME:-code-server}
    restart: unless-stopped
    ports:
      - "${PORT:-8443}:8443"
    environment:
      - PUID=${PUID:-1000}
      - PGID=${PGID:-1000}
      - TZ=${TZ:-UTC}
      - PASSWORD=${PASSWORD:-changeme}
      - SUDO_PASSWORD=${SUDO_PASSWORD:-changeme}
      - DEFAULT_WORKSPACE=${DEFAULT_WORKSPACE:-/config/workspace}
    volumes:
      - code_server_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  code_server_config:
"#,
            r#"[{"name":"PASSWORD","label":"Web UI Password","required":true,"default":"changeme","secret":true},{"name":"SUDO_PASSWORD","label":"Sudo Password","required":false,"default":"changeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8443","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false}]"#,
        ),
        (
            "tpl-gitpod",
            "Gitpod Self-Hosted",
            "Ephemeral dev environments in the cloud. Launch instant, pre-configured development environments from any Git repository.",
            "devops",
            "gitpod",
            r#"services:
  gitpod:
    image: gitpod/workspace-full:latest
    container_name: ${CONTAINER_NAME:-gitpod}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - GITPOD_DOMAIN=${GITPOD_DOMAIN:-gitpod.example.com}
    volumes:
      - gitpod_data:/workspace
    labels:
      - "rivetr.managed=true"

volumes:
  gitpod_data:
"#,
            r#"[{"name":"GITPOD_DOMAIN","label":"Gitpod Domain","required":true,"default":"gitpod.example.com","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
        (
            "tpl-sentry",
            "Sentry",
            "Open-source error tracking and performance monitoring. Helps developers monitor and fix crashes in real time.",
            "devops",
            "sentry",
            r#"services:
  sentry:
    image: sentry:latest
    container_name: ${CONTAINER_NAME:-sentry}
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
    environment:
      - SENTRY_SECRET_KEY=${SECRET_KEY:-changeme-minimum-50-characters-long-secret-key}
      - SENTRY_POSTGRES_HOST=sentry_db
      - SENTRY_DB_USER=${DB_USER:-sentry}
      - SENTRY_DB_PASSWORD=${DB_PASSWORD:-sentry}
      - SENTRY_REDIS_HOST=sentry_redis
    depends_on:
      - sentry_db
      - sentry_redis
    volumes:
      - sentry_data:/var/lib/sentry/files
    labels:
      - "rivetr.managed=true"

  sentry_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-sentry}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-sentry}
      - POSTGRES_DB=sentry
    volumes:
      - sentry_pg_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  sentry_redis:
    image: redis:7-alpine
    restart: unless-stopped
    volumes:
      - sentry_redis_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  sentry_data:
  sentry_pg_data:
  sentry_redis_data:
"#,
            r#"[{"name":"SECRET_KEY","label":"Secret Key (50+ chars)","required":true,"default":"changeme-minimum-50-characters-long-secret-key","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"sentry","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"9000","secret":false}]"#,
        ),
        (
            "tpl-phpmyadmin",
            "phpMyAdmin",
            "Web interface for MySQL/MariaDB administration. Manage databases, tables, queries, and users from a browser.",
            "devops",
            "phpmyadmin",
            r#"services:
  phpmyadmin:
    image: phpmyadmin:latest
    container_name: ${CONTAINER_NAME:-phpmyadmin}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - PMA_HOST=${MYSQL_HOST:-mysql}
      - PMA_PORT=${MYSQL_PORT:-3306}
      - MYSQL_ROOT_PASSWORD=${MYSQL_ROOT_PASSWORD:-changeme}
      - UPLOAD_LIMIT=${UPLOAD_LIMIT:-300M}
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"MYSQL_HOST","label":"MySQL Host","required":true,"default":"mysql","secret":false},{"name":"MYSQL_ROOT_PASSWORD","label":"MySQL Root Password","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-pgadmin",
            "pgAdmin",
            "The most popular administration and development platform for PostgreSQL. Feature-rich web-based database management.",
            "devops",
            "pgadmin",
            r#"services:
  pgadmin:
    image: dpage/pgadmin4:latest
    container_name: ${CONTAINER_NAME:-pgadmin}
    restart: unless-stopped
    ports:
      - "${PORT:-5050}:80"
    environment:
      - PGADMIN_DEFAULT_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - PGADMIN_DEFAULT_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - PGADMIN_LISTEN_PORT=80
    volumes:
      - pgadmin_data:/var/lib/pgadmin
    labels:
      - "rivetr.managed=true"

volumes:
  pgadmin_data:
"#,
            r#"[{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"5050","secret":false}]"#,
        ),
        (
            "tpl-adminer",
            "Adminer",
            "Lightweight database management tool for MySQL, PostgreSQL, SQLite, MS SQL, and more. Single PHP file, fast and simple.",
            "devops",
            "adminer",
            r#"services:
  adminer:
    image: adminer:latest
    container_name: ${CONTAINER_NAME:-adminer}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - ADMINER_DEFAULT_SERVER=${DB_HOST:-localhost}
      - ADMINER_DESIGN=${DESIGN:-pepa-linha}
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"DB_HOST","label":"Default Database Host","required":false,"default":"localhost","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        // ==================== MONITORING & OBSERVABILITY ====================
        (
            "tpl-thanos",
            "Thanos",
            "Highly available Prometheus setup with long-term storage. Provides global query view across multiple Prometheus instances.",
            "monitoring",
            "thanos",
            r#"services:
  thanos-sidecar:
    image: quay.io/thanos/thanos:latest
    container_name: ${CONTAINER_NAME:-thanos-sidecar}
    restart: unless-stopped
    command:
      - sidecar
      - --tsdb.path=/prometheus
      - --prometheus.url=http://prometheus:9090
      - --grpc-address=0.0.0.0:10901
      - --http-address=0.0.0.0:10902
    ports:
      - "${GRPC_PORT:-10901}:10901"
      - "${HTTP_PORT:-10902}:10902"
    volumes:
      - prometheus_data:/prometheus
    labels:
      - "rivetr.managed=true"

  thanos-querier:
    image: quay.io/thanos/thanos:latest
    container_name: ${CONTAINER_NAME:-thanos}-querier
    restart: unless-stopped
    command:
      - query
      - --http-address=0.0.0.0:10901
      - --store=thanos-sidecar:10901
    ports:
      - "${QUERY_PORT:-10900}:10901"
    labels:
      - "rivetr.managed=true"

volumes:
  prometheus_data:
"#,
            r#"[{"name":"GRPC_PORT","label":"Sidecar gRPC Port","required":false,"default":"10901","secret":false},{"name":"HTTP_PORT","label":"Sidecar HTTP Port","required":false,"default":"10902","secret":false},{"name":"QUERY_PORT","label":"Query HTTP Port","required":false,"default":"10900","secret":false}]"#,
        ),
        (
            "tpl-opensearch",
            "OpenSearch",
            "Open-source Elasticsearch alternative from AWS. Distributed search and analytics engine for logs, metrics, and full-text search.",
            "search",
            "opensearch",
            r#"services:
  opensearch:
    image: opensearchproject/opensearch:latest
    container_name: ${CONTAINER_NAME:-opensearch}
    restart: unless-stopped
    ports:
      - "${PORT:-9200}:9200"
      - "9600:9600"
    environment:
      - cluster.name=${CLUSTER_NAME:-opensearch-cluster}
      - node.name=opensearch-node1
      - discovery.type=single-node
      - OPENSEARCH_INITIAL_ADMIN_PASSWORD=${ADMIN_PASSWORD:-Admin@opensearch1}
      - DISABLE_SECURITY_PLUGIN=${DISABLE_SECURITY:-false}
    volumes:
      - opensearch_data:/usr/share/opensearch/data
    ulimits:
      memlock:
        soft: -1
        hard: -1
      nofile:
        soft: 65536
        hard: 65536
    labels:
      - "rivetr.managed=true"

volumes:
  opensearch_data:
"#,
            r#"[{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"Admin@opensearch1","secret":true},{"name":"CLUSTER_NAME","label":"Cluster Name","required":false,"default":"opensearch-cluster","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"9200","secret":false},{"name":"DISABLE_SECURITY","label":"Disable Security Plugin","required":false,"default":"false","secret":false}]"#,
        ),
        (
            "tpl-opensearch-dashboards",
            "OpenSearch Dashboards",
            "Visualization UI for OpenSearch. Explore and visualize your search and analytics data with Kibana-compatible dashboards.",
            "search",
            "opensearch",
            r#"services:
  opensearch-dashboards:
    image: opensearchproject/opensearch-dashboards:latest
    container_name: ${CONTAINER_NAME:-opensearch-dashboards}
    restart: unless-stopped
    ports:
      - "${PORT:-5601}:5601"
    environment:
      - OPENSEARCH_HOSTS=${OPENSEARCH_URL:-http://opensearch:9200}
      - DISABLE_SECURITY_DASHBOARDS_PLUGIN=${DISABLE_SECURITY:-false}
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"OPENSEARCH_URL","label":"OpenSearch URL","required":true,"default":"http://opensearch:9200","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"5601","secret":false},{"name":"DISABLE_SECURITY","label":"Disable Security Plugin","required":false,"default":"false","secret":false}]"#,
        ),
        (
            "tpl-zabbix",
            "Zabbix",
            "Enterprise-class open-source distributed monitoring solution. Monitor network, servers, cloud, applications and services.",
            "monitoring",
            "zabbix",
            r#"services:
  zabbix-server:
    image: zabbix/zabbix-server-pgsql:ubuntu-latest
    container_name: ${CONTAINER_NAME:-zabbix-server}
    restart: unless-stopped
    ports:
      - "10051:10051"
    environment:
      - DB_SERVER_HOST=zabbix_db
      - POSTGRES_USER=${DB_USER:-zabbix}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-zabbix}
      - POSTGRES_DB=zabbix
    depends_on:
      - zabbix_db
    labels:
      - "rivetr.managed=true"

  zabbix-web:
    image: zabbix/zabbix-web-nginx-pgsql:ubuntu-latest
    container_name: ${CONTAINER_NAME:-zabbix}-web
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DB_SERVER_HOST=zabbix_db
      - POSTGRES_USER=${DB_USER:-zabbix}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-zabbix}
      - POSTGRES_DB=zabbix
      - ZBX_SERVER_HOST=zabbix-server
      - PHP_TZ=${TZ:-UTC}
    depends_on:
      - zabbix-server
      - zabbix_db
    labels:
      - "rivetr.managed=true"

  zabbix_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-zabbix}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-zabbix}
      - POSTGRES_DB=zabbix
    volumes:
      - zabbix_pg_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  zabbix_pg_data:
"#,
            r#"[{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"zabbix","secret":true},{"name":"PORT","label":"Web UI Port","required":false,"default":"8080","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false}]"#,
        ),
        (
            "tpl-speedtest-tracker",
            "Speedtest Tracker",
            "Self-hosted internet performance tracking application. Runs Ookla Speedtest on a schedule and stores historical results.",
            "monitoring",
            "speedtest-tracker",
            r#"services:
  speedtest-tracker:
    image: lscr.io/linuxserver/speedtest-tracker:latest
    container_name: ${CONTAINER_NAME:-speedtest-tracker}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - PUID=${PUID:-1000}
      - PGID=${PGID:-1000}
      - APP_KEY=${APP_KEY:-base64:changeme32characterlongkeyyyyy=}
      - DB_CONNECTION=${DB_CONNECTION:-sqlite}
      - SPEEDTEST_SCHEDULE=${SCHEDULE:-0 * * * *}
    volumes:
      - speedtest_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  speedtest_config:
"#,
            r#"[{"name":"APP_KEY","label":"App Key (base64)","required":true,"default":"base64:changeme32characterlongkeyyyyy=","secret":true},{"name":"SCHEDULE","label":"Cron Schedule","required":false,"default":"0 * * * *","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-scrutiny",
            "Scrutiny",
            "Hard drive S.M.A.R.T. monitoring, historical trends, and real failure detection. WebUI for smartmontools.",
            "monitoring",
            "scrutiny",
            r#"services:
  scrutiny:
    image: ghcr.io/analogj/scrutiny:master-omnibus
    container_name: ${CONTAINER_NAME:-scrutiny}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    cap_add:
      - SYS_RAWIO
    volumes:
      - scrutiny_config:/opt/scrutiny/config
      - scrutiny_influxdb:/opt/scrutiny/influxdb
      - /run/udev:/run/udev:ro
    devices:
      - /dev/sda
    labels:
      - "rivetr.managed=true"

volumes:
  scrutiny_config:
  scrutiny_influxdb:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        // ==================== COMMUNICATION & COLLABORATION ====================
        (
            "tpl-onlyoffice",
            "OnlyOffice Document Server",
            "Online office suite for collaborative document editing. Compatible with Microsoft Office formats, works with Nextcloud, Seafile.",
            "productivity",
            "onlyoffice",
            r#"services:
  onlyoffice:
    image: onlyoffice/documentserver:latest
    container_name: ${CONTAINER_NAME:-onlyoffice}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
      - "${SSL_PORT:-443}:443"
    environment:
      - JWT_ENABLED=${JWT_ENABLED:-true}
      - JWT_SECRET=${JWT_SECRET:-changeme}
      - JWT_HEADER=AuthorizationJWT
    volumes:
      - onlyoffice_data:/var/www/onlyoffice/Data
      - onlyoffice_logs:/var/log/onlyoffice
      - onlyoffice_db:/var/lib/postgresql
      - onlyoffice_rabbitmq:/var/lib/rabbitmq
      - onlyoffice_redis:/var/lib/redis
    labels:
      - "rivetr.managed=true"

volumes:
  onlyoffice_data:
  onlyoffice_logs:
  onlyoffice_db:
  onlyoffice_rabbitmq:
  onlyoffice_redis:
"#,
            r#"[{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"changeme","secret":true},{"name":"JWT_ENABLED","label":"Enable JWT Auth","required":false,"default":"true","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"80","secret":false}]"#,
        ),
        (
            "tpl-hedgedoc",
            "HedgeDoc",
            "Real-time collaborative markdown editor. Create documents, presentations, and notes together in the browser.",
            "productivity",
            "hedgedoc",
            r#"services:
  hedgedoc:
    image: quay.io/hedgedoc/hedgedoc:latest
    container_name: ${CONTAINER_NAME:-hedgedoc}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - CMD_DB_URL=postgres://${DB_USER:-hedgedoc}:${DB_PASSWORD:-hedgedoc}@hedgedoc_db:5432/hedgedoc
      - CMD_DOMAIN=${DOMAIN:-localhost}
      - CMD_URL_ADDPORT=${ADD_PORT:-true}
      - CMD_PORT=3000
      - CMD_PROTOCOL_USESSL=${USE_SSL:-false}
      - CMD_SESSION_SECRET=${SESSION_SECRET:-changeme}
      - CMD_ALLOW_ANONYMOUS=${ALLOW_ANONYMOUS:-true}
      - CMD_ALLOW_FREEURL=${ALLOW_FREEURL:-true}
    depends_on:
      - hedgedoc_db
    volumes:
      - hedgedoc_uploads:/hedgedoc/public/uploads
    labels:
      - "rivetr.managed=true"

  hedgedoc_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-hedgedoc}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-hedgedoc}
      - POSTGRES_DB=hedgedoc
    volumes:
      - hedgedoc_pg_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  hedgedoc_uploads:
  hedgedoc_pg_data:
"#,
            r#"[{"name":"DOMAIN","label":"Domain Name","required":true,"default":"localhost","secret":false},{"name":"SESSION_SECRET","label":"Session Secret","required":true,"default":"changeme","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"hedgedoc","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
        // ==================== PROJECT MANAGEMENT ====================
        (
            "tpl-openproject",
            "OpenProject",
            "Open-source project management software. Gantt charts, kanban boards, time tracking, wiki, and agile methodologies.",
            "productivity",
            "openproject",
            r#"services:
  openproject:
    image: openproject/openproject:latest
    container_name: ${CONTAINER_NAME:-openproject}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - OPENPROJECT_HOST__NAME=${DOMAIN:-localhost}
      - OPENPROJECT_HTTPS=${HTTPS:-false}
      - OPENPROJECT_SECRET_KEY_BASE=${SECRET_KEY:-changeme}
      - DATABASE_URL=postgres://${DB_USER:-openproject}:${DB_PASSWORD:-openproject}@openproject_db:5432/openproject
    depends_on:
      - openproject_db
    volumes:
      - openproject_assets:/var/openproject/assets
    labels:
      - "rivetr.managed=true"

  openproject_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-openproject}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-openproject}
      - POSTGRES_DB=openproject
    volumes:
      - openproject_pg_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  openproject_assets:
  openproject_pg_data:
"#,
            r#"[{"name":"DOMAIN","label":"Domain Name","required":true,"default":"localhost","secret":false},{"name":"SECRET_KEY","label":"Secret Key Base","required":true,"default":"changeme","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"openproject","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-redmine",
            "Redmine",
            "Flexible project management web application. Issue tracking, Gantt charts, wikis, time tracking and more.",
            "productivity",
            "redmine",
            r#"services:
  redmine:
    image: redmine:latest
    container_name: ${CONTAINER_NAME:-redmine}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - REDMINE_DB_POSTGRES=redmine_db
      - REDMINE_DB_USERNAME=${DB_USER:-redmine}
      - REDMINE_DB_PASSWORD=${DB_PASSWORD:-redmine}
      - REDMINE_DB_DATABASE=redmine
      - REDMINE_SECRET_KEY_BASE=${SECRET_KEY:-changeme}
    depends_on:
      - redmine_db
    volumes:
      - redmine_files:/usr/src/redmine/files
      - redmine_plugins:/usr/src/redmine/plugins
    labels:
      - "rivetr.managed=true"

  redmine_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-redmine}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-redmine}
      - POSTGRES_DB=redmine
    volumes:
      - redmine_pg_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  redmine_files:
  redmine_plugins:
  redmine_pg_data:
"#,
            r#"[{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"redmine","secret":true},{"name":"SECRET_KEY","label":"Secret Key Base","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
        (
            "tpl-wekan",
            "Wekan",
            "Open-source kanban board. Drag-and-drop cards, swimlanes, checklists, WIP limits, and more. Trello alternative.",
            "productivity",
            "wekan",
            r#"services:
  wekan:
    image: ghcr.io/wekan/wekan:latest
    container_name: ${CONTAINER_NAME:-wekan}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - MONGO_URL=mongodb://wekan_db:27017/wekan
      - ROOT_URL=${ROOT_URL:-http://localhost:8080}
      - WITH_API=${WITH_API:-true}
      - BROWSER_POLICY_ENABLED=true
      - TRUSTED_URL=${TRUSTED_URL:-}
    depends_on:
      - wekan_db
    labels:
      - "rivetr.managed=true"

  wekan_db:
    image: mongo:6
    container_name: ${CONTAINER_NAME:-wekan}-db
    restart: unless-stopped
    command: mongod --oplogSize 128
    volumes:
      - wekan_db_data:/data/db
      - wekan_db_dump:/dump
    labels:
      - "rivetr.managed=true"

volumes:
  wekan_db_data:
  wekan_db_dump:
"#,
            r#"[{"name":"ROOT_URL","label":"Root URL","required":true,"default":"http://localhost:8080","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-kanboard",
            "Kanboard",
            "Simple and open-source visual task board. Kanban methodology with swimlanes, subtasks, time tracking, and plugins.",
            "productivity",
            "kanboard",
            r#"services:
  kanboard:
    image: kanboard/kanboard:latest
    container_name: ${CONTAINER_NAME:-kanboard}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    volumes:
      - kanboard_data:/var/www/app/data
      - kanboard_plugins:/var/www/app/plugins
      - kanboard_ssl:/etc/nginx/ssl
    labels:
      - "rivetr.managed=true"

volumes:
  kanboard_data:
  kanboard_plugins:
  kanboard_ssl:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false}]"#,
        ),
        // ==================== STORAGE & FILES ====================
        (
            "tpl-minio",
            "MinIO",
            "High-performance S3-compatible object storage. Deploy on-premises object storage for AI/ML, analytics, and backup workloads.",
            "storage",
            "minio",
            r#"services:
  minio:
    image: minio/minio:latest
    container_name: ${CONTAINER_NAME:-minio}
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
      - "${CONSOLE_PORT:-9001}:9001"
    environment:
      - MINIO_ROOT_USER=${ROOT_USER:-admin}
      - MINIO_ROOT_PASSWORD=${ROOT_PASSWORD:-changeme}
    command: server /data --console-address ":9001"
    volumes:
      - minio_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  minio_data:
"#,
            r#"[{"name":"ROOT_USER","label":"Root User","required":true,"default":"admin","secret":false},{"name":"ROOT_PASSWORD","label":"Root Password","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"API Port","required":false,"default":"9000","secret":false},{"name":"CONSOLE_PORT","label":"Console Port","required":false,"default":"9001","secret":false}]"#,
        ),
        (
            "tpl-sftpgo",
            "SFTPGo",
            "Fully featured and configurable SFTP server with optional HTTP/S, FTP/S and WebDAV support. Multiple storage backends.",
            "storage",
            "sftpgo",
            r#"services:
  sftpgo:
    image: drakkan/sftpgo:latest
    container_name: ${CONTAINER_NAME:-sftpgo}
    restart: unless-stopped
    ports:
      - "${SFTP_PORT:-8022}:2022"
      - "${HTTP_PORT:-8080}:8080"
      - "${FTPS_PORT:-9090}:9090"
    environment:
      - SFTPGO_LOG_LEVEL=${LOG_LEVEL:-info}
      - SFTPGO_DEFAULT_ADMIN_USERNAME=${ADMIN_USERNAME:-admin}
      - SFTPGO_DEFAULT_ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
    volumes:
      - sftpgo_data:/var/lib/sftpgo
      - sftpgo_home:/home/sftpgo
    labels:
      - "rivetr.managed=true"

volumes:
  sftpgo_data:
  sftpgo_home:
"#,
            r#"[{"name":"ADMIN_USERNAME","label":"Admin Username","required":true,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"changeme","secret":true},{"name":"SFTP_PORT","label":"SFTP Port","required":false,"default":"8022","secret":false},{"name":"HTTP_PORT","label":"Web Admin Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-tandoor-recipes",
            "Tandoor Recipes",
            "Recipe manager with meal planning, shopping lists, and nutritional values. Self-hosted cooking assistant.",
            "productivity",
            "tandoor",
            r#"services:
  tandoor:
    image: vabene1111/recipes:latest
    container_name: ${CONTAINER_NAME:-tandoor}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DB_ENGINE=django.db.backends.postgresql
      - POSTGRES_HOST=tandoor_db
      - POSTGRES_PORT=5432
      - POSTGRES_USER=${DB_USER:-tandoor}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-tandoor}
      - POSTGRES_DB=tandoor
      - SECRET_KEY=${SECRET_KEY:-changeme}
      - ALLOWED_HOSTS=${ALLOWED_HOSTS:-*}
      - GUNICORN_MEDIA=${GUNICORN_MEDIA:-0}
    depends_on:
      - tandoor_db
    volumes:
      - tandoor_media:/opt/recipes/mediafiles
      - tandoor_static:/opt/recipes/staticfiles
    labels:
      - "rivetr.managed=true"

  tandoor_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-tandoor}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-tandoor}
      - POSTGRES_DB=tandoor
    volumes:
      - tandoor_pg_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  tandoor_media:
  tandoor_static:
  tandoor_pg_data:
"#,
            r#"[{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"changeme","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"tandoor","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-mealie",
            "Mealie",
            "Self-hosted recipe manager and meal planner. Beautiful UI, recipe scraping, meal planning, and shopping lists.",
            "productivity",
            "mealie",
            r#"services:
  mealie:
    image: ghcr.io/mealie-recipes/mealie:latest
    container_name: ${CONTAINER_NAME:-mealie}
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
    environment:
      - ALLOW_SIGNUP=${ALLOW_SIGNUP:-true}
      - PUID=${PUID:-1000}
      - PGID=${PGID:-1000}
      - TZ=${TZ:-UTC}
      - MAX_WORKERS=${MAX_WORKERS:-1}
      - WEB_CONCURRENCY=${WEB_CONCURRENCY:-1}
      - BASE_URL=${BASE_URL:-http://localhost:9000}
      - DB_ENGINE=sqlite
    volumes:
      - mealie_data:/app/data
    labels:
      - "rivetr.managed=true"

volumes:
  mealie_data:
"#,
            r#"[{"name":"BASE_URL","label":"Base URL","required":false,"default":"http://localhost:9000","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"9000","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"ALLOW_SIGNUP","label":"Allow Signup","required":false,"default":"true","secret":false}]"#,
        ),
        (
            "tpl-grocy",
            "Grocy",
            "Self-hosted groceries and household management solution. Track stock, plan meals, manage chores, and more.",
            "productivity",
            "grocy",
            r#"services:
  grocy:
    image: lscr.io/linuxserver/grocy:latest
    container_name: ${CONTAINER_NAME:-grocy}
    restart: unless-stopped
    ports:
      - "${PORT:-9283}:80"
    environment:
      - PUID=${PUID:-1000}
      - PGID=${PGID:-1000}
      - TZ=${TZ:-UTC}
    volumes:
      - grocy_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  grocy_config:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"9283","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false}]"#,
        ),
        // ==================== ANALYTICS ====================
        (
            "tpl-umami",
            "Umami",
            "Simple, fast, privacy-focused web analytics. Open-source Google Analytics alternative with a clean dashboard.",
            "analytics",
            "umami",
            r#"services:
  umami:
    image: ghcr.io/umami-software/umami:postgresql-latest
    container_name: ${CONTAINER_NAME:-umami}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgresql://${DB_USER:-umami}:${DB_PASSWORD:-umami}@umami_db:5432/umami
      - DATABASE_TYPE=postgresql
      - APP_SECRET=${APP_SECRET:-changeme}
    depends_on:
      - umami_db
    labels:
      - "rivetr.managed=true"

  umami_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-umami}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-umami}
      - POSTGRES_DB=umami
    volumes:
      - umami_pg_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  umami_pg_data:
"#,
            r#"[{"name":"APP_SECRET","label":"App Secret","required":true,"default":"changeme","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"umami","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
        (
            "tpl-plausible",
            "Plausible Analytics",
            "Lightweight, open-source web analytics. Privacy-friendly Google Analytics alternative. Compliant with GDPR, CCPA.",
            "analytics",
            "plausible",
            r#"services:
  plausible:
    image: ghcr.io/plausible/community-edition:v2
    container_name: ${CONTAINER_NAME:-plausible}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    command: sh -c "sleep 10 && /entrypoint.sh db createdb && /entrypoint.sh db migrate && /entrypoint.sh run"
    environment:
      - BASE_URL=${BASE_URL:-http://localhost:8000}
      - SECRET_KEY_BASE=${SECRET_KEY:-changeme}
      - DATABASE_URL=postgres://${DB_USER:-plausible}:${DB_PASSWORD:-plausible}@plausible_db:5432/plausible
      - CLICKHOUSE_DATABASE_URL=http://plausible_ch:8123/plausible
      - MAILER_EMAIL=${MAILER_EMAIL:-noreply@example.com}
      - SMTP_HOST_ADDR=${SMTP_HOST:-}
      - SMTP_HOST_PORT=${SMTP_PORT:-587}
    depends_on:
      - plausible_db
      - plausible_ch
    labels:
      - "rivetr.managed=true"

  plausible_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-plausible}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-plausible}
      - POSTGRES_DB=plausible
    volumes:
      - plausible_pg_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  plausible_ch:
    image: clickhouse/clickhouse-server:23-alpine
    restart: unless-stopped
    volumes:
      - plausible_ch_data:/var/lib/clickhouse
      - plausible_ch_logs:/var/log/clickhouse-server
    ulimits:
      nofile:
        soft: 262144
        hard: 262144
    labels:
      - "rivetr.managed=true"

volumes:
  plausible_pg_data:
  plausible_ch_data:
  plausible_ch_logs:
"#,
            r#"[{"name":"BASE_URL","label":"Base URL","required":true,"default":"http://localhost:8000","secret":false},{"name":"SECRET_KEY","label":"Secret Key Base","required":true,"default":"changeme","secret":true},{"name":"DB_PASSWORD","label":"DB Password","required":true,"default":"plausible","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8000","secret":false}]"#,
        ),
        // ==================== SECURITY ====================
        (
            "tpl-wazuh-manager",
            "Wazuh Manager",
            "Open-source security platform for threat detection, incident response, and compliance monitoring. SIEM + XDR.",
            "security",
            "wazuh",
            r#"services:
  wazuh-manager:
    image: wazuh/wazuh-manager:latest
    container_name: ${CONTAINER_NAME:-wazuh-manager}
    restart: unless-stopped
    ports:
      - "1514:1514"
      - "1515:1515"
      - "514:514/udp"
      - "55000:55000"
    environment:
      - INDEXER_URL=https://wazuh-indexer:9200
      - INDEXER_USERNAME=${INDEXER_USER:-admin}
      - INDEXER_PASSWORD=${INDEXER_PASSWORD:-SecurePassword123!}
      - FILEBEAT_SSL_VERIFICATION_MODE=full
      - SSL_CERTIFICATE_AUTHORITIES=/etc/ssl/root-ca.pem
      - SSL_CERTIFICATE=/etc/ssl/filebeat.pem
      - SSL_KEY=/etc/ssl/filebeat.key
      - API_USERNAME=${API_USER:-wazuh-wui}
      - API_PASSWORD=${API_PASSWORD:-MyS3cr3tPassword!}
    volumes:
      - wazuh_api_configuration:/var/ossec/api/configuration
      - wazuh_etc:/var/ossec/etc
      - wazuh_logs:/var/ossec/logs
      - wazuh_queue:/var/ossec/queue
      - wazuh_var_multigroups:/var/ossec/var/multigroups
      - wazuh_integrations:/var/ossec/integrations
      - wazuh_active_response:/var/ossec/active-response/bin
      - wazuh_agentless:/var/ossec/agentless
      - wazuh_wodles:/var/ossec/wodles
    labels:
      - "rivetr.managed=true"

volumes:
  wazuh_api_configuration:
  wazuh_etc:
  wazuh_logs:
  wazuh_queue:
  wazuh_var_multigroups:
  wazuh_integrations:
  wazuh_active_response:
  wazuh_agentless:
  wazuh_wodles:
"#,
            r#"[{"name":"INDEXER_PASSWORD","label":"Indexer Password","required":true,"default":"SecurePassword123!","secret":true},{"name":"API_PASSWORD","label":"API Password","required":true,"default":"MyS3cr3tPassword!","secret":true},{"name":"INDEXER_USER","label":"Indexer Username","required":false,"default":"admin","secret":false}]"#,
        ),
        (
            "tpl-nginx-ui",
            "Nginx UI",
            "Web UI for Nginx management. Configure reverse proxies, SSL certificates (Let's Encrypt), and server blocks with ease.",
            "devops",
            "nginx-ui",
            r#"services:
  nginx-ui:
    image: uozi/nginx-ui:latest
    container_name: ${CONTAINER_NAME:-nginx-ui}
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
      - "80:80"
      - "443:443"
    environment:
      - TZ=${TZ:-UTC}
    volumes:
      - nginx_ui_config:/etc/nginx
      - nginx_ui_data:/etc/nginx-ui
    labels:
      - "rivetr.managed=true"

volumes:
  nginx_ui_config:
  nginx_ui_data:
"#,
            r#"[{"name":"PORT","label":"UI Port","required":false,"default":"9000","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false}]"#,
        ),
        (
            "tpl-step-ca",
            "Step CA",
            "Open-source private certificate authority and ACME server. Issue, manage, and renew internal TLS certificates.",
            "security",
            "step-ca",
            r#"services:
  step-ca:
    image: smallstep/step-ca:latest
    container_name: ${CONTAINER_NAME:-step-ca}
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
    environment:
      - DOCKER_STEPCA_INIT_NAME=${CA_NAME:-Smallstep}
      - DOCKER_STEPCA_INIT_DNS_NAMES=${DNS_NAMES:-localhost}
      - DOCKER_STEPCA_INIT_REMOTE_MANAGEMENT=${REMOTE_MGMT:-true}
      - DOCKER_STEPCA_INIT_PASSWORD=${CA_PASSWORD:-changeme}
    volumes:
      - step_ca_data:/home/step
    labels:
      - "rivetr.managed=true"

volumes:
  step_ca_data:
"#,
            r#"[{"name":"CA_NAME","label":"CA Name","required":false,"default":"Smallstep","secret":false},{"name":"DNS_NAMES","label":"DNS Names (comma-separated)","required":false,"default":"localhost","secret":false},{"name":"CA_PASSWORD","label":"CA Password","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"9000","secret":false}]"#,
        ),
        // ==================== AI / ML ====================
        (
            "tpl-comfyui",
            "ComfyUI",
            "Powerful modular stable diffusion GUI and backend. Node-based workflow for image generation with SD, SDXL, and more.",
            "ai-ml",
            "comfyui",
            r#"services:
  comfyui:
    image: yanwk/comfyui-boot:latest
    container_name: ${CONTAINER_NAME:-comfyui}
    restart: unless-stopped
    ports:
      - "${PORT:-8188}:8188"
    environment:
      - CLI_ARGS=${CLI_ARGS:---listen 0.0.0.0}
    volumes:
      - comfyui_data:/root
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: all
              capabilities: [gpu]
    labels:
      - "rivetr.managed=true"

volumes:
  comfyui_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8188","secret":false},{"name":"CLI_ARGS","label":"CLI Arguments","required":false,"default":"--listen 0.0.0.0","secret":false}]"#,
        ),
        (
            "tpl-stable-diffusion-webui",
            "Stable Diffusion WebUI",
            "A1111's feature-rich web UI for Stable Diffusion. Text-to-image, image-to-image, inpainting, extensions, and more.",
            "ai-ml",
            "stable-diffusion",
            r#"services:
  stable-diffusion-webui:
    image: universonic/stable-diffusion-webui:latest
    container_name: ${CONTAINER_NAME:-stable-diffusion-webui}
    restart: unless-stopped
    ports:
      - "${PORT:-7860}:7860"
    environment:
      - COMMANDLINE_ARGS=${COMMANDLINE_ARGS:---listen --api}
    volumes:
      - sd_models:/stable-diffusion-webui/models
      - sd_outputs:/stable-diffusion-webui/outputs
      - sd_extensions:/stable-diffusion-webui/extensions
    labels:
      - "rivetr.managed=true"

volumes:
  sd_models:
  sd_outputs:
  sd_extensions:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"7860","secret":false},{"name":"COMMANDLINE_ARGS","label":"CLI Arguments","required":false,"default":"--listen --api","secret":false}]"#,
        ),
        (
            "tpl-tabbyml",
            "Tabby",
            "Self-hosted AI coding assistant, open-source alternative to GitHub Copilot. Provides code completions and chat.",
            "ai-ml",
            "tabby",
            r#"services:
  tabby:
    image: tabbyml/tabby:latest
    container_name: ${CONTAINER_NAME:-tabby}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    command: serve --model ${MODEL:-StarCoder-1B} --device ${DEVICE:-cpu}
    environment:
      - TABBY_DISABLE_USAGE_COLLECTION=${DISABLE_TELEMETRY:-1}
    volumes:
      - tabby_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  tabby_data:
"#,
            r#"[{"name":"MODEL","label":"Model Name","required":false,"default":"StarCoder-1B","secret":false},{"name":"DEVICE","label":"Device (cpu/cuda)","required":false,"default":"cpu","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-openedai-speech",
            "OpenedAI Speech",
            "Self-hosted OpenAI-compatible text-to-speech API. Supports multiple TTS engines including Piper, Coqui, and more.",
            "ai-ml",
            "openai",
            r#"services:
  openedai-speech:
    image: ghcr.io/matatonic/openedai-speech:latest
    container_name: ${CONTAINER_NAME:-openedai-speech}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - TTS_HOME=/root/.local/share/tts
      - HF_HOME=/root/.cache/huggingface
    volumes:
      - openedai_speech_voices:/root/.local/share/tts
      - openedai_speech_config:/app/config
    labels:
      - "rivetr.managed=true"

volumes:
  openedai_speech_voices:
  openedai_speech_config:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8000","secret":false}]"#,
        ),
        // ==================== DATABASES ====================
        (
            "tpl-edgedb",
            "EdgeDB",
            "Graph-relational database with a modern type system. Combines the simplicity of NoSQL with the power of SQL.",
            "databases",
            "edgedb",
            r#"services:
  edgedb:
    image: edgedb/edgedb:latest
    container_name: ${CONTAINER_NAME:-edgedb}
    restart: unless-stopped
    ports:
      - "${PORT:-5656}:5656"
    environment:
      - EDGEDB_SERVER_PASSWORD=${PASSWORD:-changeme}
      - EDGEDB_SERVER_SECURITY=${SECURITY:-strict}
      - EDGEDB_SERVER_BACKEND_DSN=${BACKEND_DSN:-}
    volumes:
      - edgedb_data:/var/lib/edgedb/data
    labels:
      - "rivetr.managed=true"

volumes:
  edgedb_data:
"#,
            r#"[{"name":"PASSWORD","label":"Server Password","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"5656","secret":false},{"name":"SECURITY","label":"Security Mode","required":false,"default":"strict","secret":false}]"#,
        ),
        (
            "tpl-influxdb",
            "InfluxDB",
            "Open-source time series database built for high-performance storage of metrics, events, and real-time analytics.",
            "databases",
            "influxdb",
            r#"services:
  influxdb:
    image: influxdb:2
    container_name: ${CONTAINER_NAME:-influxdb}
    restart: unless-stopped
    ports:
      - "${PORT:-8086}:8086"
    environment:
      - DOCKER_INFLUXDB_INIT_MODE=setup
      - DOCKER_INFLUXDB_INIT_USERNAME=${ADMIN_USER:-admin}
      - DOCKER_INFLUXDB_INIT_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - DOCKER_INFLUXDB_INIT_ORG=${ORG:-myorg}
      - DOCKER_INFLUXDB_INIT_BUCKET=${BUCKET:-mybucket}
      - DOCKER_INFLUXDB_INIT_ADMIN_TOKEN=${ADMIN_TOKEN:-mytoken}
    volumes:
      - influxdb_data:/var/lib/influxdb2
      - influxdb_config:/etc/influxdb2
    labels:
      - "rivetr.managed=true"

volumes:
  influxdb_data:
  influxdb_config:
"#,
            r#"[{"name":"ADMIN_USER","label":"Admin Username","required":true,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"changeme","secret":true},{"name":"ADMIN_TOKEN","label":"Admin Token","required":true,"default":"mytoken","secret":true},{"name":"ORG","label":"Organization","required":false,"default":"myorg","secret":false},{"name":"BUCKET","label":"Initial Bucket","required":false,"default":"mybucket","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8086","secret":false}]"#,
        ),
        (
            "tpl-solr",
            "Apache Solr",
            "Enterprise-grade open-source search platform. Blazing fast full-text search, hit highlighting, faceted search, and clustering.",
            "search",
            "solr",
            r#"services:
  solr:
    image: solr:latest
    container_name: ${CONTAINER_NAME:-solr}
    restart: unless-stopped
    ports:
      - "${PORT:-8983}:8983"
    command: solr-precreate ${CORE_NAME:-gettingstarted}
    volumes:
      - solr_data:/var/solr
    labels:
      - "rivetr.managed=true"

volumes:
  solr_data:
"#,
            r#"[{"name":"CORE_NAME","label":"Initial Core Name","required":false,"default":"gettingstarted","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8983","secret":false}]"#,
        ),
        (
            "tpl-fauna",
            "FaunaDB",
            "Distributed document-relational database delivered as a cloud API. Globally distributed, serverless, and consistent.",
            "databases",
            "fauna",
            r#"services:
  faunadb:
    image: fauna/faunadb:latest
    container_name: ${CONTAINER_NAME:-faunadb}
    restart: unless-stopped
    ports:
      - "${PORT:-8443}:8443"
      - "8084:8084"
    volumes:
      - fauna_data:/var/lib/faunadb
      - fauna_logs:/var/log/faunadb
    labels:
      - "rivetr.managed=true"

volumes:
  fauna_data:
  fauna_logs:
"#,
            r#"[{"name":"PORT","label":"API Port","required":false,"default":"8443","secret":false}]"#,
        ),
        // ==================== NETWORKING ====================
        (
            "tpl-traefik",
            "Traefik",
            "Modern HTTP reverse proxy and load balancer. Automatic service discovery, SSL termination, and dynamic configuration.",
            "devops",
            "traefik",
            r#"services:
  traefik:
    image: traefik:latest
    container_name: ${CONTAINER_NAME:-traefik}
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
      - "${DASHBOARD_PORT:-8080}:8080"
    command:
      - "--api.insecure=${API_INSECURE:-true}"
      - "--api.dashboard=${ENABLE_DASHBOARD:-true}"
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - traefik_certs:/letsencrypt
    labels:
      - "rivetr.managed=true"

volumes:
  traefik_certs:
"#,
            r#"[{"name":"DASHBOARD_PORT","label":"Dashboard Port","required":false,"default":"8080","secret":false},{"name":"API_INSECURE","label":"Enable Insecure API","required":false,"default":"true","secret":false},{"name":"ENABLE_DASHBOARD","label":"Enable Dashboard","required":false,"default":"true","secret":false}]"#,
        ),
        (
            "tpl-caddy",
            "Caddy",
            "Powerful, enterprise-ready, open-source web server with automatic HTTPS. Reverse proxy, file server, and more.",
            "devops",
            "caddy",
            r#"services:
  caddy:
    image: caddy:latest
    container_name: ${CONTAINER_NAME:-caddy}
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
      - "443:443/udp"
    volumes:
      - caddy_data:/data
      - caddy_config:/config
      - ${CADDYFILE_PATH:-./Caddyfile}:/etc/caddy/Caddyfile
    labels:
      - "rivetr.managed=true"

volumes:
  caddy_data:
  caddy_config:
"#,
            r#"[{"name":"CADDYFILE_PATH","label":"Caddyfile Path","required":false,"default":"./Caddyfile","secret":false}]"#,
        ),
        // ==================== COMMUNICATION ====================
        (
            "tpl-gotify-server",
            "Gotify",
            "Simple self-hosted server for sending and receiving messages via a REST API. Push notifications for your apps.",
            "communication",
            "gotify",
            r#"services:
  gotify:
    image: gotify/server:latest
    container_name: ${CONTAINER_NAME:-gotify}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - GOTIFY_DEFAULTUSER_NAME=${ADMIN_USER:-admin}
      - GOTIFY_DEFAULTUSER_PASS=${ADMIN_PASSWORD:-changeme}
      - GOTIFY_SERVER_PORT=80
    volumes:
      - gotify_data:/app/data
    labels:
      - "rivetr.managed=true"

volumes:
  gotify_data:
"#,
            r#"[{"name":"ADMIN_USER","label":"Admin Username","required":true,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false}]"#,
        ),
        (
            "tpl-apprise",
            "Apprise",
            "Push notification service that works with every major notification platform. Send alerts to 65+ services from one API.",
            "communication",
            "apprise",
            r#"services:
  apprise:
    image: caronc/apprise:latest
    container_name: ${CONTAINER_NAME:-apprise}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - APPRISE_STATELESS_URLS=${STATELESS_URLS:-}
    volumes:
      - apprise_config:/config
      - apprise_plugin:/plugin
    labels:
      - "rivetr.managed=true"

volumes:
  apprise_config:
  apprise_plugin:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8000","secret":false},{"name":"STATELESS_URLS","label":"Default Notification URLs","required":false,"default":"","secret":false}]"#,
        ),
        // ==================== DASHBOARDS ====================
        (
            "tpl-heimdall",
            "Heimdall",
            "Application dashboard and launcher. Organize and access all your web applications from a single beautiful page.",
            "productivity",
            "heimdall",
            r#"services:
  heimdall:
    image: lscr.io/linuxserver/heimdall:latest
    container_name: ${CONTAINER_NAME:-heimdall}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
      - "${SSL_PORT:-443}:443"
    environment:
      - PUID=${PUID:-1000}
      - PGID=${PGID:-1000}
      - TZ=${TZ:-UTC}
    volumes:
      - heimdall_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  heimdall_config:
"#,
            r#"[{"name":"PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"SSL_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false}]"#,
        ),
        (
            "tpl-dasherr",
            "Dasherr",
            "Minimalist dashboard for self-hosted apps. Clean card-based layout with app health indicators and no database required.",
            "productivity",
            "dasherr",
            r#"services:
  dasherr:
    image: ersei/dasherr:latest
    container_name: ${CONTAINER_NAME:-dasherr}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    volumes:
      - dasherr_config:/www/dasherr/data
    labels:
      - "rivetr.managed=true"

volumes:
  dasherr_config:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
        // ==================== BUSINESS ====================
        (
            "tpl-crater",
            "Crater",
            "Open-source invoicing application. Create professional invoices, estimates, track expenses, and manage payments.",
            "business",
            "crater",
            r#"services:
  crater:
    image: ghcr.io/crater-invoice/crater:latest
    container_name: ${CONTAINER_NAME:-crater}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:80"
    environment:
      - APP_NAME=${APP_NAME:-Crater}
      - APP_ENV=${APP_ENV:-production}
      - APP_URL=${APP_URL:-http://localhost:8000}
      - APP_KEY=${APP_KEY:-base64:changeme32characterlongkeyyyyy=}
      - DB_CONNECTION=mysql
      - DB_HOST=crater_db
      - DB_PORT=3306
      - DB_DATABASE=crater
      - DB_USERNAME=${DB_USER:-crater}
      - DB_PASSWORD=${DB_PASSWORD:-crater}
    depends_on:
      - crater_db
    volumes:
      - crater_storage:/var/www/html/storage
    labels:
      - "rivetr.managed=true"

  crater_db:
    image: mysql:8
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-rootpassword}
      - MYSQL_DATABASE=crater
      - MYSQL_USER=${DB_USER:-crater}
      - MYSQL_PASSWORD=${DB_PASSWORD:-crater}
    volumes:
      - crater_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  crater_storage:
  crater_db_data:
"#,
            r#"[{"name":"APP_URL","label":"App URL","required":true,"default":"http://localhost:8000","secret":false},{"name":"APP_KEY","label":"App Key (base64)","required":true,"default":"base64:changeme32characterlongkeyyyyy=","secret":true},{"name":"DB_PASSWORD","label":"DB Password","required":true,"default":"crater","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8000","secret":false}]"#,
        ),
        (
            "tpl-hoppscotch",
            "Hoppscotch",
            "Open-source API development platform. Lightweight, fast alternative to Postman with a beautiful web UI.",
            "devops",
            "hoppscotch",
            r#"services:
  hoppscotch:
    image: hoppscotch/hoppscotch:latest
    container_name: ${CONTAINER_NAME:-hoppscotch}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
      - "${BACKEND_PORT:-3170}:3170"
      - "${ADMIN_PORT:-3100}:3100"
    environment:
      - DATABASE_URL=postgresql://${DB_USER:-hoppscotch}:${DB_PASSWORD:-hoppscotch}@hoppscotch_db:5432/hoppscotch
      - JWT_SECRET=${JWT_SECRET:-changeme}
      - SESSION_SECRET=${SESSION_SECRET:-changeme}
      - MAILER_SMTP_URL=${SMTP_URL:-smtp://localhost:587}
      - VITE_ALLOWED_AUTH_PROVIDERS=${AUTH_PROVIDERS:-EMAIL}
      - MAILER_ADDRESS_FROM=${FROM_EMAIL:-noreply@example.com}
    depends_on:
      - hoppscotch_db
    labels:
      - "rivetr.managed=true"

  hoppscotch_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-hoppscotch}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-hoppscotch}
      - POSTGRES_DB=hoppscotch
    volumes:
      - hoppscotch_pg_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  hoppscotch_pg_data:
"#,
            r#"[{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"changeme","secret":true},{"name":"SESSION_SECRET","label":"Session Secret","required":true,"default":"changeme","secret":true},{"name":"DB_PASSWORD","label":"DB Password","required":true,"default":"hoppscotch","secret":true},{"name":"PORT","label":"Frontend Port","required":false,"default":"3000","secret":false}]"#,
        ),
    ]
}
