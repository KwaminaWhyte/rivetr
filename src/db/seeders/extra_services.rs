//! Extra service templates based on Coolify's service catalog.
//! Covers administration dashboards, additional AI services, backup tools,
//! media servers, storage, development tools, and more.

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== ADMINISTRATION / DASHBOARDS ====================
        (
            "tpl-homepage",
            "Homepage",
            "A modern, highly customizable application dashboard. Integrates with dozens of self-hosted services.",
            "development",
            "homepage",
            r#"services:
  homepage:
    image: ghcr.io/gethomepage/homepage:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-homepage}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - HOMEPAGE_ALLOWED_HOSTS=${ALLOWED_HOSTS:-*}
    volumes:
      - homepage_config:/app/config
      - /var/run/docker.sock:/var/run/docker.sock:ro
    labels:
      - "rivetr.managed=true"

volumes:
  homepage_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"homepage","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"ALLOWED_HOSTS","label":"Allowed Hosts","required":false,"default":"*","secret":false}]"#,
        ),
        (
            "tpl-homarr",
            "Homarr",
            "Sleek, modern dashboard for your self-hosted services. Integrations, widgets, and search built in.",
            "development",
            "homarr",
            r#"services:
  homarr:
    image: ghcr.io/ajnart/homarr:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-homarr}
    restart: unless-stopped
    ports:
      - "${PORT:-7575}:7575"
    environment:
      - SECRET_ENCRYPTION_KEY=${ENCRYPTION_KEY:-change-me-to-a-64-hex-char-string}
      - AUTH_SECRET=${AUTH_SECRET:-change-me-to-a-random-string}
    volumes:
      - homarr_data:/appdata
      - /var/run/docker.sock:/var/run/docker.sock:ro
    labels:
      - "rivetr.managed=true"

volumes:
  homarr_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"homarr","secret":false},{"name":"PORT","label":"Port","required":false,"default":"7575","secret":false},{"name":"ENCRYPTION_KEY","label":"Encryption Key (64 hex chars)","required":true,"default":"","secret":true},{"name":"AUTH_SECRET","label":"Auth Secret","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-dashy",
            "Dashy",
            "Feature-rich, highly customizable personal dashboard. Status checks, widgets, and icon packs included.",
            "development",
            "dashy",
            r#"services:
  dashy:
    image: lissy93/dashy:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-dashy}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    volumes:
      - dashy_config:/app/user-data
    labels:
      - "rivetr.managed=true"

volumes:
  dashy_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"dashy","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-organizr",
            "Organizr",
            "HTPC/homelab services organizer. Puts all your web apps behind a single tabbed dashboard.",
            "development",
            "organizr",
            r#"services:
  organizr:
    image: organizr/organizr:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-organizr}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=${TZ:-UTC}
    volumes:
      - organizr_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  organizr_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"organizr","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false}]"#,
        ),

        // ==================== AI SERVICES ====================
        (
            "tpl-anything-llm",
            "AnythingLLM",
            "All-in-one AI app for chatting with documents using any LLM. Supports local and cloud models.",
            "ai",
            "anything-llm",
            r#"services:
  anythingllm:
    image: mintplexlabs/anythingllm:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-anythingllm}
    restart: unless-stopped
    ports:
      - "${PORT:-3001}:3001"
    cap_add:
      - SYS_ADMIN
    environment:
      - STORAGE_DIR=/app/server/storage
      - JWT_SECRET=${JWT_SECRET:-change-me-to-a-random-string}
      - SERVER_PORT=3001
    volumes:
      - anythingllm_storage:/app/server/storage
    labels:
      - "rivetr.managed=true"

