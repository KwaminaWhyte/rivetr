//! Additional monitoring service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== MONITORING (additional) ====================
        (
            "tpl-victoria-metrics",
            "VictoriaMetrics",
            "Fast, cost-effective, scalable time series database. Drop-in replacement for Prometheus.",
            "monitoring",
            "victoriametrics",
            r#"services:
  victoriametrics:
    image: victoriametrics/victoria-metrics:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-victoriametrics}
    restart: unless-stopped
    ports:
      - "${PORT:-8428}:8428"
    command:
      - "--storageDataPath=/storage"
      - "--httpListenAddr=:8428"
      - "--retentionPeriod=${RETENTION:-1}"
    volumes:
      - victoriametrics_data:/storage
    labels:
      - "rivetr.managed=true"

volumes:
  victoriametrics_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"victoriametrics","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8428","secret":false},{"name":"RETENTION","label":"Retention Period (months)","required":false,"default":"1","secret":false}]"#,
        ),
        (
            "tpl-netdata",
            "Netdata",
            "Real-time performance and health monitoring. Visualize server metrics with per-second granularity.",
            "monitoring",
            "netdata",
            r#"services:
  netdata:
    image: netdata/netdata:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-netdata}
    restart: unless-stopped
    pid: host
    ports:
      - "${PORT:-19999}:19999"
    cap_add:
      - SYS_PTRACE
      - SYS_ADMIN
    security_opt:
      - apparmor:unconfined
    environment:
      - NETDATA_CLAIM_TOKEN=${CLAIM_TOKEN:-}
      - NETDATA_CLAIM_URL=https://app.netdata.cloud
    volumes:
      - netdata_config:/etc/netdata
      - netdata_lib:/var/lib/netdata
      - netdata_cache:/var/cache/netdata
      - /etc/passwd:/host/etc/passwd:ro
      - /etc/group:/host/etc/group:ro
      - /proc:/host/proc:ro
      - /sys:/host/sys:ro
      - /etc/os-release:/host/etc/os-release:ro
      - /var/run/docker.sock:/var/run/docker.sock:ro
    labels:
      - "rivetr.managed=true"

volumes:
  netdata_config:
  netdata_lib:
  netdata_cache:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"netdata","secret":false},{"name":"PORT","label":"Port","required":false,"default":"19999","secret":false},{"name":"CLAIM_TOKEN","label":"Cloud Claim Token (optional)","required":false,"default":"","secret":true}]"#,
        ),
        (
            "tpl-healthchecks",
            "Healthchecks",
            "Cron job monitoring service. Get notified when your scheduled tasks don't run on time.",
            "monitoring",
            "healthchecks",
            r#"services:
  healthchecks:
    image: healthchecks/healthchecks:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-healthchecks}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
    environment:
      - SECRET_KEY=${SECRET_KEY:-change-me-to-a-random-string}
      - ALLOWED_HOSTS=${ALLOWED_HOSTS:-*}
      - DEBUG=${DEBUG:-False}
      - REGISTRATION_OPEN=${REGISTRATION_OPEN:-True}
    volumes:
      - healthchecks_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  healthchecks_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"healthchecks","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8000","secret":false},{"name":"SECRET_KEY","label":"Secret Key","required":true,"default":"","secret":true},{"name":"ALLOWED_HOSTS","label":"Allowed Hosts","required":false,"default":"*","secret":false},{"name":"REGISTRATION_OPEN","label":"Open Registration","required":false,"default":"True","secret":false}]"#,
        ),
        (
            "tpl-statping-ng",
            "Statping-NG",
            "Easy-to-use status page for websites and applications. Beautiful, feature-rich, and self-hosted.",
            "monitoring",
            "statping-ng",
            r#"services:
  statping:
    image: adamboutcher/statping-ng:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-statping}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
    environment:
      - DB_CONN=postgres
      - DB_HOST=statping_db
      - DB_USER=statping
      - DB_PASS=${DB_PASSWORD:-statping}
      - DB_DATABASE=statping
      - NAME=${SITE_NAME:-My Status Page}
      - DESCRIPTION=${SITE_DESC:-Status monitoring}
    depends_on:
      - statping_db
    volumes:
      - statping_data:/app
    labels:
      - "rivetr.managed=true"

  statping_db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      - POSTGRES_USER=statping
      - POSTGRES_PASSWORD=${DB_PASSWORD:-statping}
      - POSTGRES_DB=statping
    volumes:
      - statping_db_data:/var/lib/postgresql/data
    labels:
      - "rivetr.managed=true"

volumes:
  statping_data:
  statping_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"statping","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8080","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"SITE_NAME","label":"Site Name","required":false,"default":"My Status Page","secret":false},{"name":"SITE_DESC","label":"Site Description","required":false,"default":"Status monitoring","secret":false}]"#,
        ),
        (
            "tpl-alertmanager",
            "Alertmanager",
            "Prometheus Alertmanager handles alerts, routing to receivers like email, Slack, PagerDuty, and more.",
            "monitoring",
            "alertmanager",
            r#"services:
  alertmanager:
    image: prom/alertmanager:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-alertmanager}
    restart: unless-stopped
    ports:
      - "${PORT:-9093}:9093"
    command:
      - "--config.file=/etc/alertmanager/config.yml"
      - "--storage.path=/alertmanager"
    volumes:
      - alertmanager_config:/etc/alertmanager
      - alertmanager_data:/alertmanager
    labels:
      - "rivetr.managed=true"

volumes:
  alertmanager_config:
  alertmanager_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"alertmanager","secret":false},{"name":"PORT","label":"Port","required":false,"default":"9093","secret":false}]"#,
        ),
        (
            "tpl-loki",
            "Grafana Loki",
            "Log aggregation system by Grafana Labs. Designed to be cost-effective and easy to operate.",
            "monitoring",
            "loki",
            r#"services:
  loki:
    image: grafana/loki:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-loki}
    restart: unless-stopped
    ports:
      - "${PORT:-3100}:3100"
    command: -config.file=/etc/loki/local-config.yaml
    volumes:
      - loki_data:/loki
    labels:
      - "rivetr.managed=true"

  promtail:
    image: grafana/promtail:${VERSION:-latest}
    container_name: promtail
    restart: unless-stopped
    command: -config.file=/etc/promtail/config.yml
    volumes:
      - /var/log:/var/log:ro
      - promtail_config:/etc/promtail
    depends_on:
      - loki
    labels:
      - "rivetr.managed=true"

volumes:
  loki_data:
  promtail_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"loki","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3100","secret":false}]"#,
        ),
    ]
}
