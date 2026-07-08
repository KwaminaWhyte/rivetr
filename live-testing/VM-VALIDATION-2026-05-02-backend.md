# Backend Bug Fix Validation: v0.10.20

- **Date:** 2026-05-02
- **Target:** http://10.211.55.5:8080 (Parallels VM, ARM64 Ubuntu 24.04)
- **Build under test:** rivetr 0.10.20
- **Tester:** claude (curl + ssh + journalctl, sub-agent rate-limited so done inline)

## Result table

| ID | Bug | Status | Evidence |
|---|---|---|---|
| B3 | `/api/apps/:id/insights` returns 404 not 503 | ✅ PASS | `curl -i .../api/apps/<id>/insights` → HTTP/1.1 **404** |
| B4 | container_monitor SELECT no longer crashes | ✅ PASS | journal grep "no column found for name" → 0 since v0.10.20 deploy |
| B5 | MariaDB user-provisioning warning silenced | ✅ PASS | journal grep "Could not provision MySQL\|Could not provision MariaDB" → 0 |
| B7 | New audit event types recorded | ✅ PASS | confirmed: app.create, app.update, database.create, env_var.create, deployment.cancel, service.create, token.create, token.delete |
| B8 | Audit `ip_address` non-null | ✅ FIXED (08edab8, v0.10.21) | Was FAIL on v0.10.20: `ClientIp` extractor existed but unwired. Fixed in commit `08edab8` — `ClientIp` now wired into every `audit_log()` call site across 20 handler files, passing `client_ip.as_deref()` for the `ip_address` arg. Not re-validated live yet (re-run audit query on VM to confirm non-null). |
| B9 | `?limit=N` alias for `?per_page=N` | ✅ PASS | `?limit=3` returns 3 items, `per_page: 3` in response meta |
| B10 | POST/DELETE accept empty body | ✅ PASS | rollback empty body → 200; cancel empty body → 200 (was 415) |
| B11 | Cancel non-cancellable returns 409 | ✅ PASS | `POST .../cancel` on cancelled deployment → **409 Conflict** with `code: "conflict"` |
| B12 | Rollback commit_message | ⏸ NOT VALIDATED LIVE | No app on VM has 2+ successful deployments. Code change verified in source. |
| B13 | Old running deployment → "replaced" not "failed" | ⏸ NOT VALIDATED LIVE | Same reason, needs full rollback flow. Code path verified. |
| B14/B15 | Stable `internal_hostname` field on App | ✅ PASS | `GET /api/apps/<id>` → `internal_hostname: "rivetr-validation-app"` (custom_container_name null → fallback to `rivetr-<name>`) |
| B17 | Templates list compressed + slim | ✅ PASS | `content-encoding: gzip`, `cache-control: public, max-age=300, stale-while-revalidate=900`. List item keys = {id, name, description, category, icon, is_builtin, created_at}, `compose_template` and `env_schema` absent. `GET /api/templates/:id` returns full body with `compose_template`. Body size 86KB gzipped (was ~500KB plain). |
| B26 | SQL backup Content-Type | ✅ PASS | `curl -I .../backups/<id>/download` → `content-type: application/sql`, `content-disposition: attachment; filename="*.sql"` |
| B27 | Compose service auto-domain fallback | ✅ PASS | New service created with no `instance_domain` set → `domain: "b27-test-2.local"` auto-assigned |
| B28 | Docker network "endpoint already exists" 403 swallowed | ✅ PASS | After `systemctl restart rivetr` → journal grep "already exists in network" → 0 (was 2 lines per restart on v0.10.20-prefix) |

## Summary
- **13 of 15 PASS** ✅ on v0.10.20.
- **1 FAIL → FIXED** ✅: B8 (audit ip_address). Fixed in `08edab8` (v0.10.21); see B8 row.
- **2 not live-validated** ⏸: B12, B13 (rollback flow needs prior multi-deploy app history; code diff reviewed and looks correct).
- **0 new bugs** observed during test.

## B8 resolution (update 2026-06-29)

Fixed in commit `08edab8` — "fix(B8): wire ClientIp extractor into every audit_log call site", shipped in v0.10.21 (merge `fe2caff`). `ClientIp` (parses `X-Forwarded-For` / `X-Real-IP` / `ConnectInfo`) is now wired into every `audit_log()` call across 20 handler files, passing `client_ip.as_deref()` for the `ip_address` arg. Original estimate was ~30 sites; actual ~62 calls. Not a regression at the time; now resolved.

## What's next
- Run a full app create → deploy → redeploy → rollback test on the VM to validate B12 + B13 live.
- ~~Wire ClientIp into audit handlers (B8 second pass).~~ ✅ Done (08edab8). Live re-validation of non-null `ip_address` still pending.
- Continue frontend validation (separate agent + report).
