/**
 * Tab grouping for the app detail page (/apps/:id/*).
 * Child routes are unchanged; groups only decide how the tab strip is
 * rendered, so every existing URL keeps resolving to the same content.
 */

export interface AppTabItem {
  id: string;
  label: string;
  /** Path relative to /apps/:id ("" is the app root). */
  path: string;
}

export interface AppTabGroup {
  id: string;
  label: string;
  items: AppTabItem[];
}

export const APP_TAB_GROUPS: AppTabGroup[] = [
  {
    id: "overview",
    label: "Overview",
    items: [{ id: "general", label: "General", path: "" }],
  },
  {
    id: "deployments",
    label: "Deployments",
    items: [
      { id: "deployments", label: "Deployments", path: "/deployments" },
      { id: "previews", label: "Previews", path: "/previews" },
    ],
  },
  {
    id: "jobs",
    label: "Jobs",
    items: [{ id: "jobs", label: "Jobs", path: "/jobs" }],
  },
  {
    id: "observability",
    label: "Logs & Monitoring",
    items: [
      { id: "logs", label: "Logs", path: "/logs" },
      { id: "monitoring", label: "Monitoring", path: "/monitoring" },
      { id: "log-drains", label: "Log Drains", path: "/log-drains" },
      { id: "terminal", label: "Terminal", path: "/terminal" },
    ],
  },
  {
    id: "configuration",
    label: "Configuration",
    items: [
      { id: "env-vars", label: "Env Vars", path: "/env-vars" },
      { id: "network", label: "Network", path: "/network" },
      { id: "settings", label: "Settings", path: "/settings" },
    ],
  },
  {
    id: "security",
    label: "Security",
    items: [{ id: "security", label: "Security", path: "/security" }],
  },
];

export interface ActiveAppTab {
  group: AppTabGroup;
  item: AppTabItem;
}

/**
 * Resolve the active group and item for a pathname. Matching is prefix based
 * on whole path segments, so deep routes (/deployments/:deploymentId,
 * /settings/build, ...) highlight their parent. Unknown paths fall back to
 * the Overview tab, as the previous flat tab strip did.
 */
export function resolveActiveAppTab(
  pathname: string,
  basePath: string
): ActiveAppTab {
  const rest = pathname.startsWith(basePath) ? pathname.slice(basePath.length) : "";
  const normalized = rest.replace(/\/+$/, "");

  for (const group of APP_TAB_GROUPS) {
    for (const item of group.items) {
      if (item.path === "") continue;
      if (normalized === item.path || normalized.startsWith(item.path + "/")) {
        return { group, item };
      }
    }
  }

  const overview = APP_TAB_GROUPS[0];
  return { group: overview, item: overview.items[0] };
}
