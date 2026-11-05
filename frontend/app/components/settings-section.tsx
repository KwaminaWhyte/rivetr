import type { ReactNode } from "react";
import { Loader2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/**
 * Settings card: underlined section title (with optional actions on the right)
 * and a bordered inner body.
 */
export function SettingsSection({
  title,
  description,
  actions,
  children,
  className,
}: {
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={cn("space-y-3", className)}>
      <div className="flex flex-wrap items-end justify-between gap-2 border-b pb-2">
        <div className="min-w-0">
          <h2 className="text-base font-semibold">{title}</h2>
          {description && <p className="text-sm text-muted-foreground">{description}</p>}
        </div>
        {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
      </div>
      <div className="rounded-lg border bg-card p-4 sm:p-5">{children}</div>
    </section>
  );
}

/** Two-column form grid on desktop, single column on phones. */
export function FormGrid({ children }: { children: ReactNode }) {
  return <div className="grid gap-4 md:grid-cols-2">{children}</div>;
}

/** Sticky bar shown while a form has unsaved changes. */
export function UnsavedChangesBar({
  visible,
  saving,
  onReset,
  onSave,
}: {
  visible: boolean;
  saving?: boolean;
  onReset: () => void;
  onSave: () => void;
}) {
  if (!visible) return null;
  return (
    <div className="sticky bottom-4 z-20 mt-6">
      <div className="flex flex-col gap-3 rounded-lg border bg-background/95 p-3 shadow-lg backdrop-blur sm:flex-row sm:items-center sm:justify-between">
        <p className="text-sm">You have changes that haven't been saved yet.</p>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={onReset} disabled={saving}>
            Reset
          </Button>
          <Button size="sm" onClick={onSave} disabled={saving}>
            {saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Save changes
          </Button>
        </div>
      </div>
    </div>
  );
}
