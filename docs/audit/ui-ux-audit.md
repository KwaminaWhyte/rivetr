# Rivetr UI/UX Audit — 2026-06-10

Dashboard + public pages (React Router v7 + shadcn/ui + Tailwind). Audited by code review; grounded in source. No marketing/landing pages exist — public surface is `login`, `setup`, `invitations/accept`.

**Totals:** 6 High · 13 Medium · 7 Low

---

## PUBLIC PAGES

### UX-01 [High] — No public landing page; product domain serves login directly
- **Where:** `frontend/app/routes.ts` (no public index; unauthenticated `/` → `/login` via `useRequireAuth`)
- **Problem:** A visitor hitting `rivetr.site` gets a bare login form, zero context on what Rivetr is, features, or install. README is the only pitch. Competing PaaS (Coolify, Dokploy) ship at least a "what is this" page.
- [ ] **Task:** Add a lightweight public landing/about route — or at minimum enrich the login page right panel with feature bullets, GitHub link, docs link, install instructions — so unauthenticated visitors understand the product before being asked for credentials.

### UX-02 [Medium] — No password visibility toggle on login/setup
- **Where:** `routes/login.tsx:496-505`, `routes/setup.tsx:176-198`
- **Problem:** No Eye/EyeOff toggle, though the pattern exists elsewhere (`projects/$project-id.apps.new.tsx`, `settings/_index.tsx`). Setup requires a 12+ char password typed twice blind.
- [ ] **Task:** Add show/hide toggle (`aria-label="Show password"`) to password inputs on login, setup, invitation-accept registration.

### UX-03 [Medium] — Raw backend error text rendered verbatim on login/setup
- **Where:** `routes/login.tsx:186-188`, `routes/setup.tsx:90-92`
- **Problem:** `throw new Error(await response.text())` displays whatever server returns (JSON, HTML, stack fragment). A 500 shows raw JSON.
- [ ] **Task:** Map status codes to friendly messages (401→"Invalid email or password", 429→"Too many attempts", 5xx→"Something went wrong"); show server text only for known plain-text validation messages.

### UX-04 [Medium] — Setup form has no inline field-level validation
- **Where:** `routes/setup.tsx:74-78`
- **Problem:** Zod validation surfaces only the first issue, as a single top banner (`parsed.error.issues[0]?.message`). Mismatched-password error not shown next to Confirm field; failing field not highlighted/focused.
- [ ] **Task:** Render per-field errors under inputs (`Field`/`FieldDescription` already support it); set `aria-invalid`; focus first invalid field on submit.

### UX-05 [Low] — White-label branding not applied to login/setup
- **Where:** `routes/login.tsx:285-290,525-535`; `setup.tsx:128-133`
- **Problem:** White-labeling supported (`useWhiteLabel` in `app-sidebar.tsx:143`) but login/setup hardcode Rocket icon + "Rivetr". White-labeled instances show Rivetr branding at the front door.
- [ ] **Task:** Consume white-label config (logo, app name) on login, setup, invitation-accept; fall back to Rivetr defaults.

### UX-06 [Low] — Full-page "Loading..." text flash on every public page mount
- **Where:** `routes/login.tsx:266-272`, `setup.tsx:116-122`, `_layout.tsx:281-287`
- **Problem:** Client-side auth checks in `useEffect` render centered `animate-pulse` "Loading..." before any UI. Login waits on `checkSetupStatus()` + `validateAuth()` + OAuth providers serially. (See also PERF-FH4.)
- [ ] **Task:** Render static login shell immediately (logo, headings, disabled inputs), hydrate OAuth buttons when provider list resolves; run checks in parallel.

---

## NAVIGATION & IA

