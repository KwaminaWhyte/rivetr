import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { AnalyticsReportView, RangePicker } from "@/components/analytics/analytics-report";
import { ServerAnalyticsToggle } from "@/components/analytics/server-analytics-toggle";
import { SectionCard } from "@/components/page";
import { useServerContext } from "@/components/servers/server-context";
import { analyticsApi, type AnalyticsRange } from "@/lib/api/analytics";

export function meta() {
  return [
    { title: "Server analytics - Rivetr" },
    { name: "description", content: "Traffic analytics for apps on this server" },
  ];
}

export default function ServerAnalyticsPage() {
  const { serverId, isLocal } = useServerContext();
  const [range, setRange] = useState<AnalyticsRange>("24h");
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ["analytics", "server", serverId, range],
    queryFn: () => analyticsApi.server(serverId, range),
    refetchInterval: 60000,
  });

  return (
    <div className="space-y-6">
      <SectionCard
        title="Traffic analytics"
        description={
          isLocal
            ? "Counts requests the Rivetr proxy serves for apps on this machine. On by default."
            : "Counts requests the Rivetr proxy serves for apps on this server. Off by default for remote servers."
        }
      >
        <div className="flex flex-wrap items-center justify-between gap-3">
          <ServerAnalyticsToggle serverId={serverId} enabled={data?.enabled ?? false} disabled={!data} />
          <RangePicker value={range} onChange={setRange} />
        </div>
      </SectionCard>
      <AnalyticsReportView
        report={data}
        isLoading={isLoading}
        error={error}
        onRetry={() => refetch()}
        range={range}
        showHosts
        disabledHint='Use the "Enable traffic analytics" switch above. Data starts to appear about a minute after the next request.'
      />
    </div>
  );
}
