import { Clock, WrapText } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { isBoolean, useLocalPreference, usePersistedState } from "@/lib/persisted-state";
import { cn } from "@/lib/utils";

/**
 * Log viewer options shared by every log panel (app, service, database and
 * build logs), remembered per browser under rivetr:ui:logs:*.
 */
export function useLogViewPrefs() {
  const [follow, setFollow] = usePersistedState("logs:follow", true, { validate: isBoolean });
  const [wrap, setWrap] = usePersistedState("logs:wrap", true, { validate: isBoolean });
  const [timestamps, setTimestamps] = usePersistedState("logs:timestamps", true, { validate: isBoolean });
  return { follow, setFollow, wrap, setWrap, timestamps, setTimestamps };
}

export const LOG_LEVELS = ["all", "info", "warn", "error", "debug"] as const;
export type LogLevelFilter = (typeof LOG_LEVELS)[number];

/** Level filter for viewers whose lines carry a level. */
export function useLogLevelFilter(scope: string) {
  return useLocalPreference<LogLevelFilter>(`logs:${scope}:level`, "all", LOG_LEVELS);
}

/** Line wrapping classes for a log line. */
export function logLineWrapClass(wrap: boolean): string {
  return wrap ? "whitespace-pre-wrap break-all" : "whitespace-pre";
}

function ToggleButton({
  pressed,
  onClick,
  label,
  children,
}: {
  pressed: boolean;
  onClick: () => void;
  label: string;
  children: React.ReactNode;
}) {
  return (
    <Button
      type="button"
      variant="outline"
      size="sm"
      aria-pressed={pressed}
      title={label}
      onClick={onClick}
      className={cn(pressed && "bg-muted text-foreground")}
    >
      {children}
    </Button>
  );
}

/** Wrap and timestamp toggles for a log viewer header. */
export function LogViewToggles({
  wrap,
  onWrapChange,
  timestamps,
  onTimestampsChange,
}: {
  wrap: boolean;
  onWrapChange: (wrap: boolean) => void;
  timestamps?: boolean;
  onTimestampsChange?: (timestamps: boolean) => void;
}) {
  return (
    <>
      <ToggleButton pressed={wrap} onClick={() => onWrapChange(!wrap)} label="Wrap long lines">
        <WrapText />
        <span className="hidden sm:inline">Wrap</span>
      </ToggleButton>
      {onTimestampsChange && (
        <ToggleButton
          pressed={!!timestamps}
          onClick={() => onTimestampsChange(!timestamps)}
          label="Show timestamps"
        >
          <Clock />
          <span className="hidden sm:inline">Time</span>
        </ToggleButton>
      )}
    </>
  );
}

export function LogLevelSelect({
  value,
  onChange,
}: {
  value: LogLevelFilter;
  onChange: (value: LogLevelFilter) => void;
}) {
  return (
    <Select value={value} onValueChange={(v) => onChange(v as LogLevelFilter)}>
      <SelectTrigger size="sm" className="w-28" aria-label="Log level">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {LOG_LEVELS.map((level) => (
          <SelectItem key={level} value={level}>
            {level === "all" ? "All levels" : level.charAt(0).toUpperCase() + level.slice(1)}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
