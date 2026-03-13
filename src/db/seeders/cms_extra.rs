//! Additional CMS and headless content platform templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== CMS (additional) ====================
        (
            "tpl-keystonejs",
            "KeystoneJS",
            "Next-gen Node.js headless CMS and application framework. Schema-driven with GraphQL and rich admin UI.",
            "cms",
            "keystonejs",
            r#"services:
  keystone:
    image: keystonejs/keystone:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-keystone}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DATABASE_URL=postgresql://keystone:${DB_PASSWORD:-keystone}@keystone_db:5432/keystone
      - SESSION_SECRET=${SESSION_SECRET:-change-me-to-a-random-string}
    depends_on:
      - keystone_db
    labels:
      - "rivetr.managed=true"

  keystone_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=keystone
      - POSTGRES_PASSWORD=${DB_PASSWORD:-keystone}
      - POSTGRES_DB=keystone
    volumes:
      - keystone_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  keystone_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"keystone","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"SESSION_SECRET","label":"Session Secret","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-cockpit-cms",
            "Cockpit CMS",
            "Headless content platform with flexible content API, collection management, and media handling.",
            "cms",
            "cockpit",
            r#"services:
  cockpit:
    image: agentejo/cockpit:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-cockpit}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - APP_NAME=${APP_NAME:-Cockpit}
    volumes:
      - cockpit_data:/var/www/html/storage
    labels:
      - "rivetr.managed=true"

volumes:
  cockpit_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"cockpit","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"APP_NAME","label":"App Name","required":false,"default":"Cockpit","secret":false}]"#,
        ),
        (
            "tpl-pimcore",
            "Pimcore",
            "Open-source digital experience platform. Data management, PIM, DAM, CMS, and e-commerce.",
            "cms",
            "pimcore",
            r#"services:
  pimcore:
    image: pimcore/pimcore:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-pimcore}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:80"
    environment:
      - DATABASE_HOST=pimcore_db
      - DATABASE_USER=pimcore
      - DATABASE_PASSWORD=${DB_PASSWORD:-pimcore}
      - DATABASE_NAME=pimcore
      - PIMCORE_ENVIRONMENT=production
    depends_on:
      - pimcore_db
    volumes:
      - pimcore_var:/var/www/html/var
      - pimcore_public:/var/www/html/public
    labels:
      - "rivetr.managed=true"

  pimcore_db:
    image: mariadb:11
    restart: unless-stopped
    environment:
      - MYSQL_DATABASE=pimcore
      - MYSQL_USER=pimcore
      - MYSQL_PASSWORD=${DB_PASSWORD:-pimcore}
      - MYSQL_RANDOM_ROOT_PASSWORD=yes
    volumes:
      - pimcore_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  pimcore_var:
  pimcore_public:
  pimcore_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"pimcore","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-wagtail",
            "Wagtail",
            "Open-source Django CMS with a beautiful editor. Flexible, scalable, and easy to integrate.",
            "cms",
            "wagtail",
            r#"services:
  wagtail:
    image: wagtail/wagtail:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-wagtail}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - DATABASE_URL=postgresql://wagtail:${DB_PASSWORD:-wagtail}@wagtail_db:5432/wagtail
      - SECRET_KEY=${SECRET_KEY:-change-me-to-a-random-string}
      - DJANGO_SUPERUSER_USERNAME=${ADMIN_USER:-admin}
      - DJANGO_SUPERUSER_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - DJANGO_SUPERUSER_PASSWORD=${ADMIN_PASSWORD:-changeme}
    depends_on:
      - wagtail_db
    volumes:
      - wagtail_media:/app/media
      - wagtail_static:/app/static
    labels:
      - "rivetr.managed=true"

  wagtail_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=wagtail
      - POSTGRES_PASSWORD=${DB_PASSWORD:-wagtail}
      - POSTGRES_DB=wagtail
    volumes:
      - wagtail_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  wagtail_media:
  wagtail_static:
  wagtail_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"wagtail","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_EMAIL","label":"Admin Email","required":false,"default":"admin@example.com","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-dotcms",
            "dotCMS",
            "Enterprise-grade cloud-native CMS. Headless, multilingual, and multi-site content management.",
            "cms",
            "dotcms",
            r#"services:
  dotcms:
    image: dotcms/dotcms:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-dotcms}
    restart: unless-stopped
    ports:
      - "${PORT:-8082}:8082"
    environment:
      - DB_BASE_URL=jdbc:postgresql://dotcms_db/dotcms
      - DB_USERNAME=dotcms
      - DB_PASSWORD=${DB_PASSWORD:-dotcms}
      - DOT_ES_ENDPOINTS=http://dotcms_es:9200
    depends_on:
      - dotcms_db
    volumes:
      - dotcms_data:/srv
    labels:
      - "rivetr.managed=true"

  dotcms_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=dotcms
      - POSTGRES_PASSWORD=${DB_PASSWORD:-dotcms}
      - POSTGRES_DB=dotcms
    volumes:
      - dotcms_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  dotcms_data:
  dotcms_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"dotcms","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8082","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
    ]
}
