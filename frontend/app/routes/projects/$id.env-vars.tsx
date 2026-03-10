import { Link, useParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { ArrowLeft, Layers } from "lucide-react";
import { api } from "@/lib/api";
import { SharedEnvVarsTable } from "@/components/shared-env-vars-table";
import type { Project } from "@/types/api";

export function meta() {
  return [
    { title: "Project Shared Variables - Rivetr" },
    { name: "description", content: "Manage project-level shared environment variables" },
  ];
}

export default function ProjectEnvVarsPage() {
  const { id } = useParams<{ id: string }>();

  const { data: project, isLoading } = useQuery<Project>({
    queryKey: ["project", id],
    queryFn: () => api.getProject(id!),
    enabled: !!id,
  });

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-3">
        <Link
          to={`/projects/${id}`}
          className="text-muted-foreground hover:text-foreground transition-colors"
        >
          <ArrowLeft className="h-5 w-5" />
        </Link>
        <div className="flex items-center gap-2">
          <Layers className="h-5 w-5 text-muted-foreground" />
          <div>
            <h1 className="text-2xl font-bold">
              {isLoading ? "..." : (project?.name ?? "Project")}
            </h1>
            <p className="text-sm text-muted-foreground">
              Shared Environment Variables
            </p>
          </div>
        </div>
      </div>

      <p className="text-sm text-muted-foreground">
        Variables defined here are inherited by all apps in this project.
        They are overridden by environment-level and app-level variables.
        Team-level variables have lower priority and can be overridden here.
      </p>

      {/* Shared Env Vars Table */}
      {id && (
        <SharedEnvVarsTable
          scope="project"
          scopeId={id}
          title="Project Shared Variables"
          description="These variables are inherited by all apps in this project. App-level and environment-level variables take precedence."
        />
      )}
    </div>
  );
}
