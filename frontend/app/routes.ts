import {
  type RouteConfig,
  index,
  route,
  layout,
  prefix,
} from "@react-router/dev/routes";

export default [
  // Public routes (no auth required)
  route("login", "routes/login.tsx"),
  route("setup", "routes/setup.tsx"),
  route("logout", "routes/logout.tsx"),
  route("invitations/accept", "routes/invitations/accept.tsx"),

  // Protected routes with dashboard layout
  layout("routes/_layout.tsx", [
    index("routes/_index.tsx"),

    // Projects
    ...prefix("projects", [
      index("routes/projects/_index.tsx"),
      route(":id", "routes/projects/$id.tsx"),
      route(":id/environments", "routes/projects/$id.environments.tsx"),
      route(":id/env-vars", "routes/projects/$id.env-vars.tsx"),
      route(":projectId/apps/new", "routes/projects/$project-id.apps.new.tsx"),
    ]),

    // Apps (nested layout with tabs)
    route("apps/:id", "routes/apps/$id/_layout.tsx", [
      index("routes/apps/$id/_index.tsx"),
      route("network", "routes/apps/$id/network.tsx"),
      route("env-vars", "routes/apps/$id/env-vars.tsx"),
      route("settings", "routes/apps/$id/settings/_layout.tsx", [
        index("routes/apps/$id/settings/_index.tsx"),
        route("build", "routes/apps/$id/settings/build.tsx"),
        route("network", "routes/apps/$id/settings/network.tsx"),
        route("storage", "routes/apps/$id/settings/storage.tsx"),
        route("alerts", "routes/apps/$id/settings/alerts.tsx"),
        route("security", "routes/apps/$id/settings/security.tsx"),
        route("deployment", "routes/apps/$id/settings/deployment.tsx"),
        route("replicas", "routes/apps/$id/settings/replicas.tsx"),
        route("snapshots", "routes/apps/$id/settings/snapshots.tsx"),
        route("sharing", "routes/apps/$id/settings/sharing.tsx"),
        route("docker", "routes/apps/$id/settings/docker.tsx"),
        route("patches", "routes/apps/$id/settings/patches.tsx"),
      ]),
      route("deployments", "routes/apps/$id/deployments.tsx"),
      route("deployments/:deploymentId", "routes/apps/$id/deployment-detail.tsx"),
      route("previews", "routes/apps/$id/previews.tsx"),
      route("jobs", "routes/apps/$id/jobs.tsx"),
      route("logs", "routes/apps/$id/logs.tsx"),
      route("log-drains", "routes/apps/$id/log-drains.tsx"),
      route("monitoring", "routes/apps/$id/monitoring.tsx"),
      route("terminal", "routes/apps/$id/terminal.tsx"),
    ]),

    // Databases (nested layout with tabs)
    route("databases/:id", "routes/databases/$id/_layout.tsx", [
      index("routes/databases/$id/_index.tsx"),
      route("extensions", "routes/databases/$id/extensions.tsx"),
      route("network", "routes/databases/$id/network.tsx"),
      route("storage", "routes/databases/$id/storage.tsx"),
      route("backups", "routes/databases/$id/backups.tsx"),
      route("import", "routes/databases/$id/import.tsx"),
      route("logs", "routes/databases/$id/logs.tsx"),
      route("settings", "routes/databases/$id/settings.tsx"),
    ]),

    // Services (nested layout with tabs)
    route("services/:id", "routes/services/$id/_layout.tsx", [
      index("routes/services/$id/_index.tsx"),
      route("network", "routes/services/$id/network.tsx"),
      route("logs", "routes/services/$id/logs.tsx"),
      route("settings", "routes/services/$id/settings.tsx"),
    ]),

    // Service Templates (browse templates for reference)
    route("templates", "routes/templates.tsx"),

    // Monitoring
    route("monitoring", "routes/monitoring.tsx"),

    // Costs
    route("costs", "routes/costs.tsx"),

    // Infrastructure (top-level, no /settings/ prefix)
    route("servers", "routes/settings/servers.tsx"),
    route("build-servers", "routes/settings/build-servers.tsx"),
    route("ssh-keys", "routes/settings/ssh-keys.tsx"),
    route("swarm", "routes/settings/swarm.tsx"),
    route("tunnels", "routes/settings/tunnels.tsx"),

    // Access (top-level, no /settings/ prefix)
    route("teams", "routes/settings/teams.tsx"),
    route("teams/:id", "routes/settings/teams/$id.tsx"),
    route("git-providers", "routes/settings/git-providers.tsx"),
    route("webhooks", "routes/settings/webhooks.tsx"),
    route("webhook-events", "routes/settings/webhook-events.tsx"),
    route("tokens", "routes/settings/tokens.tsx"),

    // Settings
    ...prefix("settings", [
      index("routes/settings/_index.tsx"),
      route("white-label", "routes/settings/white-label.tsx"),
      route("notifications", "routes/settings/notifications.tsx"),
      route("audit", "routes/settings/audit.tsx"),
      route("alert-defaults", "routes/settings/alert-defaults.tsx"),
      route("auto-update", "routes/settings/auto-update.tsx"),
      route("backup", "routes/settings/backup.tsx"),
      route("s3", "routes/settings/s3.tsx"),
      route("security", "routes/settings/security.tsx"),
      route("oauth", "routes/settings/oauth.tsx"),
      route("sso", "routes/settings/sso.tsx"),
    ]),
  ]),
] satisfies RouteConfig;