### UX-07 [High] — Settings submenu: 15 flat, ungrouped entries
- **Where:** `frontend/app/components/app-sidebar.tsx:66-89`
- **Problem:** One collapsible "Settings" holds 15 children (General, White Label, Security, Auto Updates, Authentication, SSO/OIDC, Backup & Restore, S3 Storage, Alert Defaults, Notifications, Audit Log, Proxy Logs, CA Certificates, Destinations, Preferences) — no grouping, no icons, no ordering. "Authentication" vs "SSO/OIDC" (same domain) and "Audit Log" vs "Proxy Logs" require scanning the whole list.
- [ ] **Task:** Group into 3-4 labeled sections (Instance / Security & Access / Data / Observability), or a settings landing page with card grid + local sub-nav.

### UX-08 [Medium] — Identical resources split across three sidebar sections
- **Where:** `app-sidebar.tsx:50-64`, `routes.ts:94-107`
- **Problem:** "SSH Keys" under Infrastructure; "Git Integrations"/"Webhooks"/"API Tokens" under Access — all credential/integration mgmt. "Webhook Events" (log) sits by "Webhooks" (config) while analogous "Proxy Logs"/"Audit Log" viewers live under Settings. Route *files* still in `routes/settings/` (e.g. `servers`) — half-finished IA migration.
- [ ] **Task:** Define one rule (config vs observability vs platform resources); move "Webhook Events" next to other log viewers; relocate route files out of `routes/settings/` to match mounted URLs.

### UX-09 [Medium] — Stale breadcrumb map + generic placeholder labels
- **Where:** `routes/_layout.tsx:82-109,257`
- **Problem:** `routeTitles` still has `/settings/servers`, `/settings/ssh-keys`, `/settings/webhooks`, `/settings/tokens` (mounted only at top level — dead config). Fallback crumb is literal "Page"; dynamic crumbs show "Apps"/"Project"/"Detail" until per-page overrides load; database/service layouts never list sub-tabs. ~130-line hand-maintained regex chain is the root cause.
- [ ] **Task:** Delete dead `/settings/*` entries; replace regex chain with React Router route `handle`/match-based breadcrumbs; never render "Page" — fall back to last path segment.

### UX-10 [Medium] — App detail: 12 top-level tabs overlap a nested 12-entry Settings sub-area
- **Where:** `routes/apps/$id/_layout.tsx:109-122`, `routes.ts:34-46`
- **Problem:** Tabs General/Env Vars/Network/Settings/Deployments/Previews/Jobs/Logs/Log Drains/Monitoring/Security/Terminal — and "Settings" contains 12 more incl. another "Network" and another "Security" (`apps/$id/network` vs `apps/$id/settings/network`; `apps/$id/security` vs `apps/$id/settings/security`). Users can't predict which "Network" holds domains vs ports.
- [ ] **Task:** Merge `apps/$id/network` with `settings/network`, `apps/$id/security` with `settings/security`; consider consolidating Logs+Log Drains and Deployments+Previews to ~8 tabs.

---

## CONSISTENCY

### UX-11 [High] — Query errors silently swallowed; failures render as "--" or stale zeros
- **Where:** `routes/_index.tsx:154,161`; `routes/monitoring.tsx:154,161,168`; `routes/projects/_index.tsx:79,112-121`
- **Problem:** `queryFn: () => api.getSystemStats().catch(() => null)`. Backend errors show "--"/"0%" with no failure indication; `isError` never true (only 10 `isError` usages in 60+ routes). Worse: `projects/_index.tsx:119-121` maps any app-status fetch error to `"stopped"` — an API outage displays every app as stopped, reading as a production incident.
- [ ] **Task:** Remove `.catch(() => null)` wrappers; use `isError` for inline error state ("Couldn't load stats — Retry"); distinguish "status unknown" from "stopped" in projects health logic.

