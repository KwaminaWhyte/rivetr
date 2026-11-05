/**
 * Dashboard URL tree. The file layout under routes/ mirrors these URLs.
 * Target structure and naming: docs/planning/ui-information-architecture.md.
 * Breadcrumb titles for every URL come from lib/nav.ts (pages) and
 * lib/resource-nav.ts (app, database, service and server sections).
 * Old URLs are kept alive by routes/redirects.tsx (see lib/redirects.ts).
 */
import {
  type RouteConfig,
  index,
  route,
  layout,
  prefix,
} from "@react-router/dev/routes";
import { LEGACY_REDIRECT_PATHS } from "./lib/redirects";

export default [
  // Public routes (no auth required)
  route("login", "routes/login.tsx"),
  route("setup", "routes/setup.tsx"),
  route("logout", "routes/logout.tsx"),
  route("forgot-password", "routes/forgot-password.tsx"),
  route("reset-password", "routes/reset-password.tsx"),
  route("register", "routes/register.tsx"),
  route("invitations/accept", "routes/invitations/accept.tsx"),

  // Old URLs: one redirect module serves them all (ids and query strings kept)
  ...LEGACY_REDIRECT_PATHS.map((path, i) =>
    route(path, "routes/redirects.tsx", { id: `redirect-${i}` })
  ),

  // Protected routes with dashboard layout
  layout("routes/_layout.tsx", [
    index("routes/_index.tsx"),

    // Projects
    ...prefix("projects", [
      index("routes/projects/_index.tsx"),
      route(":id", "routes/projects/$id.tsx"),
      route(":id/environments", "routes/projects/$id.environments.tsx"),
      route(":id/variables", "routes/projects/$id.variables.tsx"),
      route(":id/new", "routes/projects/$id.new.tsx"),
    ]),

    route("templates", "routes/templates.tsx"),
    route("tags", "routes/tags.tsx"),

    // Monitoring
    ...prefix("monitoring", [
      index("routes/monitoring/_index.tsx"),
      route("costs", "routes/monitoring/costs.tsx"),
      route("proxy-logs", "routes/monitoring/proxy-logs.tsx"),
      route("analytics", "routes/monitoring/analytics.tsx"),
    ]),

    // Applications
    route("apps/:id", "routes/apps/$id/_layout.tsx", [
      index("routes/apps/$id/_index.tsx"),
      // Configuration
      route("build", "routes/apps/$id/build.tsx"),
      route("domains", "routes/apps/$id/domains.tsx"),
      route("environment", "routes/apps/$id/environment.tsx"),
      route("storage", "routes/apps/$id/storage.tsx"),
      route("healthcheck", "routes/apps/$id/healthcheck.tsx"),
      route("resources", "routes/apps/$id/resources.tsx"),
      route("advanced", "routes/apps/$id/advanced.tsx"),
      // Deploy
      route("deployments", "routes/apps/$id/deployments.tsx"),
      route("deployments/:deploymentId", "routes/apps/$id/deployment-detail.tsx"),
      route("previews", "routes/apps/$id/previews.tsx"),
      route("rollback", "routes/apps/$id/rollback.tsx"),
      // Observe
      route("logs", "routes/apps/$id/logs.tsx"),
      route("metrics", "routes/apps/$id/metrics.tsx"),
      route("terminal", "routes/apps/$id/terminal.tsx"),
      route("analytics", "routes/apps/$id/analytics.tsx"),
      route("files", "routes/apps/$id/files.tsx"),
      // Automate
      route("jobs", "routes/apps/$id/jobs.tsx"),
      route("alerts", "routes/apps/$id/alerts.tsx"),
      route("log-drains", "routes/apps/$id/log-drains.tsx"),
      // Security and lifecycle
      route("security", "routes/apps/$id/security.tsx"),
      route("sharing", "routes/apps/$id/sharing.tsx"),
      route("operations", "routes/apps/$id/operations.tsx"),
      route("danger", "routes/apps/$id/danger.tsx"),
    ]),

    // Databases
    route("databases/:id", "routes/databases/$id/_layout.tsx", [
      index("routes/databases/$id/_index.tsx"),
      route("configuration", "routes/databases/$id/configuration.tsx"),
      route("network", "routes/databases/$id/network.tsx"),
      route("storage", "routes/databases/$id/storage.tsx"),
      route("resources", "routes/databases/$id/resources.tsx"),
      route("extensions", "routes/databases/$id/extensions.tsx"),
      route("backups", "routes/databases/$id/backups.tsx"),
      route("import", "routes/databases/$id/import.tsx"),
      route("logs", "routes/databases/$id/logs.tsx"),
      route("files", "routes/databases/$id/files.tsx"),
      route("operations", "routes/databases/$id/operations.tsx"),
      route("danger", "routes/databases/$id/danger.tsx"),
    ]),

    // Services (Docker Compose stacks)
    route("services/:id", "routes/services/$id/_layout.tsx", [
      index("routes/services/$id/_index.tsx"),
      route("compose", "routes/services/$id/compose.tsx"),
      route("build", "routes/services/$id/build.tsx"),
      route("environment", "routes/services/$id/environment.tsx"),
      route("network", "routes/services/$id/network.tsx"),
      route("resources", "routes/services/$id/resources.tsx"),
      route("deployments", "routes/services/$id/deployments.tsx"),
      route("containers", "routes/services/$id/containers.tsx"),
      route("logs", "routes/services/$id/logs.tsx"),
      route("files", "routes/services/$id/files.tsx"),
      route("operations", "routes/services/$id/operations.tsx"),
      route("danger", "routes/services/$id/danger.tsx"),
    ]),

    // Infrastructure: servers (":id" is a server id or "localhost")
    ...prefix("servers", [
      index("routes/servers/_index.tsx"),
      route("build", "routes/servers/build.tsx"),
      route("swarm", "routes/servers/swarm.tsx"),
      route(":id", "routes/servers/$id/_layout.tsx", [
        index("routes/servers/$id/_index.tsx"),
        route("images", "routes/servers/$id/images.tsx"),
        route("docker-cleanup", "routes/servers/$id/docker-cleanup.tsx"),
        route("security", "routes/servers/$id/security.tsx"),
        route("files", "routes/servers/$id/files.tsx"),
        route("analytics", "routes/servers/$id/analytics.tsx"),
      ]),
    ]),

    // Infrastructure: integrations shared by every project
    ...prefix("integrations", [
      route("git", "routes/integrations/git.tsx"),
      route("s3", "routes/integrations/s3.tsx"),
      route("destinations", "routes/integrations/destinations.tsx"),
      route("tunnels", "routes/integrations/tunnels.tsx"),
      route("webhooks", "routes/integrations/webhooks.tsx"),
      route("webhook-events", "routes/integrations/webhook-events.tsx"),
    ]),

    // Manage: the current team
    ...prefix("team", [
      index("routes/team/_index.tsx"),
      route("members", "routes/team/members.tsx"),
      route("roles", "routes/team/roles.tsx"),
      route("variables", "routes/team/variables.tsx"),
      route("audit", "routes/team/audit.tsx"),
    ]),
    route("teams", "routes/teams.tsx"),

    // Manage: notifications
    ...prefix("notifications", [
      index("routes/notifications/_index.tsx"),
      route("alerts", "routes/notifications/alerts.tsx"),
    ]),

    // Manage: keys and tokens
    ...prefix("keys", [
      route("ssh", "routes/keys/ssh.tsx"),
      route("api-tokens", "routes/keys/api-tokens.tsx"),
      route("ca-certificates", "routes/keys/ca-certificates.tsx"),
    ]),

    // Personal account (user menu)
    ...prefix("account", [
      index("routes/account/_index.tsx"),
      route("security", "routes/account/security.tsx"),
      route("appearance", "routes/account/appearance.tsx"),
      route("preferences", "routes/account/preferences.tsx"),
    ]),

    // Instance settings (admin)
    ...prefix("settings", [
      index("routes/settings/_index.tsx"),
      route("advanced", "routes/settings/advanced.tsx"),
      route("updates", "routes/settings/updates.tsx"),
      route("email", "routes/settings/email.tsx"),
      route("authentication", "routes/settings/authentication.tsx"),
      route("backup", "routes/settings/backup.tsx"),
      route("branding", "routes/settings/branding.tsx"),
      route("ai", "routes/settings/ai.tsx"),
      route("audit", "routes/settings/audit.tsx"),
    ]),

    // Anything else: a 404 page inside the dashboard shell, with breadcrumbs
    route("*", "routes/not-found.tsx"),
  ]),
] satisfies RouteConfig;
