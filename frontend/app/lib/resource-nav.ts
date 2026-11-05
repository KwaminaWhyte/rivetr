/**
 * Contextual sidebar for resource pages (/apps/:id, /databases/:id,
 * /services/:id, /servers/:id). When the URL is inside a resource the sidebar
 * swaps the global nav for these grouped section links.
 *
 * App sections are derived from APP_TAB_GROUPS and APP_SETTINGS_TABS so a new
 * tab added there shows up here too. Database, service and server sections
 * are listed below; add new sub-pages to the matching list.
 */
import type { LucideIcon } from "lucide-react";
import {
  Activity,
  Archive,
  BarChart3,
  Box,
  CalendarClock,
  Database,
  Eye,
  FileText,
  FolderOpen,
  Globe,
  HardDrive,
  Layers,
  LayoutGrid,
  Lock,
  Logs,
  Network,
  Puzzle,
  Rocket,
  ScrollText,
  Server,
  Settings,
  SquareTerminal,
  Upload,
  Variable,
} from "lucide-react";
import { APP_SETTINGS_TABS, APP_TAB_GROUPS } from "@/lib/app-tabs";

export type ResourceKind = "app" | "database" | "service" | "server";

export interface ResourceMatch {
  kind: ResourceKind;
  id: string;
  /** e.g. /apps/abc */
  base: string;
}

export interface ResourceNavLink {
  title: string;
  url: string;
  icon: LucideIcon;
  /** Only active on an exact path match (overview pages). */
  exact?: boolean;
  children?: { title: string; url: string; exact?: boolean }[];
}

export interface ResourceNavSection {
  label?: string;
  items: ResourceNavLink[];
}

const PREFIX: Record<string, ResourceKind> = {
  apps: "app",
  databases: "database",
  services: "service",
  servers: "server",
};

export const RESOURCE_KIND_LABEL: Record<ResourceKind, string> = {
  app: "Application",
  database: "Database",
  service: "Service",
  server: "Server",
};

/** Detect a resource page from the pathname. */
export function matchResource(pathname: string): ResourceMatch | null {
  const m = pathname.match(/^\/(apps|databases|services|servers)\/([^/]+)/);
  if (!m) return null;
  return { kind: PREFIX[m[1]], id: m[2], base: `/${m[1]}/${m[2]}` };
}

const APP_ICONS: Record<string, LucideIcon> = {
  general: LayoutGrid,
  deployments: Rocket,
  previews: Eye,
  jobs: CalendarClock,
  logs: ScrollText,
  monitoring: BarChart3,
  "log-drains": Logs,
  terminal: SquareTerminal,
  files: FolderOpen,
  "env-vars": Variable,
  network: Globe,
  settings: Settings,
  security: Lock,
};

function appSections(base: string): ResourceNavSection[] {
  return APP_TAB_GROUPS.map((group) => ({
    label: group.id === "overview" ? undefined : group.label,
    items: group.items.map((item) => ({
      title: item.id === "general" ? "Overview" : item.label,
      url: `${base}${item.path}`,
      icon: APP_ICONS[item.id] ?? Box,
      exact: item.path === "",
      children:
        item.id === "settings"
          ? APP_SETTINGS_TABS.map((t) => ({
              title: t.label,
              url: `${base}/settings${t.path}`,
              exact: t.path === "",
            }))
          : undefined,
    })),
  }));
}

function databaseSections(base: string): ResourceNavSection[] {
  return [
    {
      items: [
        { title: "Overview", url: base, icon: LayoutGrid, exact: true },
        { title: "Settings", url: `${base}/settings`, icon: Settings },
      ],
    },
    {
      label: "Configuration",
      items: [
        { title: "Network", url: `${base}/network`, icon: Network },
        { title: "Storage", url: `${base}/storage`, icon: HardDrive },
        { title: "Extensions", url: `${base}/extensions`, icon: Puzzle },
      ],
    },
    {
      label: "Data",
      items: [
        { title: "Backups", url: `${base}/backups`, icon: Archive },
        { title: "Import", url: `${base}/import`, icon: Upload },
      ],
    },
    {
      label: "Observability",
      items: [{ title: "Logs", url: `${base}/logs`, icon: ScrollText }],
    },
  ];
}

function serviceSections(base: string): ResourceNavSection[] {
  return [
    {
      items: [
        { title: "Overview", url: base, icon: LayoutGrid, exact: true },
        { title: "Settings", url: `${base}/settings`, icon: Settings },
      ],
    },
    {
      label: "Configuration",
      items: [{ title: "Network", url: `${base}/network`, icon: Network }],
    },
    {
      label: "Observability",
      items: [
        { title: "Logs", url: `${base}/logs`, icon: ScrollText },
        { title: "Stats", url: `${base}/stats`, icon: Activity },
      ],
    },
  ];
}

function serverSections(base: string): ResourceNavSection[] {
  return [
    {
      items: [
        { title: "All servers", url: "/servers", icon: Server, exact: true },
        { title: "Files", url: `${base}/files`, icon: FileText },
      ],
    },
  ];
}

export function resourceSections(match: ResourceMatch): ResourceNavSection[] {
  switch (match.kind) {
    case "app":
      return appSections(match.base);
    case "database":
      return databaseSections(match.base);
    case "service":
      return serviceSections(match.base);
    case "server":
      return serverSections(match.base);
  }
}

export const RESOURCE_ICON: Record<ResourceKind, LucideIcon> = {
  app: Box,
  database: Database,
  service: Layers,
  server: Server,
};

/** Active check honouring `exact`; nested routes light up their parent. */
export function isResourceLinkActive(pathname: string, url: string, exact?: boolean): boolean {
  const path = pathname.replace(/\/+$/, "");
  if (exact) return path === url;
  return path === url || path.startsWith(`${url}/`);
}
