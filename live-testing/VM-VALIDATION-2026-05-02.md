# Rivetr v0.10.20 — VM Validation (Side Panel + MariaDB)

- **Date:** 2026-05-02
- **Target:** http://10.211.55.5:8080 (Parallels Ubuntu 24.04 ARM64 VM)
- **Build under test:** rivetr 0.10.20 (commit graph: clippy + backend + frontend + docs merged onto v0.10.19)
- **Tester:** claude (sub-agent browser session — partial; agent hit rate limit before writing this report, so it was reconstructed from the screenshots the agent left in `live-testing/screenshots/`)

> **Note on completeness:** The browser-test sub-agent was rate-limited mid-session. It captured 11 screenshots (numbered 03 → 11) covering the MariaDB flow and a service-create moment but did not file a written report. This document summarises what the screenshots show and flags what *was not* exercised.

---

## 1. Coolify-style deploy log side panel

| Surface | Captured | Result |
|---|---|---|
| Apps — click Deploy | not captured (no `01-app-deploy*` screenshot) | not exercised |
| Services — click Start / Restart | partial — `11-service-created.png` shows a service in the Services list; no panel screenshot | not exercised end-to-end |
| Databases — click Start | yes — `04-mariadb-start-panel.png`, `04b-mariadb-after-stream.png`, `05-mariadb-panel-live.png`, `06-mariadb-after-close.png` | ✅ panel slides in on Start, streams logs, closes via X |

**Verdict:** Side panel works for managed databases (verified by screenshots showing the panel docked on the right with phase badge and log lines streaming). App + service surfaces were not exercised by the screenshots — needs a follow-up manual pass.

---

## 2. MariaDB managed database type

| Step | Screenshot | Result |
|---|---|---|
| Create MariaDB instance | `03-mariadb-after-start.png` | ✅ created and reached Running |
| Side panel during start | `04-mariadb-start-panel.png` → `04b-mariadb-after-stream.png` → `05-mariadb-panel-live.png` | ✅ panel streamed image pull + container start |
| Panel close | `06-mariadb-after-close.png` | ✅ dismissable via X |
| Network tab | `07-mariadb-network-tab.png` | ✅ shows `mysql://...:3306/...` connection string |
| Storage tab | `08-mariadb-storage-tab.png` | ✅ data path `/var/lib/mysql`, `mariadb-dump` backup commands |
| Backups tab | `09-mariadb-backups-tab.png`, `10-mariadb-backup-result.png` | ✅ manual backup created and listed |

**Verdict:** MariaDB managed-DB feature works end-to-end on the VM at v0.10.20. Connection string scheme, data path, and backup tooling are all correctly mariadb-flavoured.

---

## What was NOT validated (carry into next session)

- App deploy surface for the side panel (Deploy button on an app).
- Service start/restart surface for the side panel.
- App linking a MariaDB via env vars (manual copy, since auto-injection is still B25 — open).
- Persistence of the panel across navigation.
- Side-panel `Copy` and `Download` log-buffer buttons.
- All 22 backend bug fixes (B5–B17, B26, B27) — committed but not exercised on the VM.
- All 12 frontend fixes (B1, B2, B16, B18–B24, U2, U4, U10) — committed but not browser-verified.

---

## Critical fixes already verified live on v0.10.20

- **B3** — `/api/apps/:id/insights` no longer 503; verified earlier in this session via `journalctl -u rivetr` showing zero `tower_http: response failed classification=Status code: 503` entries since the fix shipped.
- **B4** — `container_monitor::check_services` SELECT no longer crashes; verified via `journalctl --since "1 minute ago"` showing zero `no column found for name: public_access` entries (was firing every 30 s on v0.10.19).

---

## Verdict

**MariaDB:** production-ready for the v0.10.20 release.
**Side panel:** verified for managed DBs only; needs manual pass on app + service surfaces before we trust it as "done" for those flows.
**Other v0.10.20 fixes (backend + frontend bug sweep):** code merged, builds green, but not exercised live — recommend a 30-minute manual sweep on the VM before tagging the release.
