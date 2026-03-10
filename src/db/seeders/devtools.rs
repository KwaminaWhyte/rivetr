//! Development tools service templates (additional)

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== DEVELOPMENT (additional) ====================
        (
            "code-server",
            "Code Server",
            "VS Code in the browser. Full IDE experience accessible from any device with a web browser.",
            "development",
            "code-server",
            r#"services:
  code-server:
    image: lscr.io/linuxserver/code-server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-code-server}
    restart: unless-stopped
    ports:
      - "${PORT:-8443}:8443"
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=${TZ:-UTC}
      - PASSWORD=${PASSWORD:-}
      - SUDO_PASSWORD=${SUDO_PASSWORD:-}
      - DEFAULT_WORKSPACE=/config/workspace
    volumes:
      - code_server_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  code_server_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"code-server","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8443","secret":false},{"name":"TZ","label":"Timezone","required":false,"default":"UTC","secret":false},{"name":"PASSWORD","label":"Password","required":true,"default":"","secret":true},{"name":"SUDO_PASSWORD","label":"Sudo Password","required":false,"default":"","secret":true}]"#,
        ),
        (
            "supabase",
            "Supabase",
            "Open-source Firebase alternative. Postgres database, auth, instant APIs, realtime, and storage.",
            "development",
            "supabase",
            r#"services:
  supabase-studio:
    image: supabase/studio:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-supabase-studio}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - STUDIO_PG_META_URL=http://supabase-meta:8080
      - SUPABASE_URL=http://supabase-kong:8000
      - SUPABASE_REST_URL=http://supabase-kong:8000/rest/v1/
      - SUPABASE_ANON_KEY=${ANON_KEY:-eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9}
      - SUPABASE_SERVICE_KEY=${SERVICE_KEY:-eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9}
    depends_on:
      - supabase-db
    labels:
      - "rivetr.managed=true"

  supabase-db:
    image: supabase/postgres:15.6.1.120
    restart: unless-stopped
    environment:
      - POSTGRES_PASSWORD=${DB_PASSWORD:-supabase}
      - POSTGRES_DB=supabase
    volumes:
      - supabase_db_data:/var/lib/postgresql/data
    ports:
      - "${DB_PORT:-5432}:5432"
    labels:
      - "rivetr.managed=true"

  supabase-meta:
    image: supabase/postgres-meta:v0.83.2
    restart: unless-stopped
    environment:
      - PG_META_PORT=8080
      - PG_META_DB_HOST=supabase-db
      - PG_META_DB_PORT=5432
      - PG_META_DB_NAME=supabase
      - PG_META_DB_USER=supabase_admin
      - PG_META_DB_PASSWORD=${DB_PASSWORD:-supabase}
    depends_on:
      - supabase-db
    labels:
      - "rivetr.managed=true"

volumes:
  supabase_db_data:
"#,
            r#"[{"name":"VERSION","label":"Studio Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"supabase-studio","secret":false},{"name":"PORT","label":"Studio Port","required":false,"default":"3000","secret":false},{"name":"DB_PORT","label":"Database Port","required":false,"default":"5432","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"ANON_KEY","label":"Anon Key","required":true,"default":"","secret":true},{"name":"SERVICE_KEY","label":"Service Role Key","required":true,"default":"","secret":true}]"#,
        ),
        (
            "appwrite",
            "Appwrite",
            "End-to-end backend server for web, mobile, and Flutter developers. Auth, database, storage, functions.",
            "development",
            "appwrite",
            r#"services:
  appwrite:
    image: appwrite/appwrite:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-appwrite}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
    environment:
      - _APP_ENV=production
      - _APP_OPENSSL_KEY_V1=${OPENSSL_KEY:-your-secret-key}
      - _APP_DOMAIN=${DOMAIN:-localhost}
      - _APP_DOMAIN_TARGET=${DOMAIN:-localhost}
      - _APP_REDIS_HOST=appwrite_redis
      - _APP_REDIS_PORT=6379
      - _APP_DB_HOST=appwrite_db
      - _APP_DB_PORT=3306
      - _APP_DB_SCHEMA=appwrite
      - _APP_DB_USER=appwrite
      - _APP_DB_PASS=appwrite
    volumes:
      - appwrite_uploads:/storage/uploads
      - appwrite_cache:/storage/cache
      - appwrite_config:/storage/config
      - appwrite_certs:/storage/certificates
    depends_on:
      - appwrite_db
      - appwrite_redis
    labels:
      - "rivetr.managed=true"

  appwrite_db:
    image: mariadb:11
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=appwrite
      - MYSQL_DATABASE=appwrite
      - MYSQL_USER=appwrite
      - MYSQL_PASSWORD=appwrite
    volumes:
      - appwrite_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

  appwrite_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  appwrite_uploads:
  appwrite_cache:
  appwrite_config:
  appwrite_certs:
  appwrite_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"appwrite","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false},{"name":"OPENSSL_KEY","label":"OpenSSL Secret Key","required":true,"default":"","secret":true},{"name":"DOMAIN","label":"Domain","required":false,"default":"localhost","secret":false}]"#,
        ),
        (
            "pocketbase",
            "PocketBase",
            "Open-source backend in 1 file. Realtime database, auth, file storage, and admin dashboard.",
            "development",
            "pocketbase",
            r#"services:
  pocketbase:
    image: ghcr.io/muchobien/pocketbase:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-pocketbase}
    restart: unless-stopped
    ports:
      - "${PORT:-8090}:8090"
    volumes:
      - pocketbase_data:/pb/pb_data
      - pocketbase_public:/pb/pb_public
    labels:
      - "rivetr.managed=true"

volumes:
  pocketbase_data:
  pocketbase_public:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"pocketbase","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8090","secret":false}]"#,
        ),
        (
            "hoppscotch",
            "Hoppscotch",
            "Open-source API development ecosystem. Test REST, GraphQL, WebSocket, and more from the browser.",
            "development",
            "hoppscotch",
            r#"services:
  hoppscotch:
    image: hoppscotch/hoppscotch:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-hoppscotch}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgresql://hoppscotch:hoppscotch@hoppscotch_db:5432/hoppscotch
      - JWT_SECRET=${JWT_SECRET:-change-me-jwt-secret}
      - SESSION_SECRET=${SESSION_SECRET:-change-me-session-secret}
      - TOKEN_SALT_COMPLEXITY=10
      - MAGIC_LINK_TOKEN_VALIDITY=3
      - REFRESH_TOKEN_VALIDITY=604800000
      - ACCESS_TOKEN_VALIDITY=86400000
      - VITE_ALLOWED_AUTH_PROVIDERS=EMAIL
    depends_on:
      - hoppscotch_db
    labels:
      - "rivetr.managed=true"

  hoppscotch_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=hoppscotch
      - POSTGRES_USER=hoppscotch
      - POSTGRES_PASSWORD=hoppscotch
    volumes:
      - hoppscotch_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  hoppscotch_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"hoppscotch","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true},{"name":"SESSION_SECRET","label":"Session Secret","required":true,"default":"","secret":true}]"#,
        ),
        (
            "forgejo",
            "Forgejo",
            "Self-hosted Git hosting platform. Community fork of Gitea with enhanced features and governance.",
            "development",
            "forgejo",
            r#"services:
  forgejo:
    image: codeberg.org/forgejo/forgejo:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-forgejo}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-3000}:3000"
      - "${SSH_PORT:-2222}:22"
    environment:
      - USER_UID=1000
      - USER_GID=1000
      - FORGEJO__database__DB_TYPE=sqlite3
      - FORGEJO__server__ROOT_URL=${ROOT_URL:-http://localhost:3000}
    volumes:
      - forgejo_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  forgejo_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"forgejo","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"3000","secret":false},{"name":"SSH_PORT","label":"SSH Port","required":false,"default":"2222","secret":false},{"name":"ROOT_URL","label":"Root URL","required":false,"default":"http://localhost:3000","secret":false}]"#,
        ),
    ]
}
