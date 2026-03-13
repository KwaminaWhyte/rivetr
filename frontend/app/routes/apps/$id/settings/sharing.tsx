import { useOutletContext } from "react-router";
import { AppSharingCard } from "@/components/app-sharing-card";
import type { App } from "@/types/api";

export default function AppSettingsSharing() {
  const { app } = useOutletContext<{ app: App }>();
  return (
    <div className="space-y-6">
      <AppSharingCard app={app} />
    </div>
  );
}
