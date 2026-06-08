import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Label } from "@/components/ui/label";
import { api } from "@/lib/api";
import type { Project } from "@/types/apps";

const UNASSIGNED = "__unassigned__";

type ResourceKind = "app" | "database" | "service";

interface MoveToProjectCardProps {
  resourceKind: ResourceKind;
  resourceId: string;
  currentProjectId: string | null;
}

/**
 * A small settings card that lets a user move an app / database / service to a
 * different project (or unassign it). Used on each resource's settings page.
 */
export function MoveToProjectCard({
  resourceKind,
  resourceId,
  currentProjectId,
}: MoveToProjectCardProps) {
  const queryClient = useQueryClient();

  const { data: projects = [] } = useQuery<Project[]>({
    queryKey: ["projects"],
    queryFn: () => api.getProjects(),
  });

  const moveMutation = useMutation<void, Error, string | null>({
    mutationFn: async (projectId: string | null) => {
      if (resourceKind === "app") {
        await api.assignAppToProject(resourceId, projectId);
      } else if (resourceKind === "database") {
        // Empty string clears the assignment on the backend.
        await api.updateDatabase(resourceId, { project_id: projectId ?? "" });
      } else {
        await api.updateService(resourceId, { project_id: projectId ?? "" });
      }
    },
    onSuccess: () => {
      toast.success("Project updated");
      queryClient.invalidateQueries({ queryKey: [resourceKind, resourceId] });
      queryClient.invalidateQueries({ queryKey: [`${resourceKind}s`] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["project"] });
    },
    onError: (err: Error) => {
      toast.error(err.message || "Failed to move resource");
    },
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>Project</CardTitle>
        <CardDescription>
          Move this {resourceKind} to a different project, or leave it
          unassigned.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-2 max-w-sm">
          <Label htmlFor="project-select">Project</Label>
          <Select
            value={currentProjectId ?? UNASSIGNED}
            onValueChange={(value) =>
              moveMutation.mutate(value === UNASSIGNED ? null : value)
            }
            disabled={moveMutation.isPending}
          >
            <SelectTrigger id="project-select">
              <SelectValue placeholder="Select a project" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={UNASSIGNED}>Unassigned</SelectItem>
              {projects.map((project) => (
                <SelectItem key={project.id} value={project.id}>
                  {project.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </CardContent>
    </Card>
  );
}
