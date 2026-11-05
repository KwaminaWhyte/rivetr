import { Link, useLocation } from "react-router";

import { cn } from "@/lib/utils";
import { tabsForPath } from "@/lib/nav";

/**
 * Tab strip shared by pages that live under one sidebar entry
 * (the areas in lib/nav.ts with layout "tabs": Monitoring, Servers, Integrations,
 * Team, Notifications, Keys and tokens).
 * Mounted once in the dashboard layout; renders nothing on other pages.
 * Mobile only: from md up the sidebar submenus cover the same links.
 */
export function SectionTabs() {
  const { pathname } = useLocation();
  const tabs = tabsForPath(pathname);
  if (!tabs) return null;

  return (
    <nav
      aria-label="Section"
      className="-mb-1 flex gap-1 overflow-x-auto border-b md:hidden"
    >
      {tabs.map((tab) => {
        const active = pathname.replace(/\/+$/, "") === tab.url;
        return (
          <Link
            key={tab.url}
            to={tab.url}
            aria-current={active ? "page" : undefined}
            className={cn(
              "-mb-px shrink-0 border-b-2 px-3 py-2 text-sm font-medium transition-colors",
              active
                ? "border-primary text-foreground"
                : "border-transparent text-muted-foreground hover:text-foreground"
            )}
          >
            {tab.title}
          </Link>
        );
      })}
    </nav>
  );
}