volumes:
  anythingllm_storage:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"anythingllm","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3001","secret":false},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-librechat",
            "LibreChat",
            "Enhanced ChatGPT clone. Supports multiple AI providers, agents, RAG, and multi-user auth.",
            "ai",
            "librechat",
            r#"services:
  librechat:
    image: ghcr.io/danny-avila/librechat:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-librechat}
    restart: unless-stopped
    ports:
      - "${PORT:-3080}:3080"
    environment:
      - MONGO_URI=mongodb://librechat_mongo:27017/LibreChat
      - MEILI_HOST=http://librechat_meilisearch:7700
      - MEILI_MASTER_KEY=${MEILI_KEY:-change-me-to-a-secure-key}
      - JWT_SECRET=${JWT_SECRET:-change-me-to-a-random-string}
      - JWT_REFRESH_SECRET=${JWT_REFRESH_SECRET:-change-me-to-another-random-string}
      - CREDS_KEY=${CREDS_KEY:-f34be427ebb29de8d88c107a71546019685ed8b241d8f2ed00c3df97ad2566f0}
      - CREDS_IV=${CREDS_IV:-e2341419ec3dd3d19b13a1a87fafcbfb}
    volumes:
      - librechat_images:/app/client/public/images
      - librechat_logs:/app/api/logs
    depends_on:
      - librechat_mongo
      - librechat_meilisearch
    labels:
      - "rivetr.managed=true"

  librechat_mongo:
    image: mongo:7
    restart: unless-stopped
    volumes:
      - librechat_mongo_data:/data/db
    labels:
      - "rivetr.managed=true"

  librechat_meilisearch:
    image: getmeili/meilisearch:v1.7.3
    restart: unless-stopped
    environment:
      - MEILI_MASTER_KEY=${MEILI_KEY:-change-me-to-a-secure-key}
      - MEILI_NO_ANALYTICS=true
    volumes:
      - librechat_meili_data:/meili_data
    labels:
      - "rivetr.managed=true"

volumes:
  librechat_images:
  librechat_logs:
  librechat_mongo_data:
  librechat_meili_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"librechat","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3080","secret":false},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true},{"name":"JWT_REFRESH_SECRET","label":"JWT Refresh Secret","required":true,"default":"","secret":true},{"name":"MEILI_KEY","label":"Meilisearch Master Key","required":true,"default":"","secret":true},{"name":"CREDS_KEY","label":"Credentials Key","required":true,"default":"","secret":true},{"name":"CREDS_IV","label":"Credentials IV","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-langflow",
            "Langflow",
            "Visual drag-and-drop tool for building LLM pipelines and AI workflows with LangChain.",
            "ai",
            "langflow",
            r#"services:
  langflow:
    image: langflowai/langflow:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-langflow}
    restart: unless-stopped
    ports:
      - "${PORT:-7860}:7860"
    environment:
      - LANGFLOW_DATABASE_URL=postgresql://langflow:${DB_PASSWORD:-langflow}@langflow_db:5432/langflow
      - LANGFLOW_SUPERUSER=${ADMIN_USER:-admin}
      - LANGFLOW_SUPERUSER_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - LANGFLOW_SECRET_KEY=${SECRET_KEY:-change-me-to-a-random-string}
      - LANGFLOW_AUTO_LOGIN=false
    volumes:
      - langflow_data:/app/langflow
    depends_on:
      - langflow_db
    labels:
      - "rivetr.managed=true"

  langflow_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=langflow
      - POSTGRES_PASSWORD=${DB_PASSWORD:-langflow}
      - POSTGRES_DB=langflow
    volumes:
      - langflow_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  langflow_data:
  langflow_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"langflow","secret":false},{"name":"PORT","label":"Port","required":false,"default":"7860","secret":false},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-litellm",
            "LiteLLM",
            "Unified API gateway for 100+ LLMs. Load balancing, fallbacks, cost tracking, and a proxy server.",
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
      - DATABASE_URL=postgresql://litellm:${DB_PASSWORD:-litellm}@litellm_db:5432/litellm
      - LITELLM_MASTER_KEY=${MASTER_KEY:-sk-change-me}
      - LITELLM_SALT_KEY=${SALT_KEY:-change-me-to-a-random-string}
      - STORE_MODEL_IN_DB=True
    command: ["--port", "4000", "--config", "/app/config.yaml"]
    volumes:
      - litellm_config:/app
    depends_on:
      - litellm_db
    labels:
      - "rivetr.managed=true"

  litellm_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=litellm
      - POSTGRES_PASSWORD=${DB_PASSWORD:-litellm}
      - POSTGRES_DB=litellm
    volumes:
      - litellm_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  litellm_config:
  litellm_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"main-latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"litellm","secret":false},{"name":"PORT","label":"Port","required":false,"default":"4000","secret":false},{"name":"MASTER_KEY","label":"Master Key","required":true,"default":"","secret":true},{"name":"SALT_KEY","label":"Salt Key","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-libretranslate",
            "LibreTranslate",
            "Free and open-source machine translation API. Self-hosted alternative to Google Translate.",
            "ai",
            "libretranslate",
            r#"services:
  libretranslate:
    image: libretranslate/libretranslate:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-libretranslate}
    restart: unless-stopped
    ports:
      - "${PORT:-5000}:5000"
    environment:
      - LT_API_KEYS=${REQUIRE_API_KEYS:-false}
      - LT_API_KEYS_DB_PATH=/app/db/api_keys.db
      - LT_LOAD_ONLY=${LOAD_ONLY:-en,es,fr,de,zh}
    volumes:
      - libretranslate_db:/app/db
    labels:
      - "rivetr.managed=true"

