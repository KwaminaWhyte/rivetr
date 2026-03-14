//! Batch 2 project management and other service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== BATCH 2: PROJECT MANAGEMENT ====================
        (
            "tpl-batch2-plane",
            "Plane",
            "Open-source project tracking tool. A self-hosted alternative to Jira, Linear, and Asana.",
            "project-management",
            "plane",
            r#"services:
  plane:
    image: makeplane/plane-app:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-plane}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgresql://plane:${DB_PASSWORD:-plane}@plane_db:5432/plane
      - REDIS_URL=redis://plane_redis:6379
      - SECRET_KEY=${SECRET_KEY:-change-me-to-a-long-random-string}
    depends_on:
      - plane_db
      - plane_redis
    labels:
      - "rivetr.managed=true"

  plane_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=plane
      - POSTGRES_PASSWORD=${DB_PASSWORD:-plane}
      - POSTGRES_DB=plane
    volumes:
      - plane_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  plane_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  plane_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"plane","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-batch2-vikunja",
            "Vikunja",
            "Open-source to-do and Kanban app. Self-hosted alternative to Todoist and Trello.",
            "project-management",
            "vikunja",
            r#"services:
  vikunja:
    image: vikunja/vikunja:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-vikunja}
    restart: unless-stopped
    ports:
      - "${PORT:-3456}:3456"
    environment:
      - VIKUNJA_SERVICE_JWTSECRET=${JWT_SECRET:-change-me-to-a-random-string}
      - VIKUNJA_SERVICE_FRONTENDURL=${FRONTEND_URL:-http://localhost:3456}
    volumes:
      - vikunja_data:/app/vikunja/files
    labels:
      - "rivetr.managed=true"

volumes:
  vikunja_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"vikunja","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3456","secret":false},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true},{"name":"FRONTEND_URL","label":"Frontend URL","required":false,"default":"http://localhost:3456","secret":false}]"#,
        ),
        (
            "tpl-batch2-leantime",
            "Leantime",
            "Open-source project management system designed for non-project managers.",
            "project-management",
            "leantime",
            r#"services:
  leantime:
    image: leantime/leantime:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-leantime}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - LEAN_DB_HOST=leantime_db
      - LEAN_DB_USER=leantime
      - LEAN_DB_PASSWORD=${DB_PASSWORD:-leantime}
      - LEAN_DB_DATABASE=leantime
      - LEAN_SITENAME=${SITE_NAME:-Leantime}
    depends_on:
      - leantime_db
    labels:
      - "rivetr.managed=true"

  leantime_db:
    image: mysql:8.0
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=leantime
      - MYSQL_USER=leantime
      - MYSQL_PASSWORD=${DB_PASSWORD:-leantime}
      - MYSQL_RANDOM_ROOT_PASSWORD=yes
    volumes:
      - leantime_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  leantime_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"leantime","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"SITE_NAME","label":"Site Name","required":false,"default":"Leantime","secret":false}]"#,
        ),
        (
            "tpl-batch2-calcom",
            "Cal.com",
            "Open-source scheduling platform. Self-hosted alternative to Calendly.",
            "project-management",
            "calcom",
            r#"services:
  calcom:
    image: calcom/cal.com:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-calcom}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgresql://calcom:${DB_PASSWORD:-calcom}@calcom_db:5432/calcom
      - NEXTAUTH_SECRET=${NEXTAUTH_SECRET:-change-me-to-a-random-string}
      - NEXTAUTH_URL=${NEXTAUTH_URL:-http://localhost:3000}
      - CALENDSO_ENCRYPTION_KEY=${ENCRYPTION_KEY:-change-me-to-a-32-char-key}
    depends_on:
      - calcom_db
    labels:
      - "rivetr.managed=true"

  calcom_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=calcom
      - POSTGRES_PASSWORD=${DB_PASSWORD:-calcom}
      - POSTGRES_DB=calcom
    volumes:
      - calcom_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  calcom_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"calcom","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"NEXTAUTH_SECRET","label":"NextAuth Secret","required":true,"default":"","secret":true},{"name":"NEXTAUTH_URL","label":"NextAuth URL","required":true,"default":"http://localhost:3000","secret":false},{"name":"ENCRYPTION_KEY","label":"Encryption Key","required":true,"default":"","secret":true}]"#,
        ),

        // ==================== BATCH 2: OTHER ====================
        (
            "tpl-batch2-trilium",
            "Trilium",
            "Hierarchical note-taking application with focus on building personal knowledge bases.",
            "other",
            "trilium",
            r#"services:
  trilium:
    image: zadam/trilium:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-trilium}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    volumes:
      - trilium_data:/home/node/trilium-data
    labels:
      - "rivetr.managed=true"

