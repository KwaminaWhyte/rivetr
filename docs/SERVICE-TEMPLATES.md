# Rivetr Service Templates

> **IMPORTANT: Read this file before adding new templates** to avoid duplicates.
> Last updated: 2026-03-14
> Total: 285 templates across 24 categories

## How to Add Templates

1. Search this file (Ctrl+F / Cmd+F) for the app name before adding.
2. If not listed, add it to the most recent sprint seeder file (`src/db/seeders/sprintXX.rs`).
3. Update the count and list in this file.

---

## Templates by Category

### AI / LLM

| ID | Name | File |
|----|------|------|
| `tpl-anything-llm` | AnythingLLM | `extra_services.rs` |
| `tpl-anythingllm` | AnythingLLM | `sprint16.rs` |
| `tpl-chroma` | Chroma | `sprint16.rs` |
| `tpl-dify` | Dify | `ai_extras.rs` |
| `tpl-flowise` | Flowise | `sprint15.rs` |
| `tpl-langflow` | Langflow | `extra_services.rs` |
| `tpl-langfuse` | Langfuse | `ai_extras.rs` |
| `tpl-librechat` | LibreChat | `extra_services.rs` |
| `tpl-libretranslate` | LibreTranslate | `extra_services.rs` |
| `tpl-litellm` | LiteLLM | `extra_services.rs` |
| `tpl-localai` | LocalAI | `ai_extras.rs` |
| `tpl-ollama` | Ollama | `sprint15.rs` |
| `tpl-open-webui` | Open WebUI | `sprint15.rs` |
| `tpl-perplexica` | Perplexica | `ai_extras.rs` |

### AI / ML (Specialized)

| ID | Name | File |
|----|------|------|
| `tpl-comfyui` | ComfyUI | `sprint19.rs` |
| `tpl-llama-cpp-server` | llama.cpp Server | `sprint18.rs` |
| `tpl-milvus` | Milvus | `sprint18.rs` |
| `tpl-openedai-speech` | OpenedAI Speech | `sprint19.rs` |
| `tpl-stable-diffusion-webui` | Stable Diffusion WebUI | `sprint19.rs` |
| `tpl-tabbyml` | Tabby | `sprint19.rs` |

### Analytics & Finance

| ID | Name | File |
|----|------|------|
| `tpl-actual-budget` | Actual Budget | `business.rs` |
| `tpl-firefly-iii` | Firefly III | `business.rs` |
| `tpl-goatcounter` | GoatCounter | `extra_services.rs` |
| `tpl-openpanel` | OpenPanel | `extra_services.rs` |
| `tpl-plausible` | Plausible Analytics | `sprint19.rs` |
| `tpl-umami` | Umami | `sprint19.rs` |

### Automation & Workflow

| ID | Name | File |
|----|------|------|
| `tpl-activepieces` | Activepieces | `sprint18.rs` |
| `tpl-n8n` | n8n | `sprint18.rs` |
| `tpl-n8n-ai` | n8n (AI-ready) | `ai_extras.rs` |

### Business & CRM

| ID | Name | File |
|----|------|------|
| `tpl-cal-com` | Cal.com | `sprint16.rs` |
| `tpl-crater` | Crater | `sprint19.rs` |
| `tpl-dolibarr` | Dolibarr | `sprint16.rs` |
| `tpl-dolibarr-erp` | Dolibarr ERP/CRM | `sprint18.rs` |
| `tpl-erpnext` | ERPNext | `sprint18.rs` |
| `tpl-formbricks` | Formbricks | `sprint15.rs` |
| `tpl-invoice-ninja-v5` | Invoice Ninja v5 | `sprint18.rs` |
| `tpl-kimai-timetracking` | Kimai Time Tracking | `sprint18.rs` |
| `tpl-limesurvey` | LimeSurvey | `sprint15.rs` |
| `tpl-mautic` | Mautic | `sprint15.rs` |
| `tpl-odoo` | Odoo | `sprint15.rs` |
| `tpl-plane` | Plane | `sprint16.rs` |
| `tpl-taiga` | Taiga | `sprint16.rs` |
| `tpl-twenty-crm` | Twenty CRM | `sprint16.rs` |

