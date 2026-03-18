# Coolify Feature Gaps — What Rivetr Doesn't Have Yet

Observed from a live Coolify v4.0.0-beta.468 instance. Goal: achieve full parity and then exceed.

---

## 1. Proxy Layer

| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Multiple proxy choices (Traefik, Caddy) | ✅ Switch via UI | ❌ Built-in proxy only |
| Dynamic proxy configurations editor | ✅ Full file editor | ❌ |
| Proxy logs page (separate view) | ✅ | ✅ (`proxy_logs` table, migration 103, `GET /api/proxy/logs`, Settings → Proxy Logs page with filters + auto-refresh) |
| Proxy version upgrade UI | ✅ (shows new version, upgrade button) | ❌ |
| Proxy restart / stop from UI | ✅ | ✅ |
| Custom Traefik labels per-app (editable panel) | ✅ Full label editor with Traefik + Caddy labels | ✅ (`custom_labels` key-value editor in Settings → Network, migration 102) |

---

## 2. Application Deployment

### Deploy Sources
| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Public git repo | ✅ | ✅ |
| Private repo via GitHub App | ✅ | ✅ |
| Private repo via Deploy Key (SSH) | ✅ | ✅ |
| Docker Image from registry (no git) | ✅ | ✅ (via services) |
| Dockerfile without git | ✅ | ✅ (`inline_dockerfile` field, migration 098, textarea in Build Settings) |
| Docker Compose without git | ✅ | ✅ (raw compose mode) |

### Build Pack Options
| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Nixpacks | ✅ | ✅ |
| Static site | ✅ (dedicated type) | ✅ (`is_static_site` flag, migration 095) |
| Dockerfile | ✅ | ✅ |
| Docker Compose | ✅ | ✅ |

### Per-App Configuration
| Feature | Coolify | Rivetr |
|---------|---------|--------|
| www ↔ non-www redirect direction | ✅ (dropdown: allow both / redirect to www / redirect to non-www) | ✅ (`www_redirect_mode` field replaces `redirect_www`, 4-option dropdown in Domain Management card) |
| Generate domain button (auto-assign sslip.io subdomain) | ✅ | ✅ (`POST /api/apps/:id/generate-domain`, Generate button in domain card) |
| HTTP Basic Auth toggle per-app | ✅ | ✅ (`basic_auth_enabled` field, username/password hash, bypass on healthcheck path) |
| Custom Docker run options (--cap-add, --device, etc.) | ✅ | ✅ (Docker Options) |
| Network aliases per-app | ✅ | ✅ (`network_aliases` JSON field, passed to container runtime) |
| Watch paths (rebuild only when paths change) | ✅ | ✅ |
| Use a Build Server | ✅ | ✅ |
| Docker Registry push (image name + tag) | ✅ | ✅ |
| Pre/Post deployment commands | ✅ | ✅ |
| Container Labels editor (full editable panel) | ✅ (shows live Traefik/Caddy labels, editable) | ✅ (`custom_labels` JSON field, key-value editor in Settings → Network, migration `102_custom_labels.sql`) |
| Readonly labels / escape special chars options | ✅ | ❌ |

### Advanced App Options
| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Force HTTPS toggle | ✅ | ✅ |
| Enable Gzip compression | ✅ | ✅ |
| Strip Prefixes toggle | ✅ | ✅ (`strip_prefix` per-app, migration 097, Network settings UI) |
| Disable build cache | ✅ | ✅ (migration 094) |
| Inject build args into Dockerfile | ✅ | ✅ (SOURCE_COMMIT build arg) |
| Include source commit in build | ✅ | ✅ (migration 094) |
| Consistent container names | ✅ | ✅ (custom_container_name) |
| Custom container name | ✅ | ✅ (migration 094) |
| Drain logs toggle per-app | ✅ | ✅ |
| Git submodules support | ✅ | ✅ (migration 094) |
| Git LFS support | ✅ | ✅ (migration 094) |
| Shallow clone option | ✅ | ✅ (migration 094) |
| GPU support toggle | ✅ | ✅ (docker_gpus field) |
| Preview deployments (per-PR) | ✅ | ✅ |
| Allow public PR deployments | ✅ | ❌ |

