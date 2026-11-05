import type { CSSProperties } from "react";
import { Rocket, Search } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { ResourceStatus } from "@/components/page/status-pill";
import { resolveMode, themeTokens, type ThemeSettings } from "@/lib/theme";
import { cn } from "@/lib/utils";

/**
 * A small sample of the dashboard rendered in `settings`, scoped to this box:
 * colour tokens are set inline and density, radius and font come from data
 * attributes, so it can show a theme that is not the active one.
 */
export function ThemePreview({ settings, className }: { settings: ThemeSettings; className?: string }) {
  const dark = resolveMode(settings.mode) === "dark";
  const style = themeTokens(settings, dark) as CSSProperties;

  return (
    <div
      data-radius={settings.radius}
      data-density={settings.density}
      data-font={settings.font}
      style={style}
      className={cn(
        "overflow-hidden rounded-xl border bg-background font-sans text-foreground shadow-xs",
        className
      )}
      aria-label="Theme preview"
    >
      <div className="flex">
        <div className="hidden w-28 shrink-0 flex-col gap-1 border-r bg-sidebar p-2 text-sidebar-foreground sm:flex">
          <div className="mb-1 flex items-center gap-1.5 px-1 text-xs font-semibold">
            <span className="size-4 rounded bg-primary" aria-hidden="true" />
            Rivetr
          </div>
          {["Projects", "Apps", "Settings"].map((item, i) => (
            <div
              key={item}
              className={cn(
                "rounded-md px-2 py-1 text-[11px]",
                i === 0 ? "bg-sidebar-accent font-medium text-sidebar-accent-foreground" : "text-muted-foreground"
              )}
            >
              {item}
            </div>
          ))}
        </div>

        <div className="min-w-0 flex-1 space-y-3 p-3">
          <div className="flex flex-wrap items-center gap-2">
            <Button size="sm">
              <Rocket />
              Deploy
            </Button>
            <Button size="sm" variant="outline">
              Outline
            </Button>
            <Button size="sm" variant="secondary">
              Secondary
            </Button>
            <Button size="sm" variant="link" className="px-1">
              Link
            </Button>
          </div>

          <div className="flex items-center gap-2">
            <div className="relative min-w-0 flex-1">
              <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
              <Input readOnly value="" placeholder="Search apps" className="pl-8" aria-label="Sample input" />
            </div>
            <Switch checked aria-label="Sample switch" />
          </div>

          <div className="rounded-xl border bg-frame p-1">
            <div className="px-2 pb-1.5 pt-1 text-xs font-medium underline decoration-foreground/25 underline-offset-4">
              Card
            </div>
            <div className="rounded-lg border bg-card p-(--density-card-p) text-card-foreground shadow-xs">
              <div className="flex flex-wrap items-center gap-1.5">
                <Badge>Primary</Badge>
                <Badge variant="secondary">Secondary</Badge>
                <Badge variant="outline">Outline</Badge>
              </div>
              <p className="mt-2 text-xs text-muted-foreground">
                Muted text on a card, with a <span className="text-primary">primary link</span>.
              </p>
              <div className="mt-2 flex items-end gap-1" aria-hidden="true">
                {[1, 2, 3, 4, 5].map((n) => (
                  <span
                    key={n}
                    className="w-4 rounded-sm"
                    style={{ height: 8 + n * 5, background: `var(--chart-${n})` }}
                  />
                ))}
              </div>
            </div>
          </div>

          <div className="overflow-hidden rounded-lg border bg-card">
            <table className="w-full text-xs">
              <thead className="bg-muted/40 text-muted-foreground">
                <tr className="border-b">
                  <th className="h-[calc(var(--density-head-h)-0.25rem)] px-3 text-left font-medium">Name</th>
                  <th className="px-3 text-left font-medium">Status</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b">
                  <td className="px-3 py-(--density-row-py) font-medium">web</td>
                  <td className="px-3">
                    <ResourceStatus status="running" size="sm" />
                  </td>
                </tr>
                <tr className="border-b">
                  <td className="px-3 py-(--density-row-py) font-medium">worker</td>
                  <td className="px-3">
                    <ResourceStatus status="building" size="sm" />
                  </td>
                </tr>
                <tr className="bg-muted/40">
                  <td className="px-3 py-(--density-row-py) font-medium">cron</td>
                  <td className="px-3">
                    <ResourceStatus status="failed" size="sm" />
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}