### CMS & Headless API

| ID | Name | File |
|----|------|------|
| `tpl-cockpit-cms` | Cockpit CMS | `cms_extra.rs` |
| `tpl-directus` | Directus | `sprint15.rs` |
| `tpl-dotcms` | dotCMS | `cms_extra.rs` |
| `tpl-ghost` | Ghost | `sprint18.rs` |
| `tpl-keystonejs` | KeystoneJS | `cms_extra.rs` |
| `tpl-outline` | Outline | `sprint15.rs` |
| `tpl-pimcore` | Pimcore | `cms_extra.rs` |
| `tpl-strapi` | Strapi | `sprint15.rs` |
| `tpl-strapi-postgres` | Strapi (PostgreSQL) | `sprint18.rs` |
| `tpl-wagtail` | Wagtail | `cms_extra.rs` |
| `tpl-write-freely` | Write.freely | `sprint18.rs` |

### Communication & Social

| ID | Name | File |
|----|------|------|
| `tpl-apprise` | Apprise | `sprint19.rs` |
| `tpl-chatwoot` | Chatwoot | `communication_extra.rs` |
| `tpl-discourse` | Discourse | `communication_extra.rs` |
| `tpl-element-web` | Element Web | `communication_extra.rs` |
| `tpl-gotify-server` | Gotify | `sprint19.rs` |
| `tpl-humhub` | HumHub | `sprint16.rs` |
| `tpl-jitsi-meet` | Jitsi Meet | `sprint15.rs` |
| `tpl-lemmy` | Lemmy | `communication_extra.rs` |
| `tpl-listmonk` | Listmonk | `media_productivity.rs` |
| `tpl-listmonk-standalone` | Listmonk Standalone | `sprint18.rs` |
| `tpl-mastodon` | Mastodon | `communication_extra.rs` |
| `tpl-matrix-synapse` | Matrix Synapse | `extra_services.rs` |
| `tpl-mattermost` | Mattermost | `sprint15.rs` |
| `tpl-ntfy-server` | ntfy | `sprint18.rs` |
| `tpl-revolt` | Revolt | `sprint16.rs` |
| `tpl-rocketchat` | Rocket.Chat | `sprint15.rs` |
| `tpl-zulip` | Zulip | `communication_extra.rs` |

### Community

| ID | Name | File |
|----|------|------|
| `tpl-apache-answer` | Apache Answer | `sprint18.rs` |

### Database & Messaging

| ID | Name | File |
|----|------|------|
| `tpl-kafka` | Apache Kafka | `databases_tools.rs` |
| `tpl-nats` | NATS | `databases_tools.rs` |
| `tpl-qdrant` | Qdrant | `databases_tools.rs` |
| `tpl-questdb` | QuestDB | `sprint16.rs` |
| `tpl-valkey` | Valkey | `databases_tools.rs` |
| `tpl-weaviate` | Weaviate | `databases_tools.rs` |

### Databases (Additional)

| ID | Name | File |
|----|------|------|
| `tpl-apache-age` | Apache AGE | `sprint18.rs` |
| `tpl-arangodb` | ArangoDB | `sprint18.rs` |
| `tpl-cassandra` | Apache Cassandra | `sprint16.rs` |
| `tpl-clickhouse` | ClickHouse | `sprint16.rs` |
| `tpl-cockroachdb` | CockroachDB | `sprint16.rs` |
| `tpl-couchbase` | Couchbase Server | `sprint18.rs` |
| `tpl-dragonflydb` | DragonflyDB | `sprint16.rs` |
| `tpl-edgedb` | EdgeDB | `sprint19.rs` |
| `tpl-eventstoredb` | EventStoreDB | `sprint18.rs` |
| `tpl-fauna` | FaunaDB | `sprint19.rs` |
| `tpl-ferretdb` | FerretDB | `sprint18.rs` |
| `tpl-influxdb` | InfluxDB | `sprint19.rs` |
| `tpl-keydb` | KeyDB | `sprint16.rs` |
| `tpl-mariadb` | MariaDB | `sprint15.rs` |
| `tpl-neo4j` | Neo4j | `sprint16.rs` |
| `tpl-rethinkdb` | RethinkDB | `sprint18.rs` |
| `tpl-rqlite` | rqlite | `sprint18.rs` |
| `tpl-surrealdb` | SurrealDB | `sprint16.rs` |
| `tpl-tigerbeetle` | TigerBeetle | `sprint18.rs` |
| `tpl-timescaledb` | TimescaleDB | `sprint16.rs` |

