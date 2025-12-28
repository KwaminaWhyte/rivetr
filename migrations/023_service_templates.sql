-- Migration 023: Service Templates for one-click service deployments
-- Pre-configured docker-compose templates for common services

CREATE TABLE IF NOT EXISTS service_templates (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    -- Category: monitoring, database, storage, development, analytics, networking, security
    category TEXT NOT NULL,
    -- Icon name (e.g., "portainer", "grafana") or URL
    icon TEXT,
    -- Docker compose template (YAML)
    compose_template TEXT NOT NULL,
    -- JSON schema for required environment variables
    -- Format: [{"name": "VAR_NAME", "label": "Display Label", "required": true, "default": "...", "secret": false}]
    env_schema TEXT,
    -- Whether this is a built-in template (cannot be deleted)
    is_builtin INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_service_templates_category ON service_templates(category);
CREATE INDEX IF NOT EXISTS idx_service_templates_name ON service_templates(name);

-- Seed data: Built-in service templates

-- Portainer (Container Management)
INSERT OR IGNORE INTO service_templates (id, name, description, category, icon, compose_template, env_schema, is_builtin) VALUES (
    'portainer',
    'Portainer',
    'A powerful, open-source container management UI for Docker and Kubernetes. Provides an easy-to-use GUI for managing containers, images, networks, and volumes.',
    'development',
    'portainer',
    'version: "3.8"
services:
  portainer:
    image: portainer/portainer-ce:${PORTAINER_VERSION:-latest}
    container_name: ${CONTAINER_NAME:-portainer}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-9000}:9000"
      - "${HTTPS_PORT:-9443}:9443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - portainer_data:/data
    labels:
      - "rivetr.service=portainer"
      - "rivetr.template=portainer"

volumes:
  portainer_data:
',
    '[{"name":"PORTAINER_VERSION","label":"Portainer Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"portainer","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"9000","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"9443","secret":false}]',
    1
);

-- Uptime Kuma (Monitoring)
INSERT OR IGNORE INTO service_templates (id, name, description, category, icon, compose_template, env_schema, is_builtin) VALUES (
    'uptime-kuma',
    'Uptime Kuma',
    'A self-hosted monitoring tool like "Uptime Robot". Monitor HTTP(s), TCP, DNS, Docker containers, and more with beautiful status pages and notifications.',
    'monitoring',
    'uptime-kuma',
    'version: "3.8"
services:
  uptime-kuma:
    image: louislam/uptime-kuma:${UPTIME_KUMA_VERSION:-1}
    container_name: ${CONTAINER_NAME:-uptime-kuma}
    restart: unless-stopped
    ports:
      - "${PORT:-3001}:3001"
    volumes:
      - uptime_kuma_data:/app/data
      - /var/run/docker.sock:/var/run/docker.sock:ro
    labels:
      - "rivetr.service=uptime-kuma"
      - "rivetr.template=uptime-kuma"

volumes:
  uptime_kuma_data:
',
    '[{"name":"UPTIME_KUMA_VERSION","label":"Uptime Kuma Version","required":false,"default":"1","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"uptime-kuma","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3001","secret":false}]',
    1
);

-- Grafana + Prometheus (Monitoring Stack)
INSERT OR IGNORE INTO service_templates (id, name, description, category, icon, compose_template, env_schema, is_builtin) VALUES (
    'grafana-prometheus',
    'Grafana + Prometheus',
    'Complete monitoring stack with Prometheus for metrics collection and Grafana for visualization. Includes pre-configured dashboards and alerting capabilities.',
    'monitoring',
    'grafana',
    'version: "3.8"
services:
  prometheus:
    image: prom/prometheus:${PROMETHEUS_VERSION:-latest}
    container_name: ${PROMETHEUS_CONTAINER:-prometheus}
    restart: unless-stopped
    ports:
      - "${PROMETHEUS_PORT:-9090}:9090"
    volumes:
      - prometheus_data:/prometheus
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
    command:
      - "--config.file=/etc/prometheus/prometheus.yml"
      - "--storage.tsdb.path=/prometheus"
      - "--web.enable-lifecycle"
    labels:
      - "rivetr.service=prometheus"
      - "rivetr.template=grafana-prometheus"

  grafana:
    image: grafana/grafana:${GRAFANA_VERSION:-latest}
    container_name: ${GRAFANA_CONTAINER:-grafana}
    restart: unless-stopped
    ports:
      - "${GRAFANA_PORT:-3000}:3000"
    environment:
      - GF_SECURITY_ADMIN_USER=${GRAFANA_USER:-admin}
      - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_PASSWORD:-admin}
      - GF_USERS_ALLOW_SIGN_UP=false
    volumes:
      - grafana_data:/var/lib/grafana
    depends_on:
      - prometheus
    labels:
      - "rivetr.service=grafana"
      - "rivetr.template=grafana-prometheus"

