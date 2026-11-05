/**
 * Single source of truth for the dashboard's page areas (everything that is
 * not inside an app, database, service or server; those live in
 * lib/resource-nav.ts). Consumed by the sidebar (lib/sidebar-nav.ts), the
 * area sub-navs and mobile tab strips, the breadcrumbs
 * (lib/default-breadcrumbs.ts) and the Cmd+K command palette, so a page added
 * here shows up everywhere with the same title.
 *
 * Target structure: docs/planning/ui-information-architecture.md.
 */

export interface NavLink {
  title: string;
  url: string;
  /**
   * Instance-admin only. Set only where the backend rejects non-admins
   * (role != "admin" / not privileged). Hidden from everyone else.
   */
  adminOnly?: boolean;
  /** The area's landing page: its breadcrumb is just the area title. */
  index?: boolean;
}

export interface NavGroup {
  title: string;
  items: NavLink[];
}

export interface NavArea {
  id: string;
  title: string;
  /** Where the area title links to. */
  url: string;
  /** Path prefixes that belong to this area. */
  prefixes: string[];
  groups: NavGroup[];
  /**
   * "subnav": a grouped left sub-nav beside the page (areas the sidebar does
   * not list item by item). "tabs": a tab strip on narrow screens only,
   * because the sidebar submenu covers the same links from md up.
   */
  layout: "subnav" | "tabs";
}

const single = (items: NavLink[]): NavGroup[] => [{ title: "", items }];

export const navAreas: NavArea[] = [
  {
    id: "monitoring",
    title: "Monitoring",
    url: "/monitoring",
    prefixes: ["/monitoring"],
    layout: "tabs",
    groups: single([
      { title: "Overview", url: "/monitoring", index: true },
      { title: "Costs", url: "/monitoring/costs" },
      { title: "Proxy logs", url: "/monitoring/proxy-logs" },
      { title: "Analytics", url: "/monitoring/analytics" },
    ]),
  },
  {
    id: "servers",
    title: "Servers",
    url: "/servers",
    prefixes: ["/servers/build", "/servers/swarm"],
    layout: "tabs",
    groups: single([
      { title: "Servers", url: "/servers", index: true },
      { title: "Build servers", url: "/servers/build" },
      { title: "Swarm", url: "/servers/swarm" },
    ]),
  },
  {
    id: "integrations",
    title: "Integrations",
    url: "/integrations/git",
    prefixes: ["/integrations"],
    layout: "tabs",
    groups: single([
      { title: "Git sources", url: "/integrations/git" },
      { title: "S3 storage", url: "/integrations/s3" },
      { title: "Secret managers", url: "/integrations/secrets" },
      { title: "Destinations", url: "/integrations/destinations" },
      { title: "Tunnels", url: "/integrations/tunnels" },
      { title: "Webhooks", url: "/integrations/webhooks" },
      { title: "Webhook events", url: "/integrations/webhook-events" },
    ]),
  },
  {
    id: "team",
    title: "Team",
    url: "/team",
    prefixes: ["/team", "/teams"],
    layout: "tabs",
    groups: single([
      { title: "General", url: "/team", index: true },
      { title: "Members", url: "/team/members" },
      { title: "Variables", url: "/team/variables" },
      { title: "Audit log", url: "/team/audit" },
      { title: "All teams", url: "/teams" },
    ]),
  },
  {
    id: "notifications",
    title: "Notifications",
    url: "/notifications",
    prefixes: ["/notifications"],
    layout: "tabs",
    groups: single([
      { title: "Channels", url: "/notifications", index: true },
      { title: "Alert defaults", url: "/notifications/alerts" },
    ]),
  },
  {
    id: "keys",
    title: "Keys and tokens",
    url: "/keys/ssh",
    prefixes: ["/keys"],
    layout: "tabs",
    groups: single([
      { title: "SSH keys", url: "/keys/ssh" },
      { title: "API tokens", url: "/keys/api-tokens" },
      { title: "CA certificates", url: "/keys/ca-certificates" },
    ]),
  },
  {
    id: "account",
    title: "Account",
    url: "/account",
    prefixes: ["/account"],
    layout: "subnav",
    groups: single([
      { title: "Profile", url: "/account", index: true },
      { title: "Security", url: "/account/security" },
      { title: "Appearance", url: "/account/appearance" },
      { title: "Preferences", url: "/account/preferences" },
    ]),
  },
  {
    id: "settings",
    title: "Settings",
    url: "/settings",
    prefixes: ["/settings"],
    layout: "subnav",
    groups: [
      {
        title: "Instance",
        items: [
          { title: "General", url: "/settings", index: true },
          { title: "Branding", url: "/settings/branding", adminOnly: true },
          { title: "Updates", url: "/settings/updates", adminOnly: true },
          { title: "AI", url: "/settings/ai" },
          { title: "Advanced", url: "/settings/advanced", adminOnly: true },
        ],
      },
      {
        title: "Access",
        items: [
          { title: "Authentication", url: "/settings/authentication" },
          { title: "Email", url: "/settings/email", adminOnly: true },
        ],
      },
      {
        title: "Data",
        items: [
          { title: "Backup and restore", url: "/settings/backup" },
          { title: "Audit log", url: "/settings/audit" },
        ],
      },
    ],
  },
];

