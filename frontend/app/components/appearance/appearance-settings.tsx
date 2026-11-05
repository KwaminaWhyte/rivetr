import { useState } from "react";
import { Cloud, Copy, HardDrive, LayoutGrid, Palette, RotateCcw, Upload } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { clearPersistedUiState } from "@/lib/persisted-state";
import { normalizeTheme, useTheme } from "@/lib/theme";
import { ThemeOptionsForm } from "./theme-options-form";
import { ThemePreview } from "./theme-preview";

/** Parse pasted theme JSON; accepts a bare theme or `{ "theme": {...} }`. */
function parseThemeJson(text: string): unknown {
  const parsed = JSON.parse(text) as unknown;
  if (parsed && typeof parsed === "object" && "theme" in parsed) {
    return (parsed as { theme: unknown }).theme;
  }
  return parsed;
}

function ImportThemeDialog({
  open,
  onOpenChange,
  onImport,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImport: (theme: unknown) => void;
}) {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);

  const submit = () => {
    try {
      const theme = parseThemeJson(text);
      if (!theme || typeof theme !== "object") throw new Error("Expected a JSON object");
      onImport(theme);
      setText("");
      setError(null);
      onOpenChange(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Invalid JSON");
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Import theme</DialogTitle>
          <DialogDescription>
            Paste a theme copied with "Copy theme as JSON". Unknown or invalid fields keep their current value.
          </DialogDescription>
        </DialogHeader>
        <Textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          rows={10}
          className="font-mono text-xs"
          placeholder='{ "mode": "dark", "accent": "teal", ... }'
          aria-label="Theme JSON"
        />
        {error && <p className="text-xs text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={submit} disabled={!text.trim()}>
            Apply theme
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/**
 * The user's appearance settings: every theme option with a live preview,
 * reset, JSON copy/import and "Reset UI layout". Changes apply at once and
 * are saved to the user's account (or this browser when the server cannot
 * store them).
 */
export function AppearanceSettings() {
  const { settings, update, replace, reset, customised, instanceDefault, storage } = useTheme();
  const [importOpen, setImportOpen] = useState(false);

  const copyJson = async () => {
    const json = JSON.stringify(settings, null, 2);
    try {
      await navigator.clipboard.writeText(json);
      toast.success("Theme copied as JSON");
    } catch {
      toast.error("Could not copy to the clipboard");
    }
  };

  const resetLayout = () => {
    const count = clearPersistedUiState();
    toast.success(
      count > 0 ? `UI layout reset (${count} saved ${count === 1 ? "setting" : "settings"} cleared)` : "UI layout reset"
    );
  };

  return (
    <section className="overflow-hidden rounded-xl border bg-frame p-1" aria-labelledby="appearance-title">
      <header className="flex flex-wrap items-center justify-between gap-2 px-3 pb-2 pt-1.5">
        <div>
          <h2 id="appearance-title" className="flex items-center gap-2 text-sm font-medium">
            <Palette className="size-4" aria-hidden="true" />
            Appearance
          </h2>
          <p className="mt-0.5 text-xs text-muted-foreground">
            {customised
              ? "Your theme. Changes apply right away."
              : instanceDefault
                ? "Using the instance default theme. Change anything to make it yours."
                : "Using the default theme. Change anything to make it yours."}
          </p>
        </div>
        <span className="inline-flex items-center gap-1.5 rounded-full bg-muted px-2.5 py-1 text-[11px] text-muted-foreground">
          {storage === "server" ? (
            <>
              <Cloud className="size-3.5" aria-hidden="true" />
              Saved to your account
            </>
          ) : (
            <>
              <HardDrive className="size-3.5" aria-hidden="true" />
              Saved in this browser
            </>
          )}
        </span>
      </header>

      <div className="rounded-lg border bg-card p-(--density-card-p) shadow-xs sm:p-[calc(var(--density-card-p)*1.25)]">
        <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_minmax(0,24rem)]">
          <ThemeOptionsForm value={settings} onChange={update} />
          <div className="space-y-2 xl:sticky xl:top-4 xl:self-start">
            <p className="text-xs font-medium text-muted-foreground">Preview</p>
            <ThemePreview settings={settings} />
          </div>
        </div>

        <div className="mt-6 flex flex-wrap items-center gap-2 border-t pt-4">
          <Button variant="outline" size="sm" onClick={reset} disabled={!customised}>
            <RotateCcw />
            Reset to default
          </Button>
          <Button variant="outline" size="sm" onClick={copyJson}>
            <Copy />
            Copy theme as JSON
          </Button>
          <Button variant="outline" size="sm" onClick={() => setImportOpen(true)}>
            <Upload />
            Import JSON
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="sm:ml-auto"
            onClick={resetLayout}
            title="Forget sidebar, view, table, log viewer and canvas choices saved in this browser"
          >
            <LayoutGrid />
            Reset UI layout
          </Button>
        </div>
      </div>

      <ImportThemeDialog
        open={importOpen}
        onOpenChange={setImportOpen}
        onImport={(theme) => {
          replace(normalizeTheme(theme, settings));
          toast.success("Theme imported");
        }}
      />
    </section>
  );
}
