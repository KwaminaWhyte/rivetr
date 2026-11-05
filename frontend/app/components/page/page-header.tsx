import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface PageHeaderProps {
  title: ReactNode;
  /** One line under the title. */
  description?: ReactNode;
  /** Optional leading icon, rendered in a rounded tile. */
  icon?: ReactNode;
  /** Badges or pills shown next to the title. */
  meta?: ReactNode;
  /** Buttons on the right (wrap under the title on narrow screens). */
  actions?: ReactNode;
  className?: string;
}

/**
 * Standard page title row: title, optional description and badges, and
 * right-aligned actions. Every top-level page should start with one.
 */
export function PageHeader({ title, description, icon, meta, actions, className }: PageHeaderProps) {
  return (
    <div
      className={cn(
        "flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between",
        className
      )}
    >
      <div className="flex min-w-0 items-start gap-3">
        {icon && <IconTile size="lg">{icon}</IconTile>}
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1">
            <h1 className="truncate text-2xl font-semibold tracking-tight">{title}</h1>
            {meta}
          </div>
          {description && (
            <p className="mt-0.5 text-sm text-muted-foreground">{description}</p>
          )}
        </div>
      </div>
      {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
    </div>
  );
}

/** Small rounded-square bordered tile that frames an icon (sidebar, cards, headers). */
export function IconTile({
  children,
  size = "md",
  className,
}: {
  children: ReactNode;
  size?: "sm" | "md" | "lg";
  className?: string;
}) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "inline-flex shrink-0 items-center justify-center rounded-md border bg-card text-muted-foreground shadow-xs",
        size === "sm" && "size-6 [&_svg]:size-3.5",
        size === "md" && "size-8 [&_svg]:size-4",
        size === "lg" && "size-10 rounded-lg [&_svg]:size-5",
        className
      )}
    >
      {children}
    </span>
  );
}
