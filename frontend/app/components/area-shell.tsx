import type { ReactNode } from "react";
import { Link, useLocation, useNavigate } from "react-router";

import { cn } from "@/lib/utils";
import { activeAreaItem, areaForPath, visibleGroups, type NavArea } from "@/lib/nav";
import { useIsInstanceAdmin } from "@/lib/auth";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

function AreaNav({ area }: { area: NavArea }) {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const groups = visibleGroups(area, useIsInstanceAdmin());
  const current = activeAreaItem(area, pathname);

  return (
    <>
      {/* Narrow screens: compact select */}
      <div className="lg:hidden">
        <Select value={current?.url} onValueChange={(url) => navigate(url)}>
          <SelectTrigger className="w-full" aria-label={`${area.title} section`}>
            <SelectValue placeholder={area.title} />
          </SelectTrigger>
          <SelectContent>
            {groups.map((group) => (
              <SelectGroup key={group.title || area.id}>
                {group.title && <SelectLabel>{group.title}</SelectLabel>}
                {group.items.map((item) => (
                  <SelectItem key={item.url} value={item.url}>
                    {item.title}
                  </SelectItem>
                ))}
              </SelectGroup>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* md and up: grouped left sub-nav */}
      <nav aria-label={area.title} className="hidden lg:flex lg:flex-col lg:gap-5">
        {groups.map((group) => (
          <div key={group.title || area.id} className="flex flex-col gap-0.5">
            <div className="px-2 pb-1 text-[11px] font-medium text-muted-foreground/80">
              {group.title || area.title}
            </div>
            {group.items.map((item) => {
              const active = current?.url === item.url;
              return (
                <Link
                  key={item.url}
                  to={item.url}
                  aria-current={active ? "page" : undefined}
                  className={cn(
                    "rounded-md px-2 py-1.5 text-sm transition-colors",
                    active
                      ? "bg-muted font-medium text-foreground"
                      : "text-muted-foreground hover:bg-muted/50 hover:text-foreground"
                  )}
                >
                  {item.title}
                </Link>
              );
            })}
          </div>
        ))}
      </nav>
    </>
  );
}

/**
 * Wraps the page content with a grouped left sub-nav when the current route
 * belongs to an area with layout "subnav" (instance settings, account);
 * otherwise renders children untouched.
 */
export function AreaShell({ children }: { children: ReactNode }) {
  const { pathname } = useLocation();
  const area = areaForPath(pathname);
  if (!area || area.layout !== "subnav") return <>{children}</>;

  return (
    <div className="flex flex-col gap-4 lg:flex-row lg:gap-8">
      <aside className="lg:w-52 lg:shrink-0">
        <AreaNav area={area} />
      </aside>
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}
