import { useOutletContext } from "react-router";
import { VolumesCard } from "@/components/volumes-card";
import type { App } from "@/types/api";

export default function AppSettingsStorage() {
  const { app } = useOutletContext<{ app: App }>();
  return (
    <div className="space-y-6">
      <VolumesCard appId={app.id} />
    </div>
  );
}
