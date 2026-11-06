/**
 * Sections of a resource page (/apps/:id, /databases/:id, /services/:id,
 * /servers/:id). One list per kind drives the contextual sidebar, the mobile
 * section menu, the section page header and the last breadcrumb, so they
 * cannot drift apart. Section names and URLs follow
 * docs/planning/ui-information-architecture.md: the same concept has the same
 * name and URL across kinds. Add new sub-pages here.
 */
import type { LucideIcon } from "lucide-react";
import {
  Archive,
  BarChart3,
  Bell,
  Box,
  CalendarClock,
  Container,
  Cpu,
  Database,
  Eye,
  FileCode,
  FolderOpen,
  Globe,
  Hammer,
  HardDrive,
  HeartPulse,
  History,
  Info,
  Layers,
  LayoutGrid,
  Logs,
  Puzzle,
  Rocket,
  ScrollText,
  Server,
  Settings2,
  Share2,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  SquareTerminal,
  TriangleAlert,
  Upload,
  Variable,
  Wrench,
  ChartNoAxesCombined,
} from "lucide-react";

export type ResourceKind = "app" | "database" | "service" | "server";

export interface ResourceMatch {
  kind: ResourceKind;
  id: string;
  /** e.g. /apps/abc */
  base: string;
}

export interface ResourceNavLink {
  /** Stable key, e.g. "deployments" (used for badges). */
  id: string;
  title: string;
  url: string;
  icon: LucideIcon;
  /** One line shown under the section title on the page. */
  description?: string;
  /** Only active on an exact path match (General pages). */
  exact?: boolean;
}

export interface ResourceNavSection {
  label?: string;
  items: ResourceNavLink[];
}

/** Extra facts about the resource that change which sections exist. */
export interface ResourceNavOptions {
  /** Database engine, e.g. "postgres" (Extensions is Postgres only). */
  dbType?: string;
}

const PREFIX: Record<string, ResourceKind> = {
  apps: "app",
  databases: "database",
  services: "service",
  servers: "server",
};

/** Static pages under /servers that are not a server id. */
const SERVER_AREA_PAGES = new Set(["build", "swarm"]);

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
  if (m[1] === "servers" && SERVER_AREA_PAGES.has(m[2])) return null;
  return { kind: PREFIX[m[1]], id: m[2], base: `/${m[1]}/${m[2]}` };
}

type Def = Omit<ResourceNavLink, "url"> & { path: string };

const general = (description: string): Def => ({
  id: "general",
  title: "General",
  path: "",
  icon: LayoutGrid,
  exact: true,
  description,
});
const logs: Def = { id: "logs", title: "Logs", path: "/logs", icon: ScrollText, description: "Live output from the running containers." };
const files: Def = { id: "files", title: "Files", path: "/files", icon: FolderOpen, description: "Browse and edit files inside the running container." };
const resources = (description: string): Def => ({ id: "resources", title: "Resources", path: "/resources", icon: Cpu, description });
const operations = (description: string): Def => ({ id: "operations", title: "Operations", path: "/operations", icon: Wrench, description });
const danger = (description: string): Def => ({ id: "danger", title: "Danger zone", path: "/danger", icon: TriangleAlert, description });

const APP_SECTIONS: { label?: string; items: Def[] }[] = [
  { items: [general("Status, source, URLs, resource usage and recent activity.")] },
  {
    label: "Configuration",
    items: [
      { id: "build", title: "Source and build", path: "/build", icon: Hammer, description: "Repository, build pack, build options, secrets and registry." },
      { id: "domains", title: "Domains and network", path: "/domains", icon: Globe, description: "Domains, ports, proxy routing, labels and Docker network." },
      { id: "environment", title: "Environment variables", path: "/environment", icon: Variable, description: "Variables passed to the build and the running container." },
      { id: "storage", title: "Storage", path: "/storage", icon: HardDrive, description: "Persistent volumes mounted into the container." },
      { id: "healthcheck", title: "Healthcheck", path: "/healthcheck", icon: HeartPulse, description: "How Rivetr decides the app is ready and healthy." },
      resources("CPU and memory limits, replicas and autoscaling."),
      { id: "advanced", title: "Advanced", path: "/advanced", icon: SlidersHorizontal, description: "Deploy control, freeze windows, deploy commands, Docker options and patches." },
    ],
  },
  {
    label: "Deploy",
    items: [
      { id: "deployments", title: "Deployments", path: "/deployments", icon: Rocket, description: "Deployment history, approvals and build logs." },
      { id: "previews", title: "Preview deployments", path: "/previews", icon: Eye, description: "Per pull request preview environments." },
      { id: "rollback", title: "Rollback", path: "/rollback", icon: History, description: "Rollback policy, retention and configuration snapshots." },
    ],
  },
  {
    label: "Observe",
    items: [
      logs,
      { id: "metrics", title: "Metrics", path: "/metrics", icon: BarChart3, description: "CPU, memory, network and request metrics over time." },
      { id: "terminal", title: "Terminal", path: "/terminal", icon: SquareTerminal, description: "A shell inside the running container." },
      files,
      { id: "analytics", title: "Analytics", path: "/analytics", icon: ChartNoAxesCombined, description: "Requests, status codes, latency and visitors from the proxy." },
    ],
  },
  {
    label: "Automate",
    items: [
      { id: "jobs", title: "Scheduled tasks", path: "/jobs", icon: CalendarClock, description: "Commands that run on a schedule inside the app container, and scheduled restarts." },
      { id: "alerts", title: "Alerts", path: "/alerts", icon: Bell, description: "Resource usage thresholds that send notifications." },
      { id: "log-drains", title: "Log drains", path: "/log-drains", icon: Logs, description: "Forward logs to an external service." },
    ],
  },
  {
    label: "Manage",
    items: [
      { id: "security", title: "Security", path: "/security", icon: ShieldCheck, description: "Basic auth and vulnerability scans." },
      { id: "sharing", title: "Sharing", path: "/sharing", icon: Share2, description: "Share this app with other teams." },
      operations("Move to another project or clone this app."),
      danger("Delete this application."),
    ],
  },
];

