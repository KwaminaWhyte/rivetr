import { useOutletContext } from "react-router";
import { AppSharingCard } from "@/components/app-sharing-card";
import type { App } from "@/types/api";

export function meta() {
  return [
    { title: "Sharing - Rivetr" },
    { name: "description", content: "Share this application with other teams" },
  ];
}

export default function AppSharingSection() {
  const { app } = useOutletContext<{ app: App }>();
  return (
    <div className="space-y-6">
      <AppSharingCard app={app} />
    </div>
  );
}
