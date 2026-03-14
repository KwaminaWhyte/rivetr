//! Sprint 18 service templates: new databases, AI/ML, DevOps, monitoring, communication, storage, security, business

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== DATABASES / DATA ====================
        (
            "tpl-ferretdb",
            "FerretDB",
            "MongoDB-compatible database built on top of PostgreSQL. Drop-in replacement for MongoDB using open standards.",
            "databases",
            "ferretdb",
            r#"services:
  ferretdb:
    image: ghcr.io/ferretdb/ferretdb:latest
    container_name: ${CONTAINER_NAME:-ferretdb}
    restart: unless-stopped
    ports:
      - "${PORT:-27017}:27017"
    environment:
      - FERRETDB_POSTGRESQL_URL=postgres://${POSTGRES_USER:-ferretdb}:${POSTGRES_PASSWORD:-ferretdb}@postgres:5432/${POSTGRES_DB:-ferretdb}
    depends_on:
      - postgres
    labels:
      - "rivetr.managed=true"

  postgres:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-ferretdb}-postgres
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${POSTGRES_USER:-ferretdb}
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-ferretdb}
      - POSTGRES_DB=${POSTGRES_DB:-ferretdb}
    volumes:
      - ferretdb_pg_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  ferretdb_pg_data:
"#,
            r#"[{"name":"POSTGRES_USER","label":"PostgreSQL User","required":false,"default":"ferretdb","secret":false},{"name":"POSTGRES_PASSWORD","label":"PostgreSQL Password","required":true,"default":"ferretdb","secret":true},{"name":"POSTGRES_DB","label":"PostgreSQL Database","required":false,"default":"ferretdb","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"27017","secret":false}]"#,
        ),
        (
            "tpl-rethinkdb",
            "RethinkDB",
            "Real-time database that pushes updated query results to applications. Ideal for collaborative apps, streaming, and real-time analytics.",
            "databases",
            "rethinkdb",
            r#"services:
  rethinkdb:
    image: rethinkdb:latest
    container_name: ${CONTAINER_NAME:-rethinkdb}
    restart: unless-stopped
    ports:
      - "${ADMIN_PORT:-8080}:8080"
      - "${DB_PORT:-28015}:28015"
      - "${CLUSTER_PORT:-29015}:29015"
    volumes:
      - rethinkdb_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  rethinkdb_data:
"#,
            r#"[{"name":"ADMIN_PORT","label":"Admin UI Port","required":false,"default":"8080","secret":false},{"name":"DB_PORT","label":"Driver Port","required":false,"default":"28015","secret":false}]"#,
        ),
        (
            "tpl-couchbase",
            "Couchbase Server",
            "Multi-model NoSQL database delivering key-value, document, and SQL-compatible query in a single platform.",
            "databases",
            "couchbase",
            r#"services:
  couchbase:
    image: couchbase:community
    container_name: ${CONTAINER_NAME:-couchbase}
    restart: unless-stopped
    ports:
      - "${PORT:-8091}:8091"
      - "8092:8092"
      - "8093:8093"
      - "8094:8094"
      - "11210:11210"
    volumes:
      - couchbase_data:/opt/couchbase/var
    labels:
      - "rivetr.managed=true"

volumes:
  couchbase_data:
"#,
            r#"[{"name":"PORT","label":"Web Admin Port","required":false,"default":"8091","secret":false}]"#,
        ),
        (
            "tpl-apache-age",
            "Apache AGE",
            "PostgreSQL extension that enables graph database capabilities. Query relational and graph data with SQL and openCypher.",
            "databases",
            "apache-age",
            r#"services:
  apache-age:
    image: apache/age:latest
    container_name: ${CONTAINER_NAME:-apache-age}
    restart: unless-stopped
    ports:
      - "${PORT:-5432}:5432"
    environment:
      - POSTGRES_USER=${POSTGRES_USER:-age}
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-agepassword}
      - POSTGRES_DB=${POSTGRES_DB:-agedb}
    volumes:
      - apache_age_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  apache_age_data:
"#,
            r#"[{"name":"POSTGRES_USER","label":"PostgreSQL User","required":false,"default":"age","secret":false},{"name":"POSTGRES_PASSWORD","label":"PostgreSQL Password","required":true,"default":"agepassword","secret":true},{"name":"POSTGRES_DB","label":"Database Name","required":false,"default":"agedb","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"5432","secret":false}]"#,
        ),
        (
            "tpl-eventstoredb",
            "EventStoreDB",
            "Purpose-built event store for event sourcing. The world's leading event store and stream processing database.",
            "databases",
            "eventstoredb",
            r#"services:
  eventstore:
    image: eventstore/eventstore:latest
    container_name: ${CONTAINER_NAME:-eventstore}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-2113}:2113"
      - "${TCP_PORT:-1113}:1113"
    environment:
      - EVENTSTORE_CLUSTER_SIZE=1
      - EVENTSTORE_RUN_PROJECTIONS=${RUN_PROJECTIONS:-All}
      - EVENTSTORE_START_STANDARD_PROJECTIONS=true
      - EVENTSTORE_INSECURE=${INSECURE:-true}
      - EVENTSTORE_ENABLE_ATOM_PUB_OVER_HTTP=true
    volumes:
      - eventstore_data:/var/lib/eventstore
      - eventstore_logs:/var/log/eventstore
    labels:
      - "rivetr.managed=true"

volumes:
  eventstore_data:
  eventstore_logs:
"#,
            r#"[{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"2113","secret":false},{"name":"INSECURE","label":"Insecure Mode (no TLS)","required":false,"default":"true","secret":false},{"name":"RUN_PROJECTIONS","label":"Run Projections","required":false,"default":"All","secret":false}]"#,
        ),
        (
            "tpl-arangodb",
            "ArangoDB",
            "Multi-model database supporting graphs, documents, and key-value. One database, three data models, one query language.",
            "databases",
            "arangodb",
            r#"services:
  arangodb:
    image: arangodb:latest
    container_name: ${CONTAINER_NAME:-arangodb}
    restart: unless-stopped
    ports:
      - "${PORT:-8529}:8529"
    environment:
      - ARANGO_ROOT_PASSWORD=${ARANGO_ROOT_PASSWORD:-changeme}
    volumes:
      - arangodb_data:/var/lib/arangodb3
      - arangodb_apps:/var/lib/arangodb3-apps
    labels:
      - "rivetr.managed=true"

volumes:
  arangodb_data:
  arangodb_apps:
"#,
            r#"[{"name":"ARANGO_ROOT_PASSWORD","label":"Root Password","required":true,"default":"changeme","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8529","secret":false}]"#,
        ),
        (
            "tpl-rqlite",
            "rqlite",
            "Distributed relational database built on SQLite. Lightweight, highly-available, and easy to operate cluster of SQLite nodes.",
            "databases",
            "rqlite",
            r#"services:
  rqlite:
    image: rqlite/rqlite:latest
    container_name: ${CONTAINER_NAME:-rqlite}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-4001}:4001"
      - "${RAFT_PORT:-4002}:4002"
    volumes:
      - rqlite_data:/rqlite/file
    labels:
      - "rivetr.managed=true"

volumes:
  rqlite_data:
"#,
            r#"[{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"4001","secret":false},{"name":"RAFT_PORT","label":"Raft Port","required":false,"default":"4002","secret":false}]"#,
        ),
        (
            "tpl-tigerbeetle",
            "TigerBeetle",
            "Financial accounting database designed for safety, high throughput, and low latency. Built specifically for financial ledgers.",
            "databases",
            "tigerbeetle",
            r#"services:
  tigerbeetle:
    image: ghcr.io/tigerbeetle/tigerbeetle:latest
    container_name: ${CONTAINER_NAME:-tigerbeetle}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    volumes:
      - tigerbeetle_data:/data
    command: >
      sh -c "
        if [ ! -f /data/0_0.tigerbeetle ]; then
          ./tigerbeetle format --cluster=0 --replica=0 --replica-count=1 /data/0_0.tigerbeetle;
        fi &&
        ./tigerbeetle start --addresses=0.0.0.0:3000 /data/0_0.tigerbeetle
      "
    labels:
      - "rivetr.managed=true"

volumes:
  tigerbeetle_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false}]"#,
        ),
        // ==================== AI / ML / VECTOR ====================
        (
            "tpl-milvus",
            "Milvus",
            "Open-source vector database built for scalable similarity search and AI applications. Supports billions of vectors.",
            "ai-ml",
            "milvus",
            r#"services:
  milvus:
    image: milvusdb/milvus:latest
    container_name: ${CONTAINER_NAME:-milvus}
    restart: unless-stopped
    ports:
      - "${PORT:-19530}:19530"
      - "${METRICS_PORT:-9091}:9091"
    environment:
      - ETCD_ENDPOINTS=etcd:2379
      - MINIO_ADDRESS=minio:9000
    depends_on:
      - etcd
      - minio
    volumes:
      - milvus_data:/var/lib/milvus
    command: milvus run standalone
    labels:
      - "rivetr.managed=true"

  etcd:
    image: quay.io/coreos/etcd:v3.5.5
    container_name: ${CONTAINER_NAME:-milvus}-etcd
    restart: unless-stopped
    environment:
      - ETCD_AUTO_COMPACTION_MODE=revision
      - ETCD_AUTO_COMPACTION_RETENTION=1000
      - ETCD_QUOTA_BACKEND_BYTES=4294967296
      - ETCD_SNAPSHOT_COUNT=50000
    command: etcd -advertise-client-urls=http://127.0.0.1:2379 -listen-client-urls http://0.0.0.0:2379 --data-dir /etcd
    volumes:
      - milvus_etcd:/etcd
    labels:
      - "rivetr.managed=true"

  minio:
    image: minio/minio:latest
    container_name: ${CONTAINER_NAME:-milvus}-minio
    restart: unless-stopped
    environment:
      - MINIO_ACCESS_KEY=${MINIO_ACCESS_KEY:-minioadmin}
      - MINIO_SECRET_KEY=${MINIO_SECRET_KEY:-minioadmin}
    command: minio server /minio_data --console-address ":9001"
    volumes:
      - milvus_minio:/minio_data
    labels:
      - "rivetr.managed=true"

volumes:
  milvus_data:
  milvus_etcd:
  milvus_minio:
"#,
            r#"[{"name":"PORT","label":"Milvus Port","required":false,"default":"19530","secret":false},{"name":"MINIO_ACCESS_KEY","label":"MinIO Access Key","required":false,"default":"minioadmin","secret":false},{"name":"MINIO_SECRET_KEY","label":"MinIO Secret Key","required":true,"default":"minioadmin","secret":true}]"#,
        ),
        (
            "tpl-llama-cpp-server",
            "llama.cpp Server",
            "High-performance inference server for GGUF model files. Run local LLMs with an OpenAI-compatible API endpoint.",
            "ai-ml",
            "llama-cpp",
            r#"services:
  llama-cpp:
    image: ghcr.io/ggerganov/llama.cpp:server
    container_name: ${CONTAINER_NAME:-llama-cpp}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - LLAMA_ARG_HOST=0.0.0.0
      - LLAMA_ARG_PORT=8080
      - LLAMA_ARG_CTX_SIZE=${CTX_SIZE:-2048}
      - LLAMA_ARG_N_PARALLEL=${N_PARALLEL:-1}
    volumes:
      - llama_models:/models
    labels:
      - "rivetr.managed=true"

volumes:
  llama_models:
"#,
            r#"[{"name":"PORT","label":"Server Port","required":false,"default":"8080","secret":false},{"name":"CTX_SIZE","label":"Context Size","required":false,"default":"2048","secret":false},{"name":"N_PARALLEL","label":"Parallel Requests","required":false,"default":"1","secret":false}]"#,
        ),
        (
            "tpl-n8n",
            "n8n",
            "Fair-code workflow automation platform. Connect anything to everything with 400+ integrations and a visual editor.",
            "automation",
            "n8n",
            r#"services:
  n8n:
    image: n8nio/n8n:latest
    container_name: ${CONTAINER_NAME:-n8n}
    restart: unless-stopped
    ports:
      - "${PORT:-5678}:5678"
    environment:
      - N8N_BASIC_AUTH_ACTIVE=${BASIC_AUTH:-true}
      - N8N_BASIC_AUTH_USER=${N8N_USER:-admin}
      - N8N_BASIC_AUTH_PASSWORD=${N8N_PASSWORD:-changeme}
      - N8N_HOST=${N8N_HOST:-localhost}
      - N8N_PORT=5678
      - N8N_PROTOCOL=${N8N_PROTOCOL:-http}
      - WEBHOOK_URL=${WEBHOOK_URL:-http://localhost:5678/}
      - GENERIC_TIMEZONE=${TIMEZONE:-UTC}
    volumes:
      - n8n_data:/home/node/.n8n
    labels:
      - "rivetr.managed=true"

volumes:
  n8n_data:
"#,
            r#"[{"name":"N8N_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"N8N_PASSWORD","label":"Admin Password","required":true,"default":"changeme","secret":true},{"name":"N8N_HOST","label":"Host / Domain","required":false,"default":"localhost","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"5678","secret":false}]"#,
        ),
        // ==================== DEVOPS / CI / INFRASTRUCTURE ====================
        (
            "tpl-sonarqube",
            "SonarQube",
            "Continuous code quality and security inspection. Detect bugs, vulnerabilities, and code smells across 30+ languages.",
            "devtools",
            "sonarqube",
            r#"services:
  sonarqube:
    image: sonarqube:community
    container_name: ${CONTAINER_NAME:-sonarqube}
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
    environment:
      - SONAR_JDBC_URL=jdbc:postgresql://postgres:5432/${POSTGRES_DB:-sonarqube}
      - SONAR_JDBC_USERNAME=${POSTGRES_USER:-sonarqube}
      - SONAR_JDBC_PASSWORD=${POSTGRES_PASSWORD:-sonarqube}
    depends_on:
      - postgres
    volumes:
      - sonarqube_data:/opt/sonarqube/data
      - sonarqube_logs:/opt/sonarqube/logs
      - sonarqube_extensions:/opt/sonarqube/extensions
    ulimits:
      nofile:
        soft: 65536
        hard: 65536
    labels:
      - "rivetr.managed=true"

  postgres:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-sonarqube}-postgres
    restart: unless-stopped
    environment:
      - POSTGRES_DB=${POSTGRES_DB:-sonarqube}
      - POSTGRES_USER=${POSTGRES_USER:-sonarqube}
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-sonarqube}
    volumes:
      - sonarqube_pg_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  sonarqube_data:
  sonarqube_logs:
  sonarqube_extensions:
  sonarqube_pg_data:
"#,
            r#"[{"name":"POSTGRES_PASSWORD","label":"PostgreSQL Password","required":true,"default":"sonarqube","secret":true},{"name":"POSTGRES_USER","label":"PostgreSQL User","required":false,"default":"sonarqube","secret":false},{"name":"POSTGRES_DB","label":"Database Name","required":false,"default":"sonarqube","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"9000","secret":false}]"#,
        ),
        (
            "tpl-nexus-oss",
            "Nexus Repository OSS",
            "Universal repository manager supporting Maven, npm, Docker, PyPI, and more. Store and distribute all your artifacts.",
            "devtools",
            "nexus",
            r#"services:
  nexus:
    image: sonatype/nexus3:latest
    container_name: ${CONTAINER_NAME:-nexus}
    restart: unless-stopped
    ports:
      - "${PORT:-8081}:8081"
    volumes:
      - nexus_data:/nexus-data
    labels:
      - "rivetr.managed=true"

volumes:
  nexus_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8081","secret":false}]"#,
        ),
        (
            "tpl-artifactory-oss",
            "JFrog Artifactory OSS",
            "Open-source universal binary repository manager. Support for Maven, Gradle, npm, Docker, and many more package formats.",
            "devtools",
            "artifactory",
            r#"services:
  artifactory:
    image: releases-docker.jfrog.io/jfrog/artifactory-oss:latest
    container_name: ${CONTAINER_NAME:-artifactory}
    restart: unless-stopped
    ports:
      - "${PORT:-8082}:8082"
      - "8081:8081"
    environment:
      - JF_ROUTER_ENTRYPOINTS_EXTERNALPORT=${PORT:-8082}
    volumes:
      - artifactory_data:/var/opt/jfrog/artifactory
    labels:
      - "rivetr.managed=true"

volumes:
  artifactory_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8082","secret":false}]"#,
        ),
        (
            "tpl-vault",
            "HashiCorp Vault",
            "Secrets management, identity-based access, and encryption as a service. Securely store and control access to tokens, passwords, and certificates.",
            "security",
            "vault",
            r#"services:
  vault:
    image: hashicorp/vault:latest
    container_name: ${CONTAINER_NAME:-vault}
    restart: unless-stopped
    ports:
      - "${PORT:-8200}:8200"
    environment:
      - VAULT_DEV_ROOT_TOKEN_ID=${ROOT_TOKEN:-devroot}
      - VAULT_DEV_LISTEN_ADDRESS=0.0.0.0:8200
      - VAULT_ADDR=http://0.0.0.0:8200
    cap_add:
      - IPC_LOCK
    command: vault server -dev
    volumes:
      - vault_data:/vault/data
      - vault_logs:/vault/logs
    labels:
      - "rivetr.managed=true"

volumes:
  vault_data:
  vault_logs:
"#,
            r#"[{"name":"ROOT_TOKEN","label":"Dev Root Token","required":true,"default":"devroot","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8200","secret":false}]"#,
        ),
        (
            "tpl-consul",
            "HashiCorp Consul",
            "Service mesh and service discovery solution. Provides service registration, health checking, KV store, and multi-datacenter support.",
            "infrastructure",
            "consul",
            r#"services:
  consul:
    image: hashicorp/consul:latest
    container_name: ${CONTAINER_NAME:-consul}
    restart: unless-stopped
    ports:
      - "${PORT:-8500}:8500"
      - "8600:8600/udp"
    command: agent -server -bootstrap-expect=1 -ui -client=0.0.0.0
    environment:
      - CONSUL_BIND_INTERFACE=eth0
    volumes:
      - consul_data:/consul/data
    labels:
      - "rivetr.managed=true"

volumes:
  consul_data:
"#,
            r#"[{"name":"PORT","label":"HTTP Port","required":false,"default":"8500","secret":false}]"#,
        ),
        (
            "tpl-nomad",
            "HashiCorp Nomad",
            "Simple and flexible workload orchestrator. Deploy containers, binaries, and batch jobs on-prem and in the cloud.",
            "infrastructure",
            "nomad",
            r#"services:
  nomad:
    image: hashicorp/nomad:latest
    container_name: ${CONTAINER_NAME:-nomad}
    restart: unless-stopped
    ports:
      - "${PORT:-4646}:4646"
      - "4647:4647"
      - "4648:4648"
    command: agent -dev -bind=0.0.0.0 -log-level=INFO
    volumes:
      - nomad_data:/nomad/data
      - /var/run/docker.sock:/var/run/docker.sock
    labels:
      - "rivetr.managed=true"

volumes:
  nomad_data:
"#,
            r#"[{"name":"PORT","label":"HTTP Port","required":false,"default":"4646","secret":false}]"#,
        ),
        (
            "tpl-weave-gitops",
            "Weave GitOps",
            "Open-source developer platform built on Flux. Visualize and manage GitOps workflows from a clean web UI.",
            "devtools",
            "weave-gitops",
            r#"services:
  weave-gitops:
    image: ghcr.io/weaveworks/wego-app:latest
    container_name: ${CONTAINER_NAME:-weave-gitops}
    restart: unless-stopped
    ports:
      - "${PORT:-9001}:9001"
    environment:
      - WEAVE_GITOPS_FEATURE_OIDC_BUTTON_LABEL=${OIDC_LABEL:-Login with OIDC}
      - ADMIN_USERNAME=${ADMIN_USERNAME:-admin}
      - ADMIN_PASSWORD_HASH=${ADMIN_PASSWORD_HASH:-}
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"ADMIN_USERNAME","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD_HASH","label":"Admin Password Hash (bcrypt)","required":true,"default":"","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"9001","secret":false}]"#,
        ),
        // ==================== MONITORING / OBSERVABILITY ====================
        (
            "tpl-tempo",
            "Grafana Tempo",
            "Open-source, high-scale distributed tracing backend. Ingests and queries traces from any instrumented application.",
            "monitoring",
            "tempo",
            r#"services:
  tempo:
    image: grafana/tempo:latest
    container_name: ${CONTAINER_NAME:-tempo}
    restart: unless-stopped
    ports:
      - "${PORT:-3200}:3200"
      - "4317:4317"
      - "4318:4318"
      - "9411:9411"
      - "14268:14268"
    command: -config.file=/etc/tempo.yaml
    volumes:
      - tempo_data:/var/tempo
    labels:
      - "rivetr.managed=true"

volumes:
  tempo_data:
"#,
            r#"[{"name":"PORT","label":"HTTP Port","required":false,"default":"3200","secret":false}]"#,
        ),
        (
            "tpl-pyroscope",
            "Pyroscope",
            "Open-source continuous profiling platform. Find performance bottlenecks in your applications with flame graphs and comparison views.",
            "monitoring",
            "pyroscope",
            r#"services:
  pyroscope:
    image: grafana/pyroscope:latest
    container_name: ${CONTAINER_NAME:-pyroscope}
    restart: unless-stopped
    ports:
      - "${PORT:-4040}:4040"
    volumes:
      - pyroscope_data:/var/lib/pyroscope
    labels:
      - "rivetr.managed=true"

volumes:
  pyroscope_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"4040","secret":false}]"#,
        ),
        // ==================== COMMUNICATION / COLLABORATION ====================
        (
            "tpl-listmonk-standalone",
            "Listmonk Standalone",
            "High-performance, self-hosted newsletter and mailing list manager. Manage subscribers, send campaigns, and view analytics.",
            "communication",
            "listmonk",
            r#"services:
  listmonk:
    image: listmonk/listmonk:latest
    container_name: ${CONTAINER_NAME:-listmonk}
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
    environment:
      - LISTMONK_app__address=0.0.0.0:9000
      - LISTMONK_db__host=postgres
      - LISTMONK_db__port=5432
      - LISTMONK_db__user=${POSTGRES_USER:-listmonk}
      - LISTMONK_db__password=${POSTGRES_PASSWORD:-listmonk}
      - LISTMONK_db__database=${POSTGRES_DB:-listmonk}
    depends_on:
      - postgres
    command: >
      sh -c "yes | ./listmonk --install && ./listmonk"
    volumes:
      - listmonk_uploads:/listmonk/uploads
    labels:
      - "rivetr.managed=true"

  postgres:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-listmonk}-postgres
    restart: unless-stopped
    environment:
      - POSTGRES_DB=${POSTGRES_DB:-listmonk}
      - POSTGRES_USER=${POSTGRES_USER:-listmonk}
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-listmonk}
    volumes:
      - listmonk_pg_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  listmonk_uploads:
  listmonk_pg_data:
"#,
            r#"[{"name":"POSTGRES_PASSWORD","label":"PostgreSQL Password","required":true,"default":"listmonk","secret":true},{"name":"POSTGRES_USER","label":"PostgreSQL User","required":false,"default":"listmonk","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"9000","secret":false}]"#,
        ),
        // ==================== STORAGE / FILES ====================
        (
            "tpl-filerun",
            "FileRun",
            "Self-hosted file manager and sharing platform. Google Drive alternative with media previews, versioning, and sharing.",
            "storage",
            "filerun",
            r#"services:
  filerun:
    image: filerun/filerun:latest
    container_name: ${CONTAINER_NAME:-filerun}
    restart: unless-stopped
    ports:
      - "${PORT:-8082}:80"
    environment:
      - FR_DB_HOST=mysql
      - FR_DB_PORT=3306
      - FR_DB_NAME=${MYSQL_DB:-filerun}
      - FR_DB_USER=${MYSQL_USER:-filerun}
      - FR_DB_PASS=${MYSQL_PASSWORD:-changeme}
    depends_on:
      - mysql
    volumes:
      - filerun_html:/var/www/html
      - filerun_user_files:/user-files
    labels:
      - "rivetr.managed=true"

  mysql:
    image: mysql:8
    container_name: ${CONTAINER_NAME:-filerun}-mysql
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${MYSQL_ROOT_PASSWORD:-rootpassword}
      - MYSQL_DATABASE=${MYSQL_DB:-filerun}
      - MYSQL_USER=${MYSQL_USER:-filerun}
      - MYSQL_PASSWORD=${MYSQL_PASSWORD:-changeme}
    volumes:
      - filerun_mysql_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  filerun_html:
  filerun_user_files:
  filerun_mysql_data:
"#,
            r#"[{"name":"MYSQL_PASSWORD","label":"MySQL Password","required":true,"default":"changeme","secret":true},{"name":"MYSQL_ROOT_PASSWORD","label":"MySQL Root Password","required":true,"default":"rootpassword","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8082","secret":false}]"#,
        ),
        (
            "tpl-storj-gateway",
            "Storj Gateway MT",
            "S3-compatible gateway for Storj decentralized cloud storage. Use standard S3 tools to access globally distributed storage.",
            "storage",
            "storj",
            r#"services:
  storj-gateway:
    image: storjlabs/gateway:latest
    container_name: ${CONTAINER_NAME:-storj-gateway}
    restart: unless-stopped
    ports:
      - "${PORT:-7777}:7777"
    environment:
      - STORJ_ACCESS=${STORJ_ACCESS_GRANT:-}
      - STORJ_AUTH_TOKEN=${AUTH_TOKEN:-insecure}
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"STORJ_ACCESS_GRANT","label":"Storj Access Grant","required":true,"default":"","secret":true},{"name":"AUTH_TOKEN","label":"Auth Token","required":false,"default":"insecure","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"7777","secret":false}]"#,
        ),
        // ==================== SECURITY / AUTH ====================
        // ==================== BUSINESS / PRODUCTIVITY ====================
        (
            "tpl-invoice-ninja-v5",
            "Invoice Ninja v5",
            "Feature-rich invoicing, billing, and payment platform. Send quotes, collect payments, and manage clients all in one place.",
            "business",
            "invoice-ninja",
            r#"services:
  invoice-ninja:
    image: invoiceninja/invoiceninja:latest
    container_name: ${CONTAINER_NAME:-invoice-ninja}
    restart: unless-stopped
    ports:
      - "${PORT:-8087}:80"
    environment:
      - APP_URL=${APP_URL:-http://localhost:8087}
      - APP_KEY=${APP_KEY:-base64:changethisappkey32characterlong}
      - APP_DEBUG=false
      - DB_HOST=mysql
      - DB_DATABASE=${MYSQL_DB:-ninja}
      - DB_USERNAME=${MYSQL_USER:-ninja}
      - DB_PASSWORD=${MYSQL_PASSWORD:-changeme}
      - MAIL_MAILER=${MAIL_MAILER:-log}
    depends_on:
      - mysql
    volumes:
      - ninja_storage:/var/app/public/storage
      - ninja_logo:/var/app/public/logo
    labels:
      - "rivetr.managed=true"

  mysql:
    image: mysql:8
    container_name: ${CONTAINER_NAME:-invoice-ninja}-mysql
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${MYSQL_ROOT_PASSWORD:-rootpassword}
      - MYSQL_DATABASE=${MYSQL_DB:-ninja}
      - MYSQL_USER=${MYSQL_USER:-ninja}
      - MYSQL_PASSWORD=${MYSQL_PASSWORD:-changeme}
    volumes:
      - ninja_mysql_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  ninja_storage:
  ninja_logo:
  ninja_mysql_data:
"#,
            r#"[{"name":"APP_KEY","label":"App Key (base64:32chars)","required":true,"default":"base64:changethisappkey32characterlong","secret":true},{"name":"APP_URL","label":"App URL","required":false,"default":"http://localhost:8087","secret":false},{"name":"MYSQL_PASSWORD","label":"MySQL Password","required":true,"default":"changeme","secret":true},{"name":"MYSQL_ROOT_PASSWORD","label":"MySQL Root Password","required":true,"default":"rootpassword","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8087","secret":false}]"#,
        ),
        (
            "tpl-kimai-timetracking",
            "Kimai Time Tracking",
            "Open-source time tracker for teams and freelancers. Track billable hours, generate reports, and manage projects.",
            "business",
            "kimai",
            r#"services:
  kimai:
    image: kimai/kimai2:apache
    container_name: ${CONTAINER_NAME:-kimai}
    restart: unless-stopped
    ports:
      - "${PORT:-8088}:8001"
    environment:
      - ADMINMAIL=${ADMIN_EMAIL:-admin@example.com}
      - ADMINPASS=${ADMIN_PASSWORD:-changeme}
      - DATABASE_URL=mysql://${MYSQL_USER:-kimai}:${MYSQL_PASSWORD:-changeme}@mysql/kimai2
      - TRUSTED_HOSTS=${TRUSTED_HOSTS:-localhost}
    depends_on:
      - mysql
    volumes:
      - kimai_var:/opt/kimai/var
    labels:
      - "rivetr.managed=true"

  mysql:
    image: mysql:8
    container_name: ${CONTAINER_NAME:-kimai}-mysql
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${MYSQL_ROOT_PASSWORD:-rootpassword}
      - MYSQL_DATABASE=kimai2
      - MYSQL_USER=${MYSQL_USER:-kimai}
      - MYSQL_PASSWORD=${MYSQL_PASSWORD:-changeme}
    volumes:
      - kimai_mysql_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  kimai_var:
  kimai_mysql_data:
"#,
            r#"[{"name":"ADMIN_EMAIL","label":"Admin Email","required":false,"default":"admin@example.com","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"changeme","secret":true},{"name":"MYSQL_PASSWORD","label":"MySQL Password","required":true,"default":"changeme","secret":true},{"name":"MYSQL_ROOT_PASSWORD","label":"MySQL Root Password","required":true,"default":"rootpassword","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"8088","secret":false}]"#,
        ),
        (
            "tpl-erpnext",
            "ERPNext",
            "Open-source enterprise resource planning system. Manufacturing, retail, distribution, healthcare, and education in one platform.",
            "business",
            "erpnext",
            r#"services:
  erpnext:
    image: frappe/erpnext:latest
    container_name: ${CONTAINER_NAME:-erpnext}
    restart: unless-stopped
    ports:
      - "${PORT:-8089}:8080"
    environment:
      - FRAPPE_SITE_NAME_HEADER=${SITE_NAME:-erpnext.localhost}
    volumes:
      - erpnext_sites:/home/frappe/frappe-bench/sites
      - erpnext_logs:/home/frappe/frappe-bench/logs
    labels:
      - "rivetr.managed=true"

volumes:
  erpnext_sites:
  erpnext_logs:
"#,
            r#"[{"name":"SITE_NAME","label":"Site Name","required":false,"default":"erpnext.localhost","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8089","secret":false}]"#,
        ),
        (
            "tpl-rallly",
            "Rallly",
            "Open-source scheduling and polling tool. Create polls to find the best meeting time without the back-and-forth emails.",
            "productivity",
            "rallly",
            r#"services:
  rallly:
    image: lukevella/rallly:latest
    container_name: ${CONTAINER_NAME:-rallly}
    restart: unless-stopped
    ports:
      - "${PORT:-3001}:3000"
    environment:
      - DATABASE_URL=postgres://${POSTGRES_USER:-rallly}:${POSTGRES_PASSWORD:-rallly}@postgres:5432/${POSTGRES_DB:-rallly}
      - SECRET_PASSWORD=${SECRET_PASSWORD:-changethissecretpassword}
      - NEXT_PUBLIC_BASE_URL=${BASE_URL:-http://localhost:3001}
      - SUPPORT_EMAIL=${SUPPORT_EMAIL:-support@example.com}
    depends_on:
      - postgres
    labels:
      - "rivetr.managed=true"

  postgres:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-rallly}-postgres
    restart: unless-stopped
    environment:
      - POSTGRES_DB=${POSTGRES_DB:-rallly}
      - POSTGRES_USER=${POSTGRES_USER:-rallly}
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-rallly}
    volumes:
      - rallly_pg_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  rallly_pg_data:
"#,
            r#"[{"name":"SECRET_PASSWORD","label":"Secret Password","required":true,"default":"changethissecretpassword","secret":true},{"name":"POSTGRES_PASSWORD","label":"PostgreSQL Password","required":true,"default":"rallly","secret":true},{"name":"BASE_URL","label":"Base URL","required":false,"default":"http://localhost:3001","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3001","secret":false}]"#,
        ),
        (
            "tpl-apache-answer",
            "Apache Answer",
            "Q&A community platform for developer and user communities. Stack Overflow-style question and answer site you can self-host.",
            "community",
            "answer",
            r#"services:
  answer:
    image: apache/answer:latest
    container_name: ${CONTAINER_NAME:-answer}
    restart: unless-stopped
    ports:
      - "${PORT:-9080}:80"
    environment:
      - INSTALL_PORT=80
    volumes:
      - answer_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  answer_data:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"9080","secret":false}]"#,
        ),
        // ==================== BONUS ====================
        (
            "tpl-write-freely",
            "Write.freely",
            "Minimalist, federated blogging platform. Publish to the Fediverse and the open web with clean, distraction-free writing.",
            "cms",
            "write-freely",
            r#"services:
  writefreely:
    image: writeas/writefreely:latest
    container_name: ${CONTAINER_NAME:-writefreely}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    volumes:
      - writefreely_data:/go/src/app/data
      - writefreely_keys:/go/src/app/keys
    labels:
      - "rivetr.managed=true"

volumes:
  writefreely_data:
  writefreely_keys:
"#,
            r#"[{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-dolibarr-erp",
            "Dolibarr ERP/CRM",
            "All-in-one open-source ERP and CRM for small and medium businesses. Invoicing, accounting, HR, and project management.",
            "business",
            "dolibarr",
            r#"services:
  dolibarr:
    image: dolibarr/dolibarr:latest
    container_name: ${CONTAINER_NAME:-dolibarr}
    restart: unless-stopped
    ports:
      - "${PORT:-8090}:80"
    environment:
      - DOLI_DB_HOST=mysql
      - DOLI_DB_NAME=${MYSQL_DB:-dolibarr}
      - DOLI_DB_USER=${MYSQL_USER:-dolibarr}
      - DOLI_DB_PASSWORD=${MYSQL_PASSWORD:-changeme}
      - DOLI_ADMIN_LOGIN=${ADMIN_USER:-admin}
      - DOLI_ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - DOLI_URL_ROOT=${URL_ROOT:-http://localhost:8090}
    depends_on:
      - mysql
    volumes:
      - dolibarr_documents:/var/www/documents
      - dolibarr_custom:/var/www/html/custom
    labels:
      - "rivetr.managed=true"

  mysql:
    image: mysql:8
    container_name: ${CONTAINER_NAME:-dolibarr}-mysql
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${MYSQL_ROOT_PASSWORD:-rootpassword}
      - MYSQL_DATABASE=${MYSQL_DB:-dolibarr}
      - MYSQL_USER=${MYSQL_USER:-dolibarr}
      - MYSQL_PASSWORD=${MYSQL_PASSWORD:-changeme}
    volumes:
      - dolibarr_mysql_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  dolibarr_documents:
  dolibarr_custom:
  dolibarr_mysql_data:
"#,
            r#"[{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"changeme","secret":true},{"name":"MYSQL_PASSWORD","label":"MySQL Password","required":true,"default":"changeme","secret":true},{"name":"MYSQL_ROOT_PASSWORD","label":"MySQL Root Password","required":true,"default":"rootpassword","secret":true},{"name":"URL_ROOT","label":"App URL","required":false,"default":"http://localhost:8090","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8090","secret":false}]"#,
        ),
    ]
}