### App Info & Operations
| Feature | Coolify | Rivetr |
|---------|---------|--------|
| App terminal (exec into container) | ✅ | ✅ |
| Live log streaming | ✅ | ✅ |
| Deployment history | ✅ | ✅ |
| Rollback to previous deployments | ✅ | ✅ |
| Healthcheck configuration | ✅ | ✅ |
| Resource limits (CPU/RAM) | ✅ | ✅ |
| Resource operations page (start/stop/restart from settings) | ✅ | ✅ |
| Per-app metrics (CPU/RAM charts) | ✅ | ✅ |
| Tags on apps | ✅ | ✅ |
| Links button (quick-access all app URLs) | ✅ (dropdown button) | ✅ (Links dropdown in app layout nav) |
| Git Source page (change git provider, branch) | ✅ | ✅ |
| Scheduled Tasks per-app | ✅ | ✅ |
| Outbound Webhooks per-app | ✅ (notify external URLs on events) | ✅ (webhook audit) |
| Persistent storage (volume mounts) | ✅ | ✅ |
| Environment variables with preview/build mode | ✅ | ✅ |
| Danger Zone (delete, purge) | ✅ | ✅ |

---

## 3. Server Management

### General
| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Multi-server | ✅ | ✅ |
| Server wildcard domain setting | ✅ | ✅ |
| Server timezone setting | ✅ | ✅ (`timezone` field, migration 096, Edit Server dialog) |
| Fetch server details (OS, Docker version, etc.) | ✅ (button to pull live info) | ✅ (`POST /api/servers/:id/fetch-details`, "Refresh details" in server actions dropdown, Server Details dialog) |

### Server Sub-Sections
| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Sentinel (monitoring sidecar agent) | ✅ (lightweight agent, metrics push) | ❌ |
| CA Certificate management | ✅ | ✅ (migration `100_ca_certs.sql`, `GET/POST /api/ca-certificates`, `DELETE /api/ca-certificates/:id`, Settings → CA Certificates) |
| Docker Cleanup (scheduled auto-cleanup config) | ✅ | ✅ |
| Destinations (Docker networks, configurable) | ✅ | ✅ (migration `101_destinations.sql`, `GET/POST /api/destinations`, assign apps to named Docker networks, Settings → Destinations) |
| Log Drains configuration | ✅ | ✅ |
| Server-level Metrics page | ✅ | ✅ |
| Docker Swarm support | ✅ (experimental) | ✅ |
| Server Patching (check + apply OS patches) | ✅ (experimental) | ✅ (security audit via UI) |
| Terminal Access control | ✅ (who can access terminal) | ❌ |
| Server terminal | ✅ | ✅ |
| Resources list (all apps on this server) | ✅ | ✅ |

### Proxy Configuration
| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Choose proxy (Traefik vs Caddy) | ✅ | ❌ (built-in Rust proxy) |
| Edit proxy docker-compose.yml directly | ✅ | ❌ |
| Dynamic configurations (per-app overrides) | ✅ | ❌ |
| Proxy logs | ✅ | ✅ (via app logs) |

---

## 4. Databases

Both have: PostgreSQL, MySQL, MariaDB, Redis, KeyDB, Dragonfly, MongoDB, ClickHouse

| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Elasticsearch | ✅ | ✅ |
| Database terminal (shell into DB) | ✅ | ✅ |
| DB backups to S3 | ✅ | ✅ |
| Import/restore from backup | ✅ | ✅ |

---

## 5. One-Click Service Templates

Coolify has **300+ templates**. Rivetr has ~**335 templates** (Sprint 3 + Sprint 19/20/21/22/23/24/25/26).

