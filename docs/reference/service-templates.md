# Rivetr Service Templates

> **IMPORTANT: Read this file before adding new templates** to avoid duplicates.
> Last updated: 2026-03-18
> Total: 335 templates across 26 categories

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
| `tpl-chroma` | Chroma | `sprint16.rs` |
| `chromadb` | ChromaDB | `ai_ml.rs` |
| `tpl-dify` | Dify | `ai_extras.rs` |
| `flowise` | Flowise | `ai_ml.rs` |
| `langflow` | Langflow | `ai_ml.rs` |
| `tpl-langfuse` | Langfuse | `ai_extras.rs` |
| `tpl-librechat` | LibreChat | `extra_services.rs` |
| `tpl-libretranslate` | LibreTranslate | `extra_services.rs` |
| `litellm` | LiteLLM | `ai_ml.rs` |
| `tpl-localai` | LocalAI | `ai_extras.rs` |
| `ollama` | Ollama | `ai_ml.rs` |
| `open-webui` | Open WebUI | `ai_ml.rs` |
| `tpl-perplexica` | Perplexica | `ai_extras.rs` |

### AI / ML

| ID | Name | File |
|----|------|------|
| `tpl-anythingllm` | AnythingLLM | `sprint23.rs` |
| `tpl-argilla` | Argilla | `sprint22.rs` |
| `tpl-flowise` | Flowise | `sprint23.rs` |
| `tpl-langflow` | Langflow | `sprint23.rs` |
| `tpl-litellm` | LiteLLM | `sprint24.rs` |
| `tpl-mage-ai` | Mage AI | `sprint22.rs` |
| `tpl-mindsdb` | MindsDB | `sprint24.rs` |
| `tpl-open-webui` | Open WebUI | `sprint23.rs` |

### Analytics & Finance

| ID | Name | File |
|----|------|------|
| `tpl-actual-budget` | Actual Budget | `media_productivity.rs` |
| `tpl-firefly-iii` | Firefly III | `media_productivity.rs` |
| `tpl-goatcounter` | GoatCounter | `extra_services.rs` |
| `tpl-invoice-ninja` | Invoice Ninja | `media_productivity.rs` |
| `matomo` | Matomo | `analytics_automation.rs` |
| `metabase` | Metabase | `infrastructure.rs` |
| `tpl-openpanel` | OpenPanel | `extra_services.rs` |
| `plausible` | Plausible Analytics | `infrastructure.rs` |
| `posthog` | PostHog | `analytics_automation.rs` |
| `umami` | Umami | `analytics_automation.rs` |

### Automation & Workflow

| ID | Name | File |
|----|------|------|
| `activepieces` | Activepieces | `analytics_automation.rs` |
| `tpl-activepieces` | Activepieces | `sprint23.rs` |
| `tpl-hatchet` | Hatchet | `sprint24.rs` |
| `n8n` | n8n | `infrastructure.rs` |
| `tpl-n8n` | n8n | `sprint18.rs` |
| `tpl-n8n-ai` | n8n (AI-ready) | `ai_extras.rs` |
| `trigger-dev` | Trigger.dev | `analytics_automation.rs` |
| `tpl-trigger-dev` | Trigger.dev | `sprint23.rs` |
| `windmill` | Windmill | `analytics_automation.rs` |

### Business & CRM

| ID | Name | File |
|----|------|------|
| `tpl-crater` | Crater | `sprint19.rs` |
| `tpl-dolibarr` | Dolibarr | `sprint16.rs` |
| `tpl-dolibarr-erp` | Dolibarr ERP/CRM | `sprint18.rs` |
| `tpl-easyappointments` | EasyAppointments | `sprint24.rs` |
| `tpl-erpnext` | ERPNext | `sprint18.rs` |
| `tpl-formbricks` | Formbricks | `sprint15.rs` |
| `tpl-invoice-ninja-v5` | Invoice Ninja v5 | `sprint18.rs` |
| `tpl-kimai-timetracking` | Kimai Time Tracking | `sprint18.rs` |
| `tpl-limesurvey` | LimeSurvey | `sprint15.rs` |
| `tpl-mautic` | Mautic | `sprint15.rs` |
| `tpl-odoo` | Odoo | `sprint15.rs` |
| `tpl-taiga` | Taiga | `sprint16.rs` |
| `tpl-twenty-crm` | Twenty CRM | `sprint16.rs` |

