import { useState } from "react";
import { Link, useOutletContext } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { AnalyticsReportView, RangePicker } from "@/components/analytics/analytics-report";
import { analyticsApi, type AnalyticsRange } from "@/lib/api/analytics";
import type { App } from "@/types/api";

export function meta() {
  return [
    { title: "Analytics - Rivetr" },
    { name: "description", content: "Traffic analytics for this application" },
  ];
}

export default function AppAnalyticsPage() {
  const { app } = useOutletContext<{ app: App }>();
  const [range, setRange] = useState<AnalyticsRange>("24h");
  const serverId = app.server_id || "localhost";
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ["analytics", "app", app.id, range],
    queryFn: () => analyticsApi.app(app.id, range),
    refetchInterval: 60000,
  });

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">Traffic analytics</h2>
          <p className="text-sm text-muted-foreground">
            Requests to this app's domains through the Rivetr proxy, updated every minute.
          </p>
        </div>
        <RangePicker value={range} onChange={setRange} />
      </div>
      <AnalyticsReportView
        report={data}
        isLoading={isLoading}
        error={error}
        onRetry={() => refetch()}
        range={range}
        disabledHint={
          <>
            Turn on "Enable traffic analytics" for{" "}
            <Link to={`/servers/${serverId}/analytics`} className="underline underline-offset-2">
              the server this app runs on
            </Link>
            . It is on by default for localhost and off for remote servers.
          </>
        }
      />
    </div>
  );
}
