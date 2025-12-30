import { useState, useEffect } from "react";
import { useOutletContext, useNavigate } from "react-router";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Input } from "@/components/ui/input";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { toast } from "sonner";
import { api } from "@/lib/api";
import type { Service } from "@/types/api";
import { Trash2, AlertTriangle, Code, Pencil, X, Save, AlertCircle } from "lucide-react";

interface OutletContext {
  service: Service;
}

export default function ServiceSettingsTab() {
  const { service } = useOutletContext<OutletContext>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [deleteConfirmName, setDeleteConfirmName] = useState("");
  const [isEditing, setIsEditing] = useState(false);
  const [composeContent, setComposeContent] = useState(service.compose_content);

  // Mutations
  const updateComposeMutation = useMutation({
    mutationFn: (composeContent: string) =>
      api.updateService(service.id, { compose_content: composeContent }),
    onSuccess: () => {
      toast.success("Compose configuration updated. Restart the service to apply changes.");
      setIsEditing(false);
      queryClient.invalidateQueries({ queryKey: ["service", service.id] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to update compose configuration");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => api.deleteService(service.id),
    onSuccess: () => {
      toast.success("Service deleted");
      if (service.project_id) {
        navigate(`/projects/${service.project_id}`);
      } else {
        navigate("/projects");
      }
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete service");
    },
  });

  const isSubmitting = updateComposeMutation.isPending || deleteMutation.isPending;

  // Reset compose content when service changes (e.g., after save)
  useEffect(() => {
    if (!isEditing) {
      setComposeContent(service.compose_content);
    }
  }, [service.compose_content, isEditing]);

  const handleCancelEdit = () => {
    setComposeContent(service.compose_content);
    setIsEditing(false);
  };

  const handleSaveCompose = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    updateComposeMutation.mutate(composeContent);
  };

  const handleDelete = () => {
    deleteMutation.mutate();
  };

  return (
    <div className="space-y-6">
      {/* Docker Compose Configuration Editor */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Code className="h-5 w-5" />
                Docker Compose Configuration
              </CardTitle>
              <CardDescription className="mt-1.5">
                Edit the Docker Compose YAML for this service. Changes will take effect after restarting the service.
              </CardDescription>
            </div>
            {!isEditing && (
              <Button
                variant="outline"
                className="gap-2"
                onClick={() => setIsEditing(true)}
              >
                <Pencil className="h-4 w-4" />
                Edit
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {isEditing && (
            <div className="flex items-start gap-3 p-4 rounded-lg bg-amber-50 border border-amber-200 text-amber-800 dark:bg-amber-950/30 dark:border-amber-800 dark:text-amber-200">
              <AlertCircle className="h-5 w-5 flex-shrink-0 mt-0.5" />
              <p className="text-sm">
                Changes to the compose configuration require a service restart to take effect.
                Make sure your YAML syntax is valid before saving.
              </p>
            </div>
          )}

          <form onSubmit={handleSaveCompose}>
            <div className="space-y-4">
              <div className="relative">
                <Textarea
                  name="compose_content"
                  value={composeContent}
                  onChange={(e) => setComposeContent(e.target.value)}
                  readOnly={!isEditing}
                  className="font-mono text-sm min-h-[400px] resize-y bg-muted/50"
                  style={{
                    fontFamily: "'JetBrains Mono', 'Fira Code', 'Consolas', monospace",
                    lineHeight: "1.5",
                    tabSize: 2,
                  }}
                  placeholder="version: '3.8'&#10;services:&#10;  app:&#10;    image: nginx:latest&#10;    ports:&#10;      - '8080:80'"
                />
              </div>

              {isEditing && (
                <div className="flex justify-end gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    onClick={handleCancelEdit}
                    disabled={isSubmitting}
                    className="gap-2"
                  >
                    <X className="h-4 w-4" />
                    Cancel
                  </Button>
                  <Button
                    type="submit"
                    disabled={isSubmitting}
                    className="gap-2"
                  >
                    <Save className="h-4 w-4" />
                    {updateComposeMutation.isPending ? "Saving..." : "Save Changes"}
                  </Button>
                </div>
              )}
            </div>
          </form>
        </CardContent>
      </Card>

      {/* Danger Zone */}
      <Card className="border-destructive">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-destructive">
            <AlertTriangle className="h-5 w-5" />
            Danger Zone
          </CardTitle>
          <CardDescription>
            Irreversible and destructive actions
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between p-4 border border-destructive/50 rounded-lg">
            <div>
              <h4 className="font-medium">Delete Service</h4>
              <p className="text-sm text-muted-foreground">
                Permanently delete this service and all its data. This action cannot be undone.
              </p>
            </div>
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button variant="destructive" className="gap-2">
                  <Trash2 className="h-4 w-4" />
                  Delete Service
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>Delete Service</AlertDialogTitle>
                  <AlertDialogDescription>
                    This action cannot be undone. This will permanently delete the service
                    <strong className="text-foreground"> {service.name}</strong> and remove all
                    associated containers and volumes.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <div className="py-4">
                  <Label htmlFor="confirm-name" className="text-sm">
                    Type <strong>{service.name}</strong> to confirm:
                  </Label>
                  <Input
                    id="confirm-name"
                    value={deleteConfirmName}
                    onChange={(e) => setDeleteConfirmName(e.target.value)}
                    placeholder={service.name}
                    className="mt-2"
                  />
                </div>
                <AlertDialogFooter>
                  <AlertDialogCancel onClick={() => setDeleteConfirmName("")}>
                    Cancel
                  </AlertDialogCancel>
                  <AlertDialogAction
                    onClick={handleDelete}
                    disabled={deleteConfirmName !== service.name || isSubmitting}
                    className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                  >
                    {deleteMutation.isPending ? "Deleting..." : "Delete Service"}
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
