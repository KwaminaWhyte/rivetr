import { useOutletContext } from "react-router";
import { AlertsCard } from "@/components/alerts-card";
import type { App } from "@/types/api";

export function meta() {
  return [
    { title: "Alerts - Rivetr" },
    { name: "description", content: "Resource usage alerts for this application" },
  ];
}

export default function AppAlertsSection() {
  const { app } = useOutletContext<{ app: App }>();
  return (
    <div className="space-y-6">
      <AlertsCard appId={app.id} />
    </div>
  );
}
