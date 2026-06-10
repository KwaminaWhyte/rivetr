# Rivetr System Audit — 2026-06-10

Full security, performance, and UI/UX audit of the Rivetr codebase (backend, dashboard, public pages). Every finding below was verified against actual source code — no speculative issues.

> **Note:** The repo contains no marketing/landing pages. Public-facing surface is `login`, `setup`, and `invitations/accept`; everything else is the authenticated dashboard. The absence of a landing page is itself logged as a UX finding (UX-01).

## Audit Documents

| Document | Scope | Findings |
|----------|-------|----------|
| [security-audit.md](./security-audit.md) | Backend (Rust/Axum) + frontend + API surface | 3 Critical, 7 High, 10 Medium, 8 Low |
| [performance-audit.md](./performance-audit.md) | Backend hot paths, DB, background tasks + frontend bundle/rendering | 9 High, 11 Medium, 9 Low |
| [ui-ux-audit.md](./ui-ux-audit.md) | Public pages, dashboard IA, consistency, a11y, responsiveness | 6 High, 13 Medium, 7 Low |

## Fix Order (master plan)

### Phase 1 — Critical security (do before anything else, ship as emergency release)
- [x] SEC-C1: Put `/mcp` endpoint behind auth middleware — **DONE** (`src/api/mod.rs`)
- [x] SEC-C2: Fix 2FA bypass — pre-2FA temp token is a fully valid session — **DONE** (migration 111 + auth/ws/two_factor)
- [~] SEC-C3: Team-ownership authorization — **MOSTLY DONE.** Built `src/api/authz.rs` + a central `resource_authz_middleware` on the `/api` group that authorizes every `/{apps,servers,databases,services,projects,deployments}/:id/**` route (covers all per-resource sub-routes automatically). Scoped all 5 list endpoints + all 6 WS/SSE stream endpoints. Remaining: top-level config resources (build-servers, ssh-keys, git-providers, log-drains, destinations, ca-certs, notification-channels, s3), create-time `team_id` assignment, and `app_shares` view-vs-write enforcement. See security-audit.md → SEC-C3.
- [x] SEC-H4: Fix command injection in remote file browser — **DONE** (`validate_remote_path` in `src/api/filesystem.rs`)

> **Verification status:** code compiles (`cargo check`), 254/254 lib unit tests pass, `cargo clippy --all-targets` clean except one pre-existing unrelated warning in `webhooks/mod.rs`. **DEPLOYED 2026-06-10** to `root@187.124.50.183` (rivetr 0.11.0, x86_64) via `deploy-dev.sh --backend-only`. Migration 111 applied ("Migrations completed"), service active, proxy+API listening, no panics. Live-verified: MCP no-auth → 401 / +token → 200; unauth `/api/apps` → 401; admin app-by-id → 200; missing uuid → 404 (not 500); literal routes pass through; db/env reveal (admin) → 200.
>
> **Pre-existing bug noted (NOT from this work):** startup logs show repeated `WARN container_monitor::health: Failed to fetch running services error=no column found for name: cpu_limit` — present on the prior binary too. The `services` table is missing the `cpu_limit` column that migration 108 (`service_resource_limits`) should have added; that migration likely didn't apply on this server. Investigate separately.

### Phase 2 — High security
- [ ] SEC-H1: DockerHub webhook — require secret, validate `callback_url` (SSRF)
- [ ] SEC-H2: Fail closed when git webhook secrets unset
- [ ] SEC-H3: Rate limiter trusts spoofable `X-Forwarded-For`; add account lockout
- [ ] SEC-H5: OAuth login redirects with session token in URL query string
- [ ] SEC-H6: WS/SSE auth tokens in URLs (logged by proxies)
- [ ] SEC-H7: `PUT /api/white-label` missing admin check → CSS injection into login page

### Phase 3 — High performance
- [ ] PERF-H1: `proxy_logs` never pruned (unbounded DB growth)
- [ ] PERF-H2: Per-request DB insert + spawned task in proxy hot path
- [ ] PERF-H3/H4: Serial 1s-per-container docker stats (dashboard endpoint + background collectors)
- [ ] PERF-H5: Volume backup buffers whole archive in RAM + blocks runtime worker
- [ ] PERF-FH1: 57MB of stale build artifacts embedded in binary
- [ ] PERF-FH2: Dashboard N+1 stats polling (1 request per app/db per 15s)
- [ ] PERF-FH3: Deployment log viewer re-sorts entire array per WS message, unbounded
- [ ] PERF-FH4: Serialized auth round trips block first render

### Phase 4 — High UX
- [ ] UX-01: No public landing page / login page gives zero product context
- [ ] UX-02: Query errors silently swallowed — outage renders all apps as "stopped"
- [ ] UX-03: 59 icon-only buttons missing aria-labels
- [ ] UX-04: 12-tab app layout breaks on mobile (no overflow handling)
- [ ] UX-05: Settings submenu — 15 flat ungrouped entries
- [ ] UX-06: Fixed 4/5-column pickers unusable on phones

### Phase 5 — Medium/Low (batch by file/area)
See individual audit docs for the full checklists. Suggested batches:
- Security mediums: SSRF egress validation, SSH host key pinning, OAuth state enforcement, constant-time comparisons, localStorage→cookie session, CSP header
- Performance mediums: table retention (webhook_events, audit_logs, scheduled_job_runs, sessions), batched build-log inserts, broadcast-based log WS, conditional refetch intervals, Argon2 off the proxy hot path
- UX mediums: loading state standardization (skeletons), AlertDialog for all destructive actions, breadcrumb rewrite, password visibility toggles, responsive tables

## Severity Definitions
- **Critical** — exploitable now, full compromise or cross-tenant access; fix immediately
- **High** — serious security exposure, or perf/UX issue that degrades the product at realistic scale
- **Medium** — real but bounded impact, or requires preconditions
- **Low** — hardening, polish, hygiene