### CMS & Headless API

| ID | Name | File |
|----|------|------|
| `tpl-classicpress` | ClassicPress | `sprint21.rs` |
| `tpl-cockpit-cms` | Cockpit CMS | `cms_extra.rs` |
| `directus` | Directus | `cms_communication.rs` |
| `tpl-dotcms` | dotCMS | `cms_extra.rs` |
| `tpl-drupal` | Drupal | `sprint25.rs` |
| `ghost` | Ghost | `cms_communication.rs` |
| `tpl-joomla` | Joomla | `sprint25.rs` |
| `tpl-keystonejs` | KeystoneJS | `cms_extra.rs` |
| `tpl-mediawiki` | MediaWiki | `sprint26.rs` |
| `payload-cms` | Payload CMS | `cms_communication.rs` |
| `tpl-pimcore` | Pimcore | `cms_extra.rs` |
| `strapi` | Strapi | `cms_communication.rs` |
| `tpl-wagtail` | Wagtail | `cms_extra.rs` |
| `wordpress` | WordPress | `cms_communication.rs` |
| `tpl-write-freely` | Write.freely | `sprint18.rs` |

### Communication & Social

| ID | Name | File |
|----|------|------|
| `tpl-apprise` | Apprise | `sprint19.rs` |
| `tpl-bluesky-pds` | Bluesky PDS | `sprint26.rs` |
| `tpl-chatwoot` | Chatwoot | `communication_extra.rs` |
| `tpl-discourse` | Discourse | `communication_extra.rs` |
| `tpl-element-web` | Element Web | `communication_extra.rs` |
| `tpl-humhub` | HumHub | `sprint16.rs` |
| `tpl-jitsi-meet` | Jitsi Meet | `sprint15.rs` |
| `tpl-lemmy` | Lemmy | `communication_extra.rs` |
| `tpl-listmonk` | Listmonk | `media_productivity.rs` |
| `tpl-listmonk-standalone` | Listmonk Standalone | `sprint18.rs` |
| `tpl-mastodon` | Mastodon | `communication_extra.rs` |
| `matrix-synapse` | Matrix Synapse | `cms_communication.rs` |
| `tpl-matrix-synapse` | Matrix Synapse | `sprint24.rs` |
| `mattermost` | Mattermost | `cms_communication.rs` |
| `tpl-nodebb` | NodeBB | `sprint24.rs` |
| `tpl-revolt` | Revolt | `sprint16.rs` |
| `rocketchat` | Rocket.Chat | `cms_communication.rs` |
| `tpl-rocketchat` | Rocket.Chat | `sprint24.rs` |
| `tpl-zulip` | Zulip | `communication_extra.rs` |

### Community

| ID | Name | File |
|----|------|------|
| `tpl-apache-answer` | Apache Answer | `sprint18.rs` |

### Database & Messaging

| ID | Name | File |
|----|------|------|
| `adminer` | Adminer | `infrastructure.rs` |
| `tpl-kafka` | Apache Kafka | `databases_tools.rs` |
| `tpl-cloudbeaver` | CloudBeaver | `sprint21.rs` |
| `tpl-nats` | NATS | `databases_tools.rs` |
| `pgadmin` | pgAdmin | `infrastructure.rs` |
| `tpl-qdrant` | Qdrant | `databases_tools.rs` |
| `tpl-questdb` | QuestDB | `sprint16.rs` |
| `rabbitmq` | RabbitMQ | `infrastructure.rs` |
| `redis` | Redis | `infrastructure.rs` |
| `tpl-valkey` | Valkey | `databases_tools.rs` |
| `tpl-weaviate` | Weaviate | `databases_tools.rs` |

### Databases (Additional)

