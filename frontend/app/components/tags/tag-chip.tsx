import { X } from "lucide-react";
import { cn } from "@/lib/utils";

/** Small pill with the tag's color as a dot. Pass `onRemove` to show an x button. */
export function TagChip({
  name,
  color,
  onRemove,
  className,
}: {
  name: string;
  color: string;
  onRemove?: () => void;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex max-w-full items-center gap-1.5 rounded-full border bg-card px-2 py-0.5 text-xs font-medium",
        className
      )}
    >
      <span
        aria-hidden="true"
        className="size-2 shrink-0 rounded-full"
        style={{ backgroundColor: color }}
      />
      <span className="truncate">{name}</span>
      {onRemove && (
        <button
          type="button"
          onClick={onRemove}
          aria-label={`Remove tag ${name}`}
          className="-mr-1 rounded-full p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
        >
          <X className="size-3" />
        </button>
      )}
    </span>
  );
}

/** Preset colors offered when creating or recoloring a tag. */
export const TAG_COLORS = [
  "#6366f1",
  "#8b5cf6",
  "#ec4899",
  "#ef4444",
  "#f97316",
  "#eab308",
  "#22c55e",
  "#14b8a6",
  "#0ea5e9",
  "#64748b",
];

/** Row of color swatches; the selected one gets a ring. */
export function ColorSwatches({
  value,
  onChange,
}: {
  value: string;
  onChange: (color: string) => void;
}) {
  return (
    <div className="flex flex-wrap items-center gap-1.5" role="radiogroup" aria-label="Tag color">
      {TAG_COLORS.map((c) => (
        <button
          key={c}
          type="button"
          role="radio"
          aria-checked={value === c}
          aria-label={c}
          onClick={() => onChange(c)}
          className={cn(
            "size-6 rounded-full border transition-shadow",
            value === c && "ring-2 ring-ring ring-offset-2 ring-offset-background"
          )}
          style={{ backgroundColor: c }}
        />
      ))}
    </div>
  );
}