volumes:
  trilium_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"trilium","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false}]"#,
        ),
        (
            "tpl-batch2-linkwarden",
            "Linkwarden",
            "Self-hosted collaborative bookmark manager to collect, organize, and preserve web content.",
            "other",
            "linkwarden",
            r#"services:
  linkwarden:
    image: ghcr.io/linkwarden/linkwarden:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-linkwarden}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgresql://linkwarden:${DB_PASSWORD:-linkwarden}@linkwarden_db:5432/linkwarden
      - NEXTAUTH_SECRET=${NEXTAUTH_SECRET:-change-me-to-a-random-string}
      - NEXTAUTH_URL=${NEXTAUTH_URL:-http://localhost:3000}
    depends_on:
      - linkwarden_db
    labels:
      - "rivetr.managed=true"

  linkwarden_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=linkwarden
      - POSTGRES_PASSWORD=${DB_PASSWORD:-linkwarden}
      - POSTGRES_DB=linkwarden
    volumes:
      - linkwarden_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  linkwarden_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"linkwarden","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"NEXTAUTH_SECRET","label":"NextAuth Secret","required":true,"default":"","secret":true},{"name":"NEXTAUTH_URL","label":"NextAuth URL","required":false,"default":"http://localhost:3000","secret":false}]"#,
        ),
        (
            "tpl-batch2-tandoor",
            "Tandoor Recipes",
            "Application for managing recipes, meal planning, and shopping lists.",
            "other",
            "tandoor",
            r#"services:
  tandoor:
    image: ghcr.io/tandoorrecipes/recipes:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-tandoor}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DB_ENGINE=django.db.backends.postgresql
      - POSTGRES_HOST=tandoor_db
      - POSTGRES_PORT=5432
      - POSTGRES_USER=tandoor
      - POSTGRES_PASSWORD=${DB_PASSWORD:-tandoor}
      - POSTGRES_DB=tandoor
      - SECRET_KEY=${SECRET_KEY:-change-me-to-a-long-random-string}
    volumes:
      - tandoor_static:/opt/recipes/staticfiles
      - tandoor_media:/opt/recipes/mediafiles
    depends_on:
      - tandoor_db
    labels:
      - "rivetr.managed=true"

  tandoor_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=tandoor
      - POSTGRES_PASSWORD=${DB_PASSWORD:-tandoor}
      - POSTGRES_DB=tandoor
    volumes:
      - tandoor_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  tandoor_static:
  tandoor_media:
  tandoor_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"tandoor","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-batch2-stirling-pdf",
            "Stirling-PDF",
            "Self-hosted web-based PDF manipulation tool. Merge, split, convert, and edit PDF files.",
            "other",
            "stirling-pdf",
            r#"services:
  stirling-pdf:
    image: frooodle/s-pdf:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-stirling-pdf}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DOCKER_ENABLE_SECURITY=${ENABLE_SECURITY:-false}
    volumes:
      - stirling_data:/usr/share/tessdata
      - stirling_config:/configs
    labels:
      - "rivetr.managed=true"

volumes:
  stirling_data:
  stirling_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"stirling-pdf","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"ENABLE_SECURITY","label":"Enable Security","required":false,"default":"false","secret":false}]"#,
        ),
    ]
}
