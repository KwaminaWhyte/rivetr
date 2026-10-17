import { AlertCircle, RefreshCw } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";

interface QueryErrorProps {
  /** What failed to load, e.g. "system stats" */
  what?: string;
  error?: unknown;
  onRetry?: () => void;
  className?: string;
}

/** Inline error state for a failed query, with an optional Retry button. */
export function QueryError({ what = "data", error, onRetry, className }: QueryErrorProps) {
  const message = error instanceof Error && error.message ? error.message : undefined;
  return (
    <Alert variant="destructive" className={className}>
      <AlertCircle />
      <AlertTitle>Failed to load {what}</AlertTitle>
      <AlertDescription>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span>{message ?? "The request failed. Check your connection and try again."}</span>
          {onRetry && (
            <Button type="button" variant="outline" size="sm" onClick={onRetry}>
              <RefreshCw className="mr-2 h-3.5 w-3.5" />
              Retry
            </Button>
          )}
        </div>
      </AlertDescription>
    </Alert>
  );
}