volumes:
  libretranslate_db:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"libretranslate","secret":false},{"name":"PORT","label":"Port","required":false,"default":"5000","secret":false},{"name":"REQUIRE_API_KEYS","label":"Require API Keys","required":false,"default":"false","secret":false},{"name":"LOAD_ONLY","label":"Languages to Load (comma-separated)","required":false,"default":"en,es,fr,de,zh","secret":false}]"#,
        ),

        // ==================== ANALYTICS ====================
        (
            "tpl-goatcounter",
            "GoatCounter",
            "Easy, privacy-friendly web analytics. No tracking pixels or personal data collected.",
            "analytics",
            "goatcounter",
            r#"services:
  goatcounter:
    image: goatcounter/goatcounter:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-goatcounter}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - GOATCOUNTER_DB=sqlite:///data/goatcounter.sqlite3
    volumes:
      - goatcounter_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  goatcounter_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"goatcounter","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-openpanel",
            "OpenPanel",
            "Open-source alternative to Mixpanel and Amplitude. Product analytics with events and funnels.",
            "analytics",
            "openpanel",
            r#"services:
  openpanel:
    image: openpanel/openpanel:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-openpanel}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgresql://openpanel:${DB_PASSWORD:-openpanel}@openpanel_db:5432/openpanel
      - REDIS_URL=redis://openpanel_redis:6379
      - NEXTAUTH_SECRET=${NEXTAUTH_SECRET:-change-me-to-a-random-string}
      - NEXTAUTH_URL=${NEXTAUTH_URL:-http://localhost:3000}
    depends_on:
      - openpanel_db
      - openpanel_redis
    labels:
      - "rivetr.managed=true"

  openpanel_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=openpanel
      - POSTGRES_PASSWORD=${DB_PASSWORD:-openpanel}
      - POSTGRES_DB=openpanel
    volumes:
      - openpanel_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  openpanel_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  openpanel_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"openpanel","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"NEXTAUTH_SECRET","label":"NextAuth Secret","required":true,"default":"","secret":true},{"name":"NEXTAUTH_URL","label":"NextAuth URL","required":false,"default":"http://localhost:3000","secret":false}]"#,
        ),

        // ==================== BACKUP ====================
        (
            "tpl-duplicati",
            "Duplicati",
            "Free, open-source backup software. Stores encrypted, incremental, compressed backups to local or cloud storage.",
            "development",
            "duplicati",
            r#"services:
  duplicati:
    image: lscr.io/linuxserver/duplicati:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-duplicati}
    restart: unless-stopped
    ports:
      - "${PORT:-8200}:8200"
    environment:
      - PUID=0
      - PGID=0
      - TZ=${TZ:-UTC}
    volumes:
      - duplicati_config:/config
      - ${BACKUP_SOURCE:-/source}:/source:ro
      - ${BACKUP_DEST:-/backups}:/backups
    labels:
      - "rivetr.managed=true"

