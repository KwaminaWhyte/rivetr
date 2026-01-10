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

  // Protected routes with dashboard layout
  layout("routes/_layout.tsx", [
    index("routes/_index.tsx"),

    // Projects
    ...prefix("projects", [
      index("routes/projects/_index.tsx"),
      route(":id", "routes/projects/$id.tsx"),
      route(":projectId/apps/new", "routes/projects/$project-id.apps.new.tsx"),
    ]),

    // Apps (nested layout with tabs)
    route("apps/:id", "routes/apps/$id/_layout.tsx", [
      index("routes/apps/$id/_index.tsx"),
      route("network", "routes/apps/$id/network.tsx"),
      route("settings", "routes/apps/$id/settings.tsx"),
      route("deployments", "routes/apps/$id/deployments.tsx"),
      route("previews", "routes/apps/$id/previews.tsx"),
      route("logs", "routes/apps/$id/logs.tsx"),
      route("terminal", "routes/apps/$id/terminal.tsx"),
    ]),

    // Databases (nested layout with tabs)
    route("databases/:id", "routes/databases/$id/_layout.tsx", [
      index("routes/databases/$id/_index.tsx"),
      route("network", "routes/databases/$id/network.tsx"),
      route("storage", "routes/databases/$id/storage.tsx"),
      route("backups", "routes/databases/$id/backups.tsx"),
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

    // Notifications
    route("notifications", "routes/notifications.tsx"),

    // Settings
    ...prefix("settings", [
      index("routes/settings/_index.tsx"),
      route("webhooks", "routes/settings/webhooks.tsx"),
      route("tokens", "routes/settings/tokens.tsx"),
      route("ssh-keys", "routes/settings/ssh-keys.tsx"),
      route("git-providers", "routes/settings/git-providers.tsx"),
      route("notifications", "routes/settings/notifications.tsx"),
      route("teams", "routes/settings/teams.tsx"),
      route("teams/:id", "routes/settings/teams/$id.tsx"),
      route("audit", "routes/settings/audit.tsx"),
      route("alert-defaults", "routes/settings/alert-defaults.tsx"),
    ]),
  ]),
] satisfies RouteConfig;
