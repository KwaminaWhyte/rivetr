import type { ReactNode } from "react";
import { Info } from "lucide-react";
import { Label } from "@/components/ui/label";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

/** Two columns on desktop, one on mobile. Wrap FormField children in it. */
export function FormGrid({
  children,
  className,
  columns = 2,
}: {
  children: ReactNode;
  className?: string;
  columns?: 1 | 2 | 3;
}) {
  return (
    <div
      className={cn(
        "grid grid-cols-1 gap-x-5 gap-y-4",
        columns === 2 && "md:grid-cols-2",
        columns === 3 && "md:grid-cols-2 xl:grid-cols-3",
        className
      )}
    >
      {children}
    </div>
  );
}

interface FormFieldProps {
  label: ReactNode;
  htmlFor?: string;
  /** Shown in a tooltip behind a small info icon next to the label. */
  info?: ReactNode;
  /** Muted helper text under the control. */
  hint?: ReactNode;
  /** Validation message (replaces the hint). */
  error?: ReactNode;
  /** Span the full grid width. */
  full?: boolean;
  required?: boolean;
  className?: string;
  children: ReactNode;
}

/** Small label, optional info tooltip, control, and hint. */
export function FormField({
  label,
  htmlFor,
  info,
  hint,
  error,
  full,
  required,
  className,
  children,
}: FormFieldProps) {
  return (
    <div className={cn("flex min-w-0 flex-col gap-1.5", full && "md:col-span-full", className)}>
      <div className="flex items-center gap-1">
        <Label htmlFor={htmlFor} className="text-xs font-medium text-muted-foreground">
          {label}
          {required && <span className="text-destructive">*</span>}
        </Label>
        {info && (
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                className="inline-flex text-muted-foreground/70 hover:text-foreground"
                aria-label="More info"
              >
                <Info className="size-3.5" />
              </button>
            </TooltipTrigger>
            <TooltipContent className="max-w-xs">{info}</TooltipContent>
          </Tooltip>
        )}
      </div>
      {children}
      {error ? (
        <p className="text-xs text-destructive">{error}</p>
      ) : hint ? (
        <p className="text-xs text-muted-foreground">{hint}</p>
      ) : null}
    </div>
  );
}

/** A row with a label and description on the left and a control (switch, button) on the right. */
export function SettingRow({
  title,
  description,
  control,
  className,
}: {
  title: ReactNode;
  description?: ReactNode;
  control: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex items-start justify-between gap-4 py-3 first:pt-0 last:pb-0", className)}>
      <div className="min-w-0">
        <div className="text-sm font-medium">{title}</div>
        {description && <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>}
      </div>
      <div className="shrink-0">{control}</div>
    </div>
  );
}
