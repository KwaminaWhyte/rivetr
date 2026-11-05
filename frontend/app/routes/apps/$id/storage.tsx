import { useOutletContext } from "react-router";
import { VolumesCard } from "@/components/volumes-card";
import type { App } from "@/types/api";

export function meta() {
  return [
    { title: "Storage - Rivetr" },
    { name: "description", content: "Persistent volumes mounted into the container" },
  ];
}

export default function AppStorageSection() {
  const { app } = useOutletContext<{ app: App }>();
  return (
    <div className="space-y-6">
      <VolumesCard appId={app.id} />
    </div>
  );
}