### Development Tools

| ID | Name | File |
|----|------|------|
| `tpl-appflowy` | AppFlowy | `media_productivity.rs` |
| `tpl-appsmith` | Appsmith | `extra_services.rs` |
| `tpl-budibase` | Budibase | `extra_services.rs` |
| `tpl-dashy` | Dashy | `extra_services.rs` |
| `tpl-docker-registry` | Docker Registry | `devtools_extra.rs` |
| `tpl-drawio` | Draw.io (Diagrams.net) | `misc_extras.rs` |
| `tpl-duplicati` | Duplicati | `extra_services.rs` |
| `tpl-excalidraw` | Excalidraw | `media_productivity.rs` |
| `tpl-freshrss` | FreshRSS | `media_productivity.rs` |
| `tpl-gitea-runner` | Gitea Actions Runner | `devtools_extra.rs` |
| `tpl-gitlab-ce` | GitLab CE | `devtools_extra.rs` |
| `tpl-gitness` | Gitness | `devtools_extra.rs` |
| `tpl-gotify` | Gotify | `media_productivity.rs` |
| `tpl-homarr` | Homarr | `extra_services.rs` |
| `tpl-homepage` | Homepage | `extra_services.rs` |
| `tpl-it-tools` | IT Tools | `misc_extras.rs` |
| `tpl-jenkins` | Jenkins | `extra_services.rs` |
| `tpl-medusa` | Medusa | `business.rs` |
| `tpl-memos` | Memos | `media_productivity.rs` |
| `tpl-miniflux` | Miniflux | `media_productivity.rs` |
| `tpl-nocodb` | NocoDB | `extra_services.rs` |
| `tpl-ntfy` | Ntfy | `media_productivity.rs` |
| `tpl-onedev` | OneDev | `devtools_extra.rs` |
| `tpl-open-speed-test` | OpenSpeedTest | `misc_extras.rs` |
| `tpl-organizr` | Organizr | `extra_services.rs` |
| `tpl-paperless-ngx` | Paperless-ngx | `media_productivity.rs` |
| `tpl-penpot` | Penpot | `media_productivity.rs` |
| `tpl-portainer` | Portainer CE | `extra_services.rs` |
| `tpl-stirling-pdf` | Stirling PDF | `media_productivity.rs` |
| `tpl-verdaccio` | Verdaccio | `devtools_extra.rs` |
| `tpl-woodpecker-ci` | Woodpecker CI | `devtools_extra.rs` |

### DevOps & Admin

| ID | Name | File |
|----|------|------|
| `tpl-adminer` | Adminer | `sprint19.rs` |
| `tpl-caddy` | Caddy | `sprint19.rs` |
| `tpl-code-server` | Code-Server | `sprint19.rs` |
| `tpl-forgejo` | Forgejo | `sprint19.rs` |
| `tpl-gitpod` | Gitpod Self-Hosted | `sprint19.rs` |
| `tpl-hoppscotch` | Hoppscotch | `sprint19.rs` |
| `tpl-nginx-ui` | Nginx UI | `sprint19.rs` |
| `tpl-pgadmin` | pgAdmin | `sprint19.rs` |
| `tpl-phpmyadmin` | phpMyAdmin | `sprint19.rs` |
| `tpl-sentry` | Sentry | `sprint19.rs` |
| `tpl-traefik` | Traefik | `sprint19.rs` |

### CI/CD & DevTools

