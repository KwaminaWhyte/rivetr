# Dashboard information architecture (target)

Decided 2026-11-05. Every page sits in exactly one place, every page has breadcrumbs, and the
same concept has the same name and URL shape for apps, databases and services.
Old URLs keep working through redirects (`routes/redirects.tsx` style loaders), so bookmarks
and links in emails/notifications do not break.

## Principles

- Resource URLs stay flat and id-based (`/apps/:id`, `/databases/:id`, `/services/:id`,
  `/servers/:id`). Hierarchy (team, project, environment) is shown by breadcrumbs, not the URL,
  because resources can move between projects and some have no project.
- One section, one page. No page reachable from two tabs. No "Settings" tab that hides a second
  tab strip: sections are listed in the contextual sidebar (desktop) or a section menu (mobile).
- Same section names across resource kinds: General, Configuration items, Deployments, Logs,
  Metrics, Terminal, Files, Backups, Operations, Danger zone.
- Instance-wide things live under `/settings`, things about me under `/account`,
  shared integrations under `/integrations`, credentials under `/keys`, the current team under `/team`.

## Global

| URL | Page | Breadcrumb |
|-----|------|------------|
| `/` | Dashboard (canvas or list) | Team / Dashboard |
| `/projects` | Projects list | Team / Projects |
| `/projects/:id` | Project resources | Team / Projects / {project} |
| `/projects/:id/environments` | Environments | ... / {project} / Environments |
| `/projects/:id/variables` | Project shared variables | ... / {project} / Variables |
| `/projects/:id/new` | New resource wizard | ... / {project} / New resource |
| `/templates` | Service templates | Team / Templates |
| `/monitoring` | Overview | Team / Monitoring |
| `/monitoring/costs` | Costs | Team / Monitoring / Costs |
| `/monitoring/proxy-logs` | Proxy access logs | Team / Monitoring / Proxy logs |
| `/monitoring/analytics` | Traffic analytics (E3) | Team / Monitoring / Analytics |
| `/tags` | Tags (E1) | Team / Tags |

## Infrastructure

| URL | Page |
|-----|------|
| `/servers` | Servers list (incl. localhost) |
| `/servers/:id` | General |
| `/servers/:id/images`, `/docker-cleanup`, `/security`, `/files`, `/terminal`, `/analytics` | Server sections |
| `/servers/build` | Build servers |
| `/servers/swarm` | Swarm |
| `/integrations/git` | Git providers (sources) |
| `/integrations/registries` | Container registries |
| `/integrations/s3` | S3 storage |
| `/integrations/destinations` | Docker destinations |
| `/integrations/tunnels` | Cloudflare tunnels |
| `/integrations/webhooks` | Outgoing webhooks |
| `/integrations/webhook-events` | Incoming webhook deliveries |

## Manage

| URL | Page |
|-----|------|
| `/team` | Current team general |
| `/team/members` | Members and invitations |
| `/team/roles` | Role presets and custom roles |
| `/team/permissions` | Resource permissions |
| `/team/audit` | Audit log |
| `/teams` | All my teams (switch, create) |
| `/notifications` | Channels |
| `/notifications/alerts` | Alert defaults |
| `/keys/ssh` | SSH keys |
| `/keys/api-tokens` | API tokens |
| `/keys/ca-certificates` | CA certificates |
| `/account` | Profile |
| `/account/security` | Password, 2FA, passkeys (E2) |
| `/account/appearance` | Theme customisation (per user) |
| `/account/preferences` | Date format, log lines, notifications, compact mode |

## Instance settings (admin)

`/settings` General, `/settings/advanced`, `/settings/updates`, `/settings/email`,
`/settings/authentication` (OAuth + SSO/OIDC + SAML), `/settings/backup`,
`/settings/branding` (white label, instance default theme), `/settings/ai`.

## Application `/apps/:id`

