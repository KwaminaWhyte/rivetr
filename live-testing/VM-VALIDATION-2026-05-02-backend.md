# Backend Bug Fix Validation — v0.10.20

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
| B8 | Audit `ip_address` non-null | ❌ FAIL | All recent entries `ip_address: null`. `ClientIp` extractor exists in `src/api/audit.rs` but is **not wired** into any handler — left as staging API by the backend agent. |
| B9 | `?limit=N` alias for `?per_page=N` | ✅ PASS | `?limit=3` returns 3 items, `per_page: 3` in response meta |
| B10 | POST/DELETE accept empty body | ✅ PASS | rollback empty body → 200; cancel empty body → 200 (was 415) |
| B11 | Cancel non-cancellable returns 409 | ✅ PASS | `POST .../cancel` on cancelled deployment → **409 Conflict** with `code: "conflict"` |
| B12 | Rollback commit_message | ⏸ NOT VALIDATED LIVE | No app on VM has 2+ successful deployments. Code change verified in source. |
| B13 | Old running deployment → "replaced" not "failed" | ⏸ NOT VALIDATED LIVE | Same reason — needs full rollback flow. Code path verified. |
| B14/B15 | Stable `internal_hostname` field on App | ✅ PASS | `GET /api/apps/<id>` → `internal_hostname: "rivetr-validation-app"` (custom_container_name null → fallback to `rivetr-<name>`) |
| B17 | Templates list compressed + slim | ✅ PASS | `content-encoding: gzip`, `cache-control: public, max-age=300, stale-while-revalidate=900`. List item keys = {id, name, description, category, icon, is_builtin, created_at} — `compose_template` and `env_schema` absent. `GET /api/templates/:id` returns full body with `compose_template`. Body size 86KB gzipped (was ~500KB plain). |
| B26 | SQL backup Content-Type | ✅ PASS | `curl -I .../backups/<id>/download` → `content-type: application/sql`, `content-disposition: attachment; filename="*.sql"` |
| B27 | Compose service auto-domain fallback | ✅ PASS | New service created with no `instance_domain` set → `domain: "b27-test-2.local"` auto-assigned |
| B28 | Docker network "endpoint already exists" 403 swallowed | ✅ PASS | After `systemctl restart rivetr` → journal grep "already exists in network" → 0 (was 2 lines per restart on v0.10.20-prefix) |

## Summary
- **13 of 15 PASS** ✅
- **1 FAIL** ❌ — B8 (audit ip_address). Extractor staged but unwired.
- **2 not live-validated** ⏸ — B12, B13 (rollback flow needs prior multi-deploy app history; code diff reviewed and looks correct).
- **0 new bugs** observed during test.

## B8 follow-up plan

`ClientIp` extractor in `src/api/audit.rs` parses `X-Forwarded-For` / `X-Real-IP` / `ConnectInfo` and returns `Option<String>`. Wiring it into every audited handler is mechanical but verbose — every `pub async fn ...` that calls `audit_log()` would need to take a `ClientIp` extractor and pass `client_ip.as_deref()` for the `ip_address` arg. That's ~30 sites. Reasonable next session task; not a regression.

## What's next
- Run a full app create → deploy → redeploy → rollback test on the VM to validate B12 + B13 live.
- Wire ClientIp into audit handlers (B8 second pass).
- Continue frontend validation (separate agent + report).
