import { useEffect } from "react";
import { readPersisted, UI_STATE_PREFIX, useLocalPreference } from "@/lib/persisted-state";
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

/**
 * A per-viewer choice from a fixed list, kept with the other UI state under
 * `rivetr:ui:view:<key>`. Older raw keys (e.g. "rivetr_projects_view") are
 * migrated on first read.
 */
export function useStoredPreference<V extends string>(
  key: string,
  defaultValue: V,
  allowed: readonly V[]
): [V, (value: V) => void] {
  const name = `view:${key.replace(/^rivetr_/, "")}`;
  const [value, setValue] = useLocalPreference<V>(name, defaultValue, allowed);

  useEffect(() => {
    if (key.startsWith(UI_STATE_PREFIX)) return;
    try {
      const legacy = window.localStorage.getItem(key);
      if (legacy === null) return;
      window.localStorage.removeItem(key);
      if ((allowed as readonly string[]).includes(legacy) && readPersisted<string | null>(name, null) === null) {
        setValue(legacy as V);
      }
    } catch {
      // Storage unavailable: nothing to migrate.
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  return [value, setValue];
}
