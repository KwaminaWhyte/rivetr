//! Batch 2 monitoring service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== BATCH 2: MONITORING ====================
        (
            "tpl-batch2-signoz",
            "SigNoz",
            "Open-source APM and observability platform. Traces, metrics, and logs in a single pane.",
            "monitoring",
            "signoz",
            r#"services:
  signoz:
    image: signoz/signoz:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-signoz}
    restart: unless-stopped
    ports:
      - "${PORT:-3301}:3301"
      - "${OTEL_GRPC_PORT:-4317}:4317"
      - "${OTEL_HTTP_PORT:-4318}:4318"
    environment:
      - SIGNOZ_LOCAL_DB_PATH=/var/lib/signoz/signoz.db
    volumes:
      - signoz_data:/var/lib/signoz
    labels:
      - "rivetr.managed=true"

volumes:
  signoz_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"signoz","secret":false},{"name":"PORT","label":"Web UI Port","required":false,"default":"3301","secret":false},{"name":"OTEL_GRPC_PORT","label":"OTEL gRPC Port","required":false,"default":"4317","secret":false},{"name":"OTEL_HTTP_PORT","label":"OTEL HTTP Port","required":false,"default":"4318","secret":false}]"#,
        ),
        (
            "tpl-batch2-beszel",
            "Beszel",
            "Lightweight server monitoring hub with Docker stats, historical data, and alerting.",
            "monitoring",
            "beszel",
            r#"services:
  beszel:
    image: henrygd/beszel:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-beszel}
    restart: unless-stopped
    ports:
      - "${PORT:-8090}:8090"
    volumes:
      - beszel_data:/beszel_data
    labels:
      - "rivetr.managed=true"

volumes:
  beszel_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"beszel","secret":false},{"name":"PORT","label":"Port","required":false,"default":"8090","secret":false}]"#,
        ),
        (
            "tpl-batch2-checkmate",
            "Checkmate",
            "Open-source uptime and infrastructure monitoring with beautiful dashboards.",
            "monitoring",
            "checkmate",
            r#"services:
  checkmate:
    image: bluewavelabs/checkmate:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-checkmate}
    restart: unless-stopped
    ports:
      - "${PORT:-5000}:5000"
    environment:
      - JWT_SECRET=${JWT_SECRET:-change-me-to-a-random-string}
    volumes:
      - checkmate_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  checkmate_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"checkmate","secret":false},{"name":"PORT","label":"Port","required":false,"default":"5000","secret":false},{"name":"JWT_SECRET","label":"JWT Secret","required":true,"default":"","secret":true}]"#,
        ),
    ]
}