**Sprint 26 additions (11 new):** MediaWiki (CMS), SuperTokens (Auth/SSO), Netbird (Networking), AFFiNE (Productivity), HeyForm (Forms), OpnForm (Forms), GitHub Actions Runner (DevTools), Bluesky PDS (Communication), PeerTube (Media), Roundcube (Productivity), Mailserver/docker-mailserver (Infrastructure).

**Sprint 25 additions (5 new):** Joomla (CMS), Drupal (CMS), Grafana standalone (Monitoring), Etebase (Auth/SSO), Obsidian Remote (Productivity).

**Sprint 23 additions (8 new):** Flowise, Langflow, Open WebUI, AnythingLLM (AI/ML); Pocket ID (Auth/SSO); Activepieces, Trigger.dev (Automation); SigNoz (Monitoring).

**Sprint 22 additions (7 new):** Minecraft Java, Palworld, Terraria, Satisfactory (Gaming ✅); Argilla, Mage AI (AI/ML); Glitchtip (Monitoring).

**Sprint 21 additions (13 new):** Beszel Agent, ClassicPress, CloudBeaver, Diun, Homebox, Karakeep (formerly Hoarder), Linkding, PairDrop, Readeck, Ryot, Shlink, Slash, Wakapi. The templates gap vs Coolify has been significantly reduced.

### Categories where Coolify has templates Rivetr may still lack:

**AI / LLM:**
- ~~Argilla~~ ✅, ~~Mage AI~~ ✅, ~~AnythingLLM~~ ✅, Chroma, ~~Flowise~~ ✅ (with DBs), ~~Langflow~~ ✅, Langfuse, ~~LiteLLM~~ ✅, LocalAI, ~~MindsDB~~ ✅, Ollama+OpenWebUI, ~~Open WebUI~~ ✅, Rivet Engine, Unstructured, Weaviate

**Blockchain / Web3:**
- Bitcoin Core, Bluesky PDS

**Business / CRM:**
- Chaskiq, Chatwoot, Twenty (CRM), Dolibarr, ~~EasyAppointments~~ ✅, OrangeHRM, Kimai, Leantime

**Communication / Chat:**
- ~~Matrix Synapse~~ ✅ (with PostgreSQL or SQLite), Mattermost, ~~Rocket.Chat~~ ✅, Soju (IRC), ~~NodeBB~~ ✅

**Content / CMS:**
- Bookstack, ~~Drupal~~ ✅, ~~Joomla~~ ✅, ~~MediaWiki~~ ✅, Wiki.js, VVVeb (2 variants), ~~Affine~~ ✅

**Dev Tools:**
- Browserless, Code Server, Codimd, Datasette, Elasticsearch+Kibana, Faraday, Gitea (4 variants), Forgejo (4 variants), GitLab, Jenkins, Martin, Nexus (+ ARM), Onedev, Sequin, Trailbase, Windmill

**Finance:**
- Actualbudget, Budge, Firefly, Invoice Ninja, Sure

**File Storage / Sharing:**
- Chibisafe, Cloudreve, Garage, Nextcloud (4 variants), Seafile, Seaweedfs, ~~Zipline~~ ✅

**Media:**
- Audiobookshelf, Calibre Web, Emby, EmbyState, Jellyfin, Navidrome, Plex

**Monitoring / Observability:**
- ~~Glitchtip~~ ✅, Checkmate, Glances, ~~SigNoz~~ ✅, Uptime Kuma (3 variants), Grafana (with PostgreSQL), Goatcounter, Openpanel, Swetrix, Umami

**Productivity / Notes:**
- AppFlowy, ~~Joplin Server~~ ✅, Memos, ~~Siyuan~~ ✅, Triliumnext, Notesnook

**Project Management:**
- ~~Hatchet~~ ✅, Leantime, Plane, Vikunja (2 variants)

**Automation / Workflow:**
- ~~Activepieces~~ ✅, n8n (3 variants), Prefect, ~~Trigger.dev~~ ✅

