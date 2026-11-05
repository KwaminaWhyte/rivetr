import { useEffect, useState, type ReactNode } from "react";
import { Check, Monitor, Moon, Sun } from "lucide-react";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  ACCENT_PRESETS,
  DENSITIES,
  FONT_SIZES,
  FONTS,
  LABELS,
  NEUTRALS,
  RADII,
  SIDEBAR_STYLES,
  THEME_MODES,
  accentSwatch,
  isHexColor,
  neutralSwatch,
  normalizeHex,
  type ThemeSettings,
} from "@/lib/theme";
import { cn } from "@/lib/utils";

const MODE_ICONS = { light: Sun, dark: Moon, system: Monitor } as const;

/** Small radiogroup of buttons, the same look as ViewToggle. */
export function Segmented<V extends string>({
  label,
  value,
  options,
  onChange,
  renderOption,
}: {
  label: string;
  value: V;
  options: readonly V[];
  onChange: (value: V) => void;
  renderOption: (value: V) => ReactNode;
}) {
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className="inline-flex flex-wrap items-center gap-0.5 rounded-lg border bg-card p-0.5 shadow-xs"
    >
      {options.map((opt) => {
        const active = opt === value;
        return (
          <button
            key={opt}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => onChange(opt)}
            className={cn(
              "inline-flex h-7 items-center gap-1.5 rounded-md px-2.5 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50",
              active ? "bg-muted text-foreground" : "text-muted-foreground hover:text-foreground"
            )}
          >
            {renderOption(opt)}
          </button>
        );
      })}
    </div>
  );
}

function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="grid gap-2 sm:grid-cols-[9rem_1fr] sm:items-start">
      <div className="pt-1.5">
        <Label className="text-sm">{label}</Label>
        {hint && <p className="mt-0.5 text-xs text-muted-foreground">{hint}</p>}
      </div>
      <div className="min-w-0">{children}</div>
    </div>
  );
}

function Swatch({
  color,
  label,
  selected,
  onClick,
}: {
  color: string;
  label: string;
  selected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={selected}
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "relative inline-flex size-8 items-center justify-center rounded-full ring-offset-2 ring-offset-background transition-shadow focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        selected ? "ring-2 ring-foreground/70" : "ring-1 ring-foreground/10 hover:ring-foreground/30"
      )}
      style={{ background: color }}
    >
      {selected && <Check className="size-4 text-white drop-shadow-[0_1px_1px_rgb(0_0_0/0.6)]" aria-hidden="true" />}
    </button>
  );
}

/** Custom accent: native colour picker plus a hex text field. */
function CustomAccent({
  value,
  active,
  onChange,
}: {
  value: string;
  active: boolean;
  onChange: (hex: string) => void;
}) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  return (
    <div className={cn("flex items-center gap-2", !active && "opacity-80")}>
      <label
        className={cn(
          "relative inline-flex size-8 cursor-pointer items-center justify-center overflow-hidden rounded-full ring-offset-2 ring-offset-background",
          active ? "ring-2 ring-foreground/70" : "ring-1 ring-foreground/10"
        )}
        style={{
          background: active
            ? value
            : "conic-gradient(from 90deg, #ef4444, #f59e0b, #22c55e, #06b6d4, #6366f1, #d946ef, #ef4444)",
        }}
        title="Custom colour"
      >
        <span className="sr-only">Pick a custom accent colour</span>
        <input
          type="color"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="absolute inset-0 cursor-pointer opacity-0"
        />
      </label>
      <Input
        aria-label="Custom accent hex"
        value={text}
        onChange={(e) => {
          setText(e.target.value);
          if (isHexColor(e.target.value)) onChange(normalizeHex(e.target.value));
        }}
        className="h-8 w-28 font-mono text-xs"
        maxLength={7}
        placeholder="#7c3aed"
      />
    </div>
  );
}

/**
 * Every theme option as controls. Used for the user's own theme and for the
 * admin's instance default.
 */
