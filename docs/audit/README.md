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
- [ ] SEC-C1: Put `/mcp` endpoint behind auth middleware (currently anyone can deploy/restart/read all apps)
- [ ] SEC-C2: Fix 2FA bypass — pre-2FA temp token is a fully valid session
- [ ] SEC-C3: Add team-ownership authorization to ALL resource handlers (IDOR: any user can read any team's secrets, open terminals in any container, delete any app)
- [ ] SEC-H4: Fix command injection in remote file browser (`$(...)` in path → RCE on managed servers)

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