### UX-12 [Medium] — Three competing loading-state patterns (skeleton/spinner/text)
- **Where:** skeleton `projects/_index.tsx:199-209`; spinner `settings/servers.tsx:841-844`; text `monitoring.tsx:243` ("Loading..." as 2xl-bold stat), `_index.tsx:225` ("..."), `_layout.tsx:283`
- **Problem:** Only 7 of 60+ routes use `Skeleton`. Monitoring renders literal "Loading..." styled as a stat value → layout jump when data arrives.
- [ ] **Task:** Standardize on shadcn `Skeleton` matching final content dimensions for cards/tables; reserve spinners for in-button pending state.

### UX-13 [Medium] — Mixed destructive-confirmation patterns
- **Where:** native `confirm` `settings/auto-update.tsx:66` ("restart the Rivetr server"); plain `Dialog` deletes `apps/$id/jobs.tsx:627-640`, `settings/ssh-keys.tsx:107`; proper `AlertDialog` in 28 other files
- **Problem:** Restarting the entire server (most disruptive action) gets an unstyled browser confirm, while deleting a job gets a styled dialog. Plain-Dialog deletes lack `AlertDialog`'s cancel-focused default → Enter-to-destroy.
- [ ] **Task:** Replace `window.confirm` (auto-update) + plain-Dialog deletes (jobs, ssh-keys, log-drains) with `AlertDialog`; for app/server/database deletes require typing the resource name.

### UX-14 [Low] — Error feedback split between inline banners and toasts, no rule
- **Where:** banner `projects/$project-id.apps.new.tsx:322-326`, `projects/_index.tsx:247-251`; toast `settings/servers.tsx:661-667`, `apps/$id/deployment-detail.tsx:145`
- **Problem:** App-creation failures show a banner atop a very long form (scrolled out of view when submitting from the bottom); settings use `toast.error`.
- [ ] **Task:** One rule — toasts for async mutation results, inline banners only for form validation; `scrollIntoView` the banner on long forms.

### UX-15 [Low] — Empty-state coverage uneven
- **Where:** good: `projects/_index.tsx:304-316`, `settings/servers.tsx:845-858`, `components/recent-events.tsx:107-118`. weaker: `templates.tsx:279-280` (text only, no clear-filters), `monitoring.tsx:554-560` (no retry action despite refetch in scope)
- [ ] **Task:** Add "Clear filters" to templates empty state, "Retry" to monitoring health-check empty state; audit webhook-events/audit/proxy-logs for first-run empty states.

---

## ACCESSIBILITY

### UX-16 [High] — 59 icon-only buttons, only 7 `aria-label`s in the whole app
- **Where:** e.g. `routes/databases/$id/backups.tsx:544-546` (download) + `:559-561` (delete) — `size="icon"` + Lucide glyph, no label/tooltip. Same in `settings/git-providers.tsx`, `databases/$id/storage.tsx`, `components/redirect-rules-card.tsx`, ~16 more.
- **Problem:** Screen-reader users hear "button" with no name; destructive delete-backup indistinguishable from download.
- [ ] **Task:** Add `aria-label` (+ ideally Tooltip) to every `size="icon"` button; add a lint rule or a wrapped `IconButton` with required label prop.

### UX-17 [Medium] — Color-only status indicators in Recent Events + project cards
- **Where:** `components/recent-events.tsx:124-128` (2.5px colored dot, only signal; `title` not exposed to keyboard/SR), `components/project-card.tsx:14,35`
- **Problem:** Red-green colorblind users can't distinguish failed vs succeeded. `components/projects/apps-tab.tsx:46` does it right (text labels).
- [ ] **Task:** Pair every status dot with visible text or `sr-only` span; add distinct icons (check/x/spinner) for success/failure/building.

### UX-18 [Medium] — Custom source/build-type pickers are buttons with no selected-state semantics
- **Where:** `routes/projects/$project-id.apps.new.tsx:411-472,535+,768`
- **Problem:** `<button type="button">` grids whose selected state is border-color only (`border-primary bg-primary/5`). No `aria-pressed`, no `role="radiogroup"`, no arrow-key nav. Functionally radio groups.
- [ ] **Task:** Convert to shadcn `RadioGroup` with card-styled items, or add `role="radiogroup"`/`role="radio"` + `aria-checked` + roving tabindex.