| ID | Name | File |
|----|------|------|
| `tpl-apache-age` | Apache AGE | `sprint18.rs` |
| `tpl-cassandra` | Apache Cassandra | `sprint16.rs` |
| `tpl-arangodb` | ArangoDB | `sprint18.rs` |
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
| `appwrite` | Appwrite | `devtools.rs` |
| `tpl-budibase` | Budibase | `extra_services.rs` |
| `code-server` | Code Server | `devtools.rs` |
| `tpl-dashy` | Dashy | `extra_services.rs` |
| `tpl-docker-registry` | Docker Registry | `devtools_extra.rs` |
| `tpl-drawio` | Draw.io (Diagrams.net) | `misc_extras.rs` |
| `drone` | Drone CI | `infrastructure.rs` |
| `tpl-duplicati` | Duplicati | `extra_services.rs` |
| `tpl-excalidraw` | Excalidraw | `media_productivity.rs` |
| `forgejo` | Forgejo | `devtools.rs` |
| `tpl-freshrss` | FreshRSS | `media_productivity.rs` |
| `gitea` | Gitea | `infrastructure.rs` |
| `tpl-gitea-runner` | Gitea Actions Runner | `devtools_extra.rs` |
| `tpl-gitlab-ce` | GitLab CE | `devtools_extra.rs` |
| `tpl-gitness` | Gitness | `devtools_extra.rs` |
| `tpl-gotify` | Gotify | `media_productivity.rs` |
| `heimdall` | Heimdall | `infrastructure.rs` |
| `tpl-homarr` | Homarr | `extra_services.rs` |
| `tpl-homepage` | Homepage | `extra_services.rs` |
| `hoppscotch` | Hoppscotch | `devtools.rs` |
| `tpl-it-tools` | IT Tools | `misc_extras.rs` |
| `tpl-jenkins` | Jenkins | `extra_services.rs` |
| `mailhog` | Mailhog | `infrastructure.rs` |
| `tpl-medusa` | Medusa | `business.rs` |
| `tpl-memos` | Memos | `media_productivity.rs` |
| `tpl-miniflux` | Miniflux | `media_productivity.rs` |
| `nocodb` | NocoDB | `infrastructure.rs` |
| `tpl-ntfy` | Ntfy | `media_productivity.rs` |
| `tpl-onedev` | OneDev | `devtools_extra.rs` |
| `tpl-open-speed-test` | OpenSpeedTest | `misc_extras.rs` |
| `tpl-organizr` | Organizr | `extra_services.rs` |
| `outline` | Outline | `infrastructure.rs` |
| `tpl-paperless-ngx` | Paperless-ngx | `media_productivity.rs` |
| `tpl-penpot` | Penpot | `media_productivity.rs` |
| `pocketbase` | PocketBase | `devtools.rs` |
| `portainer` | Portainer | `infrastructure.rs` |
| `tpl-portainer` | Portainer CE | `extra_services.rs` |
| `tpl-stirling-pdf` | Stirling PDF | `media_productivity.rs` |
| `supabase` | Supabase | `devtools.rs` |
| `tpl-verdaccio` | Verdaccio | `devtools_extra.rs` |
| `watchtower` | Watchtower | `infrastructure.rs` |
| `tpl-woodpecker-ci` | Woodpecker CI | `devtools_extra.rs` |

### DevOps & Admin

| ID | Name | File |
|----|------|------|
| `tpl-caddy` | Caddy | `sprint19.rs` |
| `tpl-code-server` | Code-Server | `sprint19.rs` |
| `tpl-gitpod` | Gitpod Self-Hosted | `sprint19.rs` |
| `tpl-nginx-ui` | Nginx UI | `sprint19.rs` |
| `tpl-pgadmin` | pgAdmin | `sprint19.rs` |
| `tpl-phpmyadmin` | phpMyAdmin | `sprint19.rs` |
| `tpl-sentry` | Sentry | `sprint19.rs` |

### CI/CD & DevTools