volumes:
  duplicati_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"duplicati","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8200","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"BACKUP_SOURCE","label":"Source Path to Back Up","required":false,"default":"/source","secret":false},{"name":"BACKUP_DEST","label":"Backup Destination Path","required":false,"default":"/backups","secret":false}]"#,
        ),

        // ==================== COMMUNICATION ====================
        (
            "tpl-matrix-synapse",
            "Matrix Synapse",
            "Open, federated communication server. The reference homeserver for the Matrix protocol.",
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
      - SYNAPSE_SERVER_NAME=${SERVER_NAME:-matrix.example.com}
      - SYNAPSE_REPORT_STATS=${REPORT_STATS:-no}
    volumes:
      - synapse_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  synapse_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"synapse","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8008","secret":false},{"name":"SERVER_NAME","label":"Server Name (your domain)","required":true,"default":"matrix.example.com","secret":false},{"name":"REPORT_STATS","label":"Report Anonymous Stats (yes/no)","required":false,"default":"no","secret":false}]"#,
        ),

        // ==================== DEVELOPMENT TOOLS ====================
        (
            "tpl-nocodb",
            "NocoDB",
            "Open-source Airtable alternative. Turn any database into a smart spreadsheet with no-code.",
            "development",
            "nocodb",
            r#"services:
  nocodb:
    image: nocodb/nocodb:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-nocodb}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - NC_DB=pg://nocodb_db:5432?u=nocodb&p=${DB_PASSWORD:-nocodb}&d=nocodb
      - NC_AUTH_JWT_SECRET=${JWT_SECRET:-change-me-to-a-random-string}
    depends_on:
      - nocodb_db
    volumes:
      - nocodb_data:/usr/app/data
    labels:
      - "rivetr.managed=true"

  nocodb_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=nocodb
      - POSTGRES_PASSWORD=${DB_PASSWORD:-nocodb}
      - POSTGRES_DB=nocodb
    volumes:
      - nocodb_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  nocodb_data:
  nocodb_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"nocodb","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-budibase",
            "Budibase",
            "Open-source low-code platform for building internal tools, admin panels, and workflows.",
            "development",
            "budibase",
            r#"services:
  budibase:
    image: budibase/budibase:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-budibase}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - JWT_SECRET=${JWT_SECRET:-change-me-to-a-random-string}
      - MINIO_ACCESS_KEY=${MINIO_ACCESS_KEY:-budibase}
      - MINIO_SECRET_KEY=${MINIO_SECRET_KEY:-change-me-minio-key}
      - REDIS_PASSWORD=${REDIS_PASSWORD:-budibase}
      - INTERNAL_API_KEY=${INTERNAL_API_KEY:-change-me-api-key}
      - BB_ADMIN_USER_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - BB_ADMIN_USER_PASSWORD=${ADMIN_PASSWORD:-changeme}
    volumes:
      - budibase_minio_data:/minio
      - budibase_couchdb_data:/opt/couchdb/data
    labels:
      - "rivetr.managed=true"

