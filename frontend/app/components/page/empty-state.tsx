import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface EmptyStateProps {
  icon?: ReactNode;
  title: ReactNode;
  /** One line of explanation. */
  description?: ReactNode;
  /** Primary action, usually a Button. */
  action?: ReactNode;
  /** Tighter padding for use inside a card or table. */
  compact?: boolean;
  /** Draw a dashed border (when not already inside a card). */
  bordered?: boolean;
  className?: string;
}

/** Icon, title, one line, action. Use for every empty list. */
export function EmptyState({
  icon,
  title,
  description,
  action,
  compact,
  bordered,
  className,
}: EmptyStateProps) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center text-center",
        compact ? "gap-2 px-4 py-8" : "gap-3 px-6 py-14",
        bordered && "rounded-xl border border-dashed bg-card/60",
        className
      )}
    >
      {icon && (
        <span
          aria-hidden="true"
          className="mb-1 inline-flex size-11 items-center justify-center rounded-xl border bg-card text-muted-foreground shadow-xs [&_svg]:size-5"
        >
          {icon}
        </span>
      )}
      <div className="space-y-1">
        <p className="text-sm font-medium">{title}</p>
        {description && (
          <p className="mx-auto max-w-sm text-sm text-muted-foreground">{description}</p>
        )}
      </div>
      {action && <div className="mt-1 flex flex-wrap justify-center gap-2">{action}</div>}
    </div>
  );
}
