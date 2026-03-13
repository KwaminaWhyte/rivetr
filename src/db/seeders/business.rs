//! Business productivity, CRM, finance, and e-commerce service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== BUSINESS / PRODUCTIVITY ====================
        (
            "tpl-monica",
            "Monica",
            "Open-source personal CRM. Organize and log activities with friends, family, and contacts.",
            "project-management",
            "monica",
            r#"services:
  monica:
    image: monica:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-monica}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - APP_KEY=${APP_KEY:-change-me-to-a-32-char-key}
      - DB_HOST=monica_db
      - DB_USERNAME=monica
      - DB_PASSWORD=${DB_PASSWORD:-monica}
      - DB_DATABASE=monica
    depends_on:
      - monica_db
    volumes:
      - monica_storage:/var/www/html/storage
    labels:
      - "rivetr.managed=true"

  monica_db:
    image: mysql:8.0
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=monica
      - MYSQL_USER=monica
      - MYSQL_PASSWORD=${DB_PASSWORD:-monica}
      - MYSQL_RANDOM_ROOT_PASSWORD=yes
    volumes:
      - monica_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  monica_storage:
  monica_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"monica","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"APP_KEY","label":"App Key (32 chars)","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-invoice-ninja",
            "Invoice Ninja",
            "Open-source invoicing, billing, and payment platform for freelancers and businesses.",
            "project-management",
            "invoice-ninja",
            r#"services:
  invoiceninja:
    image: invoiceninja/invoiceninja:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-invoiceninja}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - APP_KEY=${APP_KEY:-change-me-to-a-32-char-key}
      - APP_URL=${APP_URL:-http://localhost:8080}
      - DB_HOST=invoiceninja_db
      - DB_USERNAME=ninja
      - DB_PASSWORD=${DB_PASSWORD:-ninja}
      - DB_DATABASE=ninja
    depends_on:
      - invoiceninja_db
    volumes:
      - invoiceninja_public:/var/www/app/public
      - invoiceninja_storage:/var/www/app/storage
    labels:
      - "rivetr.managed=true"

  invoiceninja_db:
    image: mysql:8.0
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=ninja
      - MYSQL_USER=ninja
      - MYSQL_PASSWORD=${DB_PASSWORD:-ninja}
      - MYSQL_RANDOM_ROOT_PASSWORD=yes
    volumes:
      - invoiceninja_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  invoiceninja_public:
  invoiceninja_storage:
  invoiceninja_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"invoiceninja","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"APP_KEY","label":"App Key (32 chars)","required":true,"default":"","secret":true},{"name":"APP_URL","label":"App URL","required":false,"default":"http://localhost:8080","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-kimai",
            "Kimai",
            "Open-source time-tracking application for freelancers and teams. Multi-user, reporting, invoicing.",
            "project-management",
            "kimai",
            r#"services:
  kimai:
    image: kimai/kimai2:${VERSION:-apache}
    container_name: ${CONTAINER_NAME:-kimai}
    restart: unless-stopped
    ports:
      - "${PORT:-8001}:8001"
    environment:
      - DATABASE_URL=mysql://kimai:${DB_PASSWORD:-kimai}@kimai_db/kimai
      - APP_SECRET=${APP_SECRET:-change-me-to-a-random-string}
      - TRUSTED_HOSTS=nginx,localhost,127.0.0.1
    depends_on:
      - kimai_db
    labels:
      - "rivetr.managed=true"

  kimai_db:
    image: mysql:8.0
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=kimai
      - MYSQL_USER=kimai
      - MYSQL_PASSWORD=${DB_PASSWORD:-kimai}
      - MYSQL_RANDOM_ROOT_PASSWORD=yes
    volumes:
      - kimai_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  kimai_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"apache","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"kimai","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8001","secret":false},{"name":"APP_SECRET","label":"App Secret","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-focalboard",
            "Focalboard",
            "Open-source project management tool. Kanban, table, and gallery views. Alternative to Trello and Notion.",
            "project-management",
            "focalboard",
            r#"services:
  focalboard:
    image: mattermost/focalboard:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-focalboard}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    volumes:
      - focalboard_data:/opt/focalboard/data
    labels:
      - "rivetr.managed=true"

volumes:
  focalboard_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"focalboard","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false}]"#,
        ),
        (
            "tpl-firefly-iii",
            "Firefly III",
            "Free, open-source personal finance manager. Track expenses, budgets, and savings goals.",
            "analytics",
            "firefly-iii",
            r#"services:
  firefly:
    image: fireflyiii/core:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-firefly}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - APP_KEY=${APP_KEY:-change-me-to-a-32-char-key}
      - DB_CONNECTION=pgsql
      - DB_HOST=firefly_db
      - DB_PORT=5432
      - DB_DATABASE=firefly
      - DB_USERNAME=firefly
      - DB_PASSWORD=${DB_PASSWORD:-firefly}
      - TRUSTED_PROXIES=**
      - APP_URL=${APP_URL:-http://localhost:8080}
    depends_on:
      - firefly_db
    volumes:
      - firefly_upload:/var/www/html/storage/upload
    labels:
      - "rivetr.managed=true"

  firefly_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_DB=firefly
      - POSTGRES_USER=firefly
      - POSTGRES_PASSWORD=${DB_PASSWORD:-firefly}
    volumes:
      - firefly_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  firefly_upload:
  firefly_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"firefly","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"APP_KEY","label":"App Key (32 chars)","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"APP_URL","label":"App URL","required":false,"default":"http://localhost:8080","secret":false}]"#,
        ),
        (
            "tpl-actual-budget",
            "Actual Budget",
            "Local-first personal finance app with real budgeting. Privacy-focused, works offline.",
            "analytics",
            "actual-budget",
            r#"services:
  actual:
    image: actualbudget/actual-server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-actual}
    restart: unless-stopped
    ports:
      - "${PORT:-5006}:5006"
    volumes:
      - actual_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  actual_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"actual","secret":false},{"name":"PORT","label":"Port","required":false,"default":"5006","secret":false}]"#,
        ),
        (
            "tpl-medusa",
            "Medusa",
            "Open-source composable commerce platform. Self-hosted Shopify alternative with modern architecture.",
            "development",
            "medusa",
            r#"services:
  medusa:
    image: medusajs/medusa:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-medusa}
    restart: unless-stopped
    ports:
      - "${PORT:-9000}:9000"
    environment:
      - DATABASE_URL=postgresql://medusa:${DB_PASSWORD:-medusa}@medusa_db:5432/medusa
      - REDIS_URL=redis://medusa_redis:6379
      - JWT_SECRET=${JWT_SECRET:-change-me-to-a-random-string}
      - COOKIE_SECRET=${COOKIE_SECRET:-change-me-to-a-random-string}
    depends_on:
      - medusa_db
      - medusa_redis
    labels:
      - "rivetr.managed=true"

  medusa_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=medusa
      - POSTGRES_PASSWORD=${DB_PASSWORD:-medusa}
      - POSTGRES_DB=medusa
    volumes:
      - medusa_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  medusa_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  medusa_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"medusa","secret":false},{"name":"PORT","label":"Port","required":false,"default":"9000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true},{"name":"COOKIE_SECRET","label":"Cookie Secret","required":true,"default":"","secret":true}]"#,
        ),
    ]
}
