/**
 * Tab grouping for the app detail page (/apps/:id/*).
 * Child routes are unchanged; groups only decide how the tab strip is
 * rendered, so every existing URL keeps resolving to the same content.
 */
import type { LucideIcon } from "lucide-react";
import {
  Settings,
  Hammer,
  Network,
  HardDrive,
  Bell,
  Shield,
  Rocket,
  Copy,
  Camera,
  Container,
  FileDiff,
  HeartPulse,
} from "lucide-react";

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
      { id: "analytics", label: "Analytics", path: "/analytics" },
      { id: "log-drains", label: "Log Drains", path: "/log-drains" },
      { id: "terminal", label: "Terminal", path: "/terminal" },
      { id: "files", label: "Files", path: "/files" },
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

export interface AppSettingsTab {
  id: string;
  label: string;
  /** Path relative to /apps/:id/settings ("" is the General page). */
  path: string;
  icon: LucideIcon;
}

/** Sub-pages of /apps/:id/settings. Shared by the settings tab strip and the contextual sidebar. */
export const APP_SETTINGS_TABS: AppSettingsTab[] = [
  { id: "general", label: "General", path: "", icon: Settings },
  { id: "healthcheck", label: "Health", path: "/healthcheck", icon: HeartPulse },
  { id: "build", label: "Build", path: "/build", icon: Hammer },
  { id: "network", label: "Network", path: "/network", icon: Network },
  { id: "storage", label: "Storage", path: "/storage", icon: HardDrive },
  { id: "alerts", label: "Alerts", path: "/alerts", icon: Bell },
  { id: "security", label: "Security", path: "/security", icon: Shield },
  { id: "deployment", label: "Deploy", path: "/deployment", icon: Rocket },
  { id: "replicas", label: "Replicas", path: "/replicas", icon: Copy },
  { id: "snapshots", label: "Snapshots", path: "/snapshots", icon: Camera },
  { id: "docker", label: "Docker", path: "/docker", icon: Container },
  { id: "patches", label: "Patches", path: "/patches", icon: FileDiff },
];
