import { useOutletContext } from "react-router";
import { AlertsCard } from "@/components/alerts-card";
import type { App } from "@/types/api";

export default function AppSettingsAlerts() {
  const { app } = useOutletContext<{ app: App }>();
  return (
    <div className="space-y-6">
      <AlertsCard appId={app.id} />
    </div>
  );
}