function databaseSectionDefs(opts: ResourceNavOptions): { label?: string; items: Def[] }[] {
  const showExtensions = !opts.dbType || opts.dbType === "postgres";
  return [
    { items: [general("Status, credentials, connection strings and resource usage.")] },
    {
      label: "Configuration",
      items: [
        { id: "configuration", title: "Configuration", path: "/configuration", icon: Settings2, description: "Image, init commands and SSL." },
        { id: "network", title: "Network", path: "/network", icon: Globe, description: "Public access, ports and connection URLs." },
        { id: "storage", title: "Storage", path: "/storage", icon: HardDrive, description: "Data volume and storage usage." },
        resources("CPU and memory limits."),
        ...(showExtensions
          ? [{ id: "extensions", title: "Extensions", path: "/extensions", icon: Puzzle, description: "PostgreSQL extensions." }]
          : []),
      ],
    },
    {
      label: "Data",
      items: [
        { id: "backups", title: "Backups", path: "/backups", icon: Archive, description: "Scheduled and manual backups, and restores." },
        { id: "import", title: "Import", path: "/import", icon: Upload, description: "Load a dump file into this database." },
      ],
    },
    { label: "Observe", items: [logs, files] },
    {
      label: "Manage",
      items: [
        operations("Move to another project or clone this database."),
        danger("Delete this database."),
      ],
    },
  ];
}

const SERVICE_SECTIONS: { label?: string; items: Def[] }[] = [
  { items: [general("Status, containers, URLs and resource usage.")] },
  {
    label: "Configuration",
    items: [
      { id: "compose", title: "Compose file", path: "/compose", icon: FileCode, description: "The Docker Compose file and raw compose mode." },
      { id: "build", title: "Source and build", path: "/build", icon: Hammer, description: "Repository, image builds, registry and deploy hooks." },
      { id: "environment", title: "Environment variables", path: "/environment", icon: Variable, description: "Variables substituted into the compose file." },
      { id: "network", title: "Domains and network", path: "/network", icon: Globe, description: "Domain, proxy port, isolated network and ports." },
      resources("CPU and memory limits for every container."),
    ],
  },
  {
    label: "Deploy",
    items: [
      { id: "deployments", title: "Deployments", path: "/deployments", icon: Rocket, description: "Deployment history, logs and rollback." },
      { id: "containers", title: "Containers", path: "/containers", icon: Container, description: "Each container in the stack." },
    ],
  },
  { label: "Observe", items: [logs, files] },
  {
    label: "Manage",
    items: [
      operations("Move or clone this service, and import a database dump."),
      danger("Delete this service."),
    ],
  },
];

function serverSectionDefs(base: string): { label?: string; items: Def[] }[] {
  const isLocal = base.endsWith("/localhost");
  return [
    {
      items: [
        { id: "general", title: "General", path: "", icon: Info, exact: true, description: "Connection, health and resources of this server." },
      ],
    },
    {
      label: "Docker",
      items: [
        { id: "images", title: "Images", path: "/images", icon: Layers, description: "Docker images stored on this server." },
        { id: "docker-cleanup", title: "Docker cleanup", path: "/docker-cleanup", icon: Sparkles, description: "Reclaim disk space from unused Docker data." },
      ],
    },
    {
      label: "Host",
      items: [
        { id: "security", title: "Security", path: "/security", icon: ShieldCheck, description: "Server patching and package updates." },
        ...(isLocal
          ? []
          : [{ id: "files", title: "Files", path: "/files", icon: FolderOpen, description: "Browse files on the host over SSH." }]),
        { id: "analytics", title: "Analytics", path: "/analytics", icon: ChartNoAxesCombined, description: "Traffic through the proxy on this server." },
      ],
    },
  ];
}

export function resourceSections(
  match: ResourceMatch,
  opts: ResourceNavOptions = {}
): ResourceNavSection[] {
  const defs =
    match.kind === "app"
      ? APP_SECTIONS
      : match.kind === "database"
        ? databaseSectionDefs(opts)
        : match.kind === "service"
          ? SERVICE_SECTIONS
          : serverSectionDefs(match.base);
  return defs.map((section) => ({
    label: section.label,
    items: section.items.map(({ path, ...item }) => ({ ...item, url: `${match.base}${path}` })),
  }));
}

/** The section link the pathname is on (longest match wins). */
export function activeResourceSection(
  match: ResourceMatch,
  pathname: string,
  opts: ResourceNavOptions = {}
): ResourceNavLink | null {
  let best: ResourceNavLink | null = null;
  for (const section of resourceSections(match, opts)) {
    for (const item of section.items) {
      if (isResourceLinkActive(pathname, item.url, item.exact) && (!best || item.url.length > best.url.length)) {
        best = item;
      }
    }
  }
  return best;
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
