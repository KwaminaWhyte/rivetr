import { useOutletContext } from "react-router";
import { MoveToProjectCard } from "@/components/move-to-project-card";
import { CloneResourceCard } from "@/components/clone-resource-card";
import { ResourceTagsCard } from "@/components/tags/resource-tags-card";
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
      {/* Both cards hold one or two short rows, so they share a row. */}
      <div className="grid gap-6 md:grid-cols-2">
        <ResourceTagsCard kind="app" resourceId={app.id} teamId={app.team_id} />
        <MoveToProjectCard
          resourceKind="app"
          resourceId={app.id}
          currentProjectId={app.project_id ?? null}
        />
      </div>
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
