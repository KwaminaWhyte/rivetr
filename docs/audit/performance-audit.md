# Rivetr Performance Audit — 2026-06-10

Backend (Rust/Axum/SQLite/Tokio) + dashboard frontend (React Router v7 SPA, rust_embed). All findings verified in code. Rivetr's headline pitch is low resource usage (~30MB idle) — perf is a feature.

**Backend prefix:** `PERF-`  ·  **Frontend prefix:** `PERF-F`

---

## BACKEND

### HIGH

#### PERF-H1 — `proxy_logs` grows unbounded, no pruning anywhere
- **Where:** `src/proxy/handler.rs:150-178` (writer), `migrations/103_proxy_logs.sql`
- **Problem:** Every proxied request inserts a row. No `DELETE FROM proxy_logs` exists anywhere (unlike `stats_history`, `uptime_checks`, `resource_metrics` which all have cleanup). 50 apps @ 5 req/s ≈ 432k rows/day, ~39M/90 days → multi-GB SQLite file, every backup copies it, `SELECT COUNT(*)` in `src/api/proxy_logs.rs:105` (run on every logs page view) becomes a multi-second full scan.
- [ ] **Fix:** Daily pruning job (in `log_cleaner.rs`) deleting rows older than configurable retention (default 7-14 days) and/or row-count cap. Replace `COUNT(*)` pagination with keyset pagination on `id`.

#### PERF-H2 — One DB INSERT + spawned task per proxied request
- **Where:** `src/proxy/handler.rs:150-178`
- **Problem:** `log_request()` does `tokio::spawn` + individual `sqlx::query().execute()` per request. Pool is `max_connections(5)` (`src/db/mod.rs:71-72`), shared with API/pipeline/background. At 200 req/s → 200 tasks/s contending for 5 connections + single SQLite WAL writer; access logging starves deploy status updates; spawned-task backlog unbounded in memory during bursts.
- [ ] **Fix:** Send entries to a bounded `mpsc` (drop on full), single writer task batch-inserting (multi-row every 1s/100 entries). Make access logging opt-in.

#### PERF-H3 — `GET /api/system/stats` serial 2-sample docker stats per container
- **Where:** `src/api/system/health.rs:480-535` (handler), `src/runtime/docker/container.rs:709-735` (stats), route `src/api/mod.rs:804`
- **Problem:** `runtime.stats()` opens a Docker stats stream, waits for two samples (~1s each, Docker emits ~1/s regardless of the 100ms sleep). Called serially for every running deployment/database/compose container. 50 containers → ~50+ seconds per request, holding a hyper connection, re-run on every dashboard poll.
- [ ] **Fix:** Collect concurrently (`buffer_unordered(8)`). Serve from Prometheus gauges / shared in-memory cache that `StatsCollector` already populates every 15s, instead of hitting Docker per request.

#### PERF-H4 — Background stats tasks have same serial ≥1s-per-container cost
- **Where:** `src/engine/stats_collector.rs:88-126` (15s Prometheus), `:251-281` (5-min history)
- **Problem:** Both loops call `stats()` serially per container. 50 containers → a "15-second" cycle actually takes ~50-55s (ticks skipped via `MissedTickBehavior::Skip`), so a Docker stats stream is open nearly continuously — measurable daemon + rivetr CPU, contradicting the low-overhead pitch. History task duplicates work 5 min later.
- [ ] **Fix:** Parallelize with bounded concurrency. Switch to `one_shot: true` + compute CPU delta against cached previous sample per container (eliminates the 1s in-call wait). Share one snapshot between Prometheus task, history task, and API.

#### PERF-H5 — Volume backup blocks Tokio worker + buffers whole archive in RAM
- **Where:** `src/api/volumes.rs:357-377` (`backup_from_container`)
- **Problem:** `std::process::Command...output()` runs `docker cp` synchronously (blocks a runtime worker for minutes on large volumes), then gzips into a `Vec<u8>` response body. 5GB volume = multi-GB RSS spike + frozen worker; on 1-2 vCPU droplet (workers = cores) can stall the whole runtime incl. proxy.
- [ ] **Fix:** Use `tokio::process::Command`, stream `docker cp`/tar through async gzip encoder into HTTP response body (`Body::from_stream`), never materialize in memory.

### MEDIUM

#### PERF-M1 — Argon2 password verify inline in proxy request path
- **Where:** `src/proxy/handler.rs:560-565` (`check_basic_auth`)
- **Problem:** `Argon2::verify_password` (~20-100ms, memory-hard) runs synchronously per request to a basic-auth-protected app, incl. every static asset. 20 req/s → one CPU core saturated, worker threads blocked.
- [ ] **Fix:** Run in `spawn_blocking` + small TTL cache of successful `(user, hash-of-presented-creds)` (SHA-256 keyed, 60s TTL) so repeat requests skip Argon2.