| ID | Name | File |
|----|------|------|
| `tpl-argocd` | Argo CD | `sprint16.rs` |
| `tpl-concourse-ci` | Concourse CI | `sprint16.rs` |
| `tpl-drone-ci` | Drone CI | `sprint15.rs` |
| `tpl-gitlab-runner` | GitLab Runner | `sprint16.rs` |
| `tpl-harbor` | Harbor | `sprint15.rs` |
| `tpl-artifactory-oss` | JFrog Artifactory OSS | `sprint18.rs` |
| `tpl-nexus-oss` | Nexus Repository OSS | `sprint18.rs` |
| `tpl-sonarqube` | SonarQube | `sprint18.rs` |
| `tpl-tekton-dashboard` | Tekton Dashboard | `sprint16.rs` |
| `tpl-weave-gitops` | Weave GitOps | `sprint18.rs` |
| `tpl-windmill` | Windmill | `sprint15.rs` |
| `tpl-woodpecker-agent` | Woodpecker CI Agent | `sprint16.rs` |
| `tpl-woodpecker-server` | Woodpecker CI Server | `sprint16.rs` |

### Documentation & Knowledge

| ID | Name | File |
|----|------|------|
| `tpl-batch2-bookstack` | BookStack | `documentation.rs` |
| `tpl-batch2-docmost` | Docmost | `documentation.rs` |
| `tpl-batch2-wikijs` | Wiki.js | `documentation.rs` |
| `tpl-wiki-js` | Wiki.js | `misc_extras.rs` |

### Infrastructure & Backend

| ID | Name | File |
|----|------|------|
| `tpl-adguard-home` | AdGuard Home | `networking_extra.rs` |
| `tpl-appwrite` | Appwrite | `sprint15.rs` |
| `tpl-baserow` | Baserow | `sprint15.rs` |
| `tpl-cloudflared` | Cloudflare Tunnel | `networking_extra.rs` |
| `tpl-consul` | HashiCorp Consul | `sprint18.rs` |
| `tpl-nomad` | HashiCorp Nomad | `sprint18.rs` |
| `tpl-garage` | Garage | `databases_tools.rs` |
| `tpl-haproxy` | HAProxy | `networking_extra.rs` |
| `tpl-headscale` | Headscale | `networking_extra.rs` |
| `tpl-nginx-proxy-manager` | Nginx Proxy Manager | `devtools_extra.rs` |
| `tpl-pocketbase` | PocketBase | `sprint15.rs` |
| `tpl-restic-rest-server` | Restic REST Server | `sprint16.rs` |
| `tpl-seaweedfs` | SeaweedFS | `databases_tools.rs` |
| `tpl-supabase` | Supabase | `sprint15.rs` |
| `tpl-tailscale` | Tailscale | `networking_extra.rs` |

### Media & Files

| ID | Name | File |
|----|------|------|
| `tpl-calibre-web` | Calibre-Web | `sprint19.rs` |
| `tpl-fireshare` | Fireshare | `sprint19.rs` |
| `tpl-frigate` | Frigate | `sprint19.rs` |
| `tpl-batch2-immich` | Immich | `documentation.rs` |
| `tpl-batch2-jellyfin` | Jellyfin | `documentation.rs` |
| `tpl-kavita` | Kavita | `sprint19.rs` |
| `tpl-mediamtx` | MediaMTX | `misc_extras.rs` |
| `tpl-batch2-navidrome` | Navidrome | `documentation.rs` |
| `tpl-owncast` | Owncast | `misc_extras.rs` |
| `tpl-photoprism` | PhotoPrism | `misc_extras.rs` |
| `tpl-batch2-seafile` | Seafile | `documentation.rs` |

### Monitoring & Observability