| ID | Name | File |
|----|------|------|
| `tpl-argocd` | Argo CD | `sprint16.rs` |
| `tpl-concourse-ci` | Concourse CI | `sprint16.rs` |
| `tpl-github-runner` | GitHub Actions Runner | `sprint26.rs` |
| `tpl-gitlab-runner` | GitLab Runner | `sprint16.rs` |
| `tpl-harbor` | Harbor | `sprint15.rs` |
| `tpl-artifactory-oss` | JFrog Artifactory OSS | `sprint18.rs` |
| `tpl-nexus-oss` | Nexus Repository OSS | `sprint18.rs` |
| `tpl-sonarqube` | SonarQube | `sprint18.rs` |
| `tpl-tekton-dashboard` | Tekton Dashboard | `sprint16.rs` |
| `tpl-weave-gitops` | Weave GitOps | `sprint18.rs` |
| `tpl-woodpecker-agent` | Woodpecker CI Agent | `sprint16.rs` |
| `tpl-woodpecker-server` | Woodpecker CI Server | `sprint16.rs` |

### Forms & Surveys

| ID | Name | File |
|----|------|------|
| `tpl-heyform` | HeyForm | `sprint26.rs` |
| `tpl-opnform` | OpnForm | `sprint26.rs` |

### Documentation & Knowledge

| ID | Name | File |
|----|------|------|
| `tpl-batch2-bookstack` | BookStack | `documentation.rs` |
| `tpl-batch2-docmost` | Docmost | `documentation.rs` |
| `tpl-batch2-wikijs` | Wiki.js | `documentation.rs` |

### Gaming

| ID | Name | File |
|----|------|------|
| `tpl-minecraft-java` | Minecraft Java | `sprint22.rs` |
| `tpl-palworld` | Palworld | `sprint22.rs` |
| `tpl-satisfactory` | Satisfactory | `sprint22.rs` |
| `tpl-terraria` | Terraria | `sprint22.rs` |

### Infrastructure & Backend

| ID | Name | File |
|----|------|------|
| `tpl-adguard-home` | AdGuard Home | `networking_extra.rs` |
| `tpl-baserow` | Baserow | `sprint15.rs` |
| `tpl-docker-mailserver` | Mailserver (docker-mailserver) | `sprint26.rs` |
| `tpl-cloudflared` | Cloudflare Tunnel | `networking_extra.rs` |
| `tpl-diun` | Diun | `sprint21.rs` |
| `tpl-garage` | Garage | `databases_tools.rs` |
| `tpl-haproxy` | HAProxy | `networking_extra.rs` |
| `tpl-consul` | HashiCorp Consul | `sprint18.rs` |
| `tpl-nomad` | HashiCorp Nomad | `sprint18.rs` |
| `tpl-headscale` | Headscale | `networking_extra.rs` |
| `tpl-nginx-proxy-manager` | Nginx Proxy Manager | `devtools_extra.rs` |
| `tpl-restic-rest-server` | Restic REST Server | `sprint16.rs` |
| `tpl-seaweedfs` | SeaweedFS | `databases_tools.rs` |
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
| `tpl-peertube` | PeerTube | `sprint26.rs` |
| `tpl-photoprism` | PhotoPrism | `misc_extras.rs` |
| `tpl-batch2-seafile` | Seafile | `documentation.rs` |

### Monitoring & Observability

