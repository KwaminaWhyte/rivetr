/**
 * Sidebar structure (grouped like Coolify): a top block, then
 * "Infrastructure" and "Manage" sections. Items with `children` render as a
 * collapsible submenu. Submenu links come from the areas in `lib/nav.ts`, so
 * add new pages there; the account area is reached from the user menu.
 */
import type { LucideIcon } from "lucide-react";
import {
  Activity,
  Bell,
  Blocks,
  FolderKanban,
  KeyRound,
  LayoutDashboard,
  Server,
  Settings,
  Tags,
  Users,
} from "lucide-react";
import { navAreas, pathMatches } from "@/lib/nav";

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

function areaLinks(id: string): SidebarLink[] {
  const area = navAreas.find((a) => a.id === id);
  if (!area) return [];
  return area.groups.flatMap((g) => g.items).map((i) => ({
    title: i.title,
    url: i.url,
    adminOnly: i.adminOnly,
  }));
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
      { title: "Tags", icon: Tags, url: "/tags" },
      {
        title: "Monitoring",
        icon: Activity,
        url: "/monitoring",
        children: areaLinks("monitoring"),
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
        children: areaLinks("servers"),
      },
      {
        title: "Integrations",
        icon: Blocks,
        url: "/integrations/git",
        children: areaLinks("integrations"),
      },
    ],
  },
  {
    label: "Manage",
    items: [
      {
        title: "Team",
        icon: Users,
        url: "/team",
        children: areaLinks("team"),
      },
      {
        title: "Notifications",
        icon: Bell,
        url: "/notifications",
        children: areaLinks("notifications"),
      },
      {
        title: "Keys and tokens",
        icon: KeyRound,
        url: "/keys/ssh",
        children: areaLinks("keys"),
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
 * "/team/members" lights up Team > Members rather than Team > General.
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
