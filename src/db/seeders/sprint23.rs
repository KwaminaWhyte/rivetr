//! Sprint 23 service templates: AI/LLM tools, Auth/SSO, and Automation
//!
//! Already present in earlier seeder files (skipped):
//! - Langfuse (ai_extras.rs as tpl-langfuse)
//! - LocalAI (ai_extras.rs as tpl-localai)
//! - Authentik (security_search.rs as tpl-batch2-authentik)
//! - Keycloak (security_search.rs as tpl-batch2-keycloak)
//! - Jellyfin (documentation.rs as tpl-batch2-jellyfin)
//! - Navidrome (documentation.rs as tpl-batch2-navidrome)
//! - Audiobookshelf (media_productivity.rs as tpl-audiobookshelf)
//! - Calibre Web (sprint19.rs as tpl-calibre-web)
//! - n8n (infrastructure.rs as n8n + sprint18.rs as tpl-n8n)
//! - Glances (media_productivity.rs as tpl-glances)
//! - Uptime Kuma (infrastructure.rs as uptime-kuma)
//! - Gitea (infrastructure.rs as gitea)
//! - Code Server (sprint19.rs as tpl-code-server)
//! - Jenkins (extra_services.rs as tpl-jenkins)
//! - Nextcloud (infrastructure.rs as nextcloud)
//! - Seafile (documentation.rs as tpl-batch2-seafile)
//! - Mattermost (cms_communication.rs as mattermost)
//!
//! New templates added: Flowise, Langflow, Open WebUI, AnythingLLM,
//! Pocket ID, Activepieces, Trigger.dev, Signoz

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== AI / LLM ====================
        (
            "tpl-flowise",
            "Flowise",
            "Low-code drag-and-drop UI to build LLM flows and AI agents. Connect LangChain and LlamaIndex components visually. Supports OpenAI, HuggingFace, local models via Ollama, and more.",
            "AI/ML",
            "flowise",
            r#"services:
  flowise:
    image: flowiseai/flowise:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-flowise}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - FLOWISE_USERNAME=${FLOWISE_USERNAME:-admin}
      - FLOWISE_PASSWORD=${FLOWISE_PASSWORD}
      - FLOWISE_SECRETKEY_OVERWRITE=${FLOWISE_SECRET_KEY:-}
      - DATABASE_PATH=/root/.flowise
      - APIKEY_PATH=/root/.flowise
      - SECRETKEY_PATH=/root/.flowise
      - LOG_LEVEL=${LOG_LEVEL:-info}
      - BLOB_STORAGE_PATH=/root/.flowise/storage
    volumes:
      - flowise_data:/root/.flowise
    labels:
      - "rivetr.managed=true"

volumes:
  flowise_data:
"#,
            r#"[{"name":"FLOWISE_USERNAME","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"FLOWISE_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"FLOWISE_SECRET_KEY","label":"Encryption Secret Key (for credentials)","required":false,"default":"","secret":true},{"name":"LOG_LEVEL","label":"Log Level (error, warn, info, verbose, debug)","required":false,"default":"info","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"flowise","secret":false}]"#,
        ),
        (
            "tpl-langflow",
            "Langflow",
            "Visual GUI for building, iterating, and deploying LangChain AI flows. Drag-and-drop interface for chaining LLMs, prompts, agents, and tools. Supports any OpenAI-compatible API.",
            "AI/ML",
            "langflow",
            r#"services:
  langflow:
    image: logspace/langflow:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-langflow}
    restart: unless-stopped
    ports:
      - "${PORT:-7860}:7860"
    environment:
      - LANGFLOW_SECRET_KEY=${LANGFLOW_SECRET_KEY}
      - LANGFLOW_SUPERUSER=${LANGFLOW_SUPERUSER:-admin}
      - LANGFLOW_SUPERUSER_PASSWORD=${LANGFLOW_SUPERUSER_PASSWORD}
      - LANGFLOW_AUTO_LOGIN=${LANGFLOW_AUTO_LOGIN:-false}
      - LANGFLOW_DATABASE_URL=${DATABASE_URL:-sqlite:///./langflow.db}
      - LANGFLOW_LOG_LEVEL=${LOG_LEVEL:-critical}
    volumes:
      - langflow_data:/app/langflow
    labels:
      - "rivetr.managed=true"

volumes:
  langflow_data:
"#,
            r#"[{"name":"LANGFLOW_SECRET_KEY","label":"Secret Key (generate with: openssl rand -hex 32)","required":true,"default":"","secret":true},{"name":"LANGFLOW_SUPERUSER","label":"Superuser Username","required":false,"default":"admin","secret":false},{"name":"LANGFLOW_SUPERUSER_PASSWORD","label":"Superuser Password","required":true,"default":"","secret":true},{"name":"LANGFLOW_AUTO_LOGIN","label":"Auto Login (disable for production)","required":false,"default":"false","secret":false},{"name":"DATABASE_URL","label":"Database URL (leave empty for SQLite)","required":false,"default":"sqlite:///./langflow.db","secret":true},{"name":"LOG_LEVEL","label":"Log Level","required":false,"default":"critical","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"7860","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"langflow","secret":false}]"#,
        ),
        (
            "tpl-open-webui",
            "Open WebUI",
            "Feature-rich, self-hosted web UI for Ollama and OpenAI-compatible APIs. Supports chat history, multimodal models, RAG pipelines, web search, and user management. Works offline with local models.",
            "AI/ML",
            "open-webui",
            r#"services:
  open-webui:
    image: ghcr.io/open-webui/open-webui:${VERSION:-main}
    container_name: ${CONTAINER_NAME:-open-webui}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - WEBUI_SECRET_KEY=${WEBUI_SECRET_KEY}
      - WEBUI_NAME=${WEBUI_NAME:-Open WebUI}
      - OLLAMA_BASE_URL=${OLLAMA_BASE_URL:-http://host.docker.internal:11434}
      - OPENAI_API_KEY=${OPENAI_API_KEY:-}
      - OPENAI_API_BASE_URL=${OPENAI_API_BASE_URL:-}
      - ENABLE_SIGNUP=${ENABLE_SIGNUP:-true}
      - DEFAULT_USER_ROLE=${DEFAULT_USER_ROLE:-pending}
      - ENABLE_COMMUNITY_SHARING=${ENABLE_COMMUNITY_SHARING:-false}
      - WEBUI_AUTH=${WEBUI_AUTH:-true}
    volumes:
      - open_webui_data:/app/backend/data
    extra_hosts:
      - "host.docker.internal:host-gateway"
    labels:
      - "rivetr.managed=true"

volumes:
  open_webui_data:
"#,
            r#"[{"name":"WEBUI_SECRET_KEY","label":"Secret Key (generate with: openssl rand -hex 32)","required":true,"default":"","secret":true},{"name":"WEBUI_NAME","label":"Application Name","required":false,"default":"Open WebUI","secret":false},{"name":"OLLAMA_BASE_URL","label":"Ollama Base URL","required":false,"default":"http://host.docker.internal:11434","secret":false},{"name":"OPENAI_API_KEY","label":"OpenAI API Key (optional)","required":false,"default":"","secret":true},{"name":"OPENAI_API_BASE_URL","label":"OpenAI-Compatible Base URL (optional)","required":false,"default":"","secret":false},{"name":"ENABLE_SIGNUP","label":"Allow User Sign-Up","required":false,"default":"true","secret":false},{"name":"DEFAULT_USER_ROLE","label":"Default Role for New Users (pending, user, admin)","required":false,"default":"pending","secret":false},{"name":"WEBUI_AUTH","label":"Enable Authentication","required":false,"default":"true","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"8080","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"main","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"open-webui","secret":false}]"#,
        ),
        (
            "tpl-anythingllm",
            "AnythingLLM",
            "All-in-one AI document chat application. Upload PDFs, Word docs, and websites and chat with them using any LLM. Supports OpenAI, Anthropic, Ollama, and local models. Built-in vector database.",
            "AI/ML",
            "anythingllm",
            r#"services:
  anythingllm:
    image: mintplexlabs/anythingllm:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-anythingllm}
    restart: unless-stopped
    ports:
      - "${PORT:-3001}:3001"
    environment:
      - AUTH_TOKEN=${AUTH_TOKEN:-}
      - JWT_SECRET=${JWT_SECRET}
      - STORAGE_DIR=/app/server/storage
      - LLM_PROVIDER=${LLM_PROVIDER:-openai}
      - OPEN_AI_KEY=${OPENAI_API_KEY:-}
      - OPEN_MODEL_PREF=${OPENAI_MODEL:-gpt-4o}
      - EMBEDDING_ENGINE=${EMBEDDING_ENGINE:-openai}
      - VECTOR_DB=${VECTOR_DB:-lancedb}
      - DISABLE_TELEMETRY=${DISABLE_TELEMETRY:-true}
    volumes:
      - anythingllm_storage:/app/server/storage
    labels:
      - "rivetr.managed=true"

volumes:
  anythingllm_storage:
"#,
            r#"[{"name":"AUTH_TOKEN","label":"Access Token (leave empty to disable auth)","required":false,"default":"","secret":true},{"name":"JWT_SECRET","label":"JWT Secret (generate with: openssl rand -hex 32)","required":true,"default":"","secret":true},{"name":"LLM_PROVIDER","label":"LLM Provider (openai, anthropic, ollama, etc.)","required":false,"default":"openai","secret":false},{"name":"OPENAI_API_KEY","label":"OpenAI API Key","required":false,"default":"","secret":true},{"name":"OPENAI_MODEL","label":"OpenAI Model","required":false,"default":"gpt-4o","secret":false},{"name":"EMBEDDING_ENGINE","label":"Embedding Engine (openai, native)","required":false,"default":"openai","secret":false},{"name":"VECTOR_DB","label":"Vector Database (lancedb, chroma, pinecone, etc.)","required":false,"default":"lancedb","secret":false},{"name":"DISABLE_TELEMETRY","label":"Disable Telemetry","required":false,"default":"true","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3001","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"anythingllm","secret":false}]"#,
        ),
        // ==================== AUTH / SSO ====================
        (
            "tpl-pocket-id",
            "Pocket ID",
            "Simple, self-hosted OIDC provider that supports passkeys as the primary authentication method. No passwords required. Lightweight alternative to Authentik or Keycloak for personal and small-team use.",
            "Auth/SSO",
            "pocket-id",
            r#"services:
  pocket-id:
    image: stonith404/pocket-id:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-pocket-id}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - PUBLIC_APP_URL=${PUBLIC_APP_URL:-http://localhost}
      - TRUST_PROXY=${TRUST_PROXY:-true}
      - MAXMIND_LICENSE_KEY=${MAXMIND_LICENSE_KEY:-}
      - EMAIL_ENABLED=${EMAIL_ENABLED:-false}
      - SMTP_HOST=${SMTP_HOST:-}
      - SMTP_PORT=${SMTP_PORT:-587}
      - SMTP_USER=${SMTP_USER:-}
      - SMTP_PASSWORD=${SMTP_PASSWORD:-}
      - SMTP_FROM=${SMTP_FROM:-}
    volumes:
      - pocket_id_data:/app/backend/data
    labels:
      - "rivetr.managed=true"

volumes:
  pocket_id_data:
"#,
            r#"[{"name":"PUBLIC_APP_URL","label":"Public App URL (e.g. https://auth.example.com)","required":true,"default":"http://localhost","secret":false},{"name":"TRUST_PROXY","label":"Trust Proxy Headers","required":false,"default":"true","secret":false},{"name":"EMAIL_ENABLED","label":"Enable Email (for magic link fallback)","required":false,"default":"false","secret":false},{"name":"SMTP_HOST","label":"SMTP Host","required":false,"default":"","secret":false},{"name":"SMTP_PORT","label":"SMTP Port","required":false,"default":"587","secret":false},{"name":"SMTP_USER","label":"SMTP Username","required":false,"default":"","secret":false},{"name":"SMTP_PASSWORD","label":"SMTP Password","required":false,"default":"","secret":true},{"name":"SMTP_FROM","label":"SMTP From Address","required":false,"default":"","secret":false},{"name":"MAXMIND_LICENSE_KEY","label":"MaxMind License Key (for GeoIP, optional)","required":false,"default":"","secret":true},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"pocket-id","secret":false}]"#,
        ),
        // ==================== AUTOMATION ====================
        (
            "tpl-activepieces",
            "Activepieces",
            "Open-source no-code automation platform. Build workflows by connecting apps with pre-built pieces — a self-hostable alternative to Zapier and Make. Supports 100+ integrations.",
            "Automation",
            "activepieces",
            r#"services:
  activepieces:
    image: activepieces/activepieces:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-activepieces}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - AP_ENGINE_EXECUTABLE_PATH=dist/packages/engine/main.js
      - AP_JWT_SECRET=${AP_JWT_SECRET}
      - AP_ENCRYPTION_KEY=${AP_ENCRYPTION_KEY}
      - AP_POSTGRES_DATABASE=${AP_POSTGRES_DATABASE:-activepieces}
      - AP_POSTGRES_HOST=activepieces_db
      - AP_POSTGRES_PORT=5432
      - AP_POSTGRES_USERNAME=${AP_POSTGRES_USERNAME:-activepieces}
      - AP_POSTGRES_PASSWORD=${AP_POSTGRES_PASSWORD}
      - AP_REDIS_URL=redis://activepieces_redis:6379
      - AP_FRONTEND_URL=${AP_FRONTEND_URL:-http://localhost}
      - AP_TELEMETRY_ENABLED=${AP_TELEMETRY_ENABLED:-false}
      - AP_SIGN_UP_ENABLED=${AP_SIGN_UP_ENABLED:-true}
    depends_on:
      - activepieces_db
      - activepieces_redis
    labels:
      - "rivetr.managed=true"

  activepieces_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-activepieces}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${AP_POSTGRES_USERNAME:-activepieces}
      - POSTGRES_PASSWORD=${AP_POSTGRES_PASSWORD}
      - POSTGRES_DB=${AP_POSTGRES_DATABASE:-activepieces}
    volumes:
      - activepieces_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  activepieces_redis:
    image: redis:7-alpine
    container_name: ${CONTAINER_NAME:-activepieces}-redis
    restart: unless-stopped
    volumes:
      - activepieces_redis_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  activepieces_db_data:
  activepieces_redis_data:
"#,
            r#"[{"name":"AP_JWT_SECRET","label":"JWT Secret (generate with: openssl rand -hex 32)","required":true,"default":"","secret":true},{"name":"AP_ENCRYPTION_KEY","label":"Encryption Key (generate with: openssl rand -hex 16)","required":true,"default":"","secret":true},{"name":"AP_POSTGRES_PASSWORD","label":"PostgreSQL Password","required":true,"default":"","secret":true},{"name":"AP_POSTGRES_USERNAME","label":"PostgreSQL Username","required":false,"default":"activepieces","secret":false},{"name":"AP_POSTGRES_DATABASE","label":"PostgreSQL Database Name","required":false,"default":"activepieces","secret":false},{"name":"AP_FRONTEND_URL","label":"Frontend URL (e.g. https://automation.example.com)","required":false,"default":"http://localhost","secret":false},{"name":"AP_SIGN_UP_ENABLED","label":"Allow New Sign-Ups","required":false,"default":"true","secret":false},{"name":"AP_TELEMETRY_ENABLED","label":"Enable Telemetry","required":false,"default":"false","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"80","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"activepieces","secret":false}]"#,
        ),
        (
            "tpl-trigger-dev",
            "Trigger.dev",
            "Open-source background jobs and workflow platform. Write persistent, long-running jobs as regular TypeScript with automatic retries, scheduling, and real-time monitoring. Alternative to Inngest and Temporal.",
            "Automation",
            "trigger-dev",
            r#"services:
  trigger-dev:
    image: ghcr.io/triggerdotdev/trigger.dev:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-trigger-dev}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - LOGIN_ORIGIN=${LOGIN_ORIGIN:-http://localhost:3000}
      - APP_ORIGIN=${APP_ORIGIN:-http://localhost:3000}
      - SECRET_KEY=${SESSION_SECRET}
      - MAGIC_LINK_SECRET=${MAGIC_LINK_SECRET}
      - DATABASE_URL=postgresql://${DB_USER:-trigger}:${DB_PASSWORD}@trigger_db:5432/${DB_NAME:-trigger}
      - DIRECT_URL=postgresql://${DB_USER:-trigger}:${DB_PASSWORD}@trigger_db:5432/${DB_NAME:-trigger}
      - REDIS_TLS_DISABLED=true
      - REDIS_HOST=trigger_redis
      - REDIS_PORT=6379
      - NODE_ENV=${NODE_ENV:-production}
    depends_on:
      - trigger_db
      - trigger_redis
    labels:
      - "rivetr.managed=true"

  trigger_db:
    image: postgres:16-alpine
    container_name: ${CONTAINER_NAME:-trigger}-db
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER:-trigger}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=${DB_NAME:-trigger}
    volumes:
      - trigger_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  trigger_redis:
    image: redis:7-alpine
    container_name: ${CONTAINER_NAME:-trigger}-redis
    restart: unless-stopped
    volumes:
      - trigger_redis_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  trigger_db_data:
  trigger_redis_data:
"#,
            r#"[{"name":"SESSION_SECRET","label":"Session Secret (generate with: openssl rand -hex 32)","required":true,"default":"","secret":true},{"name":"MAGIC_LINK_SECRET","label":"Magic Link Secret (generate with: openssl rand -hex 32)","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"PostgreSQL Password","required":true,"default":"","secret":true},{"name":"DB_USER","label":"PostgreSQL Username","required":false,"default":"trigger","secret":false},{"name":"DB_NAME","label":"PostgreSQL Database Name","required":false,"default":"trigger","secret":false},{"name":"LOGIN_ORIGIN","label":"Login Origin URL (e.g. https://trigger.example.com)","required":false,"default":"http://localhost:3000","secret":false},{"name":"APP_ORIGIN","label":"App Origin URL (e.g. https://trigger.example.com)","required":false,"default":"http://localhost:3000","secret":false},{"name":"NODE_ENV","label":"Node Environment","required":false,"default":"production","secret":false},{"name":"PORT","label":"Host Port","required":false,"default":"3000","secret":false},{"name":"VERSION","label":"Image Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"trigger-dev","secret":false}]"#,
        ),
        // ==================== MONITORING ====================
        (
            "tpl-signoz",
            "SigNoz",
            "Open-source APM and observability platform. Monitor application metrics, traces, and logs in one place — a self-hosted alternative to Datadog and New Relic. Built on OpenTelemetry and ClickHouse.",
            "Monitoring",
            "signoz",
            r#"services:
  signoz-frontend:
    image: signoz/frontend:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-signoz-frontend}
    restart: unless-stopped
    ports:
      - "${PORT:-3301}:3301"
    depends_on:
      - signoz-query-service
    labels:
      - "rivetr.managed=true"

  signoz-query-service:
    image: signoz/query-service:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-signoz}-query-service
    restart: unless-stopped
    environment:
      - ClickHouseUrl=tcp://signoz-clickhouse:9000
      - STORAGE=clickhouse
      - GODEBUG=netdns=go
      - TELEMETRY_ENABLED=${TELEMETRY_ENABLED:-false}
      - DEPLOYMENT_TYPE=docker-standalone-amd
    volumes:
      - signoz_data:/var/lib/signoz
    depends_on:
      - signoz-clickhouse
    labels:
      - "rivetr.managed=true"

  signoz-clickhouse:
    image: clickhouse/clickhouse-server:${CLICKHOUSE_VERSION:-24.1.2-alpine}
    container_name: ${CONTAINER_NAME:-signoz}-clickhouse
    restart: unless-stopped
    environment:
      - CLICKHOUSE_DB=signoz_traces
      - CLICKHOUSE_USER=${CLICKHOUSE_USER:-default}
      - CLICKHOUSE_PASSWORD=${CLICKHOUSE_PASSWORD:-}
    volumes:
      - signoz_clickhouse_data:/var/lib/clickhouse
    ulimits:
      nofile:
        soft: 262144
        hard: 262144
    labels:
      - "rivetr.managed=true"

  signoz-otel-collector:
    image: signoz/signoz-otel-collector:${OTEL_VERSION:-0.88.11}
    container_name: ${CONTAINER_NAME:-signoz}-otel-collector
    restart: unless-stopped
    ports:
      - "${OTEL_GRPC_PORT:-4317}:4317"
      - "${OTEL_HTTP_PORT:-4318}:4318"
    depends_on:
      - signoz-clickhouse
    labels:
      - "rivetr.managed=true"

volumes:
  signoz_data:
  signoz_clickhouse_data:
"#,
            r#"[{"name":"PORT","label":"Frontend Host Port","required":false,"default":"3301","secret":false},{"name":"OTEL_GRPC_PORT","label":"OpenTelemetry gRPC Port","required":false,"default":"4317","secret":false},{"name":"OTEL_HTTP_PORT","label":"OpenTelemetry HTTP Port","required":false,"default":"4318","secret":false},{"name":"CLICKHOUSE_PASSWORD","label":"ClickHouse Password (leave empty for no auth)","required":false,"default":"","secret":true},{"name":"TELEMETRY_ENABLED","label":"Enable SigNoz Telemetry","required":false,"default":"false","secret":false},{"name":"VERSION","label":"SigNoz Image Version","required":false,"default":"latest","secret":false},{"name":"CLICKHOUSE_VERSION","label":"ClickHouse Image Version","required":false,"default":"24.1.2-alpine","secret":false},{"name":"OTEL_VERSION","label":"OTel Collector Version","required":false,"default":"0.88.11","secret":false},{"name":"CONTAINER_NAME","label":"Container Name Prefix","required":false,"default":"signoz","secret":false}]"#,
        ),
    ]
}
