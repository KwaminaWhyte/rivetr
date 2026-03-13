//! Additional AI/ML service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== AI / ML (additional) ====================
        (
            "tpl-langfuse",
            "Langfuse",
            "Open-source LLM observability platform. Tracing, evals, and analytics for LLM applications.",
            "ai",
            "langfuse",
            r#"services:
  langfuse:
    image: langfuse/langfuse:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-langfuse}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgresql://langfuse:${DB_PASSWORD:-langfuse}@langfuse_db:5432/langfuse
      - NEXTAUTH_SECRET=${NEXTAUTH_SECRET:-change-me-to-a-random-string}
      - NEXTAUTH_URL=${NEXTAUTH_URL:-http://localhost:3000}
      - SALT=${SALT:-change-me-to-a-random-string}
      - ENCRYPTION_KEY=${ENCRYPTION_KEY:-0000000000000000000000000000000000000000000000000000000000000000}
    depends_on:
      - langfuse_db
    labels:
      - "rivetr.managed=true"

  langfuse_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=langfuse
      - POSTGRES_PASSWORD=${DB_PASSWORD:-langfuse}
      - POSTGRES_DB=langfuse
    volumes:
      - langfuse_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  langfuse_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"langfuse","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"NEXTAUTH_SECRET","label":"NextAuth Secret","required":true,"default":"","secret":true},{"name":"NEXTAUTH_URL","label":"NextAuth URL","required":false,"default":"http://localhost:3000","secret":false},{"name":"SALT","label":"Salt","required":true,"default":"","secret":true},{"name":"ENCRYPTION_KEY","label":"Encryption Key (64 hex chars)","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-localai",
            "LocalAI",
            "Free, open-source, self-hosted OpenAI-compatible REST API. Run LLMs without GPU requirements.",
            "ai",
            "localai",
            r#"services:
  localai:
    image: localai/localai:${VERSION:-latest-cpu}
    container_name: ${CONTAINER_NAME:-localai}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - MODELS_PATH=/models
      - CONTEXT_SIZE=${CONTEXT_SIZE:-512}
      - THREADS=${THREADS:-4}
    volumes:
      - localai_models:/models
    labels:
      - "rivetr.managed=true"

volumes:
  localai_models:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest-cpu","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"localai","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"CONTEXT_SIZE","label":"Context Size","required":false,"default":"512","secret":false},{"name":"THREADS","label":"CPU Threads","required":false,"default":"4","secret":false}]"#,
        ),
        (
            "tpl-perplexica",
            "Perplexica",
            "Open-source AI search engine powered by SearXNG. Privacy-respecting Perplexity alternative.",
            "ai",
            "perplexica",
            r#"services:
  perplexica:
    image: itzcrazykns1337/perplexica:${VERSION:-main}
    container_name: ${CONTAINER_NAME:-perplexica}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - OPENAI=${OPENAI_KEY:-}
      - OLLAMA_API_URL=${OLLAMA_URL:-http://host.docker.internal:11434}
    volumes:
      - perplexica_data:/home/perplexica/data
    labels:
      - "rivetr.managed=true"

volumes:
  perplexica_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"main","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"perplexica","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"OPENAI_KEY","label":"OpenAI API Key (optional)","required":false,"default":"","secret":true},{"name":"OLLAMA_URL","label":"Ollama API URL","required":false,"default":"http://host.docker.internal:11434","secret":false}]"#,
        ),
        (
            "tpl-searxng",
            "SearXNG",
            "Privacy-respecting metasearch engine. Aggregates results from 70+ search engines without tracking.",
            "search",
            "searxng",
            r#"services:
  searxng:
    image: searxng/searxng:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-searxng}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - INSTANCE_NAME=${INSTANCE_NAME:-SearXNG}
      - BASE_URL=${BASE_URL:-http://localhost:8080/}
      - AUTOCOMPLETE=${AUTOCOMPLETE:-false}
    volumes:
      - searxng_data:/etc/searxng
    labels:
      - "rivetr.managed=true"

volumes:
  searxng_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"searxng","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"INSTANCE_NAME","label":"Instance Name","required":false,"default":"SearXNG","secret":false},{"name":"BASE_URL","label":"Base URL","required":false,"default":"http://localhost:8080/","secret":false}]"#,
        ),
        (
            "tpl-n8n-ai",
            "n8n (AI-ready)",
            "Workflow automation with native AI nodes. Connect LLMs to any API or service with a visual editor.",
            "automation",
            "n8n",
            r#"services:
  n8n:
    image: n8nio/n8n:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-n8n-ai}
    restart: unless-stopped
    ports:
      - "${PORT:-5678}:5678"
    environment:
      - N8N_HOST=${HOST:-0.0.0.0}
      - N8N_PORT=5678
      - N8N_PROTOCOL=${PROTOCOL:-http}
      - WEBHOOK_URL=${WEBHOOK_URL:-http://localhost:5678/}
      - DB_TYPE=postgresdb
      - DB_POSTGRESDB_HOST=n8n_ai_db
      - DB_POSTGRESDB_DATABASE=n8n
      - DB_POSTGRESDB_USER=n8n
      - DB_POSTGRESDB_PASSWORD=${DB_PASSWORD:-n8n}
      - N8N_ENCRYPTION_KEY=${ENCRYPTION_KEY:-change-me-to-a-random-string}
    depends_on:
      - n8n_ai_db
    volumes:
      - n8n_ai_data:/home/node/.n8n
    labels:
      - "rivetr.managed=true"

  n8n_ai_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=n8n
      - POSTGRES_PASSWORD=${DB_PASSWORD:-n8n}
      - POSTGRES_DB=n8n
    volumes:
      - n8n_ai_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  n8n_ai_data:
  n8n_ai_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"n8n-ai","secret":false},{"name":"PORT","label":"Port","required":false,"default":"5678","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"ENCRYPTION_KEY","label":"Encryption Key","required":true,"default":"","secret":true},{"name":"WEBHOOK_URL","label":"Webhook URL","required":false,"default":"http://localhost:5678/","secret":false}]"#,
        ),
        (
            "tpl-dify",
            "Dify",
            "Open-source LLM app development platform. Build AI apps with RAG pipelines, agents, and workflows.",
            "ai",
            "dify",
            r#"services:
  dify-api:
    image: langgenius/dify-api:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-dify-api}
    restart: unless-stopped
    ports:
      - "${PORT:-5001}:5001"
    environment:
      - MODE=api
      - SECRET_KEY=${SECRET_KEY:-change-me-to-a-random-string}
      - DB_USERNAME=dify
      - DB_PASSWORD=${DB_PASSWORD:-dify}
      - DB_HOST=dify_db
      - DB_PORT=5432
      - DB_DATABASE=dify
      - REDIS_HOST=dify_redis
      - REDIS_PASSWORD=${REDIS_PASSWORD:-dify}
      - CELERY_BROKER_URL=redis://:${REDIS_PASSWORD:-dify}@dify_redis:6379/1
      - STORAGE_TYPE=local
      - STORAGE_LOCAL_PATH=/app/api/storage
    depends_on:
      - dify_db
      - dify_redis
    volumes:
      - dify_storage:/app/api/storage
    labels:
      - "rivetr.managed=true"

  dify-web:
    image: langgenius/dify-web:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-dify-web}
    restart: unless-stopped
    ports:
      - "${WEB_PORT:-3000}:3000"
    environment:
      - EDITION=SELF_HOSTED
      - CONSOLE_API_URL=${API_URL:-http://localhost:5001}
      - APP_API_URL=${API_URL:-http://localhost:5001}
    labels:
      - "rivetr.managed=true"

  dify_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=dify
      - POSTGRES_PASSWORD=${DB_PASSWORD:-dify}
      - POSTGRES_DB=dify
    volumes:
      - dify_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  dify_redis:
    image: redis:7-alpine
    restart: unless-stopped
    command: redis-server --requirepass ${REDIS_PASSWORD:-dify}
    labels:
      - "rivetr.managed=true"

volumes:
  dify_storage:
  dify_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"dify-api","secret":false},{"name":"PORT","label":"API Port","required":false,"default":"5001","secret":false},{"name":"WEB_PORT","label":"Web Port","required":false,"default":"3000","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"REDIS_PASSWORD","label":"Redis Password","required":true,"default":"","secret":true},{"name":"API_URL","label":"API URL","required":false,"default":"http://localhost:5001","secret":false}]"#,
        ),
    ]
}