| ID | Name | File |
|----|------|------|
| `tpl-alertmanager` | Alertmanager | `monitoring_extra.rs` |
| `tpl-batch2-beszel` | Beszel | `media_monitoring.rs` |
| `tpl-beszel-agent` | Beszel Agent | `sprint21.rs` |
| `tpl-changedetection` | Changedetection.io | `media_productivity.rs` |
| `tpl-batch2-checkmate` | Checkmate | `media_monitoring.rs` |
| `tpl-checkmk` | Checkmk | `sprint15.rs` |
| `dozzle` | Dozzle | `infrastructure.rs` |
| `tpl-glances` | Glances | `media_productivity.rs` |
| `tpl-glitchtip` | Glitchtip | `sprint22.rs` |
| `tpl-grafana` | Grafana | `sprint25.rs` |
| `grafana-prometheus` | Grafana + Prometheus | `infrastructure.rs` |
| `tpl-loki` | Grafana Loki | `monitoring_extra.rs` |
| `tpl-tempo` | Grafana Tempo | `sprint18.rs` |
| `tpl-graylog` | Graylog | `sprint16.rs` |
| `tpl-healthchecks` | Healthchecks | `monitoring_extra.rs` |
| `tpl-jaeger` | Jaeger | `sprint16.rs` |
| `tpl-netdata` | Netdata | `monitoring_extra.rs` |
| `tpl-otel-collector` | OpenTelemetry Collector | `sprint16.rs` |
| `tpl-prometheus` | Prometheus | `sprint16.rs` |
| `tpl-pyroscope` | Pyroscope | `sprint18.rs` |
| `tpl-scrutiny` | Scrutiny | `sprint19.rs` |
| `tpl-batch2-signoz` | SigNoz | `media_monitoring.rs` |
| `tpl-signoz` | SigNoz | `sprint23.rs` |
| `tpl-speedtest-tracker` | Speedtest Tracker | `sprint19.rs` |
| `tpl-statping-ng` | Statping-NG | `monitoring_extra.rs` |
| `tpl-thanos` | Thanos | `sprint19.rs` |
| `uptime-kuma` | Uptime Kuma | `infrastructure.rs` |
| `tpl-victoria-metrics` | VictoriaMetrics | `monitoring_extra.rs` |
| `tpl-zabbix` | Zabbix | `sprint19.rs` |

### Networking & VPN

| ID | Name | File |
|----|------|------|
| `tpl-netbird` | Netbird | `sprint26.rs` |
| `nginx` | Nginx | `infrastructure.rs` |
| `traefik` | Traefik | `infrastructure.rs` |
| `tpl-wireguard-easy` | WireGuard Easy | `media_productivity.rs` |

### Productivity

| ID | Name | File |
|----|------|------|
| `tpl-affine` | AFFiNE | `sprint26.rs` |
| `tpl-dasherr` | Dasherr | `sprint19.rs` |
| `tpl-grocy` | Grocy | `sprint19.rs` |
| `tpl-hedgedoc` | HedgeDoc | `sprint19.rs` |
| `tpl-homebox` | Homebox | `sprint21.rs` |
| `tpl-joplin-server` | Joplin Server | `sprint24.rs` |
| `tpl-kanboard` | Kanboard | `sprint19.rs` |
| `tpl-karakeep` | Karakeep | `sprint21.rs` |
| `tpl-linkding` | Linkding | `sprint21.rs` |
| `tpl-mealie` | Mealie | `sprint19.rs` |
| `tpl-obsidian-remote` | Obsidian Remote (obsidian-remote) | `sprint25.rs` |
| `tpl-onlyoffice` | OnlyOffice Document Server | `sprint19.rs` |
| `tpl-openproject` | OpenProject | `sprint19.rs` |
| `tpl-pairdrop` | PairDrop | `sprint21.rs` |
| `tpl-rallly` | Rallly | `sprint18.rs` |
| `tpl-readeck` | Readeck | `sprint21.rs` |
| `tpl-redmine` | Redmine | `sprint19.rs` |
| `tpl-roundcube` | Roundcube | `sprint26.rs` |
| `tpl-ryot` | Ryot | `sprint21.rs` |
| `tpl-siyuan` | Siyuan Notes | `sprint24.rs` |
| `tpl-wekan` | Wekan | `sprint19.rs` |

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
| `tpl-batch2-authentik` | Authentik | `security_search.rs` |
| `tpl-casdoor` | Casdoor | `auth_identity.rs` |
| `tpl-crowdsec` | CrowdSec | `sprint15.rs` |
| `tpl-crowdsec-dashboard` | CrowdSec Dashboard | `sprint16.rs` |
| `tpl-etebase` | Etebase | `sprint25.rs` |
| `tpl-vault` | HashiCorp Vault | `sprint18.rs` |
| `tpl-batch2-infisical` | Infisical | `security_search.rs` |
| `tpl-batch2-keycloak` | Keycloak | `security_search.rs` |
| `tpl-logto` | Logto | `auth_identity.rs` |
| `tpl-ory-kratos` | Ory Kratos | `auth_identity.rs` |
| `tpl-passbolt` | Passbolt | `sprint16.rs` |
| `tpl-pihole` | Pi-hole | `extra_services.rs` |
| `tpl-pocket-id` | Pocket ID | `sprint23.rs` |
| `tpl-step-ca` | Step CA | `sprint19.rs` |
| `tpl-supertokens` | SuperTokens | `sprint26.rs` |
| `vaultwarden` | Vaultwarden | `infrastructure.rs` |
| `tpl-vaultwarden` | Vaultwarden | `sprint24.rs` |
| `tpl-wazuh` | Wazuh | `sprint16.rs` |
| `tpl-wazuh-manager` | Wazuh Manager | `sprint19.rs` |
| `tpl-zitadel` | ZITADEL | `auth_identity.rs` |

