//! Sprint 15 service templates: communication, BaaS, AI, monitoring, and more

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== COMMUNICATION ====================
        (
            "tpl-jitsi-meet",
            "Jitsi Meet",
            "Fully encrypted, open-source video conferencing. No account required. Host meetings on your own server.",
            "communication",
            "jitsi",
            r#"services:
  jitsi-web:
    image: jitsi/web:${VERSION:-stable-9646}
    container_name: ${CONTAINER_NAME:-jitsi-web}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-8000}:80"
      - "${HTTPS_PORT:-8443}:443"
    environment:
      - PUBLIC_URL=${PUBLIC_URL:-http://localhost:8000}
      - XMPP_SERVER=jitsi-prosody
      - XMPP_DOMAIN=meet.jitsi
      - XMPP_AUTH_DOMAIN=auth.meet.jitsi
      - XMPP_INTERNAL_MUC_DOMAIN=internal-muc.meet.jitsi
      - XMPP_MUC_DOMAIN=muc.meet.jitsi
      - JICOFO_AUTH_USER=focus
      - ENABLE_LETSENCRYPT=0
    volumes:
      - jitsi_web_config:/config
      - jitsi_web_letsencrypt:/etc/letsencrypt
    depends_on:
      - jitsi-prosody
    labels:
      - "rivetr.managed=true"

  jitsi-prosody:
    image: jitsi/prosody:${VERSION:-stable-9646}
    restart: unless-stopped
    expose:
      - "5222"
      - "5347"
      - "5280"
    environment:
      - XMPP_DOMAIN=meet.jitsi
      - XMPP_AUTH_DOMAIN=auth.meet.jitsi
      - XMPP_MUC_DOMAIN=muc.meet.jitsi
      - XMPP_INTERNAL_MUC_DOMAIN=internal-muc.meet.jitsi
      - XMPP_RECORDER_DOMAIN=recorder.meet.jitsi
      - JICOFO_COMPONENT_SECRET=${JICOFO_COMPONENT_SECRET:-s3cr37}
      - JICOFO_AUTH_USER=focus
      - JICOFO_AUTH_PASSWORD=${JICOFO_AUTH_PASSWORD:-passw0rd}
      - JVB_AUTH_USER=jvb
      - JVB_AUTH_PASSWORD=${JVB_AUTH_PASSWORD:-passw0rd}
      - JIBRI_XMPP_USER=jibri
      - JIBRI_XMPP_PASSWORD=${JIBRI_XMPP_PASSWORD:-passw0rd}
      - JIBRI_RECORDER_USER=recorder
      - JIBRI_RECORDER_PASSWORD=${JIBRI_RECORDER_PASSWORD:-passw0rd}
      - TZ=UTC
    volumes:
      - jitsi_prosody_config:/config
      - jitsi_prosody_plugins:/prosody-plugins-custom
    labels:
      - "rivetr.managed=true"

  jitsi-jicofo:
    image: jitsi/jicofo:${VERSION:-stable-9646}
    restart: unless-stopped
    environment:
      - XMPP_SERVER=jitsi-prosody
      - XMPP_DOMAIN=meet.jitsi
      - XMPP_AUTH_DOMAIN=auth.meet.jitsi
      - XMPP_INTERNAL_MUC_DOMAIN=internal-muc.meet.jitsi
      - JICOFO_COMPONENT_SECRET=${JICOFO_COMPONENT_SECRET:-s3cr37}
      - JICOFO_AUTH_USER=focus
      - JICOFO_AUTH_PASSWORD=${JICOFO_AUTH_PASSWORD:-passw0rd}
      - TZ=UTC
    depends_on:
      - jitsi-prosody
    labels:
      - "rivetr.managed=true"

  jitsi-jvb:
    image: jitsi/jvb:${VERSION:-stable-9646}
    restart: unless-stopped
    ports:
      - "${JVB_PORT:-10000}:10000/udp"
    environment:
      - XMPP_SERVER=jitsi-prosody
      - XMPP_DOMAIN=meet.jitsi
      - XMPP_AUTH_DOMAIN=auth.meet.jitsi
      - XMPP_INTERNAL_MUC_DOMAIN=internal-muc.meet.jitsi
      - JVB_AUTH_USER=jvb
      - JVB_AUTH_PASSWORD=${JVB_AUTH_PASSWORD:-passw0rd}
      - JVB_ADVERTISE_IPS=${PUBLIC_IP:-127.0.0.1}
      - TZ=UTC
    depends_on:
      - jitsi-prosody
    volumes:
      - jitsi_jvb_config:/config
    labels:
      - "rivetr.managed=true"

volumes:
  jitsi_web_config:
  jitsi_web_letsencrypt:
  jitsi_prosody_config:
  jitsi_prosody_plugins:
  jitsi_jvb_config:
"#,
            r#"[{"name":"VERSION","label":"Version Tag","required":false,"default":"stable-9646","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"jitsi-web","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"8000","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"8443","secret":false},{"name":"PUBLIC_URL","label":"Public URL","required":true,"default":"http://localhost:8000","secret":false},{"name":"PUBLIC_IP","label":"Public IP (for JVB)","required":true,"default":"127.0.0.1","secret":false},{"name":"JVB_PORT","label":"JVB UDP Port","required":false,"default":"10000","secret":false},{"name":"JICOFO_COMPONENT_SECRET","label":"Jicofo Component Secret","required":true,"default":"","secret":true},{"name":"JICOFO_AUTH_PASSWORD","label":"Jicofo Auth Password","required":true,"default":"","secret":true},{"name":"JVB_AUTH_PASSWORD","label":"JVB Auth Password","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== BAAS / BACKEND-AS-A-SERVICE ====================
        // ==================== CMS / HEADLESS ====================
        // ==================== DEVTOOLS / CI-CD ====================
        // ==================== AI / ML ====================
        // ==================== MONITORING ====================
        (
            "tpl-checkmk",
            "Checkmk",
            "Comprehensive IT monitoring for servers, networks, cloud, containers, and applications.",
            "monitoring",
            "checkmk",
            r#"services:
  checkmk:
    image: checkmk/check-mk-raw:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-checkmk}
    restart: unless-stopped
    ports:
      - "${PORT:-5000}:5000"
      - "${AGENT_PORT:-8000}:8000"
    environment:
      - CMK_PASSWORD=${CMK_PASSWORD:-changeme}
      - CMK_SITE_ID=${SITE_ID:-cmk}
    volumes:
      - checkmk_data:/omd/sites
    tmpfs:
      - /opt/omd/sites/cmk/tmp:uid=1000,gid=1000
    labels:
      - "rivetr.managed=true"

volumes:
  checkmk_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"checkmk","secret":false},{"name":"PORT","label":"Web UI Port","required":false,"default":"5000","secret":false},{"name":"AGENT_PORT","label":"Agent Port","required":false,"default":"8000","secret":false},{"name":"CMK_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"SITE_ID","label":"Site ID","required":false,"default":"cmk","secret":false}]"#,
        ),
        // ==================== DATABASES ====================
        (
            "tpl-mariadb",
            "MariaDB",
            "Community-developed, commercially supported fork of MySQL. Drop-in replacement with extra features.",
            "databases",
            "database",
            r#"services:
  mariadb:
    image: mariadb:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-mariadb}
    restart: unless-stopped
    ports:
      - "${PORT:-3306}:3306"
    environment:
      - MARIADB_ROOT_PASSWORD=${ROOT_PASSWORD:-changeme}
      - MARIADB_DATABASE=${DATABASE:-app}
      - MARIADB_USER=${DB_USER:-app}
      - MARIADB_PASSWORD=${DB_PASSWORD:-app}
    volumes:
      - mariadb_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  mariadb_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mariadb","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3306","secret":false},{"name":"ROOT_PASSWORD","label":"Root Password","required":true,"default":"","secret":true},{"name":"DATABASE","label":"Database Name","required":false,"default":"app","secret":false},{"name":"DB_USER","label":"Database User","required":false,"default":"app","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== SECURITY ====================
        (
            "tpl-crowdsec",
            "CrowdSec",
            "Open-source security engine that analyzes logs and blocks attacks. Crowdsourced threat intelligence.",
            "security",
            "crowdsec",
            r#"services:
  crowdsec:
    image: crowdsecurity/crowdsec:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-crowdsec}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
      - "${METRICS_PORT:-6060}:6060"
    environment:
      - COLLECTIONS=crowdsecurity/linux crowdsecurity/sshd
      - BOUNCER_KEY_FIREWALL=${BOUNCER_KEY:-change-me-to-a-random-key}
    volumes:
      - crowdsec_data:/var/lib/crowdsec/data
      - crowdsec_config:/etc/crowdsec
      - /var/log:/var/log:ro
    labels:
      - "rivetr.managed=true"

