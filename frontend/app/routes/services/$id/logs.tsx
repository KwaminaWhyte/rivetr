import { useOutletContext } from "react-router";
import { ServiceLogs } from "@/components/service-logs";
import type { Service } from "@/types/api";

interface OutletContext {
  service: Service;
  token: string;
}

export default function ServiceLogsTab() {
  const { service, token } = useOutletContext<OutletContext>();

  return (
    <div className="space-y-6">
      <ServiceLogs
        serviceId={service.id}
        serviceName={service.name}
        serviceStatus={service.status}
        token={token}
      />
    </div>
  );
}
