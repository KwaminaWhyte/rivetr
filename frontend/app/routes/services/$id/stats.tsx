import { useOutletContext } from "react-router";
import { Card, CardContent } from "@/components/ui/card";
import { ResourceMonitor } from "@/components/resource-monitor";
import type { Service } from "@/types/api";

export function meta() {
  return [
    { title: "Service Stats - Rivetr" },
    { name: "description", content: "CPU, RAM, storage, and network usage for this service" },
  ];
}

interface OutletContext {
  service: Service;
}

export default function ServiceStatsPage() {
  const { service } = useOutletContext<OutletContext>();
  const isStopped = service.status !== "running";

  return (
    <div className="space-y-6">
      <p className="text-sm text-muted-foreground">
        Live CPU, RAM, and network metrics aggregated across all containers in this service.
        {(service.cpu_limit || service.memory_limit) && (
          <>
            {" "}
            Configured limits (per container):{" "}
            <span className="font-medium text-foreground">
              {service.cpu_limit ? `${service.cpu_limit} CPU` : "no CPU cap"}
            </span>
            {" · "}
            <span className="font-medium text-foreground">
              {service.memory_limit ?? "no memory cap"}
            </span>
            .
          </>
        )}
      </p>

      {isStopped ? (
        <Card>
          <CardContent className="py-8 text-center text-sm text-muted-foreground">
            No live metrics available, start the service to begin monitoring.
          </CardContent>
        </Card>
      ) : (
        <ResourceMonitor serviceId={service.id} />
      )}
    </div>
  );
}
