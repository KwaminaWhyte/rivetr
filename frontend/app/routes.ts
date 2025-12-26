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
      route("settings", "routes/apps/$id/settings.tsx"),
      route("deployments", "routes/apps/$id/deployments.tsx"),
      route("logs", "routes/apps/$id/logs.tsx"),
      route("terminal", "routes/apps/$id/terminal.tsx"),
    ]),

    // Deployments
    route("deployments", "routes/deployments.tsx"),

    // Monitoring
    route("monitoring", "routes/monitoring.tsx"),

    // Notifications
    route("notifications", "routes/notifications.tsx"),

    // Settings
    ...prefix("settings", [
      index("routes/settings/_index.tsx"),
      route("webhooks", "routes/settings/webhooks.tsx"),
      route("tokens", "routes/settings/tokens.tsx"),
      route("ssh-keys", "routes/settings/ssh-keys.tsx"),
      route("git-providers", "routes/settings/git-providers.tsx"),
    ]),
  ]),
] satisfies RouteConfig;