volumes:
  prometheus_data:
  grafana_data:
',
    '[{"name":"PROMETHEUS_VERSION","label":"Prometheus Version","required":false,"default":"latest","secret":false},{"name":"GRAFANA_VERSION","label":"Grafana Version","required":false,"default":"latest","secret":false},{"name":"PROMETHEUS_CONTAINER","label":"Prometheus Container Name","required":false,"default":"prometheus","secret":false},{"name":"GRAFANA_CONTAINER","label":"Grafana Container Name","required":false,"default":"grafana","secret":false},{"name":"PROMETHEUS_PORT","label":"Prometheus Port","required":false,"default":"9090","secret":false},{"name":"GRAFANA_PORT","label":"Grafana Port","required":false,"default":"3000","secret":false},{"name":"GRAFANA_USER","label":"Grafana Admin User","required":false,"default":"admin","secret":false},{"name":"GRAFANA_PASSWORD","label":"Grafana Admin Password","required":true,"default":"","secret":true}]',
    1
);

-- Gitea (Git Server)
INSERT OR IGNORE INTO service_templates (id, name, description, category, icon, compose_template, env_schema, is_builtin) VALUES (
    'gitea',
    'Gitea',
    'A lightweight, self-hosted Git service. Provides GitHub-like functionality including issues, pull requests, wikis, and CI/CD integration.',
    'development',
    'gitea',
    'version: "3.8"
services:
  gitea:
    image: gitea/gitea:${GITEA_VERSION:-latest}
    container_name: ${CONTAINER_NAME:-gitea}
    restart: unless-stopped
    environment:
      - USER_UID=1000
      - USER_GID=1000
      - GITEA__database__DB_TYPE=${DB_TYPE:-sqlite3}
      - GITEA__database__HOST=${DB_HOST:-}
      - GITEA__database__NAME=${DB_NAME:-gitea}
      - GITEA__database__USER=${DB_USER:-gitea}
      - GITEA__database__PASSWD=${DB_PASSWORD:-}
      - GITEA__server__ROOT_URL=${ROOT_URL:-http://localhost:3000}
      - GITEA__server__SSH_DOMAIN=${SSH_DOMAIN:-localhost}
      - GITEA__server__SSH_PORT=${SSH_PORT:-2222}
    ports:
      - "${HTTP_PORT:-3000}:3000"
      - "${SSH_PORT:-2222}:22"
    volumes:
      - gitea_data:/data
      - /etc/timezone:/etc/timezone:ro
      - /etc/localtime:/etc/localtime:ro
    labels:
      - "rivetr.service=gitea"
      - "rivetr.template=gitea"

volumes:
  gitea_data:
',
    '[{"name":"GITEA_VERSION","label":"Gitea Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"gitea","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"3000","secret":false},{"name":"SSH_PORT","label":"SSH Port","required":false,"default":"2222","secret":false},{"name":"ROOT_URL","label":"Root URL","required":false,"default":"http://localhost:3000","secret":false},{"name":"SSH_DOMAIN","label":"SSH Domain","required":false,"default":"localhost","secret":false},{"name":"DB_TYPE","label":"Database Type","required":false,"default":"sqlite3","secret":false},{"name":"DB_HOST","label":"Database Host","required":false,"default":"","secret":false},{"name":"DB_NAME","label":"Database Name","required":false,"default":"gitea","secret":false},{"name":"DB_USER","label":"Database User","required":false,"default":"gitea","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":false,"default":"","secret":true}]',
    1
);