**Forms / Surveys:**
- Formbricks, ~~Heyform~~ ✅, LimeSurvey, ~~Opnform~~ ✅

**Auth / SSO:**
- Authentik, Keycloak (with PostgreSQL), Logto, ~~Pocket ID~~ ✅ (2 variants), ~~Supertokens~~ ✅

**Gaming ✅:**
- ~~Minecraft Java~~ ✅, ~~Palworld~~ ✅, ~~Satisfactory~~ ✅, ~~Terraria~~ ✅

**Misc notable:**
- Cloudflared (tunnel), ~~GitHub Runner~~ ✅, ~~Netbird~~ ✅, Tailscale Client, Wireguard Easy
- Cal.com, Documenso, Stirling PDF, ~~Vaultwarden~~ ✅

---

## 6. Sources (Git Providers — Global Level)

| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Manage GitHub App integrations (global) | ✅ `/sources` page | ✅ (per-app setup) |
| Manage GitLab OAuth apps (global) | ✅ | ✅ |
| Gitea sources | ✅ | ✅ |
| Bitbucket sources | ✅ | ✅ (git provider OAuth + API token, clone via x-token-auth) |

---

## 7. Destinations (Docker Networks)

| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Named Docker networks (destinations) | ✅ (manage from `/destinations`) | ✅ (migration `101_destinations.sql`, Settings → Destinations, `GET/POST /api/destinations`) |
| Assign apps to specific networks | ✅ | ✅ (apps join the selected destination network instead of the default `rivetr` bridge) |
| Multiple networks per server | ✅ | ✅ |

---

## 8. Shared Variables

| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Team-level shared env vars | ✅ (shared-variables page) | ✅ (shared env vars feature) |
| Reference shared vars in app env vars | ✅ | ✅ |

---

## 9. Notifications

| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Email (SMTP) | ✅ | ✅ |
| Email (Resend API) | ✅ | ✅ (validation fixed, full backend + frontend already existed) |
| Discord | ✅ | ✅ |
| Telegram | ✅ | ✅ |
| Slack | ✅ | ✅ |
| Pushover | ✅ | ✅ (migration 041) |
| Webhook | ✅ | ✅ |
| Per-channel event selection | ✅ (per channel: deployments, backups, scheduled tasks, server events) | ✅ |
| Container status change alerts | ✅ | ✅ |
| Server disk usage alerts | ✅ | ✅ |
| Proxy version outdated alerts | ✅ | ❌ |
| Server patching alerts | ✅ | ❌ |

---

## 10. Keys & Tokens

| Feature | Coolify | Rivetr |
|---------|---------|--------|
| SSH Private Key management | ✅ (global pool, assign to servers) | ✅ |
| API Tokens | ✅ | ✅ |
| Finger printing / key info display | ✅ | ❌ |

---

## 11. OAuth / SSO

| Feature | Coolify | Rivetr |
|---------|---------|--------|
| GitHub OAuth | ✅ | ✅ |
| GitLab OAuth | ✅ | ✅ |
| Google OAuth | ✅ | ✅ |
| Discord OAuth | ✅ | ✅ |
| Azure AD OAuth | ✅ | ✅ |
| Bitbucket OAuth | ✅ | ✅ (OAuth login + git provider — fully implemented) |
| Authentik OIDC | ✅ | ✅ (generic OIDC) |
| Clerk | ✅ | ❌ |
| Zitadel | ✅ | ❌ |
| Infomaniak | ✅ | ❌ |

---

## 12. Instance Settings

| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Instance URL setting | ✅ | ✅ |
| Instance name | ✅ | ✅ |
| Instance timezone | ✅ | ✅ (stored in `instance_settings` as `instance_timezone`, configurable from Settings → General) |
| Public IPv4 / IPv6 config | ✅ | ❌ |
| Instance backup to S3 | ✅ | ✅ |
| Transactional email (SMTP config) | ✅ | ✅ |
| OAuth provider configs | ✅ | ✅ |
| Instance-level scheduled jobs | ✅ (cron-based, manage from UI) | ✅ |
| Auto-update Coolify/Rivetr | ✅ | ❌ |