| ID | Name | File |
|----|------|------|
| `tpl-alertmanager` | Alertmanager | `monitoring_extra.rs` |
| `tpl-batch2-beszel` | Beszel | `media_monitoring.rs` |
| `tpl-changedetection` | Changedetection.io | `media_productivity.rs` |
| `tpl-batch2-checkmate` | Checkmate | `media_monitoring.rs` |
| `tpl-checkmk` | Checkmk | `sprint15.rs` |
| `tpl-dozzle` | Dozzle | `extra_services.rs` |
| `tpl-glances` | Glances | `media_productivity.rs` |
| `tpl-grafana-prometheus` | Grafana + Prometheus | `sprint15.rs` |
| `tpl-graylog` | Graylog | `sprint16.rs` |
| `tpl-healthchecks` | Healthchecks | `monitoring_extra.rs` |
| `tpl-jaeger` | Jaeger | `sprint16.rs` |
| `tpl-loki` | Grafana Loki | `monitoring_extra.rs` |
| `tpl-netdata` | Netdata | `monitoring_extra.rs` |
| `tpl-otel-collector` | OpenTelemetry Collector | `sprint16.rs` |
| `tpl-prometheus` | Prometheus | `sprint16.rs` |
| `tpl-pyroscope` | Pyroscope | `sprint18.rs` |
| `tpl-scrutiny` | Scrutiny | `sprint19.rs` |
| `tpl-batch2-signoz` | SigNoz | `media_monitoring.rs` |
| `tpl-signoz` | SigNoz | `sprint15.rs` |
| `tpl-speedtest-tracker` | Speedtest Tracker | `sprint19.rs` |
| `tpl-statping-ng` | Statping-NG | `monitoring_extra.rs` |
| `tpl-tempo` | Grafana Tempo | `sprint18.rs` |
| `tpl-thanos` | Thanos | `sprint19.rs` |
| `tpl-uptime-kuma` | Uptime Kuma | `sprint16.rs` |
| `tpl-victoria-metrics` | VictoriaMetrics | `monitoring_extra.rs` |
| `tpl-victoriametrics` | VictoriaMetrics | `sprint20.rs` |
| `tpl-zabbix` | Zabbix | `sprint19.rs` |

### Networking & VPN

| ID | Name | File |
|----|------|------|
| `tpl-wireguard-easy` | WireGuard Easy | `media_productivity.rs` |

### Other / Utility

| ID | Name | File |
|----|------|------|
| `tpl-batch2-linkwarden` | Linkwarden | `project_mgmt.rs` |
| `tpl-batch2-paperless-ngx` | Paperless-ngx | `project_mgmt.rs` |
| `tpl-batch2-stirling-pdf` | Stirling-PDF | `project_mgmt.rs` |
| `tpl-batch2-tandoor` | Tandoor Recipes | `project_mgmt.rs` |
| `tpl-batch2-trilium` | Trilium | `project_mgmt.rs` |

### Productivity

| ID | Name | File |
|----|------|------|
| `tpl-cal-com-server` | Cal.com | `sprint18.rs` |
| `tpl-dasherr` | Dasherr | `sprint19.rs` |
| `tpl-focalboard-server` | Focalboard | `sprint18.rs` |
| `tpl-grocy` | Grocy | `sprint19.rs` |
| `tpl-hedgedoc` | HedgeDoc | `sprint19.rs` |
| `tpl-heimdall` | Heimdall | `sprint19.rs` |
| `tpl-kanboard` | Kanboard | `sprint19.rs` |
| `tpl-mealie` | Mealie | `sprint19.rs` |
| `tpl-onlyoffice` | OnlyOffice Document Server | `sprint19.rs` |
| `tpl-openproject` | OpenProject | `sprint19.rs` |
| `tpl-rallly` | Rallly | `sprint18.rs` |
| `tpl-redmine` | Redmine | `sprint19.rs` |
| `tpl-tandoor-recipes` | Tandoor Recipes | `sprint19.rs` |
| `tpl-wekan` | Wekan | `sprint19.rs` |

### Project Management

| ID | Name | File |
|----|------|------|
| `tpl-batch2-calcom` | Cal.com | `project_mgmt.rs` |
| `tpl-focalboard` | Focalboard | `business.rs` |
| `tpl-invoice-ninja` | Invoice Ninja | `business.rs` |
| `tpl-kimai` | Kimai | `business.rs` |
| `tpl-batch2-leantime` | Leantime | `project_mgmt.rs` |
| `tpl-monica` | Monica | `business.rs` |
| `tpl-obsidian-livesync` | Obsidian LiveSync | `misc_extras.rs` |
| `tpl-batch2-plane` | Plane | `project_mgmt.rs` |
| `tpl-silverbullet` | SilverBullet | `misc_extras.rs` |
| `tpl-batch2-vikunja` | Vikunja | `project_mgmt.rs` |