export function ThemeOptionsForm({
  value,
  onChange,
}: {
  value: ThemeSettings;
  onChange: (patch: Partial<ThemeSettings>) => void;
}) {
  return (
    <div className="space-y-5">
      <Row label="Mode" hint="System follows your OS.">
        <Segmented
          label="Mode"
          value={value.mode}
          options={THEME_MODES}
          onChange={(mode) => onChange({ mode })}
          renderOption={(m) => {
            const Icon = MODE_ICONS[m];
            return (
              <>
                <Icon className="size-3.5" aria-hidden="true" />
                {LABELS.mode[m]}
              </>
            );
          }}
        />
      </Row>

      <Row label="Accent" hint="Buttons, links, focus rings and charts.">
        <div className="flex flex-wrap items-center gap-2.5">
          <div role="radiogroup" aria-label="Accent colour" className="flex flex-wrap gap-2">
            {ACCENT_PRESETS.map((accent) => (
              <Swatch
                key={accent}
                label={LABELS.accent[accent]}
                color={accentSwatch({ accent, customAccent: value.customAccent })}
                selected={value.accent === accent}
                onClick={() => onChange({ accent })}
              />
            ))}
          </div>
          <CustomAccent
            value={value.customAccent}
            active={value.accent === "custom"}
            onChange={(hex) => onChange({ accent: "custom", customAccent: hex })}
          />
        </div>
      </Row>

      <Row label="Neutral" hint="Tint of backgrounds and borders.">
        <div role="radiogroup" aria-label="Neutral base" className="flex flex-wrap gap-2">
          {NEUTRALS.map((neutral) => (
            <button
              key={neutral}
              type="button"
              role="radio"
              aria-checked={value.neutral === neutral}
              onClick={() => onChange({ neutral })}
              className={cn(
                "inline-flex h-8 items-center gap-2 rounded-lg border px-2.5 text-xs font-medium transition-colors",
                value.neutral === neutral
                  ? "border-foreground/40 bg-muted text-foreground"
                  : "text-muted-foreground hover:text-foreground"
              )}
            >
              <span className="flex -space-x-1" aria-hidden="true">
                {[0.92, 0.7, 0.4].map((l) => (
                  <span
                    key={l}
                    className="size-3.5 rounded-full ring-1 ring-background"
                    style={{ background: neutralSwatch(neutral, l) }}
                  />
                ))}
              </span>
              {LABELS.neutral[neutral]}
            </button>
          ))}
        </div>
      </Row>

      <Row label="Corners">
        <Segmented
          label="Corner radius"
          value={value.radius}
          options={RADII}
          onChange={(radius) => onChange({ radius })}
          renderOption={(r) => (
            <>
              <span
                aria-hidden="true"
                data-radius={r}
                className="size-3 border-t-2 border-l-2 border-current"
                style={{ borderTopLeftRadius: "calc(var(--radius) * 0.6)" }}
              />
              {LABELS.radius[r]}
            </>
          )}
        />
      </Row>

      <Row label="Density" hint="Control heights, table rows and card padding.">
        <Segmented
          label="Density"
          value={value.density}
          options={DENSITIES}
          onChange={(density) => onChange({ density })}
          renderOption={(d) => LABELS.density[d]}
        />
      </Row>

      <Row label="Font">
        <div className="flex flex-wrap items-center gap-2">
          <Select value={value.font} onValueChange={(font) => onChange({ font: font as ThemeSettings["font"] })}>
            <SelectTrigger className="w-44" aria-label="Font family">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {FONTS.map((f) => (
                <SelectItem key={f} value={f}>
                  <span data-font={f} className="font-sans">
                    {LABELS.font[f]}
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Segmented
            label="Font size"
            value={value.fontSize}
            options={FONT_SIZES}
            onChange={(fontSize) => onChange({ fontSize })}
            renderOption={(s) => LABELS.fontSize[s]}
          />
        </div>
      </Row>

      <Row label="Sidebar">
        <Segmented
          label="Sidebar style"
          value={value.sidebar}
          options={SIDEBAR_STYLES}
          onChange={(sidebar) => onChange({ sidebar })}
          renderOption={(s) => LABELS.sidebar[s]}
        />
      </Row>

      <Row label="Accessibility">
        <div className="space-y-3">
          <label className="flex items-center justify-between gap-4 rounded-lg border bg-card px-3 py-2">
            <span>
              <span className="block text-sm font-medium">Reduce motion</span>
              <span className="block text-xs text-muted-foreground">Turn off animations and transitions.</span>
            </span>
            <Switch
              checked={value.reducedMotion}
              onCheckedChange={(reducedMotion) => onChange({ reducedMotion })}
            />
          </label>
          <label className="flex items-center justify-between gap-4 rounded-lg border bg-card px-3 py-2">
            <span>
              <span className="block text-sm font-medium">High contrast</span>
              <span className="block text-xs text-muted-foreground">
                Darker text, stronger borders and focus outlines.
              </span>
            </span>
            <Switch
              checked={value.highContrast}
              onCheckedChange={(highContrast) => onChange({ highContrast })}
            />
          </label>
        </div>
      </Row>
    </div>
  );
}
