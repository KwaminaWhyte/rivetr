import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { api } from "@/lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { RecentEvent } from "@/types/api";

function getStatusColor(status: RecentEvent["status"]) {
  switch (status) {
    case "success":
      return "bg-green-500";
    case "error":
      return "bg-red-500";
    case "warning":
      return "bg-yellow-500";
    case "info":
    default:
      return "bg-blue-500";
  }
}

function formatRelativeTime(timestamp: string): string {
  const now = new Date();
  const date = new Date(timestamp);
  const diffMs = now.getTime() - date.getTime();
  const diffSecs = Math.floor(diffMs / 1000);
  const diffMins = Math.floor(diffSecs / 60);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffSecs < 60) {
    return "just now";
  } else if (diffMins < 60) {
    return `${diffMins}m ago`;
  } else if (diffHours < 24) {
    return `${diffHours}h ago`;
  } else if (diffDays < 7) {
    return `${diffDays}d ago`;
  } else {
    return date.toLocaleDateString();
  }
}

interface RecentEventsProps {
  initialEvents?: RecentEvent[];
}

export function RecentEvents({ initialEvents }: RecentEventsProps) {
  const { data: events = [], isLoading } = useQuery<RecentEvent[]>({
    queryKey: ["recent-events"],
    queryFn: () => api.getRecentEvents(),
    initialData: initialEvents,
    refetchInterval: 15000, // Refresh every 15 seconds
  });

  if (isLoading && !initialEvents) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Recent Events</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {[1, 2, 3, 4, 5].map((i) => (
              <div key={i} className="flex items-start gap-3">
                <div className="mt-1.5 h-2.5 w-2.5 rounded-full bg-muted animate-pulse" />
                <div className="flex-1 space-y-1">
                  <div className="h-4 w-3/4 bg-muted animate-pulse rounded" />
                  <div className="h-3 w-1/4 bg-muted animate-pulse rounded" />
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Recent Events</CardTitle>
      </CardHeader>
      <CardContent>
        {events.length === 0 ? (
          <p className="text-muted-foreground text-sm">
            No recent events. Deploy your first app to see activity here.
          </p>
        ) : (
          <div className="space-y-4 max-h-[400px] overflow-y-auto pr-2">
            {events.map((event) => (
              <div key={event.id} className="flex items-start gap-3">
                <div
                  className={`mt-1.5 h-2.5 w-2.5 rounded-full shrink-0 ${getStatusColor(event.status)}`}
                />
                <div className="flex-1 min-w-0">
                  <Link
                    to={`/apps/${event.app_id}`}
                    className="text-sm font-medium hover:underline truncate block"
                  >
                    {event.message}
                  </Link>
                  <p className="text-xs text-muted-foreground">
                    {formatRelativeTime(event.timestamp)}
                  </p>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
