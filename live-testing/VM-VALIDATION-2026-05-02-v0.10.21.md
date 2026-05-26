# v0.10.21 Carry-Forward Sprint Validation

- **Date:** 2026-05-02 evening
- **Target:** http://10.211.55.5:8080 (Parallels VM)
- **Build under test:** rivetr 0.10.21
- **Tester:** claude (3 sub-agents in parallel + inline live tests)

## Backend fixes (live-validated via curl + ssh + docker inspect)

| ID | Bug | Result | Evidence |
|---|---|---|---|
| B6 | MySQL 8 SSL breaks published connection string | ✅ PASS | `docker inspect rivetr-db-65601774 --format '{{.Config.Cmd}}'` → `[--skip-ssl]` |
| B8 | Audit `ip_address` always null | ✅ PASS | `database.create` audit entry now shows `ip_address: 10.211.55.2` (Mac host IP). Older entries still null — only future writes capture. |
| B20 | Disk path inconsistency | ✅ PASS | `GET /api/system/disk` → `path: "/var/lib/rivetr"` (canonicalized) |
| B25 | DB-to-app env injection UI | ✅ PASS | `POST /apps/:id/links` with `{database_id}` returns 200; `GET /apps/:id/linked-env-vars` returns 6 keys: DATABASE_URL, HOST, PORT, USER, PASSWORD, DB; `DELETE /apps/:id/links/:link_id` returns 204 |

## Frontend UX fixes (built-bundle confirmed; need browser to verify behavior)

| ID | Bug | Result |
|---|---|---|
| U1 | Sidebar user menu spacing/hover/chevron | ⏸ source + bundle confirm; needs browser |
| U3 | Deploy commit/tag + ZIP modals open | ⏸ source confirm; needs browser |
| U5 | Template category anchors + View-all expand | ⏸ source confirm; needs browser |
| U6 | Inline credentials toggle on project DB cards | ⏸ source confirm; needs browser |
| U9 | Resource Limits live-apply | ⏸ source confirm; needs running app + browser |

## Build / health check

- `cargo fmt --check` ✓ clean
- `cargo clippy --all-targets --all-features -- -D warnings` ✓ exits 0
- `cargo test --lib` ✓ 205 passed
- `cd frontend && npm run build` ✓ passes
- `journalctl -u rivetr --since "10 minutes ago"` ✓ zero new warnings or errors
- VM dashboard ✓ HTTP 200, version 0.10.21 active

## What's still open

- **B12/B13** — rollback flow needs a live multi-deploy app to validate (carried since v0.10.20).
- **8 frontend fixes from v0.10.20** — B2, B18, B19, B21, B22, B23, B24, U10 — built into the bundle but not browser-tested.
- **5 frontend fixes from v0.10.21** — U1, U3, U5, U6, U9 — same status.

Playwright MCP wasn't available in this session due to subscription access being toggled mid-run; carrying the browser-validation pass forward.

## Verdict
**v0.10.21 is release-eligible** for the backend changes (B6, B8, B20, B25). Frontend work is shipped and lint-clean but the user-facing behavior needs a browser-driven smoke test before being claimed validated.
