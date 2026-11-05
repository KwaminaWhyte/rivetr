import { useEffect, useState } from "react";
import { Palette, Save, Trash2, UserRound } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { useIsInstanceAdmin } from "@/lib/auth";
import { DEFAULT_THEME, normalizeTheme, themesEqual, useTheme, type ThemeSettings } from "@/lib/theme";
import { ThemeOptionsForm } from "./theme-options-form";
import { ThemePreview } from "./theme-preview";

/**
 * Admin control for the instance default theme: what users who never
 * customised their own theme see. Self-contained so it can be mounted on the
 * branding page or anywhere else; renders nothing for non-admins.
 */
export function InstanceThemeDefault() {
  const isAdmin = useIsInstanceAdmin();
  const { instanceDefault, saveInstanceDefault, settings, syncFromServer } = useTheme();
  const [draft, setDraft] = useState<ThemeSettings>(instanceDefault ?? DEFAULT_THEME);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    void syncFromServer();
  }, [syncFromServer]);

  useEffect(() => {
    setDraft(instanceDefault ?? DEFAULT_THEME);
  }, [instanceDefault]);

  if (!isAdmin) return null;

  const dirty = !themesEqual(draft, instanceDefault ?? DEFAULT_THEME);

  const save = async (theme: ThemeSettings | null) => {
    setSaving(true);
    try {
      await saveInstanceDefault(theme);
      toast.success(theme ? "Instance default theme saved" : "Instance default theme cleared");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Could not save the default theme");
    } finally {
      setSaving(false);
    }
  };

  return (
    <section className="overflow-hidden rounded-xl border bg-frame p-1" aria-labelledby="instance-theme-title">
      <header className="px-3 pb-2 pt-1.5">
        <h2 id="instance-theme-title" className="flex items-center gap-2 text-sm font-medium">
          <Palette className="size-4" aria-hidden="true" />
          Default theme
        </h2>
        <p className="mt-0.5 text-xs text-muted-foreground">
          Used by everyone who has not customised their own appearance.
          {instanceDefault ? "" : " Not set: users get the built-in theme."}
        </p>
      </header>
      <div className="rounded-lg border bg-card p-(--density-card-p) shadow-xs sm:p-[calc(var(--density-card-p)*1.25)]">
        <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_minmax(0,24rem)]">
          <ThemeOptionsForm value={draft} onChange={(patch) => setDraft((d) => normalizeTheme({ ...d, ...patch }))} />
          <div className="space-y-2">
            <p className="text-xs font-medium text-muted-foreground">Preview</p>
            <ThemePreview settings={draft} />
          </div>
        </div>
        <div className="mt-6 flex flex-wrap items-center gap-2 border-t pt-4">
          <Button size="sm" onClick={() => save(draft)} disabled={saving || (!dirty && instanceDefault !== null)}>
            <Save />
            Save as default
          </Button>
          <Button size="sm" variant="outline" onClick={() => setDraft(settings)} disabled={saving}>
            <UserRound />
            Start from my theme
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="sm:ml-auto"
            onClick={() => save(null)}
            disabled={saving || instanceDefault === null}
          >
            <Trash2 />
            Clear default
          </Button>
        </div>
      </div>
    </section>
  );
}