### UX-19 [Low] — Status communicated by text color only on monitoring stats
- **Where:** `routes/monitoring.tsx:206-217` (CPU steal severity as text color), `:100` (`UsageGauge` 75/90% thresholds as ring color)
- [ ] **Task:** Add a small text/badge severity label ("High"/"Critical") next to color-coded values and gauges.

---

## RESPONSIVENESS

### UX-20 [High] — App detail's 12-tab `TabsList` has no overflow handling on small screens
- **Where:** `routes/apps/$id/_layout.tsx:664-680` + `components/ui/tabs.tsx` (no overflow on `TabsList`)
- **Problem:** 12 triggers in `w-full justify-start` flex row → on a phone compress to unreadable slivers or clip; no `overflow-x-auto`, no scroll affordance, no collapse to Select. Same for database (8 tabs), service (5 tabs).
- [ ] **Task:** Wrap app/database/service `TabsList` in `overflow-x-auto` + `flex-nowrap` (or `Select` below `md:`); verify deployment-detail + settings sub-tab rows too.

### UX-21 [High] — Source/build-type pickers fixed at 4/5 columns, no breakpoints
- **Where:** `routes/projects/$project-id.apps.new.tsx:410,535,768`
- **Problem:** `grid-cols-4` / `grid-cols-5` at all widths. On 375px phone each build-type card gets ~60px — icon+label+description clip/wrap. Other grids on the page use `md:grid-cols-3`.
- [ ] **Task:** `grid-cols-2 sm:grid-cols-3 lg:grid-cols-5` (build type) and `grid-cols-2 lg:grid-cols-4` (source).

### UX-22 [Medium] — Wide tables rely solely on wrapper `overflow-x-auto`, no mobile alternative
- **Where:** 21 routes use `<Table>` (e.g. `settings/servers.tsx:860-869` — 6 cols + 160px actions); `components/ui/table.tsx:9` is the only overflow handling
- **Problem:** Horizontal scroll works but status + actions scroll out of view together; no `hidden md:table-cell` anywhere.
- [ ] **Task:** For highest-traffic tables (servers, deployments, backups, tokens) hide secondary columns below `md:` or render a stacked card list on mobile.

### UX-23 [Medium] — Monitoring detail panels use rigid horizontal layout
- **Where:** `routes/monitoring.tsx:460-468,505-513`
- **Problem:** `flex items-center justify-between gap-8` pairs a fixed 128px gauge + stats column + flex-1 detail grid, no `flex-col` below `sm:` → detail grid crushed on phones.
- [ ] **Task:** `flex flex-col sm:flex-row` (or `grid gap-6 sm:grid-cols-[auto_1fr]`) so the gauge stacks above the breakdown.

### UX-24 [Low] — Dashboard header action row can collide on small screens
- **Where:** `routes/_index.tsx:192-219`
- **Problem:** `flex items-center justify-between` with 3xl title + "Quick Deploy" button, no wrap allowance → title wraps awkwardly on ~360px.
- [ ] **Task:** Add `flex-wrap gap-4` (or `flex-col sm:flex-row`) to dashboard, monitoring, projects page headers.

---

## Summary

| Severity | Count |
|----------|-------|
| High | 6 |
| Medium | 13 |
| Low | 7 |

**Top priorities:** (1) UX-11 stop swallowing query errors — it misreports app status as a production incident; (2) UX-16 add aria-labels to 50+ unlabeled icon buttons; (3) UX-20/UX-21 fix 12-tab overflow + fixed-column grids that break the core new-app and app-detail flows on mobile; (4) UX-07/UX-10 rationalize Settings IA + duplicated Network/Security tabs; (5) UX-01 give the product a public face.