---

## 13. Teams / Multi-Tenant

| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Multiple teams | ✅ | ✅ |
| Team switcher in nav | ✅ | ✅ |
| Invite members | ✅ | ✅ |
| Role-based access | ✅ | ✅ |
| Team-scoped resources | ✅ | ✅ |
| 2FA enforcement | ✅ | ✅ |

---

## 14. UI / UX Gaps

| Feature | Coolify | Rivetr |
|---------|---------|--------|
| Tags (global tag management page) | ✅ `/tags` page to manage all tags | ✅ |
| Resource search (by name or FQDN) | ✅ (search box in environment view) | ✅ |
| Environment clone | ✅ (Clone button in environment) | ✅ (Clone button per env tab, POST /api/projects/:id/environments/:env_id/clone) |
| Multiple environments per project | ✅ (e.g. production + staging) | ✅ |
| Breadcrumb navigation with dropdowns | ✅ (click any crumb to switch resource) | ✅ |
| Preferences panel (theme, etc.) | ✅ (top-right button) | ✅ (Settings → Preferences: theme, date format, log lines, notifications, compact mode — localStorage-based) |
| "Generate Domain" auto-assign | ✅ | ✅ (Generate button in domain management card) |
| Feedback / support button in nav | ✅ | ✅ (Feedback link in sidebar footer → GitHub Issues) |
| Version display in nav | ✅ (links to changelog) | ✅ (version + GitHub releases link in sidebar footer) |
| Dark/light mode | ✅ | ✅ |

---

## 15. Sentinel (Monitoring Agent)

Coolify's **Sentinel** is a lightweight sidecar agent container deployed on each server:
- Collects server metrics (CPU, RAM, disk, network) at configurable intervals
- Pushes metrics back to Coolify
- Configurable: rate (seconds), history (days), push interval
- Has its own token, restart button, and logs viewer
- Separate from the app containers

**Rivetr status:** Has resource monitoring but no dedicated sidecar agent concept. Worth implementing similarly.

---

## 16. GitHub Actions Self-Hosted Runner

Coolify has a one-click **GitHub Actions Runner** service template that deploys a self-hosted runner connected to your GitHub org/repo.

**Rivetr status:** ✅ Implemented in Sprint 26 (`tpl-github-runner`, `sprint26.rs`). Uses `myoung34/github-runner` — requires `GITHUB_URL` and `RUNNER_TOKEN`.

---

## Priority Summary

