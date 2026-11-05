import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface SectionCardProps {
  /** Small underlined title on the outer frame. */
  title?: ReactNode;
  /** One line of help next to or under the title. */
  description?: ReactNode;
  /** Buttons aligned to the right of the title. */
  actions?: ReactNode;
  /** Optional row under the body (e.g. Save button). */
  footer?: ReactNode;
  /** Anchor id so the contextual sidebar can link to the section. */
  id?: string;
  /** "danger" tints the frame for destructive sections. */
  tone?: "default" | "danger";
  /** Remove the inner padding (tables, lists that bleed to the edge). */
  flush?: boolean;
  className?: string;
  bodyClassName?: string;
  children: ReactNode;
}

/**
 * Two-layer section card: a soft grey outer frame carrying a small underlined
 * title, and a white bordered inner body. Use for every settings or detail
 * section so pages read the same everywhere.
 */
export function SectionCard({
  title,
  description,
  actions,
  footer,
  id,
  tone = "default",
  flush = false,
  className,
  bodyClassName,
  children,
}: SectionCardProps) {
  return (
    <section
      id={id}
      className={cn(
        "scroll-mt-20 rounded-xl border bg-frame p-1",
        tone === "danger" && "border-destructive/30 bg-destructive/5",
        className
      )}
    >
      {(title || actions) && (
        <header className="flex flex-wrap items-center justify-between gap-2 px-3 pb-1.5 pt-1.5">
          <div className="min-w-0">
            {title && (
              <h2
                className={cn(
                  "text-sm font-medium underline decoration-foreground/25 decoration-1 underline-offset-[5px]",
                  tone === "danger" && "text-destructive decoration-destructive/40"
                )}
              >
                {title}
              </h2>
            )}
            {description && (
              <p className="mt-1 text-xs text-muted-foreground">{description}</p>
            )}
          </div>
          {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
        </header>
      )}
      <div
        className={cn(
          "rounded-lg border bg-card text-card-foreground shadow-xs",
          !flush && "p-(--density-card-p) sm:p-[calc(var(--density-card-p)*1.25)]",
          flush && "overflow-hidden",
          bodyClassName
        )}
      >
        {children}
      </div>
      {footer && (
        <div className="flex flex-wrap items-center justify-end gap-2 px-2 pb-1 pt-2">{footer}</div>
      )}
    </section>
  );
}
