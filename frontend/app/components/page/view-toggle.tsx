import { useCallback, useEffect, useState } from "react";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";

export interface ViewOption<V extends string> {
  value: V;
  label: string;
  icon: LucideIcon;
  /** Small badge after the label, e.g. "Try it". */
  badge?: string;
}

/** Segmented control for list / grid / canvas style view switches. */
export function ViewToggle<V extends string>({
  value,
  options,
  onChange,
  className,
}: {
  value: V;
  options: ViewOption<V>[];
  onChange: (value: V) => void;
  className?: string;
}) {
  return (
    <div
      role="radiogroup"
      aria-label="View"
      className={cn("inline-flex items-center rounded-lg border bg-card p-0.5 shadow-xs", className)}
    >
      {options.map((opt) => {
        const active = opt.value === value;
        return (
          <button
            key={opt.value}
            type="button"
            role="radio"
            aria-checked={active}
            title={opt.label}
            onClick={() => onChange(opt.value)}
            className={cn(
              "inline-flex h-7 items-center gap-1.5 rounded-md px-2 text-xs font-medium transition-colors",
              active
                ? "bg-muted text-foreground"
                : "text-muted-foreground hover:text-foreground"
            )}
          >
            <opt.icon className="size-3.5" aria-hidden="true" />
            <span className={cn(!active && "sr-only sm:not-sr-only")}>{opt.label}</span>
            {opt.badge && (
              <span className="rounded-full bg-primary/10 px-1.5 py-px text-[10px] font-medium text-primary">
                {opt.badge}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}

function readStored(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

/**
 * A per-viewer preference kept in localStorage. Reads after mount so server
 * and client render the same first frame; storage failures (private mode,
 * blocked site data) fall back to the default silently.
 */
export function useStoredPreference<V extends string>(
  key: string,
  defaultValue: V,
  allowed: readonly V[]
): [V, (value: V) => void] {
  const [value, setValue] = useState<V>(defaultValue);

  useEffect(() => {
    const stored = readStored(key);
    if (stored && (allowed as readonly string[]).includes(stored)) setValue(stored as V);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  const update = useCallback(
    (next: V) => {
      setValue(next);
      try {
        window.localStorage.setItem(key, next);
      } catch {
        // Storage unavailable: keep the in-memory choice only.
      }
    },
    [key]
  );

  return [value, update];
}
