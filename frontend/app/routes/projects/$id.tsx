import { QueryError } from "@/components/query-error";
import { useEffect, useMemo, useState } from "react";
import { useBreadcrumb } from "@/lib/breadcrumb-context";
import { Link, useNavigate, useParams } from "react-router";
import { useQuery, useQueryClient, useMutation } from "@tanstack/react-query";
import { toast } from "sonner";
import { ArrowLeft, Edit2, FolderKanban, MoreVertical, Trash2, Variable } from "lucide-react";
import { HeaderSkeleton, PageHeader, SectionSkeleton } from "@/components/page";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import type { ProjectWithApps, UpdateProjectRequest } from "@/types/api";
import { AppsTab } from "@/components/projects/apps-tab";
import { DatabasesTab } from "@/components/projects/databases-tab";
import { ServicesTab } from "@/components/projects/services-tab";
import { TagFilter } from "@/components/tags/tag-filter";
import { useTagAssignments, useTags } from "@/components/tags/use-tags";

export function meta() {
  return [
    { title: "Project - Rivetr" },
    {
      name: "description",
      content: "Manage project and its applications",
    },
  ];
}

export default function ProjectDetailPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);
  const [tagFilter, setTagFilter] = useState<string | null>(null);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [editData, setEditData] = useState<UpdateProjectRequest>({});

  const { data: project, isLoading: projectLoading, error: projectError, refetch: refetchProject } = useQuery<ProjectWithApps>({
    queryKey: ["project", id],
    queryFn: () => api.getProject(id!),
    refetchInterval: 5000,
  });

  const { setItems } = useBreadcrumb();
  useEffect(() => {
    if (project) {
      setItems([{ label: "Projects", href: "/projects" }, { label: project.name }]);
    }
  }, [project, setItems]);

  const { data: allTags = [] } = useTags(project?.team_id ?? null);
  const { hasTag } = useTagAssignments(project?.team_id ?? null, allTags.length > 0);
  // Only offer tags that are on something in this project.
  const projectTags = useMemo(
    () =>
      allTags.filter(
        (t) =>
          !!project &&
          (project.apps.some((a) => hasTag("app", a.id, t.id)) ||
            (project.databases ?? []).some((d) => hasTag("database", d.id, t.id)) ||
            (project.services ?? []).some((s) => hasTag("service", s.id, t.id)))
      ),
    [allTags, project, hasTag]
  );
  const visibleProject = useMemo<ProjectWithApps | undefined>(
    () =>
      project && tagFilter
        ? {
            ...project,
            apps: project.apps.filter((a) => hasTag("app", a.id, tagFilter)),
            databases: (project.databases ?? []).filter((d) => hasTag("database", d.id, tagFilter)),
            services: (project.services ?? []).filter((s) => hasTag("service", s.id, tagFilter)),
          }
        : project,
    [project, tagFilter, hasTag]
  );

  const openEditDialog = () => {
    if (project) {
      setEditData({
        name: project.name,
        description: project.description || "",
      });
      setIsEditDialogOpen(true);
    }
  };

  const updateProjectMutation = useMutation({
    mutationFn: async (data: UpdateProjectRequest) => {
      if (!data.name?.trim()) {
        throw new Error("Project name is required");
      }
      return api.updateProject(id!, data);
    },
    onSuccess: () => {
      toast.success("Project updated");
      setIsEditDialogOpen(false);
      queryClient.invalidateQueries({ queryKey: ["project", id] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const deleteProjectMutation = useMutation({
    mutationFn: () => api.deleteProject(id!),
    onSuccess: () => {
      toast.success("Project deleted");
      navigate("/projects");
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  if (projectLoading) {
    return (
      <div className="space-y-6">
        <HeaderSkeleton />
        <SectionSkeleton rows={3} />
        <SectionSkeleton rows={2} />
      </div>
    );
  }

  if (!project) {
    return (
      <div className="space-y-6">
        <Button variant="ghost" asChild>
          <Link to="/projects">
            <ArrowLeft className="mr-2 h-4 w-4" />
            Back to Projects
          </Link>
        </Button>
        {projectError ? (
          <QueryError what="project" error={projectError} onRetry={() => refetchProject()} />
        ) : (
          <Card>
            <CardContent className="py-8 text-center text-destructive">
              Failed to load project. It may have been deleted.
            </CardContent>
          </Card>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<FolderKanban />}
        title={project.name}
        description={project.description || "No description"}
        actions={
          <>
            <Button variant="outline" onClick={openEditDialog}>
              <Edit2 className="h-4 w-4" />
              Edit
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button aria-label="More actions" variant="ghost" size="icon">
                  <MoreVertical className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={openEditDialog}>
                  <Edit2 className="mr-2 h-4 w-4" />
                  Edit Project
                </DropdownMenuItem>
                <DropdownMenuItem asChild>
                  <Link to={`/projects/${project.id}/variables`}>
                    <Variable className="mr-2 h-4 w-4" />
                    Shared variables
                  </Link>
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  className="text-destructive focus:text-destructive"
                  onClick={() => setIsDeleteDialogOpen(true)}
                >
                  <Trash2 className="mr-2 h-4 w-4" />
                  Delete Project
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </>
        }
      />

      {projectTags.length > 0 && (
        <div className="flex justify-end">
          <TagFilter tags={projectTags} value={tagFilter} onChange={setTagFilter} />
        </div>
      )}

      {/* Applications */}
      <AppsTab project={visibleProject ?? project} projectId={id!} />

      {/* Databases */}
      <DatabasesTab project={visibleProject ?? project} projectId={id!} />

      {/* Services */}
      <ServicesTab project={visibleProject ?? project} projectId={id!} />

      {/* Edit Dialog */}
      <Dialog open={isEditDialogOpen} onOpenChange={setIsEditDialogOpen}>
        <DialogContent>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              updateProjectMutation.mutate(editData);
            }}
          >
            <DialogHeader>
              <DialogTitle>Edit Project</DialogTitle>
              <DialogDescription>Update your project details.</DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="edit-name">Name</Label>
                <Input
                  id="edit-name"
                  value={editData.name || ""}
                  onChange={(e) => setEditData({ ...editData, name: e.target.value })}
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="edit-description">Description</Label>
                <Textarea
                  id="edit-description"
                  value={editData.description || ""}
                  onChange={(e) => setEditData({ ...editData, description: e.target.value })}
                  rows={3}
                />
              </div>
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setIsEditDialogOpen(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={updateProjectMutation.isPending}>
                {updateProjectMutation.isPending ? "Saving..." : "Save Changes"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={isDeleteDialogOpen} onOpenChange={setIsDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Project</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete "{project.name}"? This will not delete the apps,
              but they will be unassigned from this project.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={() => deleteProjectMutation.mutate()}
              disabled={deleteProjectMutation.isPending}
            >
              {deleteProjectMutation.isPending ? "Deleting..." : "Delete"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
