/**
 * Sidebar structure (grouped like Coolify): a top block, then
 * "Infrastructure" and "Manage" sections. Items with `children` render as a
 * collapsible submenu. Add new pages here so the sidebar picks them up; the
 * settings sub-nav and the command palette still read `lib/nav.ts`.
 */
import type { LucideIcon } from "lucide-react";
import {
  Activity,
  Bell,
  Blocks,
  FolderKanban,
  KeyRound,
  LayoutDashboard,
  LayoutTemplate,
  Server,
  Settings,
  Users,
} from "lucide-react";
import { pathMatches } from "@/lib/nav";

export interface SidebarLink {
  title: string;
  url: string;
  adminOnly?: boolean;
}

export interface SidebarItem {
  title: string;
  icon: LucideIcon;
  /** Where the entry links to (first child when it has a submenu). */
  url: string;
  /** Extra path prefixes that mark this entry active. */
  match?: string[];
  children?: SidebarLink[];
}

export interface SidebarSection {
  label?: string;
  items: SidebarItem[];
}

export const sidebarSections: SidebarSection[] = [
  {
    items: [
      { title: "Dashboard", icon: LayoutDashboard, url: "/" },
      {
        title: "Projects",
        icon: FolderKanban,
        url: "/projects",
        match: ["/projects", "/apps", "/databases", "/services"],
      },
      { title: "Templates", icon: LayoutTemplate, url: "/templates" },
      {
        title: "Monitoring",
        icon: Activity,
        url: "/monitoring",
        children: [
          { title: "Metrics", url: "/monitoring" },
          { title: "Costs", url: "/costs" },
          { title: "Proxy logs", url: "/settings/proxy-logs" },
        ],
      },
    ],
  },
  {
    label: "Infrastructure",
    items: [
      {
        title: "Servers",
        icon: Server,
        url: "/servers",
        children: [
          { title: "Servers", url: "/servers" },
          { title: "Build servers", url: "/build-servers" },
          { title: "Swarm", url: "/swarm" },
          { title: "Tunnels", url: "/tunnels" },
        ],
      },
      {
        title: "Integrations",
        icon: Blocks,
        url: "/git-providers",
        children: [
          { title: "Sources", url: "/git-providers" },
          { title: "Destinations", url: "/settings/destinations" },
          { title: "S3 storage", url: "/settings/s3" },
          { title: "CA certificates", url: "/settings/ca-certificates" },
          { title: "Webhooks", url: "/webhooks" },
          { title: "Webhook events", url: "/webhook-events" },
        ],
      },
    ],
  },
  {
    label: "Manage",
    items: [
      {
        title: "Team",
        icon: Users,
        url: "/teams",
        children: [
          { title: "Teams", url: "/teams" },
          { title: "Audit log", url: "/settings/audit" },
        ],
      },
      {
        title: "Notifications",
        icon: Bell,
        url: "/settings/notifications",
        children: [
          { title: "Channels", url: "/settings/notifications" },
          { title: "Alert defaults", url: "/settings/alert-defaults" },
        ],
      },
      {
        title: "Keys & Tokens",
        icon: KeyRound,
        url: "/ssh-keys",
        children: [
          { title: "Private keys", url: "/ssh-keys" },
          { title: "API tokens", url: "/tokens" },
        ],
      },
      { title: "Settings", icon: Settings, url: "/settings" },
    ],
  },
];

function itemUrls(item: SidebarItem): string[] {
  return [item.url, ...(item.match ?? []), ...(item.children ?? []).map((c) => c.url)];
}

/**
 * The longest URL across the whole sidebar that matches the pathname wins, so
 * "/settings/audit" lights up Team > Audit log rather than Settings.
 */
export function activeSidebarUrl(pathname: string): string | null {
  let best: string | null = null;
  for (const section of sidebarSections) {
    for (const item of section.items) {
      for (const url of itemUrls(item)) {
        if (pathMatches(pathname, url) && (!best || url.length > best.length)) best = url;
      }
    }
  }
  return best;
}

export function isSidebarItemActive(item: SidebarItem, activeUrl: string | null): boolean {
  return activeUrl !== null && itemUrls(item).includes(activeUrl);
}
