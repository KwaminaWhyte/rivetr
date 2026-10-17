/**
 * Single source of truth for dashboard navigation.
 * Consumed by the sidebar, the section tab strips, the settings sub-nav and
 * the Cmd+K command palette so they cannot drift apart.
 */

import type { LucideIcon } from "lucide-react";
import {
  LayoutDashboard,
  FolderKanban,
  LayoutTemplate,
  BarChart3,
  Server,
  Settings,
} from "lucide-react";

export interface NavLink {
  title: string;
  url: string;
}

export interface PrimaryNavItem {
  title: string;
  /** Where the sidebar entry links to */
  url: string;
  icon: LucideIcon;
  /** Path prefixes for which this entry shows as active */
  match: string[];
}

export interface NavGroup {
  title: string;
  items: NavLink[];
}

export const monitoringTabs: NavLink[] = [
  { title: "Monitoring", url: "/monitoring" },
  { title: "Costs", url: "/costs" },
];

export const infrastructureTabs: NavLink[] = [
  { title: "Servers", url: "/servers" },
  { title: "Build Servers", url: "/build-servers" },
  { title: "SSH Keys", url: "/ssh-keys" },
  { title: "Swarm", url: "/swarm" },
  { title: "Tunnels", url: "/tunnels" },
];

export const settingsGroups: NavGroup[] = [
  {
    title: "Instance",
    items: [
      { title: "General", url: "/settings" },
      { title: "Preferences", url: "/settings/preferences" },
      { title: "White Label", url: "/settings/white-label" },
      { title: "Auto Updates", url: "/settings/auto-update" },
    ],
  },
  {
    title: "Team and access",
    items: [
      { title: "Teams", url: "/teams" },
      { title: "Authentication", url: "/settings/oauth" },
      { title: "SSO / OIDC", url: "/settings/sso" },
      { title: "Security", url: "/settings/security" },
      { title: "API Tokens", url: "/tokens" },
    ],
  },
  {
    title: "Integrations",
    items: [
      { title: "Git Providers", url: "/git-providers" },
      { title: "Webhooks", url: "/webhooks" },
      { title: "Webhook Events", url: "/webhook-events" },
      { title: "Notifications", url: "/settings/notifications" },
      { title: "Alert Defaults", url: "/settings/alert-defaults" },
      { title: "S3 Storage", url: "/settings/s3" },
      { title: "Destinations", url: "/settings/destinations" },
      { title: "CA Certificates", url: "/settings/ca-certificates" },
    ],
  },
  {
    title: "Data and logs",
    items: [
      { title: "Backup and Restore", url: "/settings/backup" },
      { title: "Audit Log", url: "/settings/audit" },
      { title: "Proxy Logs", url: "/settings/proxy-logs" },
    ],
  },
];

const settingsUrls = settingsGroups.flatMap((g) => g.items.map((i) => i.url));

export const primaryNav: PrimaryNavItem[] = [
  { title: "Dashboard", url: "/", icon: LayoutDashboard, match: ["/"] },
  {
    title: "Projects",
    url: "/projects",
    icon: FolderKanban,
    match: ["/projects", "/apps", "/databases", "/services"],
  },
  { title: "Templates", url: "/templates", icon: LayoutTemplate, match: ["/templates"] },
  {
    title: "Monitoring",
    url: "/monitoring",
    icon: BarChart3,
    match: monitoringTabs.map((t) => t.url),
  },
  {
    title: "Infrastructure",
    url: "/servers",
    icon: Server,
    match: infrastructureTabs.map((t) => t.url),
  },
  {
    title: "Settings",
    url: "/settings",
    icon: Settings,
    match: settingsUrls,
  },
];

/** True when pathname is exactly `url` or nested below it. */
export function pathMatches(pathname: string, url: string): boolean {
  if (url === "/") return pathname === "/";
  return pathname === url || pathname.startsWith(`${url}/`);
}

export function isPrimaryActive(pathname: string, item: PrimaryNavItem): boolean {
  return item.match.some((url) => pathMatches(pathname, url));
}

/** True when the page belongs to the settings area (shows the settings sub-nav). */
export function isSettingsPath(pathname: string): boolean {
  return settingsUrls.some((url) => pathMatches(pathname, url));
}

/** Top-level pages only (no nested detail pages) for the tab strips. */
export function tabsForPath(pathname: string): NavLink[] | null {
  const path = pathname.replace(/\/+$/, "") || "/";
  if (monitoringTabs.some((t) => t.url === path)) return monitoringTabs;
  if (infrastructureTabs.some((t) => t.url === path)) return infrastructureTabs;
  return null;
}

/** Flat list of every navigable page for the command palette. */
export function allNavPages(): { group: string; title: string; url: string }[] {
  const seen = new Set<string>();
  const out: { group: string; title: string; url: string }[] = [];
  const add = (group: string, title: string, url: string) => {
    if (seen.has(url)) return;
    seen.add(url);
    out.push({ group, title, url });
  };
  // Tabs and settings first so their specific labels win the dedupe over the
  // sidebar entries that link to the same URL (e.g. Infrastructure -> Servers).
  for (const t of monitoringTabs) add("Monitoring", t.title, t.url);
  for (const t of infrastructureTabs) add("Infrastructure", t.title, t.url);
  for (const g of settingsGroups) {
    for (const i of g.items) add(`Settings: ${g.title}`, i.title, i.url);
  }
  for (const item of primaryNav) add("Pages", item.title, item.url);
  return out;
}