/** Top-level pages that are not part of an area. */
export const standalonePages: NavLink[] = [
  { title: "Dashboard", url: "/" },
  { title: "Projects", url: "/projects" },
  { title: "Templates", url: "/templates" },
];

/** True when pathname is exactly `url` or nested below it. */
export function pathMatches(pathname: string, url: string): boolean {
  if (url === "/") return pathname === "/";
  return pathname === url || pathname.startsWith(`${url}/`);
}

function normalize(pathname: string): string {
  return pathname.replace(/\/+$/, "") || "/";
}

export function areaItems(area: NavArea, isAdmin = true): NavLink[] {
  return area.groups.flatMap((g) => g.items).filter((i) => isAdmin || !i.adminOnly);
}

/** Groups with admin-only links removed for non-admins. Empty groups are dropped. */
export function visibleGroups(area: NavArea, isAdmin: boolean): NavGroup[] {
  if (isAdmin) return area.groups;
  return area.groups
    .map((g) => ({ ...g, items: g.items.filter((i) => !i.adminOnly) }))
    .filter((g) => g.items.length > 0);
}

/** The area a pathname belongs to (resource pages under /servers/:id excluded). */
export function areaForPath(pathname: string): NavArea | null {
  const path = normalize(pathname);
  for (const area of navAreas) {
    if (areaItems(area).some((i) => i.url === path)) return area;
  }
  for (const area of navAreas) {
    if (area.prefixes.some((p) => pathMatches(path, p))) return area;
  }
  return null;
}

/** The link that best matches the pathname inside an area (longest URL wins). */
export function activeAreaItem(area: NavArea, pathname: string): NavLink | undefined {
  const path = normalize(pathname);
  let best: NavLink | undefined;
  for (const item of areaItems(area)) {
    const hit = item.index ? path === item.url : pathMatches(path, item.url);
    if (hit && (!best || item.url.length > best.url.length)) best = item;
  }
  return best;
}

/** Narrow-screen tab strip for areas whose links live in a sidebar submenu. */
export function tabsForPath(pathname: string): NavLink[] | null {
  const area = areaForPath(pathname);
  if (!area || area.layout !== "tabs") return null;
  return areaItems(area);
}

/** Flat list of every navigable page for the command palette. */
export function allNavPages(isAdmin = true): { group: string; title: string; url: string }[] {
  const seen = new Set<string>();
  const out: { group: string; title: string; url: string }[] = [];
  const add = (group: string, title: string, url: string) => {
    if (seen.has(url)) return;
    seen.add(url);
    out.push({ group, title, url });
  };
  for (const page of standalonePages) add("Pages", page.title, page.url);
  for (const area of navAreas) {
    for (const g of visibleGroups(area, isAdmin)) {
      const group = g.title ? `${area.title}: ${g.title}` : area.title;
      for (const i of g.items) add(group, i.title, i.url);
    }
  }
  return out;
}