### Search

| ID | Name | File |
|----|------|------|
| `tpl-solr` | Apache Solr | `sprint19.rs` |
| `tpl-batch2-meilisearch` | Meilisearch | `security_search.rs` |
| `tpl-opensearch` | OpenSearch | `sprint19.rs` |
| `tpl-opensearch-dashboards` | OpenSearch Dashboards | `sprint19.rs` |
| `tpl-searxng` | SearXNG | `ai_extras.rs` |
| `tpl-batch2-typesense` | Typesense | `security_search.rs` |

### Security & Auth

| ID | Name | File |
|----|------|------|
| `tpl-authelia` | Authelia | `auth_identity.rs` |
| `tpl-authentik` | Authentik | `sprint20.rs` |
| `tpl-batch2-authentik` | Authentik | `security_search.rs` |
| `tpl-casdoor` | Casdoor | `auth_identity.rs` |
| `tpl-crowdsec` | CrowdSec | `sprint15.rs` |
| `tpl-crowdsec-dashboard` | CrowdSec Dashboard | `sprint16.rs` |
| `tpl-vault` | HashiCorp Vault | `sprint18.rs` |
| `tpl-infisical` | Infisical | `sprint20.rs` |
| `tpl-batch2-infisical` | Infisical | `security_search.rs` |
| `tpl-infisical-server` | Infisical | `sprint18.rs` |
| `tpl-batch2-keycloak` | Keycloak | `security_search.rs` |
| `tpl-keycloak-server` | Keycloak | `sprint18.rs` |
| `tpl-logto` | Logto | `auth_identity.rs` |
| `tpl-ory-kratos` | Ory Kratos | `auth_identity.rs` |
| `tpl-passbolt` | Passbolt | `sprint16.rs` |
| `tpl-pihole` | Pi-hole | `extra_services.rs` |
| `tpl-step-ca` | Step CA | `sprint19.rs` |
| `tpl-vaultwarden` | Vaultwarden | `sprint18.rs` |
| `tpl-vaultwarden-advanced` | Vaultwarden (Advanced) | `misc_extras.rs` |
| `tpl-wazuh` | Wazuh | `sprint16.rs` |
| `tpl-wazuh-manager` | Wazuh Manager | `sprint19.rs` |
| `tpl-zitadel` | ZITADEL | `auth_identity.rs` |

### Storage & Media Server

| ID | Name | File |
|----|------|------|
| `tpl-audiobookshelf` | Audiobookshelf | `media_productivity.rs` |
| `tpl-emby` | Emby | `extra_services.rs` |
| `tpl-filebrowser` | Filebrowser | `sprint18.rs` |
| `tpl-filerun` | FileRun | `sprint18.rs` |
| `tpl-immich` | Immich | `media_productivity.rs` |
| `tpl-jellyfin` | Jellyfin | `media_productivity.rs` |
| `tpl-minio` | MinIO | `sprint19.rs` |
| `tpl-navidrome` | Navidrome | `media_productivity.rs` |
| `tpl-nextcloud` | Nextcloud | `extra_services.rs` |
| `tpl-plex` | Plex Media Server | `extra_services.rs` |
| `tpl-qbittorrent` | qBittorrent | `extra_services.rs` |
| `tpl-radarr` | Radarr | `extra_services.rs` |
| `tpl-seafile` | Seafile | `extra_services.rs` |
| `tpl-sftpgo` | SFTPGo | `sprint19.rs` |
| `tpl-sonarr` | Sonarr | `extra_services.rs` |
| `tpl-storj-gateway` | Storj Gateway MT | `sprint18.rs` |
| `tpl-syncthing` | Syncthing | `media_productivity.rs` |

---

*Generated from `src/db/seeders/*.rs`. Each template is a Docker Compose stack seeded into the database at startup.*
