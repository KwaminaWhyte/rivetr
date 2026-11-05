/**
 * Fallback breadcrumbs for pages that do not set their own through
 * useBreadcrumb(). Titles come from the nav data, so a page added to
 * lib/nav.ts or lib/sidebar-nav.ts gets a sensible crumb automatically.
 */
import type { BreadcrumbItem } from "@/lib/breadcrumb-context";
import { infrastructureTabs, monitoringTabs, primaryNav, settingsGroups } from "@/lib/nav";
import { sidebarSections } from "@/lib/sidebar-nav";

const EXTRA_TITLES: Record<string, string> = {
  "/": "Dashboard",
  "/projects": "Projects",
  "/settings": "Settings",
  "/monitoring": "Monitoring",
  "/costs": "Cost analysis",
  "/templates": "Templates",
};

function titleFor(path: string): string | undefined {
  if (EXTRA_TITLES[path]) return EXTRA_TITLES[path];
  for (const g of settingsGroups) {
    const hit = g.items.find((i) => i.url === path);
    if (hit) return hit.title;
  }
  for (const t of [...monitoringTabs, ...infrastructureTabs]) {
    if (t.url === path) return t.title;
  }
  for (const s of sidebarSections) {
    for (const item of s.items) {
      if (item.url === path && !item.children) return item.title;
      const child = item.children?.find((c) => c.url === path);
      if (child) return child.title;
    }
  }
  return primaryNav.find((p) => p.url === path)?.title;
}

export function getDefaultBreadcrumbs(pathname: string): BreadcrumbItem[] {
  const path = pathname.replace(/\/+$/, "") || "/";

  const title = titleFor(path);
  if (title) {
    if (path.startsWith("/settings/")) {
      return [{ label: "Settings", href: "/settings" }, { label: title }];
    }
    return [{ label: title }];
  }

  if (path.startsWith("/projects/") && path.includes("/apps/new")) {
    return [{ label: "Projects", href: "/projects" }, { label: "New application" }];
  }
  const envs = path.match(/^\/projects\/([^/]+)\/environments$/);
  if (envs) {
    return [
      { label: "Projects", href: "/projects" },
      { label: "Project", href: `/projects/${envs[1]}` },
      { label: "Environments" },
    ];
  }
  const vars = path.match(/^\/projects\/([^/]+)\/env-vars$/);
  if (vars) {
    return [
      { label: "Projects", href: "/projects" },
      { label: "Project", href: `/projects/${vars[1]}` },
      { label: "Shared variables" },
    ];
  }
  if (/^\/projects\/[^/]+$/.test(path)) {
    return [{ label: "Projects", href: "/projects" }, { label: "Project" }];
  }
  if (/^\/teams\/[^/]+$/.test(path)) {
    return [{ label: "Teams", href: "/teams" }, { label: "Team details" }];
  }
  if (path.startsWith("/settings/")) {
    const last = path.split("/").pop() ?? "";
    const label = last.replace(/-/g, " ");
    return [
      { label: "Settings", href: "/settings" },
      { label: label.charAt(0).toUpperCase() + label.slice(1) },
    ];
  }
  return [{ label: "Page" }];
}