| Group | Section | URL | Replaces |
|-------|---------|-----|----------|
| | General | `` | overview (status, source, URLs, activity, resource usage) |
| Configuration | Source and build | `/build` | settings/_index (source part), settings/build |
| | Domains and network | `/domains` | network, settings/network |
| | Environment variables | `/environment` | env-vars |
| | Storage | `/storage` | settings/storage |
| | Healthcheck | `/healthcheck` | batch C |
| | Resources | `/resources` | resource limits on overview, settings/replicas, autoscaling |
| | Advanced | `/advanced` | settings/docker, settings/patches, settings/deployment |
| Deploy | Deployments | `/deployments`, `/deployments/:deploymentId` | |
| | Preview deployments | `/previews` | |
| | Rollback | `/rollback` | settings/snapshots + rollback retention |
| Observe | Logs | `/logs` | |
| | Metrics | `/metrics` | monitoring |
| | Terminal | `/terminal` | |
| | Files | `/files` | batch C |
| | Analytics | `/analytics` | E3 |
| Automate | Scheduled tasks | `/jobs` | |
| | Alerts | `/alerts` | settings/alerts |
| | Log drains | `/log-drains` | |
| Security | Security | `/security` | security (scans), settings/security (basic auth, IP rules) merged into one page with sections |
| | Sharing | `/sharing` | settings/sharing |
| | Operations | `/operations` | clone, move (batch C), tags |
| | Danger zone | `/danger` | delete |

## Database `/databases/:id`

General ``, Configuration (`/configuration`: version, image, credentials, init commands, SSL),
`/network` (public access, ports), `/storage`, `/extensions` (Postgres only), `/backups`,
`/import`, `/logs`, `/metrics`, `/terminal`, `/files`, `/operations`, `/danger`.

## Service `/services/:id`

General ``, `/compose` (compose file + raw mode), `/build`, `/environment`, `/network`
(domains, ports), `/storage`, `/deployments`, `/containers`, `/logs`, `/metrics`, `/terminal`,
`/files`, `/operations`, `/danger`.

## Breadcrumbs

Every route declares `handle.crumb` (string or function of loader/query data). The top bar
renders: Team switcher / [Project switcher] / [Environment switcher] / Resource switcher /
Section. Section pages show the section title as the last crumb. No page renders without
breadcrumbs, including settings, account, errors and 404.

## Persisted UI state (per browser, survives reloads)

Stored with `usePersistedState(key, default)` in localStorage (namespaced `rivetr:ui:*`,
wrapped in try/catch, versioned). Covers: sidebar collapsed, sidebar submenu open/closed,
projects/dashboard view (canvas/list/grid), canvas zoom and viewport per page, table page size,
sort and filters per table, log viewer options (follow, wrap, timestamps, lines, level filter),
terminal font size, collapsed cards, last selected tab inside a page, date range on metrics,
command palette recent items. Theme settings are per user (server) with a local cache to avoid
a flash on load.

## Implementation notes (2026-11-05)

Implemented on `feat/ui-ia`. Where the build differs from the target above, and why:

- **Breadcrumbs come from two central maps, not `handle.crumb`.** `lib/nav.ts` lists every
  page area (monitoring, servers, integrations, team, notifications, keys, account, settings)
  and `lib/resource-nav.ts` lists every app, database, service and server section. The same
  lists drive the sidebar, sub-navs, mobile menus, command palette and section headers, so a
  per-route `handle` would duplicate them. The dashboard 404 route and the layout error
  boundary render inside the shell with a "Page not found" / error crumb.
- **Old URLs** are listed in `lib/redirects.ts` and served by `routes/redirects.tsx`.
- **App General** keeps name and environment (editable); repository, branch and port moved to
  Source and build. The app "Security" group is labelled "Manage" because it also holds
  Operations and Danger zone.
- **Databases and services also get `/resources`** (CPU and memory limits) so "Resources"
  means the same thing for every kind. Database credentials stay on General.
- **Service `/compose`** also hosts raw compose mode; generated variables sit on
  Environment variables; the database dump import sits on Operations.
- **Not built yet** (no backing feature, nothing to move): database metrics and terminal,
  service storage, metrics and terminal (per container terminals live on Containers), app and
  server analytics, server terminal, `/integrations/registries` (registries are set per app or
  service), `/monitoring/analytics`, `/tags`, `/settings/authentication` SAML.
- **Team**: `/team/permissions` is not a separate page; resource permissions are edited per
  member on `/team/members`. `/team/variables` was added for team shared variables (mirrors
  `/projects/:id/variables`). `/team/audit` is the team's activity; the instance-wide audit log
  stays at `/settings/audit`. Team pages follow the team chosen in the team switcher;
  `/teams/:id` switches to that team and redirects to `/team`.
- **Appearance**: `/account/preferences` still has its theme card until the theme work lands
  on `/account/appearance`; drop it from preferences then.
