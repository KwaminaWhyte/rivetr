//! AI/ML service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== AI / ML ====================
        (
            "ollama",
            "Ollama",
            "Run large language models locally. Supports Llama, Mistral, Code Llama, and many more.",
            "ai",
            "ollama",
            r#"services:
  ollama:
    image: ollama/ollama:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-ollama}
    restart: unless-stopped
    ports:
      - "${PORT:-11434}:11434"
    volumes:
      - ollama_models:/root/.ollama
    labels:
      - "rivetr.managed=true"

volumes:
  ollama_models:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"ollama","secret":false},{"name":"PORT","label":"Port","required":false,"default":"11434","secret":false}]"#,
        ),
        (
            "open-webui",
            "Open WebUI",
            "ChatGPT-like web interface for Ollama and other LLM backends. Feature-rich and extensible.",
            "ai",
            "open-webui",
            r#"services:
  open-webui:
    image: ghcr.io/open-webui/open-webui:${VERSION:-main}
    container_name: ${CONTAINER_NAME:-open-webui}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - OLLAMA_BASE_URL=${OLLAMA_BASE_URL:-http://host.docker.internal:11434}
    volumes:
      - open_webui_data:/app/backend/data
    labels:
      - "rivetr.managed=true"

volumes:
  open_webui_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"main","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"open-webui","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"OLLAMA_BASE_URL","label":"Ollama Base URL","required":false,"default":"http://host.docker.internal:11434","secret":false}]"#,
        ),
        (
            "litellm",
            "LiteLLM",
            "Unified LLM proxy server. Call 100+ LLM APIs in OpenAI-compatible format.",
            "ai",
            "litellm",
            r#"services:
  litellm:
    image: ghcr.io/berriai/litellm:${VERSION:-main-latest}
    container_name: ${CONTAINER_NAME:-litellm}
    restart: unless-stopped
    ports:
      - "${PORT:-4000}:4000"
    environment:
      - LITELLM_MASTER_KEY=${MASTER_KEY:-sk-litellm-master-key}
    volumes:
      - litellm_data:/app/data
    labels:
      - "rivetr.managed=true"

volumes:
  litellm_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"main-latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"litellm","secret":false},{"name":"PORT","label":"Port","required":false,"default":"4000","secret":false},{"name":"MASTER_KEY","label":"Master API Key","required":true,"default":"","secret":true}]"#,
        ),
        (
            "langflow",
            "Langflow",
            "Visual framework for building multi-agent and RAG applications. Drag-and-drop LLM app builder.",
            "ai",
            "langflow",
            r#"services:
  langflow:
    image: langflowai/langflow:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-langflow}
    restart: unless-stopped
    ports:
      - "${PORT:-7860}:7860"
    volumes:
      - langflow_data:/app/langflow
    labels:
      - "rivetr.managed=true"

volumes:
  langflow_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"langflow","secret":false},{"name":"PORT","label":"Port","required":false,"default":"7860","secret":false}]"#,
        ),
        (
            "flowise",
            "Flowise",
            "Low-code LLM orchestration tool. Build customized LLM flows with drag-and-drop UI.",
            "ai",
            "flowise",
            r#"services:
  flowise:
    image: flowiseai/flowise:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-flowise}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - FLOWISE_USERNAME=${USERNAME:-admin}
      - FLOWISE_PASSWORD=${PASSWORD:-}
    volumes:
      - flowise_data:/root/.flowise
    labels:
      - "rivetr.managed=true"

volumes:
  flowise_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"flowise","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"USERNAME","label":"Username","required":false,"default":"admin","secret":false},{"name":"PASSWORD","label":"Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "chromadb",
            "ChromaDB",
            "Open-source AI-native vector database. Store and query embeddings for LLM applications.",
            "ai",
            "chromadb",
            r#"services:
  chromadb:
    image: chromadb/chroma:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-chromadb}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - IS_PERSISTENT=TRUE
      - ANONYMIZED_TELEMETRY=FALSE
    volumes:
      - chromadb_data:/chroma/chroma
    labels:
      - "rivetr.managed=true"

volumes:
  chromadb_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"chromadb","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false}]"#,
        ),
    ]
}