#### PERF-M2 — Redirect-rule regexes recompiled per request
- **Where:** `src/proxy/handler.rs:577-584` (`apply_redirect_rules`)
- **Problem:** `Regex::new(&rule.source_pattern)` per rule per request — pure waste + heap alloc in hottest path.
- [ ] **Fix:** Compile regexes once when `Backend`/`RouteTable` is built; skip invalid patterns at load time.

#### PERF-M3 — No retention for webhook_events, audit_logs, team_audit_logs, scheduled_job_runs, expired sessions
- **Where:** writers `src/api/webhook_events.rs:54,150`, `src/db/models/audit.rs:188`, `src/api/teams/audit.rs:33`, `src/engine/scheduler.rs:225`; sessions deleted only on logout `src/api/auth.rs:234`
- **Problem:** None pruned. `webhook_events` stores full payloads; a `* * * * *` job = 525k `scheduled_job_runs` rows/year. 90 days → millions of dead rows.
- [ ] **Fix:** Extend daily `log_cleaner`: delete `webhook_events` older than N days, resolved audit rows past retention (configurable, e.g. 90 days), `scheduled_job_runs` keep last N per job, `sessions WHERE expires_at < datetime('now')`.

#### PERF-M4 — Remote build-server image transfer uses blocking std::process in async
- **Where:** `src/engine/pipeline/build.rs:1040-1090`
- **Problem:** `ssh docker save | gzip → docker load` driven with `std::process::Command`; `wait_with_output()`/`wait()` block a Tokio worker for the whole GB-image transfer. On 2-worker VPS halves capacity; proxy/API share workers.
- [ ] **Fix:** `tokio::process::Command` with piped async stdio, or wrap pipeline in `spawn_blocking`.

#### PERF-M5 — Build log pipeline: unbounded channel + one INSERT per line
- **Where:** `src/engine/pipeline/build.rs:634-641` (+ siblings at 381, 530, 1203, 1342, 1443)
- **Problem:** Build output sent line-by-line into unbounded channel; drain issues one INSERT transaction per line. 50k-line build = 50k write txns contending with WAL writer; if inserts lag, channel buffers build output unbounded in memory.
- [ ] **Fix:** Bounded channel (~10k, drop-oldest/coalesce) + batch-insert lines (multi-row per 100 lines / 250ms).

#### PERF-M6 — Deployment-log WS loads full history per client + polls SQLite every 500ms
- **Where:** `src/api/ws.rs:91-137`
- **Problem:** On connect, `fetch_all` materializes entire log history in memory (100k-line build = tens of MB per viewer); each client independently polls every 500ms. 10 tabs = 20 queries/s + duplicated buffers.
- [ ] **Fix:** Send history in chunks (LIMIT-paged / row stream); replace per-client polling with `tokio::sync::broadcast` fed by `add_deployment_log` (DB poll only as fallback).

### LOW

#### PERF-L1 — N+1 query in `list_apps_with_sharing`
- **Where:** `src/api/apps/sharing.rs:346-355`
- [ ] **Fix:** Single join query `SELECT a.*, t.name FROM app_shares s JOIN apps a ... WHERE s.shared_with_team_id = ?` (join at :332 already does most of it).

#### PERF-L2 — `audit_logs` distinct-value endpoints do full-table scans
- **Where:** `src/api/audit.rs:136-151`
- [ ] **Fix:** Mostly covered by PERF-M3 retention; or serve known enum values from constants instead of querying.

#### PERF-L3 — `record_stats_snapshot` fetches full rows just to count
- **Where:** `src/engine/stats_collector.rs:223-244`
- [ ] **Fix:** `SELECT COUNT(*)` for counts; `SELECT container_id` for the stats loop (apps rows are very wide post-migration-066).

#### PERF-L4 — Per-request String allocations even when DB logging disabled
- **Where:** `src/proxy/handler.rs:192-206,490-500`
- [ ] **Fix:** Build `ProxyLogEntry` lazily, gated on `self.db.is_some()`.

---

## FRONTEND

### HIGH

