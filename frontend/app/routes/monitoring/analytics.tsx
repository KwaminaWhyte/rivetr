import { useEffect, useState } from "react";
import { Link } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { BarChart3 } from "lucide-react";
import { AnalyticsReportView, RangePicker } from "@/components/analytics/analytics-report";
import { PageHeader } from "@/components/page";
import { useBreadcrumb } from "@/lib/breadcrumb-context";
import { useTeamContext } from "@/lib/team-context";
import { analyticsApi, type AnalyticsRange } from "@/lib/api/analytics";

export function meta() {
  return [
    { title: "Traffic analytics - Rivetr" },
    { name: "description", content: "Requests, latency and visitors across your apps" },
  ];
}

export default function TeamAnalyticsPage() {
  const { setItems } = useBreadcrumb();
  const { currentTeamId } = useTeamContext();
  const [range, setRange] = useState<AnalyticsRange>("24h");

  useEffect(() => {
    setItems([{ label: "Monitoring", href: "/monitoring" }, { label: "Analytics" }]);
  }, [setItems]);

  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ["analytics", "team", currentTeamId, range],
    queryFn: () => analyticsApi.team(range, currentTeamId),
    refetchInterval: 60000,
  });

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<BarChart3 />}
        title="Traffic analytics"
        description="Requests, status codes, latency and visitors for every app behind the Rivetr proxy."
        actions={<RangePicker value={range} onChange={setRange} />}
      />
      <AnalyticsReportView
        report={data}
        isLoading={isLoading}
        error={error}
        onRetry={() => refetch()}
        range={range}
        showHosts
        disabledHint={
          <>
            Turn it on per server under{" "}
            <Link to="/servers/localhost/analytics" className="underline underline-offset-2">
              Servers &gt; localhost &gt; Analytics
            </Link>{" "}
            (or the remote server's Analytics page). Retention and the optional GeoIP database live in Settings &gt; Advanced.
          </>
        }
      />
    </div>
  );
}
