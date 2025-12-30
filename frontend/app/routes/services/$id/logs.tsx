import { useOutletContext } from "react-router";
import { ServiceLogs } from "@/components/service-logs";
import type { Service } from "@/types/api";

interface OutletContext {
  service: Service;
}

export default function ServiceLogsTab() {
  const { service } = useOutletContext<OutletContext>();

  return (
    <div className="space-y-6">
      <ServiceLogs
        serviceId={service.id}
        serviceName={service.name}
        serviceStatus={service.status}
      />
    </div>
  );
}
