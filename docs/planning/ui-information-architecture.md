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