#### PERF-FH1 — 57MB of stale build artifacts embedded in binary (3,141 asset files)
- **Where:** `frontend/package.json:7` (postbuild), `src/api/mod.rs:78-80` (`#[folder = "static/dist/client"]`)
- **Problem:** `postbuild` copies `build/client` → `static/dist/client` without cleaning the destination. Every hashed chunk from every historical build is kept and embedded (5 copies of `AreaChart-*.js` @ 319KB, builds dating to March). Tens of MB of dead bytes in the release binary every deploy → binary size, embed memory, link time, upload time. Grows unboundedly.
- [ ] **Fix:** In postbuild, `fs.rmSync(dest, {recursive:true, force:true})` before copying so only current build's assets embed.

#### PERF-FH2 — Dashboard N+1 stats polling (1 request per app/db every 15s)
- **Where:** `frontend/app/components/running-services-card.tsx:53-123`
- **Problem:** `Promise.all(apps.map(app => api.getAppStats(app.id)))` @ `refetchInterval: 15000`; same per running DB. Plus 3 list queries @ 30s. 20 apps + 5 DBs → ~27 requests/15s, each hitting Docker stats server-side. Also `queryKey: ["app-stats", apps.map(a=>a.id)]` changes identity on every apps refetch, resetting cache.
- [ ] **Fix:** Add batch endpoint `GET /api/system/container-stats` (all running containers, one response); replace per-resource fan-out with single polled query. Key on a stable joined string.

#### PERF-FH3 — DeploymentLogs: unbounded array, full re-sort + re-render per WS message, no virtualization
- **Where:** `frontend/app/components/deployment-logs.tsx:58-66,116-132,228-244`
- **Problem:** `mergeLogs([data])` per WS message does `new Set(prev.map())` + `[...prev,...fresh].sort()` — O(n log n) over the entire array per line + one setState per line. No cap (unlike `runtime-logs.tsx` last-500). `scrollIntoView({behavior:"smooth"})` per line. Long builds freeze the tab — exactly when users watch.
- [ ] **Fix:** Batch WS messages via ref buffer flushed ~250ms (single setState); skip sort when appending ordered WS lines (sort only after REST merges); cap rendered lines (last 1000) or virtualize; use `behavior:"auto"` / set scrollTop directly.

#### PERF-FH4 — Startup waterfall: two serialized auth round trips block first render
- **Where:** `frontend/app/lib/auth.ts:91-135` (`useRequireAuth`), `routes/_layout.tsx:264,281-292`
- **Problem:** Layout renders "Loading..." until `useRequireAuth` finishes; `checkAuth` awaits `checkSetupStatus()` THEN `validateAuth()` sequentially. Route queries can't mount until both complete. SPA adds: HTML → JS → hydrate → fetch#1 → fetch#2 → render → route queries.
- [ ] **Fix:** `Promise.all` the two checks; skip `checkSetupStatus` when a token exists (valid token implies setup done); cache validation result in React Query.

### MEDIUM

#### PERF-FM1 — DeploymentLogs polls REST every 3s even while WS connected
- **Where:** `frontend/app/components/deployment-logs.tsx:91-99`
- **Problem:** `setInterval(fetchLogs, 3000)` whenever `isActive`, no `connected` check. Re-fetches entire log list each time, through O(n) dedupe. Duplicate transport on top of live WS.
- [ ] **Fix:** Disable interval while `connected === true` (fallback only); or pass `since`/`after_id` cursor for deltas.

#### PERF-FM2 — Index-keyed sliding-window log lists force full re-render per SSE line
- **Where:** `runtime-logs.tsx:126,231-233` (`key={i}`), `service-logs.tsx:90,204`, `databases/$id/logs.tsx:232`
- **Problem:** After 500-line cap, `slice(-500)` shifts every index so `key={i}` invalidates all 500 rows per line. SSE parsing calls `setLogs` once per `data:` line, not per chunk. `scrollIntoView({behavior:"smooth"})` compounds.
- [ ] **Fix:** Monotonic id as key; accumulate all lines from one chunk, single `setLogs` per chunk; non-smooth scroll.

#### PERF-FM3 — ResourceChart re-renders whole recharts tree every 10s; 319KB chunk in dashboard critical path
- **Where:** `frontend/app/components/resource-chart.tsx:113-126`, `routes/_index.tsx:270-273`
- **Problem:** Live cpu/memory props from 10s `system-stats` poll passed in but only used as fallback; component not `React.memo`'d → full recharts `AreaChart` re-renders each tick. `AreaChart-*.js` (319KB) loaded eagerly on the landing route.
- [ ] **Fix:** `React.memo` + stop passing live props (read latest inside via shared `["system-stats",teamId]` query, or pass only when `historyData` empty). Optionally `React.lazy` + `Suspense` the chart.

