//! Additional communication service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== COMMUNICATION (additional) ====================
        (
            "tpl-zulip",
            "Zulip",
            "Powerful open-source group chat. Topic-based threading keeps conversations organized.",
            "communication",
            "zulip",
            r#"services:
  zulip:
    image: zulip/docker-zulip:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-zulip}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
    environment:
      - ZULIP_AUTH_BACKENDS=${AUTH_BACKENDS:-EmailAuthBackend}
      - ZULIP_ADMINISTRATOR=${ADMIN_EMAIL:-admin@example.com}
      - SETTING_EXTERNAL_HOST=${EXTERNAL_HOST:-localhost}
      - POSTGRES_PASSWORD=${DB_PASSWORD:-zulip}
      - RABBITMQ_DEFAULT_PASS=${RABBITMQ_PASSWORD:-zulip}
      - REDIS_PASSWORD=${REDIS_PASSWORD:-zulip}
      - MEMCACHED_PASSWORD=${MEMCACHED_PASSWORD:-zulip}
      - SSL_CERTIFICATE_GENERATION=self-signed
    volumes:
      - zulip_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  zulip_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"zulip","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"EXTERNAL_HOST","label":"External Hostname","required":true,"default":"localhost","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"RABBITMQ_PASSWORD","label":"RabbitMQ Password","required":true,"default":"","secret":true},{"name":"REDIS_PASSWORD","label":"Redis Password","required":true,"default":"","secret":true},{"name":"AUTH_BACKENDS","label":"Auth Backends","required":false,"default":"EmailAuthBackend","secret":false}]"#,
        ),
        (
            "tpl-chatwoot",
            "Chatwoot",
            "Open-source customer engagement platform. Live chat, email, social, and WhatsApp in one inbox.",
            "communication",
            "chatwoot",
            r#"services:
  chatwoot:
    image: chatwoot/chatwoot:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-chatwoot}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - SECRET_KEY_BASE=${SECRET_KEY_BASE:-change-me-to-a-long-random-string}
      - FRONTEND_URL=${FRONTEND_URL:-http://localhost:3000}
      - RAILS_ENV=production
      - RAILS_LOG_TO_STDOUT=true
      - POSTGRES_HOST=chatwoot_db
      - POSTGRES_USERNAME=chatwoot
      - POSTGRES_PASSWORD=${DB_PASSWORD:-chatwoot}
      - POSTGRES_DATABASE=chatwoot
      - REDIS_URL=redis://chatwoot_redis:6379
    command: bundle exec rails s -p 3000 -b 0.0.0.0
    depends_on:
      - chatwoot_db
      - chatwoot_redis
    labels:
      - "rivetr.managed=true"

  chatwoot_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=chatwoot
      - POSTGRES_PASSWORD=${DB_PASSWORD:-chatwoot}
      - POSTGRES_DB=chatwoot
    volumes:
      - chatwoot_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  chatwoot_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  chatwoot_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"chatwoot","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"SECRET_KEY_BASE","label":"Secret Key Base","required":true,"default":"","secret":true},{"name":"FRONTEND_URL","label":"Frontend URL","required":true,"default":"http://localhost:3000","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-element-web",
            "Element Web",
            "Feature-rich Matrix web client. Connect to any Matrix homeserver for decentralized messaging.",
            "communication",
            "element-web",
            r#"services:
  element-web:
    image: vectorim/element-web:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-element-web}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    volumes:
      - element_config:/app/config
    labels:
      - "rivetr.managed=true"

