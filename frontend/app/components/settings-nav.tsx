import type { ReactNode } from "react";
import { Link, useLocation, useNavigate } from "react-router";

import { cn } from "@/lib/utils";
import { isSettingsPath, pathMatches, settingsGroups } from "@/lib/nav";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

function isItemActive(pathname: string, url: string): boolean {
  // /settings is the General page; do not match every /settings/* child
  if (url === "/settings") return pathname.replace(/\/+$/, "") === "/settings";
  return pathMatches(pathname, url);
}

function SettingsNav() {
  const { pathname } = useLocation();
  const navigate = useNavigate();

  const current = settingsGroups
    .flatMap((g) => g.items)
    .find((i) => isItemActive(pathname, i.url));

  return (
    <>
      {/* Narrow screens: compact select */}
      <div className="md:hidden">
        <Select value={current?.url} onValueChange={(url) => navigate(url)}>
          <SelectTrigger className="w-full" aria-label="Settings section">
            <SelectValue placeholder="Settings" />
          </SelectTrigger>
          <SelectContent>
            {settingsGroups.map((group) => (
              <SelectGroup key={group.title}>
                <SelectLabel>{group.title}</SelectLabel>
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
      <nav aria-label="Settings" className="hidden md:flex md:flex-col md:gap-5">
        {settingsGroups.map((group) => (
          <div key={group.title} className="flex flex-col gap-0.5">
            <div className="px-2 pb-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {group.title}
            </div>
            {group.items.map((item) => {
              const active = isItemActive(pathname, item.url);
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
 * Wraps the page content with the grouped settings sub-nav when the current
 * route belongs to the settings area; otherwise renders children untouched.
 */
export function SettingsShell({ children }: { children: ReactNode }) {
  const { pathname } = useLocation();
  if (!isSettingsPath(pathname)) return <>{children}</>;

  return (
    <div className="flex flex-col gap-4 md:flex-row md:gap-8">
      <aside className="md:w-52 md:shrink-0">
        <SettingsNav />
      </aside>
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}
