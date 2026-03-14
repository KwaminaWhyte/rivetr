//! Sprint 16 service templates: databases, monitoring, devops, storage, communication, AI, security, productivity

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== DATABASES ====================
        (
            "tpl-keydb",
            "KeyDB",
            "Redis-compatible database with multithreading support. Drop-in replacement for Redis with higher throughput.",
            "databases",
            "keydb",
            r#"services:
  keydb:
    image: eqalpha/keydb:latest
    container_name: ${CONTAINER_NAME:-keydb}
    restart: unless-stopped
    ports:
      - "${PORT:-6379}:6379"
    command: keydb-server /etc/keydb/keydb.conf --requirepass ${REQUIREPASS:-changeme} --server-threads 2
    volumes:
      - keydb_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  keydb_data:
"#,
            r#"[{"name":"REQUIREPASS","label":"Password","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"6379","secret":false}]"#,
        ),
        (
            "tpl-dragonflydb",
            "DragonflyDB",
            "Ultra-fast Redis replacement with 25x higher throughput. Compatible with Redis clients and commands.",
            "databases",
            "dragonflydb",
            r#"services:
  dragonfly:
    image: docker.dragonflydb.io/dragonflydb/dragonfly:latest
    container_name: ${CONTAINER_NAME:-dragonfly}
    restart: unless-stopped
    ports:
      - "${PORT:-6379}:6379"
    ulimits:
      memlock: -1
    volumes:
      - dragonfly_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  dragonfly_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"6379","secret":false}]"#,
        ),
        (
            "tpl-clickhouse",
            "ClickHouse",
            "Column-oriented analytics database for real-time queries on large datasets. Blazing fast OLAP performance.",
            "databases",
            "clickhouse",
            r#"services:
  clickhouse:
    image: clickhouse/clickhouse-server:latest
    container_name: ${CONTAINER_NAME:-clickhouse}
    restart: unless-stopped
    ports:
      - "${PORT:-8123}:8123"
      - "${NATIVE_PORT:-9000}:9000"
    environment:
      - CLICKHOUSE_DB=${CLICKHOUSE_DB:-default}
      - CLICKHOUSE_USER=${CLICKHOUSE_USER:-default}
      - CLICKHOUSE_PASSWORD=${CLICKHOUSE_PASSWORD:-changeme}
    volumes:
      - clickhouse_data:/var/lib/clickhouse
      - clickhouse_logs:/var/log/clickhouse-server
    ulimits:
      nofile:
        soft: 262144
        hard: 262144
    labels:
      - "rivetr.managed=true"

volumes:
  clickhouse_data:
  clickhouse_logs:
"#,
            r#"[{"name":"CLICKHOUSE_DB","label":"Database Name","required":false,"default":"default","secret":false},{"name":"CLICKHOUSE_USER","label":"Username","required":false,"default":"default","secret":false},{"name":"CLICKHOUSE_PASSWORD","label":"Password","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"HTTP Port","required":false,"default":"8123","secret":false},{"name":"NATIVE_PORT","label":"Native Port","required":false,"default":"9000","secret":false}]"#,
        ),
        (
            "tpl-cockroachdb",
            "CockroachDB",
            "Distributed SQL database with horizontal scalability and strong consistency. PostgreSQL-compatible wire protocol.",
            "databases",
            "cockroachdb",
            r#"services:
  cockroachdb:
    image: cockroachdb/cockroach:latest
    container_name: ${CONTAINER_NAME:-cockroachdb}
    restart: unless-stopped
    ports:
      - "${PORT:-26257}:26257"
      - "${ADMIN_PORT:-8080}:8080"
    command: start-single-node --insecure --advertise-addr=localhost
    volumes:
      - cockroachdb_data:/cockroach/cockroach-data
    labels:
      - "rivetr.managed=true"

volumes:
  cockroachdb_data:
"#,
            r#"[{"name":"PORT","label":"SQL Port","required":false,"default":"26257","secret":false},{"name":"ADMIN_PORT","label":"Admin UI Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-timescaledb",
            "TimescaleDB",
            "Time-series database built on PostgreSQL. Ideal for IoT, metrics, and time-series workloads with full SQL support.",
            "databases",
            "timescaledb",
            r#"services:
  timescaledb:
    image: timescale/timescaledb:latest-pg16
    container_name: ${CONTAINER_NAME:-timescaledb}
    restart: unless-stopped
    ports:
      - "${PORT:-5432}:5432"
    environment:
      - POSTGRES_USER=${POSTGRES_USER:-postgres}
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-changeme}
      - POSTGRES_DB=${POSTGRES_DB:-timeseries}
    volumes:
      - timescaledb_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  timescaledb_data:
"#,
            r#"[{"name":"POSTGRES_USER","label":"PostgreSQL User","required":false,"default":"postgres","secret":false},{"name":"POSTGRES_PASSWORD","label":"PostgreSQL Password","required":true,"default":"changeme","secret":true},{"name":"POSTGRES_DB","label":"Database Name","required":false,"default":"timeseries","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"5432","secret":false}]"#,
        ),
        (
            "tpl-surrealdb",
            "SurrealDB",
            "Multi-model database supporting document, graph, relational, and time-series data. Built-in auth and real-time queries.",
            "databases",
            "surrealdb",
            r#"services:
  surrealdb:
    image: surrealdb/surrealdb:latest
    container_name: ${CONTAINER_NAME:-surrealdb}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    command: start --user ${SURREAL_USER:-root} --pass ${SURREAL_PASS:-changeme} file://data/surreal.db
    environment:
      - SURREAL_USER=${SURREAL_USER:-root}
      - SURREAL_PASS=${SURREAL_PASS:-changeme}
    volumes:
      - surrealdb_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  surrealdb_data:
"#,
            r#"[{"name":"SURREAL_USER","label":"Root Username","required":false,"default":"root","secret":false},{"name":"SURREAL_PASS","label":"Root Password","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8000","secret":false}]"#,
        ),
        (
            "tpl-cassandra",
            "Apache Cassandra",
            "Wide-column NoSQL database designed for high availability and linear scalability across multiple data centers.",
            "databases",
            "cassandra",
            r#"services:
  cassandra:
    image: cassandra:latest
    container_name: ${CONTAINER_NAME:-cassandra}
    restart: unless-stopped
    ports:
      - "${PORT:-9042}:9042"
    environment:
      - MAX_HEAP_SIZE=${MAX_HEAP_SIZE:-512M}
      - HEAP_NEWSIZE=${HEAP_NEWSIZE:-100M}
      - CASSANDRA_CLUSTER_NAME=${CLUSTER_NAME:-RivetrCluster}
      - CASSANDRA_DC=${DATACENTER:-dc1}
    volumes:
      - cassandra_data:/var/lib/cassandra
    labels:
      - "rivetr.managed=true"

volumes:
  cassandra_data:
"#,
            r#"[{"name":"MAX_HEAP_SIZE","label":"Max Heap Size","required":false,"default":"512M","secret":false},{"name":"HEAP_NEWSIZE","label":"Heap New Size","required":false,"default":"100M","secret":false},{"name":"CLUSTER_NAME","label":"Cluster Name","required":false,"default":"RivetrCluster","secret":false},{"name":"DATACENTER","label":"Datacenter Name","required":false,"default":"dc1","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"9042","secret":false}]"#,
        ),
        (
            "tpl-neo4j",
            "Neo4j",
            "Leading graph database for connected data. Ideal for social networks, recommendation engines, and fraud detection.",
            "databases",
            "neo4j",
            r#"services:
  neo4j:
    image: neo4j:latest
    container_name: ${CONTAINER_NAME:-neo4j}
    restart: unless-stopped
    ports:
      - "${PORT:-7474}:7474"
      - "${BOLT_PORT:-7687}:7687"
    environment:
      - NEO4J_AUTH=${NEO4J_AUTH:-neo4j/changeme}
      - NEO4J_PLUGINS=["apoc"]
    volumes:
      - neo4j_data:/data
      - neo4j_logs:/logs
      - neo4j_plugins:/plugins
    labels:
      - "rivetr.managed=true"

volumes:
  neo4j_data:
  neo4j_logs:
  neo4j_plugins:
"#,
            r#"[{"name":"NEO4J_AUTH","label":"Auth (user/password)","required":true,"default":"neo4j/changeme","secret":true},{"name":"PORT","label":"Browser Port","required":false,"default":"7474","secret":false},{"name":"BOLT_PORT","label":"Bolt Port","required":false,"default":"7687","secret":false}]"#,
        ),

        // ==================== MONITORING & OBSERVABILITY ====================
        (
            "tpl-uptime-kuma",
            "Uptime Kuma",
            "Self-hosted uptime monitoring tool. Monitor HTTP(S), TCP, DNS, and more with beautiful status pages and notifications.",
            "monitoring",
            "uptime-kuma",
            r#"services:
  uptime-kuma:
    image: louislam/uptime-kuma:latest
    container_name: ${CONTAINER_NAME:-uptime-kuma}
    restart: unless-stopped
    ports:
      - "${PORT:-3001}:3001"
    volumes:
      - uptime_kuma_data:/app/data
    labels:
      - "rivetr.managed=true"

volumes:
  uptime_kuma_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"3001","secret":false}]"#,
        ),
        (
            "tpl-prometheus",
            "Prometheus",
            "Open-source monitoring and alerting toolkit. Collects metrics from configured targets and stores them in a time-series database.",
            "monitoring",
            "prometheus",
            r#"services:
  prometheus:
    image: prom/prometheus:latest
    container_name: ${CONTAINER_NAME:-prometheus}
    restart: unless-stopped
    ports:
      - "${PORT:-9090}:9090"
    command:
      - "--config.file=/etc/prometheus/prometheus.yml"
      - "--storage.tsdb.path=/prometheus"
      - "--storage.tsdb.retention.time=${RETENTION:-15d}"
      - "--web.enable-lifecycle"
    volumes:
      - prometheus_config:/etc/prometheus
      - prometheus_data:/prometheus
    labels:
      - "rivetr.managed=true"

volumes:
  prometheus_config:
  prometheus_data:
"#,
            r#"[{"name":"RETENTION","label":"Data Retention","required":false,"default":"15d","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"9090","secret":false}]"#,
        ),
        (
            "tpl-jaeger",
            "Jaeger",
            "Open-source distributed tracing platform. Monitor and troubleshoot transactions in complex distributed systems.",
            "monitoring",
            "jaeger",
            r#"services:
  jaeger:
    image: jaegertracing/all-in-one:latest
    container_name: ${CONTAINER_NAME:-jaeger}
    restart: unless-stopped
    ports:
      - "${PORT:-16686}:16686"
      - "${COLLECTOR_PORT:-14268}:14268"
      - "6831:6831/udp"
      - "6832:6832/udp"
    environment:
      - COLLECTOR_OTLP_ENABLED=true
      - SPAN_STORAGE_TYPE=badger
      - BADGER_EPHEMERAL=false
      - BADGER_DIRECTORY_VALUE=/badger/data
      - BADGER_DIRECTORY_KEY=/badger/key
    volumes:
      - jaeger_data:/badger
    labels:
      - "rivetr.managed=true"

volumes:
  jaeger_data:
"#,
            r#"[{"name":"PORT","label":"UI Port","required":false,"default":"16686","secret":false},{"name":"COLLECTOR_PORT","label":"Collector Port","required":false,"default":"14268","secret":false}]"#,
        ),
        (
            "tpl-otel-collector",
            "OpenTelemetry Collector",
            "Vendor-agnostic pipeline for collecting, processing, and exporting telemetry data (traces, metrics, logs).",
            "monitoring",
            "opentelemetry",
            r#"services:
  otel-collector:
    image: otel/opentelemetry-collector-contrib:latest
    container_name: ${CONTAINER_NAME:-otel-collector}
    restart: unless-stopped
    ports:
      - "${GRPC_PORT:-4317}:4317"
      - "${HTTP_PORT:-4318}:4318"
      - "8888:8888"
      - "8889:8889"
    volumes:
      - otel_config:/etc/otelcol-contrib
    labels:
      - "rivetr.managed=true"

volumes:
  otel_config:
"#,
            r#"[{"name":"GRPC_PORT","label":"gRPC Port","required":false,"default":"4317","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"4318","secret":false}]"#,
        ),
        (
            "tpl-graylog",
            "Graylog",
            "Centralized log management platform. Collect, index, and analyze log data from any source with powerful search and alerting.",
            "monitoring",
            "graylog",
            r#"services:
  graylog:
    image: graylog/graylog:latest
    container_name: ${CONTAINER_NAME:-graylog}
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
      - "12201:12201/udp"
      - "1514:1514"
    environment:
      - GRAYLOG_PASSWORD_SECRET=${GRAYLOG_PASSWORD_SECRET:-somepasswordsecret}
      - GRAYLOG_ROOT_PASSWORD_SHA2=${GRAYLOG_ROOT_PASSWORD_SHA2:-8c6976e5b5410415bde908bd4dee15dfb167a9c873fc4bb8a81f6f2ab448a918}
      - GRAYLOG_HTTP_EXTERNAL_URI=http://127.0.0.1:${PORT:-9000}/
      - GRAYLOG_ELASTICSEARCH_HOSTS=http://graylog_search:9200
      - GRAYLOG_MONGODB_URI=mongodb://graylog_mongo:27017/graylog
    depends_on:
      - graylog_mongo
      - graylog_search
    volumes:
      - graylog_data:/usr/share/graylog/data
    labels:
      - "rivetr.managed=true"

  graylog_mongo:
    image: mongo:6
    restart: unless-stopped
    volumes:
      - graylog_mongo_data:/data/db
    labels:
      - "rivetr.managed=true"

  graylog_search:
    image: opensearchproject/opensearch:2
    restart: unless-stopped
    environment:
      - discovery.type=single-node
      - plugins.security.disabled=true
      - OPENSEARCH_JAVA_OPTS=-Xms512m -Xmx512m
    volumes:
      - graylog_search_data:/usr/share/opensearch/data
    labels:
      - "rivetr.managed=true"

volumes:
  graylog_data:
  graylog_mongo_data:
  graylog_search_data:
"#,
            r#"[{"name":"GRAYLOG_PASSWORD_SECRET","label":"Password Secret (min 16 chars)","required":true,"default":"somepasswordsecret","secret":true},{"name":"GRAYLOG_ROOT_PASSWORD_SHA2","label":"Root Password SHA2 (default: admin)","required":true,"default":"8c6976e5b5410415bde908bd4dee15dfb167a9c873fc4bb8a81f6f2ab448a918","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"9000","secret":false}]"#,
        ),

        // ==================== DEVOPS / CI ====================
        (
            "tpl-concourse-ci",
            "Concourse CI",
            "Container-native CI/CD system with a clean pipeline model. Every task runs in an isolated container.",
            "devtools",
            "concourse",
            r#"services:
  concourse:
    image: concourse/concourse:latest
    container_name: ${CONTAINER_NAME:-concourse}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    command: web
    environment:
      - CONCOURSE_EXTERNAL_URL=http://localhost:${PORT:-8080}
      - CONCOURSE_POSTGRES_HOST=concourse_db
      - CONCOURSE_POSTGRES_USER=concourse
      - CONCOURSE_POSTGRES_PASSWORD=${CONCOURSE_POSTGRES_PASSWORD:-concourse}
      - CONCOURSE_POSTGRES_DATABASE=concourse
      - CONCOURSE_ADD_LOCAL_USER=${CONCOURSE_ADD_LOCAL_USER:-admin:changeme}
      - CONCOURSE_MAIN_TEAM_LOCAL_USER=admin
      - CONCOURSE_CLUSTER_NAME=${CLUSTER_NAME:-rivetr}
    depends_on:
      - concourse_db
    volumes:
      - concourse_keys:/concourse-keys
    labels:
      - "rivetr.managed=true"

  concourse_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=concourse
      - POSTGRES_PASSWORD=${CONCOURSE_POSTGRES_PASSWORD:-concourse}
      - POSTGRES_DB=concourse
    volumes:
      - concourse_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  concourse_keys:
  concourse_db_data:
"#,
            r#"[{"name":"CONCOURSE_POSTGRES_PASSWORD","label":"PostgreSQL Password","required":true,"default":"concourse","secret":true},{"name":"CONCOURSE_ADD_LOCAL_USER","label":"Local User (user:password)","required":false,"default":"admin:changeme","secret":true},{"name":"CLUSTER_NAME","label":"Cluster Name","required":false,"default":"rivetr","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-argocd",
            "Argo CD",
            "Declarative GitOps continuous delivery tool for Kubernetes. NOTE: Requires a running Kubernetes cluster to function.",
            "devtools",
            "argocd",
            r#"services:
  argocd:
    image: quay.io/argoproj/argocd:latest
    container_name: ${CONTAINER_NAME:-argocd}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
      - "${HTTPS_PORT:-8443}:8443"
    command:
      - argocd-server
      - --insecure
    volumes:
      - argocd_data:/home/argocd
    labels:
      - "rivetr.managed=true"

volumes:
  argocd_data:
"#,
            r#"[{"name":"PORT","label":"HTTP Port","required":false,"default":"8080","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"8443","secret":false}]"#,
        ),
        (
            "tpl-tekton-dashboard",
            "Tekton Dashboard",
            "Web-based UI for Tekton Pipelines. Visualize and manage your Kubernetes-native CI/CD pipelines.",
            "devtools",
            "tekton",
            r#"services:
  tekton-dashboard:
    image: gcr.io/tekton-releases/github.com/tektoncd/dashboard/cmd/dashboard:latest
    container_name: ${CONTAINER_NAME:-tekton-dashboard}
    restart: unless-stopped
    ports:
      - "${PORT:-9097}:9097"
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"9097","secret":false}]"#,
        ),
        (
            "tpl-gitlab-runner",
            "GitLab Runner",
            "CI/CD runner that executes GitLab pipeline jobs. Supports Docker, shell, and Kubernetes executors.",
            "devtools",
            "gitlab",
            r#"services:
  gitlab-runner:
    image: gitlab/gitlab-runner:latest
    container_name: ${CONTAINER_NAME:-gitlab-runner}
    restart: unless-stopped
    environment:
      - CI_SERVER_URL=${GITLAB_URL:-https://gitlab.com}
      - REGISTRATION_TOKEN=${REGISTRATION_TOKEN:-your-registration-token}
    volumes:
      - gitlab_runner_config:/etc/gitlab-runner
      - /var/run/docker.sock:/var/run/docker.sock
    labels:
      - "rivetr.managed=true"

volumes:
  gitlab_runner_config:
"#,
            r#"[{"name":"GITLAB_URL","label":"GitLab Instance URL","required":false,"default":"https://gitlab.com","secret":false},{"name":"REGISTRATION_TOKEN","label":"Registration Token","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== STORAGE & FILES ====================
        (
            "tpl-restic-rest-server",
            "Restic REST Server",
            "HTTP backend for Restic backups. Lightweight and self-hosted storage target for the Restic backup tool.",
            "infrastructure",
            "restic",
            r#"services:
  restic-rest-server:
    image: restic/rest-server:latest
    container_name: ${CONTAINER_NAME:-restic-rest-server}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - OPTIONS=${OPTIONS:---no-auth}
      - PASSWORD_FILE=${PASSWORD_FILE:-}
    volumes:
      - restic_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  restic_data:
"#,
            r#"[{"name":"OPTIONS","label":"Server Options","required":false,"default":"--no-auth","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8000","secret":false}]"#,
        ),

        // ==================== COMMUNICATION & COLLABORATION ====================
        (
            "tpl-revolt",
            "Revolt",
            "Open-source Discord alternative with end-to-end encryption support. Privacy-first team communication platform.",
            "communication",
            "revolt",
            r#"services:
  revolt-web:
    image: ghcr.io/revoltchat/web:latest
    container_name: ${CONTAINER_NAME:-revolt-web}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - VITE_API_URL=http://revolt-server:8000
    depends_on:
      - revolt-server
    labels:
      - "rivetr.managed=true"

  revolt-server:
    image: ghcr.io/revoltchat/server:latest
    container_name: revolt-server
    restart: unless-stopped
    environment:
      - DB_URI=${DB_URI:-mongodb://revolt_mongo:27017/revolt}
      - REDIS_URI=${REDIS_URI:-redis://revolt_redis:6379}
      - PUBLIC_URL=http://localhost:${PORT:-3000}
    depends_on:
      - revolt_mongo
      - revolt_redis
    volumes:
      - revolt_uploads:/uploads
    labels:
      - "rivetr.managed=true"

  revolt_mongo:
    image: mongo:6
    restart: unless-stopped
    volumes:
      - revolt_mongo_data:/data/db
    labels:
      - "rivetr.managed=true"

  revolt_redis:
    image: redis:7-alpine
    restart: unless-stopped
    volumes:
      - revolt_redis_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  revolt_uploads:
  revolt_mongo_data:
  revolt_redis_data:
"#,
            r#"[{"name":"DB_URI","label":"MongoDB URI","required":false,"default":"mongodb://revolt_mongo:27017/revolt","secret":false},{"name":"REDIS_URI","label":"Redis URI","required":false,"default":"redis://revolt_redis:6379","secret":false},{"name":"PORT","label":"Web Port","required":false,"default":"3000","secret":false}]"#,
        ),
        (
            "tpl-humhub",
            "HumHub",
            "Open-source social network platform for teams and communities. Create your own private social network.",
            "communication",
            "humhub",
            r#"services:
  humhub:
    image: ghcr.io/humhub/humhub:latest
    container_name: ${CONTAINER_NAME:-humhub}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - HUMHUB_DB_HOST=humhub_db
      - HUMHUB_DB_USER=humhub
      - HUMHUB_DB_PASSWORD=${DB_PASSWORD:-changeme}
      - HUMHUB_DB_NAME=humhub
    depends_on:
      - humhub_db
    volumes:
      - humhub_data:/var/www/localhost/htdocs/protected/runtime
      - humhub_uploads:/var/www/localhost/htdocs/uploads
    labels:
      - "rivetr.managed=true"

  humhub_db:
    image: mariadb:11
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=humhub
      - MYSQL_USER=humhub
      - MYSQL_PASSWORD=${DB_PASSWORD:-changeme}
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-rootchangeme}
    volumes:
      - humhub_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  humhub_data:
  humhub_uploads:
  humhub_db_data:
"#,
            r#"[{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"changeme","secret":true},{"name":"DB_ROOT_PASSWORD","label":"Database Root Password","required":false,"default":"rootchangeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false}]"#,
        ),
        (
            "tpl-dolibarr",
            "Dolibarr",
            "Open-source ERP and CRM for small and medium businesses. Manage customers, invoices, orders, stock, and more.",
            "business",
            "dolibarr",
            r#"services:
  dolibarr:
    image: dolibarr/dolibarr:latest
    container_name: ${CONTAINER_NAME:-dolibarr}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - DOLI_DB_HOST=dolibarr_db
      - DOLI_DB_USER=dolibarr
      - DOLI_DB_PASSWORD=${DOLI_DB_PASSWORD:-changeme}
      - DOLI_DB_NAME=dolibarr
      - DOLI_ADMIN_LOGIN=${DOLI_ADMIN_LOGIN:-admin}
      - DOLI_ADMIN_PASSWORD=${DOLI_ADMIN_PASSWORD:-admin123}
      - DOLI_URL_ROOT=http://localhost:${PORT:-80}
    depends_on:
      - dolibarr_db
    volumes:
      - dolibarr_html:/var/www/html
      - dolibarr_docs:/var/www/documents
    labels:
      - "rivetr.managed=true"

  dolibarr_db:
    image: mariadb:11
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=dolibarr
      - MYSQL_USER=dolibarr
      - MYSQL_PASSWORD=${DOLI_DB_PASSWORD:-changeme}
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-rootchangeme}
    volumes:
      - dolibarr_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  dolibarr_html:
  dolibarr_docs:
  dolibarr_db_data:
"#,
            r#"[{"name":"DOLI_DB_PASSWORD","label":"Database Password","required":true,"default":"changeme","secret":true},{"name":"DOLI_ADMIN_LOGIN","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"DOLI_ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"admin123","secret":true},{"name":"DB_ROOT_PASSWORD","label":"DB Root Password","required":false,"default":"rootchangeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false}]"#,
        ),

        // ==================== AI & ML ====================
        (
            "tpl-chroma",
            "Chroma",
            "Open-source vector database for AI embeddings. Simple API for storing and querying embeddings for LLM applications.",
            "ai",
            "chroma",
            r#"services:
  chroma:
    image: chromadb/chroma:latest
    container_name: ${CONTAINER_NAME:-chroma}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - CHROMA_SERVER_AUTHN_CREDENTIALS=${CHROMA_API_TOKEN:-}
      - CHROMA_SERVER_AUTHN_PROVIDER=${CHROMA_AUTH_PROVIDER:-}
      - PERSIST_DIRECTORY=/chroma/chroma
    volumes:
      - chroma_data:/chroma/chroma
    labels:
      - "rivetr.managed=true"

volumes:
  chroma_data:
"#,
            r#"[{"name":"CHROMA_API_TOKEN","label":"API Token (optional)","required":false,"default":"","secret":true},{"name":"CHROMA_AUTH_PROVIDER","label":"Auth Provider","required":false,"default":"","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8000","secret":false}]"#,
        ),

        // ==================== SECURITY ====================
        (
            "tpl-wazuh",
            "Wazuh",
            "Open-source SIEM and XDR platform. Unified security monitoring, threat detection, and incident response.",
            "security",
            "wazuh",
            r#"services:
  wazuh-manager:
    image: wazuh/wazuh-manager:latest
    container_name: ${CONTAINER_NAME:-wazuh-manager}
    restart: unless-stopped
    ports:
      - "${AGENT_PORT:-1514}:1514"
      - "${REGISTRATION_PORT:-1515}:1515"
      - "${API_PORT:-55000}:55000"
    environment:
      - WAZUH_API_USER=${WAZUH_API_USER:-wazuh-wui}
      - WAZUH_API_PASSWORD=${WAZUH_API_PASSWORD:-MyS3cr37P450r.*-}
    volumes:
      - wazuh_api_config:/var/ossec/api/configuration
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
  wazuh_api_config:
  wazuh_etc:
  wazuh_logs:
  wazuh_queue:
  wazuh_var_multigroups:
  wazuh_integrations:
  wazuh_active_response:
  wazuh_agentless:
  wazuh_wodles:
"#,
            r#"[{"name":"WAZUH_API_USER","label":"API Username","required":false,"default":"wazuh-wui","secret":false},{"name":"WAZUH_API_PASSWORD","label":"API Password","required":true,"default":"MyS3cr37P450r.*-","secret":true},{"name":"AGENT_PORT","label":"Agent Port","required":false,"default":"1514","secret":false},{"name":"REGISTRATION_PORT","label":"Registration Port","required":false,"default":"1515","secret":false},{"name":"API_PORT","label":"API Port","required":false,"default":"55000","secret":false}]"#,
        ),
        (
            "tpl-passbolt",
            "Passbolt",
            "Open-source password manager built for teams. Secure credential sharing with GPG encryption and audit logs.",
            "security",
            "passbolt",
            r#"services:
  passbolt:
    image: passbolt/passbolt:latest-ce
    container_name: ${CONTAINER_NAME:-passbolt}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
    environment:
      - APP_FULL_BASE_URL=${APP_FULL_BASE_URL:-http://localhost}
      - DATASOURCES_DEFAULT_HOST=passbolt_db
      - DATASOURCES_DEFAULT_USERNAME=passbolt
      - DATASOURCES_DEFAULT_PASSWORD=${DATASOURCES_DEFAULT_PASSWORD:-changeme}
      - DATASOURCES_DEFAULT_DATABASE=passbolt
      - EMAIL_DEFAULT_FROM=${EMAIL_FROM:-no-reply@example.com}
      - EMAIL_TRANSPORT_DEFAULT_HOST=${SMTP_HOST:-smtp.example.com}
      - EMAIL_TRANSPORT_DEFAULT_PORT=${SMTP_PORT:-587}
      - EMAIL_TRANSPORT_DEFAULT_USERNAME=${SMTP_USER:-}
      - EMAIL_TRANSPORT_DEFAULT_PASSWORD=${SMTP_PASSWORD:-}
    depends_on:
      - passbolt_db
    volumes:
      - passbolt_gpg:/etc/passbolt/gpg
      - passbolt_jwt:/etc/passbolt/jwt
    labels:
      - "rivetr.managed=true"

  passbolt_db:
    image: mariadb:11
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=passbolt
      - MYSQL_USER=passbolt
      - MYSQL_PASSWORD=${DATASOURCES_DEFAULT_PASSWORD:-changeme}
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-rootchangeme}
    volumes:
      - passbolt_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  passbolt_gpg:
  passbolt_jwt:
  passbolt_db_data:
"#,
            r#"[{"name":"APP_FULL_BASE_URL","label":"App Base URL","required":true,"default":"http://localhost","secret":false},{"name":"DATASOURCES_DEFAULT_PASSWORD","label":"Database Password","required":true,"default":"changeme","secret":true},{"name":"DB_ROOT_PASSWORD","label":"DB Root Password","required":false,"default":"rootchangeme","secret":true},{"name":"EMAIL_FROM","label":"From Email","required":false,"default":"no-reply@example.com","secret":false},{"name":"SMTP_HOST","label":"SMTP Host","required":false,"default":"smtp.example.com","secret":false},{"name":"SMTP_PORT","label":"SMTP Port","required":false,"default":"587","secret":false},{"name":"SMTP_USER","label":"SMTP Username","required":false,"default":"","secret":false},{"name":"SMTP_PASSWORD","label":"SMTP Password","required":false,"default":"","secret":true},{"name":"PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false}]"#,
        ),

        // ==================== PRODUCTIVITY ====================
        (
            "tpl-taiga",
            "Taiga",
            "Open-source project management tool for agile teams. Kanban, Scrum, and issue tracking in one platform.",
            "business",
            "taiga",
            r#"services:
  taiga-back:
    image: taigaio/taiga-back:latest
    container_name: ${CONTAINER_NAME:-taiga-back}
    restart: unless-stopped
    environment:
      - POSTGRES_DB=taiga
      - POSTGRES_USER=taiga
      - POSTGRES_PASSWORD=${DB_PASSWORD:-changeme}
      - POSTGRES_HOST=taiga_db
      - TAIGA_SECRET_KEY=${SECRET_KEY:-changeme-secret-key-32chars-min}
      - TAIGA_SITES_SCHEME=${SCHEME:-http}
      - TAIGA_SITES_DOMAIN=${DOMAIN:-localhost:9000}
      - TAIGA_SUBPATH=${SUBPATH:-}
      - RABBITMQ_USER=taiga
      - RABBITMQ_PASS=${RABBITMQ_PASS:-taiga}
      - RABBITMQ_HOST=taiga_rabbitmq
    depends_on:
      - taiga_db
      - taiga_rabbitmq
    volumes:
      - taiga_media:/taiga-back/media
      - taiga_static:/taiga-back/static
    labels:
      - "rivetr.managed=true"

  taiga-front:
    image: taigaio/taiga-front:latest
    container_name: taiga-front
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:80"
    environment:
      - TAIGA_URL=${SCHEME:-http}://${DOMAIN:-localhost:9000}
      - TAIGA_WEBSOCKETS_URL=ws://${DOMAIN:-localhost:9000}
    depends_on:
      - taiga-back
    labels:
      - "rivetr.managed=true"

  taiga-events:
    image: taigaio/taiga-events:latest
    container_name: taiga-events
    restart: unless-stopped
    environment:
      - RABBITMQ_USER=taiga
      - RABBITMQ_PASS=${RABBITMQ_PASS:-taiga}
      - TAIGA_SECRET_KEY=${SECRET_KEY:-changeme-secret-key-32chars-min}
    depends_on:
      - taiga_rabbitmq
    labels:
      - "rivetr.managed=true"

  taiga_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=taiga
      - POSTGRES_USER=taiga
      - POSTGRES_PASSWORD=${DB_PASSWORD:-changeme}
    volumes:
      - taiga_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  taiga_rabbitmq:
    image: rabbitmq:3-alpine
    restart: unless-stopped
    environment:
      - RABBITMQ_DEFAULT_USER=taiga
      - RABBITMQ_DEFAULT_PASS=${RABBITMQ_PASS:-taiga}
    volumes:
      - taiga_rabbitmq_data:/var/lib/rabbitmq
    labels:
      - "rivetr.managed=true"

volumes:
  taiga_media:
  taiga_static:
  taiga_db_data:
  taiga_rabbitmq_data:
"#,
            r#"[{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"changeme-secret-key-32chars-min","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"changeme","secret":true},{"name":"RABBITMQ_PASS","label":"RabbitMQ Password","required":false,"default":"taiga","secret":true},{"name":"DOMAIN","label":"Domain","required":false,"default":"localhost:9000","secret":false},{"name":"SCHEME","label":"Scheme (http/https)","required":false,"default":"http","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"9000","secret":false}]"#,
        ),
        // ==================== DEVOPS / CI (Woodpecker) ====================
        (
            "tpl-woodpecker-server",
            "Woodpecker CI Server",
            "Lightweight open-source CI/CD server. Integrates with Gitea, GitHub, and GitLab to run pipeline jobs in containers.",
            "devtools",
            "woodpecker",
            r#"services:
  woodpecker-server:
    image: woodpeckerci/woodpecker-server:latest
    container_name: ${CONTAINER_NAME:-woodpecker-server}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
      - "${GRPC_PORT:-9000}:9000"
    environment:
      - WOODPECKER_OPEN=true
      - WOODPECKER_HOST=${WOODPECKER_HOST:-http://localhost:8000}
      - WOODPECKER_AGENT_SECRET=${WOODPECKER_AGENT_SECRET:-changeme-agent-secret}
      - WOODPECKER_GITEA=${WOODPECKER_GITEA:-true}
      - WOODPECKER_GITEA_URL=${WOODPECKER_GITEA_URL:-http://localhost:3000}
      - WOODPECKER_GITEA_CLIENT=${WOODPECKER_GITEA_CLIENT:-}
      - WOODPECKER_GITEA_SECRET=${WOODPECKER_GITEA_SECRET:-}
      - WOODPECKER_DATABASE_DRIVER=sqlite3
      - WOODPECKER_DATABASE_DATASOURCE=/var/lib/woodpecker/woodpecker.sqlite
    volumes:
      - woodpecker_server_data:/var/lib/woodpecker
    labels:
      - "rivetr.managed=true"

volumes:
  woodpecker_server_data:
"#,
            r#"[{"name":"WOODPECKER_HOST","label":"Server URL","required":true,"default":"http://localhost:8000","secret":false},{"name":"WOODPECKER_AGENT_SECRET","label":"Agent Secret","required":true,"default":"changeme-agent-secret","secret":true},{"name":"WOODPECKER_GITEA_URL","label":"Gitea URL","required":false,"default":"http://localhost:3000","secret":false},{"name":"WOODPECKER_GITEA_CLIENT","label":"Gitea OAuth Client ID","required":false,"default":"","secret":false},{"name":"WOODPECKER_GITEA_SECRET","label":"Gitea OAuth Client Secret","required":false,"default":"","secret":true},{"name":"PORT","label":"HTTP Port","required":false,"default":"8000","secret":false},{"name":"GRPC_PORT","label":"gRPC Port","required":false,"default":"9000","secret":false}]"#,
        ),
        (
            "tpl-woodpecker-agent",
            "Woodpecker CI Agent",
            "CI agent that connects to a Woodpecker server and executes pipeline jobs inside Docker containers.",
            "devtools",
            "woodpecker",
            r#"services:
  woodpecker-agent:
    image: woodpeckerci/woodpecker-agent:latest
    container_name: ${CONTAINER_NAME:-woodpecker-agent}
    restart: unless-stopped
    environment:
      - WOODPECKER_SERVER=${WOODPECKER_SERVER:-woodpecker-server:9000}
      - WOODPECKER_AGENT_SECRET=${WOODPECKER_AGENT_SECRET:-changeme-agent-secret}
      - WOODPECKER_MAX_PROCS=${WOODPECKER_MAX_PROCS:-1}
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - woodpecker_agent_data:/etc/woodpecker
    labels:
      - "rivetr.managed=true"

volumes:
  woodpecker_agent_data:
"#,
            r#"[{"name":"WOODPECKER_SERVER","label":"Server Address (host:port)","required":true,"default":"woodpecker-server:9000","secret":false},{"name":"WOODPECKER_AGENT_SECRET","label":"Agent Secret","required":true,"default":"changeme-agent-secret","secret":true},{"name":"WOODPECKER_MAX_PROCS","label":"Max Parallel Jobs","required":false,"default":"1","secret":false}]"#,
        ),

        // ==================== COMMUNICATION ====================

        // ==================== AI & ML ====================
        (
            "tpl-anythingllm",
            "AnythingLLM",
            "All-in-one AI assistant. Chat with documents, use any LLM, build agents — all in a private self-hosted package.",
            "ai",
            "anythingllm",
            r#"services:
  anythingllm:
    image: mintplexlabs/anythingllm:latest
    container_name: ${CONTAINER_NAME:-anythingllm}
    restart: unless-stopped
    ports:
      - "${PORT:-3001}:3001"
    environment:
      - STORAGE_DIR=/app/server/storage
      - JWT_SECRET=${JWT_SECRET:-changeme-jwt-secret}
      - LLM_PROVIDER=${LLM_PROVIDER:-openai}
      - OPEN_AI_KEY=${OPEN_AI_KEY:-}
      - OPEN_MODEL_PREF=${OPEN_MODEL_PREF:-gpt-4o}
      - EMBEDDING_ENGINE=${EMBEDDING_ENGINE:-openai}
      - EMBEDDING_MODEL_PREF=${EMBEDDING_MODEL_PREF:-text-embedding-3-small}
      - VECTOR_DB=${VECTOR_DB:-lancedb}
      - AUTH_TOKEN=${AUTH_TOKEN:-}
    volumes:
      - anythingllm_storage:/app/server/storage
    labels:
      - "rivetr.managed=true"

volumes:
  anythingllm_storage:
"#,
            r#"[{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"changeme-jwt-secret","secret":true},{"name":"LLM_PROVIDER","label":"LLM Provider","required":false,"default":"openai","secret":false},{"name":"OPEN_AI_KEY","label":"OpenAI API Key","required":false,"default":"","secret":true},{"name":"OPEN_MODEL_PREF","label":"OpenAI Model","required":false,"default":"gpt-4o","secret":false},{"name":"AUTH_TOKEN","label":"Access Token (optional)","required":false,"default":"","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"3001","secret":false}]"#,
        ),

        // ==================== PRODUCTIVITY / BUSINESS ====================
        (
            "tpl-plane",
            "Plane",
            "Open-source project management tool. Track issues, sprints, and roadmaps — a powerful GitHub Issues alternative.",
            "business",
            "plane",
            r#"services:
  plane-web:
    image: makeplane/plane-frontend:latest
    container_name: ${CONTAINER_NAME:-plane-web}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - NEXT_PUBLIC_API_BASE_URL=${API_BASE_URL:-http://localhost:8000}
    depends_on:
      - plane-api
    labels:
      - "rivetr.managed=true"

  plane-api:
    image: makeplane/plane-backend:latest
    container_name: plane-api
    restart: unless-stopped
    ports:
      - "${API_PORT:-8000}:8000"
    command: ./bin/beat &  ./bin/worker & gunicorn -w 4 -b 0.0.0.0:8000 plane.asgi:application
    environment:
      - DJANGO_SETTINGS_MODULE=plane.settings.production
      - SECRET_KEY=${SECRET_KEY:-changeme-django-secret-50chars}
      - DATABASE_URL=postgres://plane:${DB_PASSWORD:-changeme}@plane_db:5432/plane
      - REDIS_URL=redis://plane_redis:6379
      - WEB_URL=${WEB_URL:-http://localhost:3000}
      - CORS_ALLOWED_ORIGINS=${WEB_URL:-http://localhost:3000}
      - EMAIL_HOST=${SMTP_HOST:-smtp.example.com}
      - EMAIL_FROM=${EMAIL_FROM:-no-reply@example.com}
    depends_on:
      - plane_db
      - plane_redis
    volumes:
      - plane_uploads:/app/uploads
    labels:
      - "rivetr.managed=true"

  plane_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=plane
      - POSTGRES_USER=plane
      - POSTGRES_PASSWORD=${DB_PASSWORD:-changeme}
    volumes:
      - plane_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  plane_redis:
    image: redis:7-alpine
    restart: unless-stopped
    volumes:
      - plane_redis_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  plane_uploads:
  plane_db_data:
  plane_redis_data:
"#,
            r#"[{"name":"SECRET_KEY","label":"Django Secret Key","required":true,"default":"changeme-django-secret-50chars","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"changeme","secret":true},{"name":"WEB_URL","label":"Web URL","required":false,"default":"http://localhost:3000","secret":false},{"name":"API_BASE_URL","label":"API Base URL","required":false,"default":"http://localhost:8000","secret":false},{"name":"SMTP_HOST","label":"SMTP Host","required":false,"default":"smtp.example.com","secret":false},{"name":"EMAIL_FROM","label":"From Email","required":false,"default":"no-reply@example.com","secret":false},{"name":"PORT","label":"Web Port","required":false,"default":"3000","secret":false},{"name":"API_PORT","label":"API Port","required":false,"default":"8000","secret":false}]"#,
        ),
        (
            "tpl-cal-com",
            "Cal.com",
            "Open-source Calendly alternative. Self-host your scheduling infrastructure with full control over your data.",
            "business",
            "calcom",
            r#"services:
  calcom:
    image: calcom/cal.com:latest
    container_name: ${CONTAINER_NAME:-calcom}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgres://calcom:${DB_PASSWORD:-changeme}@calcom_db:5432/calcom
      - NEXTAUTH_SECRET=${NEXTAUTH_SECRET:-changeme-nextauth-secret}
      - NEXTAUTH_URL=${NEXTAUTH_URL:-http://localhost:3000}
      - CALENDSO_ENCRYPTION_KEY=${ENCRYPTION_KEY:-changeme-encryption-key-32chars}
      - EMAIL_FROM=${EMAIL_FROM:-no-reply@example.com}
      - EMAIL_SERVER_HOST=${SMTP_HOST:-smtp.example.com}
      - EMAIL_SERVER_PORT=${SMTP_PORT:-587}
      - EMAIL_SERVER_USER=${SMTP_USER:-}
      - EMAIL_SERVER_PASSWORD=${SMTP_PASSWORD:-}
    depends_on:
      - calcom_db
    labels:
      - "rivetr.managed=true"

  calcom_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=calcom
      - POSTGRES_USER=calcom
      - POSTGRES_PASSWORD=${DB_PASSWORD:-changeme}
    volumes:
      - calcom_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  calcom_db_data:
"#,
            r#"[{"name":"NEXTAUTH_SECRET","label":"NextAuth Secret","required":true,"default":"changeme-nextauth-secret","secret":true},{"name":"NEXTAUTH_URL","label":"App URL","required":true,"default":"http://localhost:3000","secret":false},{"name":"ENCRYPTION_KEY","label":"Encryption Key","required":true,"default":"changeme-encryption-key-32chars","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"changeme","secret":true},{"name":"EMAIL_FROM","label":"From Email","required":false,"default":"no-reply@example.com","secret":false},{"name":"SMTP_HOST","label":"SMTP Host","required":false,"default":"smtp.example.com","secret":false},{"name":"SMTP_PORT","label":"SMTP Port","required":false,"default":"587","secret":false},{"name":"SMTP_USER","label":"SMTP Username","required":false,"default":"","secret":false},{"name":"SMTP_PASSWORD","label":"SMTP Password","required":false,"default":"","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),

        // ==================== DATABASES (continued) ====================
        (
            "tpl-questdb",
            "QuestDB",
            "High-performance time-series database with SQL. Sub-millisecond query latency for financial and IoT data.",
            "database",
            "questdb",
            r#"services:
  questdb:
    image: questdb/questdb:latest
    container_name: ${CONTAINER_NAME:-questdb}
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
      - "${INFLUX_PORT:-9009}:9009"
      - "${PG_PORT:-8812}:8812"
    environment:
      - QDB_CAIRO_MAX_UNCOMMITTED_ROWS=${MAX_UNCOMMITTED_ROWS:-500000}
      - QDB_HTTP_ENABLED=${HTTP_ENABLED:-true}
    volumes:
      - questdb_data:/root/.questdb
    labels:
      - "rivetr.managed=true"

volumes:
  questdb_data:
"#,
            r#"[{"name":"PORT","label":"HTTP/Console Port","required":false,"default":"9000","secret":false},{"name":"INFLUX_PORT","label":"InfluxDB Line Protocol Port","required":false,"default":"9009","secret":false},{"name":"PG_PORT","label":"PostgreSQL Wire Port","required":false,"default":"8812","secret":false}]"#,
        ),

        // ==================== SECURITY ====================
        (
            "tpl-crowdsec-dashboard",
            "CrowdSec Dashboard",
            "Metabase-powered dashboard for visualizing CrowdSec security events. Monitor threats, bans, and alerts.",
            "security",
            "crowdsec",
            r#"services:
  crowdsec-dashboard:
    image: metabase/metabase:latest
    container_name: ${CONTAINER_NAME:-crowdsec-dashboard}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - MB_DB_TYPE=postgres
      - MB_DB_DBNAME=${MB_DB_DBNAME:-metabase}
      - MB_DB_PORT=5432
      - MB_DB_USER=metabase
      - MB_DB_PASS=${MB_DB_PASS:-changeme}
      - MB_DB_HOST=crowdsec_meta_db
    depends_on:
      - crowdsec_meta_db
    labels:
      - "rivetr.managed=true"

  crowdsec_meta_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=${MB_DB_DBNAME:-metabase}
      - POSTGRES_USER=metabase
      - POSTGRES_PASSWORD=${MB_DB_PASS:-changeme}
    volumes:
      - crowdsec_meta_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  crowdsec_meta_db_data:
"#,
            r#"[{"name":"MB_DB_PASS","label":"Metabase DB Password","required":true,"default":"changeme","secret":true},{"name":"MB_DB_DBNAME","label":"Metabase DB Name","required":false,"default":"metabase","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),

        // ==================== TWENTY CRM (keep existing) ====================
        (
            "tpl-twenty-crm",
            "Twenty CRM",
            "Open-source Salesforce alternative. Modern CRM with a beautiful UI, built-in automations, and extensible data model.",
            "business",
            "twenty",
            r#"services:
  twenty:
    image: twentycrm/twenty:latest
    container_name: ${CONTAINER_NAME:-twenty}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - APP_SECRET=${APP_SECRET:-changeme-app-secret}
      - DATABASE_PG_URL=postgres://twenty:${DB_PASSWORD:-changeme}@twenty_db:5432/twenty
      - REDIS_URL=redis://twenty_redis:6379
      - STORAGE_TYPE=local
      - MESSAGE_QUEUE_TYPE=pg-boss
      - ENABLE_DB_MIGRATIONS=true
    depends_on:
      - twenty_db
      - twenty_redis
    volumes:
      - twenty_storage:/app/.local-storage
    labels:
      - "rivetr.managed=true"

  twenty_db:
    image: twentycrm/twenty-postgres:latest
    restart: unless-stopped
    environment:
      - POSTGRES_USER=twenty
      - POSTGRES_PASSWORD=${DB_PASSWORD:-changeme}
      - POSTGRES_DB=twenty
    volumes:
      - twenty_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  twenty_redis:
    image: redis:7-alpine
    restart: unless-stopped
    volumes:
      - twenty_redis_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  twenty_storage:
  twenty_db_data:
  twenty_redis_data:
"#,
            r#"[{"name":"APP_SECRET","label":"App Secret","required":true,"default":"changeme-app-secret","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
    ]
}
