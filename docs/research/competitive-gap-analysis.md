# Competitive gap analysis (2026-11-05)

Baseline: Rivetr 1.0.0 (632ee77) vs Coolify 4.6.0 (2026-11-02), Dokploy 0.31.0 (2026-11-05), CapRover 1.16.0, Dokku 0.38.34, Kamal 2.12.0, Railway, Render. Researched from each project's release notes and docs in November 2026; this file replaces the May 2026 analysis.

Impact H/M/L. Effort S (days), M (1 to 2 weeks), L (weeks).

| # | Gap | Impact | Effort | Who has it | Rivetr evidence |
|---|-----|--------|--------|-----------|-----------------|
| 1 | ~~Backup success/failure/missed-run notifications~~ Done | H | S | Coolify 4.3/4.4 | Done: `backup_succeeded`, `backup_failed`, `backup_missed` in `NotificationEventType`, rendered by every channel in `src/notifications/`; resource-scoped subscriptions (migration 133); missed-run check in `src/engine/backup_jobs/missed.rs` |
| 2 | HTTP/2 on proxy listeners | H | S | Traefik, Caddy, nginx, kamal-proxy | Done: ALPN h2/http1.1 on TLS and h2c on plain HTTP via `hyper_util` auto builder, `src/proxy/handler.rs` `serve`; tests `src/proxy/http2_tests.rs` |
| 3 | ~~Scoped API tokens (read/deploy/write/delete), pause and roll~~ **Done** | H | M | Coolify 4.6 | scopes read/deploy/write/delete/secrets/admin from one route table `src/api/scopes.rs` (coverage test `every_api_route_has_a_scope`), team/resource restriction, expiry, pause, roll with grace, last used IP; MCP tools scoped; migration 134; `docs/reference/api.md#api-token-scopes` |
| 4 | Per-domain certs via SNI resolver, remove 100-SAN cap | H | M | Coolify, Dokploy, Kamal | Done: `ResolvesServerCert` store `src/proxy/certs/store.rs`, per-domain orders with jittered renewal and backoff `src/proxy/certs/manager.rs`, migration 132 |
| 5 | Custom certificate upload per domain | H | S | Dokploy, kamal-proxy | Done: `POST /api/{apps,services}/{id}/certificates/{domain}/custom`, `src/api/certificates.rs`; Certificates card on Domains and service Network |
| 6 | ~~One-click restore of DB and volume backups from S3~~ Done | H | M | Coolify, Dokploy, Easypanel | Done: verified streaming download (size + SHA-256) and safety backup, then the rollback-with-data restore helpers, `src/engine/backup_jobs/restore.rs`; local and S3 backups listed together with Restore on the database Backups page |
| 7 | ~~Scheduled volume backups to S3 with restore~~ Done | H | M | Dokploy 0.24, Coolify 4.3 | Done: per-volume cron schedule, retention, local or S3, live or stop snapshot, history and restore in `src/engine/backup_jobs/volume.rs` (migration 133); App > Storage > Volume Backups |
| 8 | Wildcard certs via DNS-01 plus DNS provider APIs (Cloudflare, Route53) | H | M | Dokploy 0.30, Coolify 4.6 | Done: Cloudflare, Route 53, Hetzner, DigitalOcean in `src/proxy/certs/dns.rs`; `/integrations/dns` with instance wildcard |
| 9 | External secret managers at deploy time (Vault/OpenBao, Infisical, Doppler, AWS SM) | H | M | Coolify, Dokploy 0.30, Kamal | **Done**: `secret://<connection>/<path>#<key>` references and path imports resolved at deploy/restart/service start, `src/secret_managers/`, API `src/api/secret_managers.rs`, migration 135, UI `/integrations/secrets`; see `docs/reference/secret-managers.md` |
| 10 | ~~Granular RBAC ability presets~~ **Done** | H | M | Coolify 4.6, Dokploy Enterprise | 24 abilities and presets owner/admin/developer/deployer/viewer in `src/db/models/rbac.rs`, custom roles per team (`team_roles`, migration 134, `/team/roles`), central `require`/`authorize_action` in `src/api/authz.rs` with allow/deny overrides enforced on top |
| 11 | ~~One app across several servers behind the proxy~~ Done | H | L | Coolify, CapRover, Dokploy (Swarm) | Done: placement list per app (migration 136, `app_server_placements`), rolling multi-server rollout with per-target health checks and threshold rollback in `src/engine/multi_server/`, upstream pools with round robin, least connections, passive health and sticky cookie in `src/proxy/balancer.rs` |
| 12 | Published OpenAPI spec | M | S | Coolify, Dokploy, Render | none; hand-written `docs/reference/api.md` |
| 13 | Per-app proxy rate limiting and IP allow/deny | M | S | Traefik middlewares | API-only limiter `src/api/rate_limit.rs` |
| 14 | Proxy compression and custom headers | M | S | Traefik middlewares | missing |
| 15 | Official GitHub Action in the marketplace | M | S | Coolify, Railway, Render | generated workflow file only, `routes/apps/$id/build.tsx:837` |
| 16 | MCP write tools (env, domains, DBs, backups, rollback) | M | S | Coolify 4.6, Dokploy MCP, Railway | 11 tools, `src/mcp/tools.rs` |
| 17 | OTLP export of logs/metrics; CloudWatch, Loki sinks | M | M | Dokploy 0.31 (Vector), Coolify, Kamal, Render | 5 drains, `src/db/models/log_drain.rs` |
| 18 | Standalone CLI with full coverage | M | M | Coolify, Dokploy, Railway, Render | subcommands in server binary, `src/cli/mod.rs` |
| 19 | Hetzner (then DO) server provisioning | M | M | Coolify 4.4/4.5 | out of scope in `docs/planning/coolify-parity.md` |
| 20 | Forward-auth SSO in front of apps | M | M | Dokploy 0.29.8 | Basic Auth only, `src/api/basic_auth.rs` |
| 21 | Image vulnerability scan (Trivy) and SBOM | M | M | Coolify 4.6 | AI-based scan only, `src/api/ai_features.rs:531` |
| 22 | Continuous SQLite replication of the control plane | M | M | none (differentiator) | snapshot backups only |
| 23 | Template catalogue to ~600 | M | M | Dokploy 602, Easypanel 750+ | 365 |
| 24 | SCIM 2.0 | M | M | Dokploy Enterprise, Render | missing |
| 25 | Terraform / OpenTofu provider | M | L | Render official; Coolify, Dokploy community | missing; needs #12 first |

Deferred or not a goal: SAML (see `docs/research/saml-decision.md`), HTTP/3, WAF, PITR, Kubernetes, functions/workflows, hosted billing, control-plane active/standby.

## Where Rivetr leads

Single ~30MB binary with embedded SQLite; Podman support; preview environments on GitHub, GitLab, Gitea and Bitbucket; SSO, audit log, white label and fine-grained permissions without a paid tier; traffic analytics without an extra agent; rollback with pre-deploy database snapshots; cost tracking.
