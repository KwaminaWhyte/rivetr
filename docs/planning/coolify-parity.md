# Coolify v4.6 parity plan

Source: walkthrough of a live Coolify v4.6.0 instance on 2026-11-05, compared against Rivetr `main` at f954e44.
Goal: close every gap below and make the dashboard look and feel as polished as Coolify's new UI.

Status key: `[ ]` todo, `[x]` done.

## Shared rules for every batch

- Migrations: each batch owns its numbers (A 123, B 124, C 125, E1 126, E2 127, E3 128). SQL must be idempotent; register it at the end of `run_migrations` in `src/db/mod.rs`. ALTER TABLE needs a column-exists guard like the existing ones.
- Instance-level toggles live in the `instance_settings` key/value table (`src/db/models/instance_settings.rs`). A DB value overrides the matching `rivetr.toml` value; the toml stays the default.
- Routes: register in `src/api/mod.rs`, admin-only where the setting is instance-wide (follow the existing settings routes).
- Frontend: shadcn/ui + Tailwind v4, React Router v8, React Query. API calls in `frontend/app/lib/api/*`, re-exported from `index.ts`. Types in `frontend/app/types/`.
- Every destructive action uses the shared confirm dialog (batch A adds the two-step mode).
- Verify before calling a batch done: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cd frontend && npm run typecheck && npm run build`, plus a look in a browser.

## Batch A: instance settings

- [x] **Update settings editable in UI** (`settings/auto-update.tsx`, `src/engine/updater.rs`, `src/api/system/updates.rs`)
  - Channel: `stable` | `beta` (beta = include prereleases). Check-frequency cron, auto-update on/off, auto-update cron. Saved in instance_settings, read by the updater loop live (no restart).
  - "Check now" button with "Rivetr is up to date" / "vX available" status in the card header.
- [x] **Advanced settings page** (`/settings/advanced`)
  - Registration: `disabled` | `enabled` (new users only via invite when disabled; OAuth/SSO auto-signup obeys it). "Allow password registration" when OAuth is on.
  - Destructive action confirmation: `simple` | `two-step` (type the resource name). Applied by the shared confirm dialog.
    - Shared dialog: `frontend/app/components/confirm-dialog.tsx` (`ConfirmDialog`, `useConfirmationMode`). Existing per-page delete dialogs still need to switch to it as their pages are touched.
  - API access on/off (applies to `rvt_` tokens and the admin token from API clients; dashboard session unaffected) + allowed IPs/CIDRs list, with a warning banner when open to every source.
  - MCP server on/off (gate `/mcp`; default off for new installs, on for existing ones to avoid breaking users).
  - DNS validation on/off + custom DNS servers (used by `/api/domains/check`; use hickory-resolver with the given nameservers).
  - Outbound endpoints: allowed internal targets (CIDRs) and localhost blocked/allowed, wired into `src/api/ssrf.rs`.
- [x] **Public IP detection** on `/settings` General: "Detect" button fills public IPv4 and IPv6 (query ipify/ifconfig.co v4 and v6 endpoints with short timeouts). Values masked with show/copy buttons. Used as default `server_ip`.
- [x] **Transactional email in UI** (`/settings/email`): from name/address; SMTP (host, port, encryption none/STARTTLS/TLS, user, password encrypted at rest, timeout, EHLO domain) or Resend API key. "Send test email". DB overrides `EmailConfig` from toml.

## Batch B: server operations

- [ ] **Images page** per server (`/servers/:id/images`, and `localhost` for the local runtime): repo, tag, short ID with copy, created, size, used-by (container names), "Unused" badge, delete for unused; header chips "Total X GB" and "Reclaimable Y GB (Z%)"; search; pagination. Local via runtime API, remote via SSH `docker image ls --format json` + `docker ps -a`.
- [ ] **Docker cleanup** page per server: schedule (cron), trigger (`every schedule` | `disk above N%`), toggles with warning for unused volumes / unused networks / retained rollback images, "Run cleanup", and a recent executions list (status, started, duration, freed bytes, log). New table `docker_cleanup_runs`.
- [ ] **Server patching**: settings per server (check frequency, automatic security updates on/off for apt/dnf, maintenance window time), package list with "restarts Docker" warning for docker/containerd packages, per-package Update and Update All, reboot-required badge (`/var/run/reboot-required` or `needs-restarting -r`), Reboot Server with two-step confirm, last checked time.
- [ ] **Servers list**: columns Server, Resources count, Patches (N updates, Reboot required chips), Status (Reachable / Attention required / Unreachable).

## Batch C: app configuration

- [ ] **Structured healthcheck** (`apps/:id/settings` Healthcheck section): enable, type HTTP | command, method, scheme, host, port, path, expected status code, expected body text, interval, timeout, retries, start period. Used by the pipeline health check and mapped to the container HEALTHCHECK. The old single `healthcheck` path keeps working (migrated into the new fields).
- [ ] **Domains table extras**: per domain internal port, search indexing (adds `X-Robots-Tag: noindex, nofollow` in the proxy when off), DNS status column with "Check" action, redirect type temporary/permanent.
- [ ] **Global unsaved-changes bar**: a hook `useDirtyForm` + sticky bar component ("You have changes that haven't been saved yet", Reset, Save, Enter submits). Adopt on app, database, service and instance settings forms.
- [ ] **Container file browser** (`apps/:id/files`, also databases/services): list dir via `docker exec ls -la --time-style=+%s`, columns name, perms, owner, group, size, modified; open/download file, upload, new file/folder, delete with confirm. Mark as beta.
- [ ] **Clone resource**: target server, project, environment, optional "clone volume data" (copy volumes with a helper container). Move resource stays as is.

## Batch D: canvas and UI polish

- [ ] **Projects canvas** (`/projects?view=canvas`, also dashboard default): `@xyflow/react`, pan/zoom controls with %, fit view, project > environment > resource cards with type icon, source line (e.g. "Dockerfile · main"), URL, live status dot. Click opens a right side panel (type, status, details, URL, server, Open button). Filters: all projects / all environments. List/canvas toggle remembered per viewer.
- [ ] **App shell**: sidebar grouped like Coolify (top: Dashboard, Projects, Monitoring, Terminal; Infrastructure: Servers, Integrations, Shared variables; Manage: Team, Notifications, Keys and tokens, Tags, Settings), collapsible submenus, context sidebar when inside a resource or server (back arrow + section list), user menu at the bottom (profile, appearance, what's new, docs, log out).
- [ ] **Breadcrumb switchers**: Team / Project / Environment / Resource each with a dropdown to jump to siblings, plus a status pill and primary action (Redeploy with dropdown) on the right.
- [ ] **Page patterns**: card sections with an underlined title and an inner bordered body, consistent form grid (2 columns on desktop), consistent empty states (icon, title, one line, action), table style shared across lists, skeleton loaders. Apply to every route.
- [ ] **Deployment history table**: search, status filter, source (webhook/manual/API/rollback), short commit + message, started (relative), duration, server, pagination.

## Batch E: larger features

- [ ] **E1 Resource tags** (migration 126): tags on apps, databases, services; tag manager page `/tags`; filter lists/canvas by tag; "deploy all with tag" action.
- [ ] **E2 Passkeys** (migration 127): WebAuthn register/login via `webauthn-rs`, RP ID from instance domain; manage passkeys under security settings; toggle in Advanced settings.
- [ ] **E3 Traffic analytics** (migration 128): aggregate proxy access logs into per-minute rollups per app (requests, status classes, bytes, p50/p95 latency, top paths, top referrers); optional GeoIP country via a user-supplied MMDB path (no bundled database); per-app Analytics tab and per-server Analytics page; enable toggle per server.

## Out of scope

- Sentinel-style remote agent (Rivetr monitors over SSH and its own runtime).
- Cloud provider tokens / cloud-init scripts (separate decision).