volumes:
  crowdsec_data:
  crowdsec_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"crowdsec","secret":false},{"name":"PORT","label":"API Port","required":false,"default":"8080","secret":false},{"name":"METRICS_PORT","label":"Metrics Port","required":false,"default":"6060","secret":false},{"name":"BOUNCER_KEY","label":"Firewall Bouncer Key","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-harbor",
            "Harbor",
            "Cloud-native container registry with vulnerability scanning, RBAC, replication, and audit logging.",
            "devtools",
            "harbor",
            r#"services:
  harbor-core:
    image: goharbor/harbor-core:${VERSION:-v2.10.0}
    container_name: ${CONTAINER_NAME:-harbor-core}
    restart: unless-stopped
    environment:
      - CONFIG_PATH=/etc/core/app.conf
      - CORE_SECRET=${CORE_SECRET:-change-me-secret}
      - JOBSERVICE_SECRET=${JOBSERVICE_SECRET:-change-me-jobservice}
      - DATABASE_TYPE=postgresql
      - POSTGRESQL_HOST=harbor_db
      - POSTGRESQL_PORT=5432
      - POSTGRESQL_DATABASE=registry
      - POSTGRESQL_USERNAME=harbor
      - POSTGRESQL_PASSWORD=${DB_PASSWORD:-harbor}
      - REGISTRY_URL=http://harbor-registry:5000
      - TOKEN_SERVICE_URL=http://harbor-core:8080/service/token
      - HARBOR_ADMIN_PASSWORD=${HARBOR_ADMIN_PASSWORD:-Harbor12345}
      - CSRF_KEY=${CSRF_KEY:-change-me-csrf-key-32-chars-min}
      - RELOAD_KEY=${RELOAD_KEY:-reload-key}
    depends_on:
      - harbor_db
      - harbor-registry
    volumes:
      - harbor_core_config:/etc/core
    labels:
      - "rivetr.managed=true"

  harbor-registry:
    image: goharbor/registry-photon:${VERSION:-v2.10.0}
    restart: unless-stopped
    volumes:
      - harbor_registry_data:/storage
      - harbor_registry_config:/etc/registry
    labels:
      - "rivetr.managed=true"

  harbor-registryctl:
    image: goharbor/harbor-registryctl:${VERSION:-v2.10.0}
    restart: unless-stopped
    environment:
      - CORE_SECRET=${CORE_SECRET:-change-me-secret}
      - JOBSERVICE_SECRET=${JOBSERVICE_SECRET:-change-me-jobservice}
    volumes:
      - harbor_registry_data:/storage
      - harbor_registry_config:/etc/registry
      - harbor_registryctl_config:/etc/registryctl
    labels:
      - "rivetr.managed=true"

  harbor-portal:
    image: goharbor/harbor-portal:${VERSION:-v2.10.0}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:8080"
    labels:
      - "rivetr.managed=true"

  harbor_db:
    image: goharbor/harbor-db:${VERSION:-v2.10.0}
    restart: unless-stopped
    environment:
      - POSTGRES_PASSWORD=${DB_PASSWORD:-harbor}
    volumes:
      - harbor_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  harbor_core_config:
  harbor_registry_data:
  harbor_registry_config:
  harbor_registryctl_config:
  harbor_db_data:
"#,
            r#"[{"name":"VERSION","label":"Harbor Version","required":false,"default":"v2.10.0","secret":false},{"name":"CONTAINER_NAME","label":"Core Container Name","required":false,"default":"harbor-core","secret":false},{"name":"PORT","label":"Portal Port","required":false,"default":"80","secret":false},{"name":"HARBOR_ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"CORE_SECRET","label":"Core Secret","required":true,"default":"","secret":true},{"name":"JOBSERVICE_SECRET","label":"Jobservice Secret","required":true,"default":"","secret":true},{"name":"CSRF_KEY","label":"CSRF Key (min 32 chars)","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== ERP / BUSINESS ====================
        (
            "tpl-odoo",
            "Odoo",
            "Complete open-source ERP suite. CRM, sales, inventory, accounting, HR, manufacturing in one platform.",
            "business",
            "odoo",
            r#"services:
  odoo:
    image: odoo:${VERSION:-17}
    container_name: ${CONTAINER_NAME:-odoo}
    restart: unless-stopped
    ports:
      - "${PORT:-8069}:8069"
      - "${CHAT_PORT:-8072}:8072"
    environment:
      - HOST=odoo_db
      - USER=odoo
      - PASSWORD=${DB_PASSWORD:-odoo}
    depends_on:
      - odoo_db
    volumes:
      - odoo_data:/var/lib/odoo
      - odoo_config:/etc/odoo
      - odoo_addons:/mnt/extra-addons
    labels:
      - "rivetr.managed=true"

  odoo_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=odoo
      - POSTGRES_PASSWORD=${DB_PASSWORD:-odoo}
      - POSTGRES_DB=postgres
    volumes:
      - odoo_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  odoo_data:
  odoo_config:
  odoo_addons:
  odoo_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"17","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"odoo","secret":false},{"name":"PORT","label":"Web Port","required":false,"default":"8069","secret":false},{"name":"CHAT_PORT","label":"Live Chat Port","required":false,"default":"8072","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-mautic",
            "Mautic",
            "Open-source marketing automation. Email campaigns, lead management, landing pages, and analytics.",
            "business",
            "mautic",
            r#"services:
  mautic:
    image: mautic/mautic:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-mautic}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
    environment:
      - MAUTIC_DB_HOST=mautic_db
      - MAUTIC_DB_USER=mautic
      - MAUTIC_DB_PASSWORD=${DB_PASSWORD:-mautic}
      - MAUTIC_DB_NAME=mautic
      - MAUTIC_TRUSTED_PROXIES=0.0.0.0/0
      - MAUTIC_RUN_CRON_JOBS=true
      - MAUTIC_ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - MAUTIC_ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - MAUTIC_ADMIN_USERNAME=${ADMIN_USERNAME:-admin}
      - MAUTIC_ADMIN_FIRSTNAME=${ADMIN_FIRSTNAME:-Admin}
      - MAUTIC_ADMIN_LASTNAME=${ADMIN_LASTNAME:-User}
    depends_on:
      - mautic_db
    volumes:
      - mautic_data:/var/www/html
    labels:
      - "rivetr.managed=true"

  mautic_db:
    image: mariadb:10.11
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-rootpassword}
      - MYSQL_DATABASE=mautic
      - MYSQL_USER=mautic
      - MYSQL_PASSWORD=${DB_PASSWORD:-mautic}
    volumes:
      - mautic_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  mautic_data:
  mautic_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"mautic","secret":false},{"name":"PORT","label":"Port","required":false,"default":"80","secret":false},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"ADMIN_USERNAME","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"DB_ROOT_PASSWORD","label":"DB Root Password","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== FORMS / SURVEYS ====================
        (
            "tpl-limesurvey",
            "LimeSurvey",
            "Professional online survey platform. Advanced question types, branching logic, and statistical analysis.",
            "business",
            "limesurvey",
            r#"services:
  limesurvey:
    image: martialblog/limesurvey:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-limesurvey}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DB_TYPE=mysql
      - DB_HOST=limesurvey_db
      - DB_PORT=3306
      - DB_NAME=limesurvey
      - DB_USERNAME=limesurvey
      - DB_PASSWORD=${DB_PASSWORD:-limesurvey}
      - ADMIN_USER=${ADMIN_USER:-admin}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - ADMIN_NAME=${ADMIN_NAME:-LimeSurvey Admin}
      - ADMIN_EMAIL=${ADMIN_EMAIL:-admin@example.com}
      - PUBLIC_URL=${PUBLIC_URL:-http://localhost:8080}
    depends_on:
      - limesurvey_db
    volumes:
      - limesurvey_data:/var/www/html/upload
    labels:
      - "rivetr.managed=true"

  limesurvey_db:
    image: mariadb:10.11
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-rootpassword}
      - MYSQL_DATABASE=limesurvey
      - MYSQL_USER=limesurvey
      - MYSQL_PASSWORD=${DB_PASSWORD:-limesurvey}
    volumes:
      - limesurvey_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  limesurvey_data:
  limesurvey_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"limesurvey","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"ADMIN_USER","label":"Admin Username","required":false,"default":"admin","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"ADMIN_EMAIL","label":"Admin Email","required":true,"default":"admin@example.com","secret":false},{"name":"PUBLIC_URL","label":"Public URL","required":false,"default":"http://localhost:8080","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"DB_ROOT_PASSWORD","label":"DB Root Password","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-formbricks",
            "Formbricks",
            "Open-source survey and experience management platform. In-app surveys, website surveys, and link surveys.",
            "business",
            "formbricks",
            r#"services:
  formbricks:
    image: ghcr.io/formbricks/formbricks:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-formbricks}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - WEBAPP_URL=${WEBAPP_URL:-http://localhost:3000}
      - DATABASE_URL=postgresql://formbricks:${DB_PASSWORD:-formbricks}@formbricks_db:5432/formbricks
      - NEXTAUTH_SECRET=${NEXTAUTH_SECRET:-change-me-to-a-long-random-string}
      - NEXTAUTH_URL=${WEBAPP_URL:-http://localhost:3000}
      - ENCRYPTION_KEY=${ENCRYPTION_KEY:-change-me-to-a-32-char-random-key}
      - TELEMETRY_DISABLED=true
    depends_on:
      - formbricks_db
    labels:
      - "rivetr.managed=true"

  formbricks_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=formbricks
      - POSTGRES_PASSWORD=${DB_PASSWORD:-formbricks}
      - POSTGRES_DB=formbricks
    volumes:
      - formbricks_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  formbricks_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"formbricks","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"WEBAPP_URL","label":"Web App URL","required":true,"default":"http://localhost:3000","secret":false},{"name":"NEXTAUTH_SECRET","label":"NextAuth Secret","required":true,"default":"","secret":true},{"name":"ENCRYPTION_KEY","label":"Encryption Key (32 chars)","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== NO-CODE / LOW-CODE ====================
        (
            "tpl-baserow",
            "Baserow",
            "Open-source no-code database and Airtable alternative. Build collaborative databases without SQL.",
            "infrastructure",
            "baserow",
            r#"services:
  baserow:
    image: baserow/baserow:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-baserow}
    restart: unless-stopped
    ports:
      - "${PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
    environment:
      - BASEROW_PUBLIC_URL=${PUBLIC_URL:-http://localhost}
      - SECRET_KEY=${SECRET_KEY:-change-me-to-a-long-random-string}
      - DATABASE_PASSWORD=${DB_PASSWORD:-baserow}
      - DATABASE_USER=baserow
      - DATABASE_NAME=baserow
      - DATABASE_HOST=baserow_db
      - REDIS_HOST=baserow_redis
    depends_on:
      - baserow_db
      - baserow_redis
    volumes:
      - baserow_data:/baserow/data
    labels:
      - "rivetr.managed=true"

  baserow_db:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=baserow
      - POSTGRES_PASSWORD=${DB_PASSWORD:-baserow}
      - POSTGRES_DB=baserow
    volumes:
      - baserow_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

  baserow_redis:
    image: redis:7-alpine
    restart: unless-stopped
    labels:
      - "rivetr.managed=true"

volumes:
  baserow_data:
  baserow_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"baserow","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false},{"name":"PUBLIC_URL","label":"Public URL","required":true,"default":"http://localhost","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true}]"#,
        ),
        // ==================== EXTRA MONITORING / OBSERVABILITY ====================
    ]
}
