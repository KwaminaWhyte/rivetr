/**
 * Breadcrumbs for every page outside a resource (resource pages build theirs
 * from lib/resource-nav.ts in components/shell/top-bar.tsx). Titles come from
 * the areas in lib/nav.ts, so a page added there gets the right crumb.
 * Pages may still override with useBreadcrumb().setItems() when the last
 * crumb needs live data.
 */
import type { BreadcrumbItem } from "@/lib/breadcrumb-context";
import { activeAreaItem, areaForPath, standalonePages } from "@/lib/nav";

/** Shown for routes that matched nothing (the dashboard 404 page). */
export const NOT_FOUND_CRUMBS: BreadcrumbItem[] = [{ label: "Page not found" }];

const PROJECT_SECTIONS: Record<string, string> = {
  environments: "Environments",
  variables: "Variables",
  new: "New resource",
};

export function getDefaultBreadcrumbs(pathname: string): BreadcrumbItem[] {
  const path = pathname.replace(/\/+$/, "") || "/";

  const page = standalonePages.find((p) => p.url === path);
  if (page) return [{ label: page.title }];

  const project = path.match(/^\/projects\/([^/]+)(?:\/([^/]+))?$/);
  if (project) {
    const crumbs: BreadcrumbItem[] = [
      { label: "Projects", href: "/projects" },
      { label: "Project", href: `/projects/${project[1]}` },
    ];
    const section = project[2] ? PROJECT_SECTIONS[project[2]] : undefined;
    if (section) crumbs.push({ label: section });
    return crumbs;
  }

  const area = areaForPath(path);
  if (area) {
    const item = activeAreaItem(area, path);
    if (!item) return [{ label: area.title, href: area.url }, { label: "Page not found" }];
    if (item.index && item.url === path) return [{ label: area.title }];
    return [{ label: area.title, href: area.url }, { label: item.title }];
  }

  return NOT_FOUND_CRUMBS;
}
