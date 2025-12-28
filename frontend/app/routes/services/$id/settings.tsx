import { useState, useEffect } from "react";
import { useOutletContext, Form, useNavigation, useActionData } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
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
import type { Route } from "./+types/settings";
import type { Service } from "@/types/api";
import { Trash2, AlertTriangle, Code, Pencil, X, Save, AlertCircle } from "lucide-react";

interface OutletContext {
  service: Service;
  token: string;
}

export async function action({ request, params }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");
  const { redirect } = await import("react-router");

  const token = await requireAuth(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "update-compose") {
    const composeContent = formData.get("compose_content");
    if (typeof composeContent !== "string") {
      return { error: "Invalid compose content" };
    }

    try {
      await api.updateService(token, params.id!, { compose_content: composeContent });
      return { success: true, action: "update-compose" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to update compose configuration" };
    }
  }

  if (intent === "delete") {
    const projectId = formData.get("projectId");
    try {
      await api.deleteService(token, params.id!);
      if (projectId) {
        return redirect(`/projects/${projectId}`);
      }
      return redirect("/projects");
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to delete service" };
    }
  }

  return { error: "Unknown action" };
}

export default function ServiceSettingsTab() {
  const { service } = useOutletContext<OutletContext>();
  const navigation = useNavigation();
  const actionData = useActionData<typeof action>();
  const [deleteConfirmName, setDeleteConfirmName] = useState("");
  const [isEditing, setIsEditing] = useState(false);
  const [composeContent, setComposeContent] = useState(service.compose_content);

  const isSubmitting = navigation.state === "submitting";
  const submittingIntent = navigation.formData?.get("intent");

  // Handle action results
  useEffect(() => {
    if (actionData?.success && actionData.action === "update-compose") {
      toast.success("Compose configuration updated. Restart the service to apply changes.");
      setIsEditing(false);
    }
    if (actionData?.error) {
      toast.error(actionData.error);
    }
  }, [actionData]);

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

          <Form method="post">
            <input type="hidden" name="intent" value="update-compose" />
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
                    disabled={isSubmitting && submittingIntent === "update-compose"}
                    className="gap-2"
                  >
                    <X className="h-4 w-4" />
                    Cancel
                  </Button>
                  <Button
                    type="submit"
                    disabled={isSubmitting && submittingIntent === "update-compose"}
                    className="gap-2"
                  >
                    <Save className="h-4 w-4" />
                    {isSubmitting && submittingIntent === "update-compose"
                      ? "Saving..."
                      : "Save Changes"}
                  </Button>
                </div>
              )}
            </div>
          </Form>
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
                  <Form method="post">
                    <input type="hidden" name="intent" value="delete" />
                    <input type="hidden" name="projectId" value={service.project_id || ""} />
                    <AlertDialogAction
                      type="submit"
                      disabled={deleteConfirmName !== service.name || isSubmitting}
                      className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                    >
                      {isSubmitting ? "Deleting..." : "Delete Service"}
                    </AlertDialogAction>
                  </Form>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
