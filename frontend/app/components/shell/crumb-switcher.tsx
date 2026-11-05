import { useState, type ReactNode } from "react";
import { Link } from "react-router";
import { Check, ChevronDown, Search } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";

export interface CrumbOption {
  id: string;
  label: string;
  href: string;
  /** Small muted text on the right (type, count). */
  hint?: string;
  icon?: ReactNode;
}

interface CrumbSwitcherProps {
  label: ReactNode;
  /** Where clicking the label goes. */
  href?: string;
  /** Sibling entries for the dropdown. Without options it renders a plain crumb. */
  options?: CrumbOption[];
  activeId?: string | null;
  /** Heading inside the dropdown, e.g. "Projects". */
  heading?: string;
  /** Extra items at the end of the dropdown (create, manage). */
  footer?: ReactNode;
  /** Last crumb: bold, not a link. */
  current?: boolean;
  icon?: ReactNode;
  className?: string;
}

/**
 * One breadcrumb with an attached "⌄" switcher listing its siblings
 * (teams, projects, environments, resources). Shows a filter box when the
 * list is long.
 */
export function CrumbSwitcher({
  label,
  href,
  options,
  activeId,
  heading,
  footer,
  current,
  icon,
  className,
}: CrumbSwitcherProps) {
  const [query, setQuery] = useState("");
  const hasMenu = (options && options.length > 0) || !!footer;
  const filtered =
    options?.filter((o) => o.label.toLowerCase().includes(query.trim().toLowerCase())) ?? [];

  const text = (
    <span className={cn("inline-flex min-w-0 items-center gap-1.5", current ? "font-medium text-foreground" : "")}>
      {icon}
      <span className="truncate">{label}</span>
    </span>
  );

  return (
    <span
      className={cn(
        "inline-flex min-w-0 items-center rounded-md text-sm text-muted-foreground",
        className
      )}
    >
      {href && !current ? (
        <Link
          to={href}
          className="min-w-0 max-w-[14rem] truncate rounded-md px-1.5 py-1 transition-colors hover:bg-muted hover:text-foreground"
        >
          {text}
        </Link>
      ) : (
        <span className="min-w-0 max-w-[16rem] truncate px-1.5 py-1" aria-current={current ? "page" : undefined}>
          {text}
        </span>
      )}
      {hasMenu && (
        <DropdownMenu onOpenChange={(open) => !open && setQuery("")}>
          <DropdownMenuTrigger
            className="inline-flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring data-[state=open]:bg-muted"
            aria-label={heading ? `Switch ${heading.toLowerCase()}` : "Switch"}
          >
            <ChevronDown className="size-3.5" />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" className="w-64">
            {heading && (
              <DropdownMenuLabel className="text-xs text-muted-foreground">{heading}</DropdownMenuLabel>
            )}
            {options && options.length > 7 && (
              <div className="relative px-1 pb-1">
                <Search className="pointer-events-none absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
                <input
                  autoFocus
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  onKeyDown={(e) => e.stopPropagation()}
                  placeholder="Filter"
                  aria-label="Filter"
                  className="h-8 w-full rounded-md bg-field pl-7 pr-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                />
              </div>
            )}
            <div className="max-h-72 overflow-y-auto">
              {filtered.map((o) => (
                <DropdownMenuItem key={o.id} asChild>
                  <Link to={o.href} className="flex items-center gap-2">
                    {o.icon}
                    <span className="min-w-0 flex-1 truncate">{o.label}</span>
                    {o.hint && <span className="text-xs text-muted-foreground">{o.hint}</span>}
                    {o.id === activeId && <Check className="size-3.5 text-primary" />}
                  </Link>
                </DropdownMenuItem>
              ))}
              {options && options.length > 0 && filtered.length === 0 && (
                <p className="px-2 py-1.5 text-sm text-muted-foreground">No matches</p>
              )}
            </div>
            {footer && (
              <>
                {options && options.length > 0 && <DropdownMenuSeparator />}
                {footer}
              </>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </span>
  );
}

export function CrumbSeparator() {
  return (
    <span aria-hidden="true" className="select-none px-0.5 text-muted-foreground/50">
      /
    </span>
  );
}