### Storage & Media Server

| ID | Name | File |
|----|------|------|
| `tpl-audiobookshelf` | Audiobookshelf | `media_productivity.rs` |
| `tpl-emby` | Emby | `extra_services.rs` |
| `filebrowser` | Filebrowser | `infrastructure.rs` |
| `tpl-filerun` | FileRun | `sprint18.rs` |
| `minio` | MinIO | `infrastructure.rs` |
| `nextcloud` | Nextcloud | `infrastructure.rs` |
| `tpl-plex` | Plex Media Server | `extra_services.rs` |
| `tpl-qbittorrent` | qBittorrent | `extra_services.rs` |
| `tpl-radarr` | Radarr | `extra_services.rs` |
| `tpl-sftpgo` | SFTPGo | `sprint19.rs` |
| `tpl-sonarr` | Sonarr | `extra_services.rs` |
| `tpl-storj-gateway` | Storj Gateway MT | `sprint18.rs` |
| `tpl-syncthing` | Syncthing | `media_productivity.rs` |
| `tpl-zipline` | Zipline | `sprint24.rs` |

### Other / Utility

| ID | Name | File |
|----|------|------|
| `tpl-batch2-calcom` | Cal.com | `project_mgmt.rs` |
| `tpl-comfyui` | ComfyUI | `sprint19.rs` |
| `tpl-focalboard` | Focalboard | `business.rs` |
| `tpl-kimai` | Kimai | `business.rs` |
| `tpl-batch2-leantime` | Leantime | `project_mgmt.rs` |
| `tpl-batch2-linkwarden` | Linkwarden | `project_mgmt.rs` |
| `tpl-llama-cpp-server` | llama.cpp Server | `sprint18.rs` |
| `tpl-milvus` | Milvus | `sprint18.rs` |
| `tpl-monica` | Monica | `business.rs` |
| `tpl-obsidian-livesync` | Obsidian LiveSync | `misc_extras.rs` |
| `tpl-openedai-speech` | OpenedAI Speech | `sprint19.rs` |
| `tpl-batch2-plane` | Plane | `project_mgmt.rs` |
| `tpl-silverbullet` | SilverBullet | `misc_extras.rs` |
| `tpl-stable-diffusion-webui` | Stable Diffusion WebUI | `sprint19.rs` |
| `tpl-batch2-stirling-pdf` | Stirling-PDF | `project_mgmt.rs` |
| `tpl-tabbyml` | Tabby | `sprint19.rs` |
| `tpl-batch2-tandoor` | Tandoor Recipes | `project_mgmt.rs` |
| `tpl-batch2-trilium` | Trilium | `project_mgmt.rs` |
| `tpl-batch2-vikunja` | Vikunja | `project_mgmt.rs` |

### Uncategorized

| ID | Name | File |
|----|------|------|
| `tpl-shlink` | Shlink (`Development`) | `sprint21.rs` |
| `tpl-slash` | Slash (`Development`) | `sprint21.rs` |
| `tpl-wakapi` | Wakapi (`Development`) | `sprint21.rs` |

---

*Generated from `src/db/seeders/*.rs`. Each template is a Docker Compose stack seeded into the database at startup.*
