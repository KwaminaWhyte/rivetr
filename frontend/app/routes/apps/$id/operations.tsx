import { useOutletContext } from "react-router";
import { MoveToProjectCard } from "@/components/move-to-project-card";
import { CloneResourceCard } from "@/components/clone-resource-card";
import type { App } from "@/types/api";

export function meta() {
  return [
    { title: "Operations - Rivetr" },
    { name: "description", content: "Move this application to another project or clone it" },
  ];
}

export default function AppOperationsSection() {
  const { app } = useOutletContext<{ app: App }>();
  return (
    <div className="space-y-6">
      <MoveToProjectCard
        resourceKind="app"
        resourceId={app.id}
        currentProjectId={app.project_id ?? null}
      />
      <CloneResourceCard
        resourceKind="app"
        resourceId={app.id}
        resourceName={app.name}
        currentProjectId={app.project_id ?? null}
        currentEnvironmentId={app.environment_id ?? null}
        currentServerId={app.server_id ?? null}
      />
    </div>
  );
}