volumes:
  budibase_minio_data:
  budibase_couchdb_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"budibase","secret":false},{"name":"PORT","label":"Port","required":false,"default":"80","secret":false},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true},{"name":"MINIO_ACCESS_KEY","label":"MinIO Access Key","required":false,"default":"budibase","secret":false},{"name":"MINIO_SECRET_KEY","label":"MinIO Secret Key","required":true,"default":"","secret":true},{"name":"REDIS_PASSWORD","label":"Redis Password","required":true,"default":"","secret":true},{"name":"INTERNAL_API_KEY","label":"Internal API Key","required":true,"default":"","secret":true},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-dozzle",
            "Dozzle",
            "Realtime log viewer for Docker containers. Lightweight, no storage — just live streaming logs.",
            "monitoring",
            "dozzle",
            r#"services:
  dozzle:
    image: amir20/dozzle:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-dozzle}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DOZZLE_LEVEL=${LOG_LEVEL:-info}
      - DOZZLE_NO_ANALYTICS=true
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"dozzle","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"LOG_LEVEL","label":"Log Level","required":false,"default":"info","secret":false}]"#,
        ),
        (
            "tpl-portainer",
            "Portainer CE",
            "Lightweight Docker and Kubernetes management UI. Manage containers, images, volumes, and networks.",
            "development",
            "portainer",
            r#"services:
  portainer:
    image: portainer/portainer-ce:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-portainer}
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
      - "${HTTPS_PORT:-9443}:9443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - portainer_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  portainer_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"portainer","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"9000","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"9443","secret":false}]"#,
        ),
        (
            "tpl-jenkins",
            "Jenkins",
            "Leading open-source automation server. Build, deploy, and automate any project with plugins.",
            "development",
            "jenkins",
            r#"services:
  jenkins:
    image: jenkins/jenkins:${VERSION:-lts-jdk17}
    container_name: ${CONTAINER_NAME:-jenkins}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
      - "${AGENT_PORT:-50000}:50000"
    environment:
      - JAVA_OPTS=${JAVA_OPTS:--Dhudson.footerURL=https://rivetr.io}
    volumes:
      - jenkins_home:/var/jenkins_home
    labels:
      - "rivetr.managed=true"

volumes:
  jenkins_home:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"lts-jdk17","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"jenkins","secret":false},{"name":"PORT","label":"Web UI Port","required":false,"default":"8080","secret":false},{"name":"AGENT_PORT","label":"Agent Port","required":false,"default":"50000","secret":false}]"#,
        ),
        (
            "tpl-appsmith",
            "Appsmith",
            "Open-source low-code platform for building internal dashboards, CRUD apps, and admin panels.",
            "development",
            "appsmith",
            r#"services:
  appsmith:
    image: index.docker.io/appsmith/appsmith-ee:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-appsmith}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
    volumes:
      - appsmith_stacks:/appsmith-stacks
    labels:
      - "rivetr.managed=true"

volumes:
  appsmith_stacks:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"appsmith","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false}]"#,
        ),

        // ==================== MEDIA SERVERS ====================
        (
            "tpl-plex",
            "Plex Media Server",
            "Organize and stream your personal media collection to all your devices. Movies, TV, music, and photos.",
            "storage",
            "plex",
            r#"services:
  plex:
    image: lscr.io/linuxserver/plex:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-plex}
    restart: unless-stopped
    network_mode: host
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=${TZ:-UTC}
      - VERSION=docker
      - PLEX_CLAIM=${PLEX_CLAIM:-}
    volumes:
      - plex_config:/config
      - ${MEDIA_PATH:-/media}:/media:ro
    labels:
      - "rivetr.managed=true"

volumes:
  plex_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"plex","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"PLEX_CLAIM","label":"Plex Claim Token (from plex.tv/claim)","required":false,"default":"","secret":true},{"name":"MEDIA_PATH","label":"Media Path on Host","required":false,"default":"/media","secret":false}]"#,
        ),
        (
            "tpl-emby",
            "Emby",
            "Personal media server that automatically organizes your content and makes it look great on any screen.",
            "storage",
            "emby",
            r#"services:
  emby:
    image: lscr.io/linuxserver/emby:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-emby}
    restart: unless-stopped
    ports:
      - "${PORT:-8096}:8096"
      - "${HTTPS_PORT:-8920}:8920"
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=${TZ:-UTC}
    volumes:
      - emby_config:/config
      - ${MEDIA_PATH:-/media}:/media:ro
    labels:
      - "rivetr.managed=true"