volumes:
  element_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"element-web","secret":false},{"name":"PORT","label":"Port","required":false,"default":"80","secret":false}]"#,
        ),
        (
            "tpl-discourse",
            "Discourse",
            "Modern community discussion platform. Forum, mailing list, and long-form chat in one.",
            "communication",
            "discourse",
            r#"services:
  discourse:
    image: bitnami/discourse:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-discourse}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - DISCOURSE_HOST=${DISCOURSE_HOST:-localhost}
      - DISCOURSE_USERNAME=${ADMIN_USERNAME:-admin}
      - DISCOURSE_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - DISCOURSE_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - DISCOURSE_DATABASE_HOST=discourse_db
      - DISCOURSE_DATABASE_USER=discourse
      - DISCOURSE_DATABASE_PASSWORD=${DB_PASSWORD:-discourse}
      - DISCOURSE_DATABASE_NAME=discourse
      - DISCOURSE_REDIS_HOST=discourse_redis
      - POSTGRESQL_PASSWORD=${DB_PASSWORD:-discourse}
    depends_on:
      - discourse_db
      - discourse_redis
    labels:
      - "rivetr.managed=true"

  discourse_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=discourse
      - POSTGRES_PASSWORD=${DB_PASSWORD:-discourse}
      - POSTGRES_DB=discourse
    volumes:
      - discourse_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  discourse_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  discourse_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"discourse","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"DISCOURSE_HOST","label":"Discourse Hostname","required":true,"default":"localhost","secret":false},{"name":"ADMIN_USERNAME","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-lemmy",
            "Lemmy",
            "Link aggregator and discussion platform. Federated Reddit alternative for the Fediverse.",
            "communication",
            "lemmy",
            r#"services:
  lemmy:
    image: dessalines/lemmy:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-lemmy}
    restart: unless-stopped
    ports:
      - "${PORT:-8536}:8536"
    environment:
      - RUST_LOG=info
      - LEMMY_DATABASE_URL=postgresql://lemmy:${DB_PASSWORD:-lemmy}@lemmy_db:5432/lemmy
    depends_on:
      - lemmy_db
    volumes:
      - lemmy_config:/config
    labels:
      - "rivetr.managed=true"

  lemmy-ui:
    image: dessalines/lemmy-ui:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-lemmy-ui}
    restart: unless-stopped
    ports:
      - "${UI_PORT:-1234}:1234"
    environment:
      - LEMMY_UI_LEMMY_INTERNAL_HOST=lemmy:8536
      - LEMMY_UI_LEMMY_EXTERNAL_HOST=${EXTERNAL_HOST:-localhost:8536}
    depends_on:
      - lemmy
    labels:
      - "rivetr.managed=true"

  lemmy_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=lemmy
      - POSTGRES_PASSWORD=${DB_PASSWORD:-lemmy}
      - POSTGRES_DB=lemmy
    volumes:
      - lemmy_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  lemmy_config:
  lemmy_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"lemmy","secret":false},{"name":"PORT","label":"API Port","required":false,"default":"8536","secret":false},{"name":"UI_PORT","label":"UI Port","required":false,"default":"1234","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"EXTERNAL_HOST","label":"External Host","required":false,"default":"localhost:8536","secret":false}]"#,
        ),
        (
            "tpl-mastodon",
            "Mastodon",
            "Open-source decentralized social network. Build your own corner of the Fediverse.",
            "communication",
            "mastodon",
            r#"services:
  mastodon:
    image: tootsuite/mastodon:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-mastodon}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
      - "${STREAMING_PORT:-4000}:4000"
    environment:
      - LOCAL_DOMAIN=${LOCAL_DOMAIN:-localhost}
      - SECRET_KEY_BASE=${SECRET_KEY_BASE:-change-me-to-a-long-random-string}
      - OTP_SECRET=${OTP_SECRET:-change-me-to-another-long-random-string}
      - VAPID_PRIVATE_KEY=${VAPID_PRIVATE_KEY:-change-me}
      - VAPID_PUBLIC_KEY=${VAPID_PUBLIC_KEY:-change-me}
      - DB_HOST=mastodon_db
      - DB_USER=mastodon
      - DB_NAME=mastodon_production
      - DB_PASS=${DB_PASSWORD:-mastodon}
      - REDIS_HOST=mastodon_redis
      - RAILS_ENV=production
      - NODE_ENV=production
      - SMTP_SERVER=${SMTP_SERVER:-smtp.example.com}
      - SMTP_PORT=${SMTP_PORT:-587}
      - SMTP_FROM_ADDRESS=${SMTP_FROM:-notifications@example.com}
    command: bash -c "rm -f /mastodon/tmp/pids/server.pid; bundle exec rails s -p 3000"
    depends_on:
      - mastodon_db
      - mastodon_redis
    volumes:
      - mastodon_public:/mastodon/public/system
    labels:
      - "rivetr.managed=true"

  mastodon_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=mastodon
      - POSTGRES_PASSWORD=${DB_PASSWORD:-mastodon}
      - POSTGRES_DB=mastodon_production
    volumes:
      - mastodon_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  mastodon_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  mastodon_public:
  mastodon_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mastodon","secret":false},{"name":"PORT","label":"Web Port","required":false,"default":"3000","secret":false},{"name":"STREAMING_PORT","label":"Streaming Port","required":false,"default":"4000","secret":false},{"name":"LOCAL_DOMAIN","label":"Local Domain","required":true,"default":"localhost","secret":false},{"name":"SECRET_KEY_BASE","label":"Secret Key Base","required":true,"default":"","secret":true},{"name":"OTP_SECRET","label":"OTP Secret","required":true,"default":"","secret":true},{"name":"VAPID_PRIVATE_KEY","label":"VAPID Private Key","required":true,"default":"","secret":true},{"name":"VAPID_PUBLIC_KEY","label":"VAPID Public Key","required":true,"default":"","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"SMTP_SERVER","label":"SMTP Server","required":false,"default":"smtp.example.com","secret":false},{"name":"SMTP_PORT","label":"SMTP Port","required":false,"default":"587","secret":false},{"name":"SMTP_FROM","label":"SMTP From Address","required":false,"default":"notifications@example.com","secret":false}]"#,
        ),
    ]
}