#### PERF-FM4 — High-frequency 5s polling on list pages with full payloads
- **Where:** `settings/tunnels.tsx:64`, `settings/proxy-logs.tsx:93`, `projects/$id.tsx:63`, `apps/$id/deployments.tsx:110`
- **Problem:** 5s polls regardless of in-flight state. Good pattern already exists (`apps/$id/_layout.tsx:199-206`: 2s while active, 30s otherwise).
- [ ] **Fix:** Convert these four to conditional `refetchInterval` callbacks (fast only during transitional state, 30s+ otherwise).

#### PERF-FM5 — Embedded static assets gzipped on the fly per request, no precompression
- **Where:** `src/api/mod.rs:1049` (`CompressionLayer::gzip(true)`), `:1074-1096`
- **Problem:** Raw embedded bytes gzipped on every cold request (server CPU on 300KB+ chunks). Mitigated by `immutable` cache on `assets/` so only first-visit pays. No brotli.
- [ ] **Fix (backend):** Precompress `.br`/`.gz` at build time (or enable brotli on `CompressionLayer`); serve precompressed variant by `Accept-Encoding`.

### LOW

#### PERF-FL1 — Duplicate xterm dependency
- **Where:** `frontend/package.json:34-35,54` — legacy `xterm@5.3.0` + `@xterm/xterm@6.0.0`; only `@xterm/xterm` imported (always dynamically).
- [ ] **Fix:** Remove `xterm` from dependencies.

#### PERF-FL2 — Dead route files with full page implementations (incl. polling)
- **Where:** `routes/apps/$id.tsx` (28KB, `refetchInterval` :128), `routes/services/$id.tsx` (21KB, `refetchInterval:5000` :116) — not registered in `routes.ts`.
- [ ] **Fix:** Delete both dead files (slow typechecking, mislead future edits).

#### PERF-FL3 — Breadcrumb context churns 2-3 setState per navigation, unmemoized provider value
- **Where:** `routes/_layout.tsx:267-276`, `lib/breadcrumb-context.tsx:18-20,34-43`
- **Problem:** Two overlapping effects both `setItems` (first fully shadowed); `useSetBreadcrumbs` abuses `useState(() => setItems(items))` (anti-pattern, never updates).
- [ ] **Fix:** Drop first effect (:267-271); rewrite `useSetBreadcrumbs` as `useEffect` keyed on serialized items; `useMemo` the provider value.

#### PERF-FL4 — Templates "all" view re-expands to ~1,300 cards, no virtualization
- **Where:** `routes/templates.tsx:284-299`
- **Problem:** Initial paint capped at 6/category, but `expandedCategories` removes the cap; category tabs render full filtered list as full Card components.
- [ ] **Fix:** Incremental "Show 24 more", or virtualize grid (cheap first step: CSS `content-visibility: auto` on cards).

#### PERF-FL5 — `fetchStatsHistory` bypasses shared API client
- **Where:** `routes/.../resource-chart.tsx:48-57`
- **Problem:** Raw `fetch` + direct `localStorage.getItem` instead of `apiRequest`; errors swallowed → `[]` cached as success.
- [ ] **Fix:** Move into `lib/api/system.ts` using `apiRequest`; let errors propagate to React Query.

---

## Verified healthy (no change needed)
- Release profile (`Cargo.toml`): `lto="thin"`, `codegen-units=1`, `strip=true`.
- Proxy body handling (`src/proxy/service.rs`): streamed `Incoming`/`BoxBody`, pooled backend conns, WS via `copy_bidirectional`.
- Route lookup (`src/proxy/mod.rs:219`): ArcSwap + DashMap O(1).
- Indexes exist: `deployments(app_id)`, `deployment_logs(deployment_id)`, `proxy_logs(ts DESC, host)`, `uptime_checks(checked_at)`, `resource_metrics(app_id, timestamp)`.
- Retention exists for: `deployment_logs` (30d), `stats_history/hourly/daily`, `uptime_checks`, `resource_metrics`, `cost_snapshots`, `alert_events`, `sso_states`.
- Rate limiter DashMap has periodic `cleanup_expired` — no leak.
- `start_logs` buffers: broadcast + bounded VecDeque.
- Deployments list (`src/api/deployments/handlers.rs:358`): paginated.
- React Query v5 pauses polls on hidden tab (`refetchIntervalInBackground` defaults false).
- Route-level code splitting auto-applied; xterm dynamically imported.
- Hashed-asset caching correct (`immutable` / `must-revalidate`).
- `recent-events` deduped via shared `["recent-events"]` key.
- lucide-react named imports tree-shake under Vite prod.