volumes:
  emby_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"emby","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"8096","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"8920","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"MEDIA_PATH","label":"Media Path on Host","required":false,"default":"/media","secret":false}]"#,
        ),
        (
            "tpl-qbittorrent",
            "qBittorrent",
            "Free, open-source BitTorrent client with a web interface. Fast, lightweight, and feature-rich.",
            "storage",
            "qbittorrent",
            r#"services:
  qbittorrent:
    image: lscr.io/linuxserver/qbittorrent:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-qbittorrent}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
      - "${TORRENT_PORT:-6881}:6881"
      - "${TORRENT_PORT:-6881}:6881/udp"
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=${TZ:-UTC}
      - WEBUI_PORT=8080
      - TORRENTING_PORT=6881
    volumes:
      - qbittorrent_config:/config
      - ${DOWNLOAD_PATH:-/downloads}:/downloads
    labels:
      - "rivetr.managed=true"

volumes:
  qbittorrent_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"qbittorrent","secret":false},{"name":"PORT","label":"Web UI Port","required":false,"default":"8080","secret":false},{"name":"TORRENT_PORT","label":"Torrent Port","required":false,"default":"6881","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"DOWNLOAD_PATH","label":"Download Path on Host","required":false,"default":"/downloads","secret":false}]"#,
        ),
        (
            "tpl-sonarr",
            "Sonarr",
            "Smart PVR for newsgroup and bittorrent users. Automatically downloads, sorts, and renames TV shows.",
            "storage",
            "sonarr",
            r#"services:
  sonarr:
    image: lscr.io/linuxserver/sonarr:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-sonarr}
    restart: unless-stopped
    ports:
      - "${PORT:-8989}:8989"
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=${TZ:-UTC}
    volumes:
      - sonarr_config:/config
      - ${TV_PATH:-/tv}:/tv
      - ${DOWNLOAD_PATH:-/downloads}:/downloads
    labels:
      - "rivetr.managed=true"

volumes:
  sonarr_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"sonarr","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8989","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"TV_PATH","label":"TV Shows Path","required":false,"default":"/tv","secret":false},{"name":"DOWNLOAD_PATH","label":"Downloads Path","required":false,"default":"/downloads","secret":false}]"#,
        ),
        (
            "tpl-radarr",
            "Radarr",
            "Movie collection manager for Usenet and BitTorrent users. Automatically downloads and organizes movies.",
            "storage",
            "radarr",
            r#"services:
  radarr:
    image: lscr.io/linuxserver/radarr:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-radarr}
    restart: unless-stopped
    ports:
      - "${PORT:-7878}:7878"
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=${TZ:-UTC}
    volumes:
      - radarr_config:/config
      - ${MOVIES_PATH:-/movies}:/movies
      - ${DOWNLOAD_PATH:-/downloads}:/downloads
    labels:
      - "rivetr.managed=true"