### Completed ✅:
- ~~Environment clone~~ ✅ (`POST /api/projects/:id/environments/:env_id/clone`, Clone button in UI)
- ~~Git submodules + LFS + shallow clone~~ ✅ (migration 094, Build Settings UI)
- ~~Disable build cache~~ ✅ (migration 094)
- ~~Include source commit in build~~ ✅ (migration 094)
- ~~Custom container name~~ ✅ (migration 094)
- ~~GPU support toggle~~ ✅ (docker_gpus field exists)
- ~~Pushover notification~~ ✅ (migration 041)
- ~~Sprint 21 templates~~ ✅ (13 new templates, total ~87)
- ~~Sprint 22 templates~~ ✅ (7 new templates: Minecraft Java, Palworld, Terraria, Satisfactory, Argilla, Mage AI, Glitchtip — total ~94)
- ~~Sprint 23 templates~~ ✅ (8 new templates: Flowise, Langflow, Open WebUI, AnythingLLM, Pocket ID, Activepieces, Trigger.dev, SigNoz — total ~102)
- ~~Sprint 24 templates~~ ✅ (11 new: Vaultwarden, LiteLLM, MindsDB, Matrix Synapse, Rocket.Chat, NodeBB, Zipline, Joplin Server, Siyuan Notes, Hatchet, EasyAppointments — total ~113)
- ~~Bitbucket OAuth + source~~ ✅ (OAuth login, git provider OAuth + API token, clone via x-token-auth)
- ~~Dockerfile without git~~ ✅ (`inline_dockerfile` field migration 098, skip git clone, Build Settings textarea)
- ~~Resend email notification~~ ✅ (validation fixed)
- ~~Version display in nav~~ ✅ (sidebar footer version link to GitHub releases)
- ~~Feedback button in nav~~ ✅ (sidebar footer Feedback → GitHub Issues link)
- ~~Theme toggle in top nav~~ ✅ (ThemeToggle added to header bar)
- ~~Server timezone setting~~ ✅ (`timezone` field migration 096, Edit Server dialog)
- ~~Strip Prefixes per-app~~ ✅ (`strip_prefix` migration 097, proxy strips prefix in HTTP + WebSocket, Network settings UI)
- ~~Static site build type~~ ✅ (`is_static_site` flag, migration 095, UI toggle in Build Settings)
- ~~Generate domain button~~ ✅ (`POST /api/apps/:id/generate-domain`, Generate button in domain card)
- ~~Links button~~ ✅ (Links dropdown in app layout nav, shows all app URLs)
- ~~www ↔ non-www redirect direction~~ ✅ (`www_redirect_mode` field, 4-option dropdown in Domain Management card)
- ~~Fetch server details~~ ✅ (`POST /api/servers/:id/fetch-details`, "Refresh details" in server actions, Server Details dialog)
- ~~CA Certificate management~~ ✅ (migration 100, `GET/POST /api/ca-certificates`, Settings → CA Certificates)
- ~~Destinations (Docker networks)~~ ✅ (migration 101, `GET/POST /api/destinations`, assign apps to named Docker networks, Settings → Destinations)
- ~~Instance timezone setting~~ ✅ (stored in `instance_settings` as `instance_timezone`, Settings → General)
- ~~Container Labels editor~~ ✅ (`custom_labels` JSON field, key-value editor in Settings → Network, migration 102)
- ~~Sprint 25 templates~~ ✅ (5 new: Joomla, Drupal, Grafana standalone, Etebase, Obsidian Remote — total ~315)
- ~~Sprint 26 templates~~ ✅ (11 new: MediaWiki, SuperTokens, Netbird, AFFiNE, HeyForm, OpnForm, GitHub Actions Runner, Bluesky PDS, PeerTube, Roundcube, Mailserver — total ~335)
- ~~Proxy logs page~~ ✅ (`proxy_logs` table migration 103, `GET /api/proxy/logs`, Settings → Proxy Logs with domain/status filters + auto-refresh toggle)
- ~~Preferences panel~~ ✅ (Settings → Preferences: theme, date/time format, default log lines, compact mode — localStorage-based, instant save)

### High Priority (remaining):
1. **More service templates** — AI (Langfuse, LocalAI, Chroma, Weaviate), CMS (Bookstack, Wiki.js), monitoring (Glances, Uptime Kuma), automation (n8n), auth (Authentik, Keycloak), media (Jellyfin, Navidrome); forms (~~Heyform~~ ✅, ~~Opnform~~ ✅, LimeSurvey, Formbricks)

### Medium Priority:
2. HTTP Basic Auth per-app toggle — ✅ already implemented (`BasicAuthCard` in app Security settings)
3. Clerk, Zitadel OAuth providers
4. ~~Preferences panel~~ ✅ (Settings → Preferences: theme, date format, log lines, compact mode)
5. Network aliases per-app — ✅ already implemented (`network_aliases` JSON field)

### Low Priority / Nice-to-have:
6. Terminal Access control (per-server)
7. Server Patching UI (auto-apply OS patches)
8. ~~GitHub Actions self-hosted runner template~~ ✅ (`tpl-github-runner`)
9. ~~Proxy logs page~~ ✅ (`proxy_logs` table migration 103, Settings → Proxy Logs)
10. SSH key fingerprint display