volumes:
  radarr_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"radarr","secret":false},{"name":"PORT","label":"Port","required":false,"default":"7878","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"MOVIES_PATH","label":"Movies Path","required":false,"default":"/movies","secret":false},{"name":"DOWNLOAD_PATH","label":"Downloads Path","required":false,"default":"/downloads","secret":false}]"#,
        ),

        // ==================== STORAGE ====================
        (
            "tpl-nextcloud",
            "Nextcloud",
            "Self-hosted file sync and share. Includes calendar, contacts, office suite, and 200+ apps.",
            "storage",
            "nextcloud",
            r#"services:
  nextcloud:
    image: nextcloud:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-nextcloud}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - POSTGRES_HOST=nextcloud_db
      - POSTGRES_DB=nextcloud
      - POSTGRES_USER=nextcloud
      - POSTGRES_PASSWORD=${DB_PASSWORD:-nextcloud}
      - NEXTCLOUD_ADMIN_USER=${ADMIN_USER:-admin}
      - NEXTCLOUD_ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - NEXTCLOUD_TRUSTED_DOMAINS=${TRUSTED_DOMAINS:-localhost}
      - REDIS_HOST=nextcloud_redis
    volumes:
      - nextcloud_data:/var/www/html
    depends_on:
      - nextcloud_db
      - nextcloud_redis
    labels:
      - "rivetr.managed=true"

  nextcloud_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=nextcloud
      - POSTGRES_PASSWORD=${DB_PASSWORD:-nextcloud}
      - POSTGRES_DB=nextcloud
    volumes:
      - nextcloud_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  nextcloud_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  nextcloud_data:
  nextcloud_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"nextcloud","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"TRUSTED_DOMAINS","label":"Trusted Domains (space-separated)","required":false,"default":"localhost","secret":false}]"#,
        ),
        (
            "tpl-seafile",
            "Seafile",
            "High-performance file sync and share. Encryption, versioning, and collaboration for teams.",
            "storage",
            "seafile",
            r#"services:
  seafile:
    image: seafileltd/seafile-mc:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-seafile}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - DB_HOST=seafile_db
      - DB_ROOT_PASSWD=${DB_ROOT_PASSWORD:-root}
      - SEAFILE_ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - SEAFILE_ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - SEAFILE_SERVER_LETSENCRYPT=false
      - SEAFILE_SERVER_HOSTNAME=${SERVER_HOSTNAME:-localhost}
    volumes:
      - seafile_data:/shared
    depends_on:
      - seafile_db
      - seafile_memcached
    labels:
      - "rivetr.managed=true"

  seafile_db:
    image: mariadb:10.11
    restart: unless-stopped
    environment:
      - MARIADB_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-root}
      - MARIADB_AUTO_UPGRADE=1
    volumes:
      - seafile_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

  seafile_memcached:
    image: memcached:1.6.18
    restart: unless-stopped
    entrypoint: memcached -m 256
    labels:
      - "rivetr.managed=true"

volumes:
  seafile_data:
  seafile_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"seafile","secret":false},{"name":"PORT","label":"Port","required":false,"default":"80","secret":false},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"DB_ROOT_PASSWORD","label":"DB Root Password","required":true,"default":"","secret":true},{"name":"SERVER_HOSTNAME","label":"Server Hostname","required":false,"default":"localhost","secret":false}]"#,
        ),

        // ==================== SECURITY ====================
        (
            "tpl-pihole",
            "Pi-hole",
            "Network-wide ad blocking and DNS sinkhole. Block ads, trackers, and malware domains for your whole network.",
            "security",
            "pihole",
            r#"services:
  pihole:
    image: pihole/pihole:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-pihole}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
      - "${DNS_PORT:-53}:53/tcp"
      - "${DNS_PORT:-53}:53/udp"
    environment:
      - TZ=${TZ:-UTC}
      - WEBPASSWORD=${WEB_PASSWORD:-changeme}
      - PIHOLE_DNS_=${DNS_SERVER:-8.8.8.8;8.8.4.4}
    volumes:
      - pihole_etc:/etc/pihole
      - pihole_dnsmasq:/etc/dnsmasq.d
    labels:
      - "rivetr.managed=true"

volumes:
  pihole_etc:
  pihole_dnsmasq:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"pihole","secret":false},{"name":"PORT","label":"Web UI Port","required":false,"default":"80","secret":false},{"name":"DNS_PORT","label":"DNS Port","required":false,"default":"53","secret":false},{"name":"WEB_PASSWORD","label":"Web UI Password","required":true,"default":"","secret":true},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"DNS_SERVER","label":"Upstream DNS Servers","required":false,"default":"8.8.8.8;8.8.4.4","secret":false}]"#,
        ),
    ]
}
