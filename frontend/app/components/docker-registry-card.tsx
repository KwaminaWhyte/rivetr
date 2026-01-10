import { useState, useEffect } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Eye, EyeOff, Package, GitBranch } from "lucide-react";
import api from "@/lib/api";
import type { App, UpdateAppRequest } from "@/types/api";

interface DockerRegistryCardProps {
  app: App;
  token?: string;
}

export function DockerRegistryCard({ app, token }: DockerRegistryCardProps) {
  const queryClient = useQueryClient();

  // Determine if using registry mode based on whether docker_image is set
  const [useRegistry, setUseRegistry] = useState(!!app.docker_image);
  const [dockerImage, setDockerImage] = useState(app.docker_image || "");
  const [dockerImageTag, setDockerImageTag] = useState(
    app.docker_image_tag || "latest",
  );
  const [registryUrl, setRegistryUrl] = useState(app.registry_url || "");
  const [registryUsername, setRegistryUsername] = useState(
    app.registry_username || "",
  );
  const [registryPassword, setRegistryPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [isDirty, setIsDirty] = useState(false);

  // Reset form when app changes
  useEffect(() => {
    setUseRegistry(!!app.docker_image);
    setDockerImage(app.docker_image || "");
    setDockerImageTag(app.docker_image_tag || "latest");
    setRegistryUrl(app.registry_url || "");
    setRegistryUsername(app.registry_username || "");
    setRegistryPassword("");
    setIsDirty(false);
  }, [app]);

  const updateMutation = useMutation({
    mutationFn: (data: UpdateAppRequest) => api.updateApp(app.id, data, token),
    onSuccess: () => {
      toast.success("Docker registry settings updated");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
      setRegistryPassword(""); // Clear password after save
      setIsDirty(false);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to update registry settings");
    },
  });

  const handleModeChange = (checked: boolean) => {
    setUseRegistry(checked);
    setIsDirty(true);
    if (!checked) {
      // Switching to Git mode - clear registry fields
      setDockerImage("");
      setDockerImageTag("latest");
      setRegistryUrl("");
      setRegistryUsername("");
      setRegistryPassword("");
    }
  };

  const handleSave = () => {
    const updates: UpdateAppRequest = {};

    if (useRegistry) {
      // Validate image is provided
      if (!dockerImage.trim()) {
        toast.error("Docker image name is required");
        return;
      }

      updates.docker_image = dockerImage.trim();
      updates.docker_image_tag = dockerImageTag.trim() || "latest";
      updates.registry_url = registryUrl.trim() || undefined;
      updates.registry_username = registryUsername.trim() || undefined;

      // Only include password if it was provided
      if (registryPassword) {
        updates.registry_password = registryPassword;
      }

      // Clear git URL when switching to registry mode
      updates.git_url = "";
    } else {
      // Clear registry fields
      updates.docker_image = "";
      updates.docker_image_tag = "";
      updates.registry_url = "";
      updates.registry_username = "";
      updates.registry_password = "";
    }

    updateMutation.mutate(updates);
  };

  const isSaving = updateMutation.isPending;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Package className="h-5 w-5" />
          Deployment Source
        </CardTitle>
        <CardDescription>
          Choose to build from a Git repository or deploy a pre-built Docker
          image from a registry.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Mode toggle */}
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <Label
              htmlFor="use-registry"
              className="text-base flex items-center gap-2"
            >
              {useRegistry ? (
                <>
                  <Package className="h-4 w-4" />
                  Pull from Docker Registry
                </>
              ) : (
                <>
                  <GitBranch className="h-4 w-4" />
                  Build from Git
                </>
              )}
            </Label>
            <p className="text-sm text-muted-foreground">
              {useRegistry
                ? "Deploy a pre-built image from Docker Hub or a private registry"
                : "Clone and build from a Git repository (default)"}
            </p>
          </div>
          <Switch
            id="use-registry"
            checked={useRegistry}
            onCheckedChange={handleModeChange}
            disabled={isSaving}
          />
        </div>

        {/* Registry configuration (shown when using registry mode) */}
        {useRegistry && (
          <div className="space-y-4 pt-4 border-t">
            <div className="space-y-2">
              <Label htmlFor="docker-image">Docker Image *</Label>
              <Input
                id="docker-image"
                value={dockerImage}
                onChange={(e) => {
                  setDockerImage(e.target.value);
                  setIsDirty(true);
                }}
                placeholder="nginx, ghcr.io/user/app, registry.example.com/image"
              />
              <p className="text-xs text-muted-foreground">
                Image name with optional registry prefix (e.g., nginx,
                ghcr.io/user/app)
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="docker-image-tag">Image Tag</Label>
              <Input
                id="docker-image-tag"
                value={dockerImageTag}
                onChange={(e) => {
                  setDockerImageTag(e.target.value);
                  setIsDirty(true);
                }}
                placeholder="latest"
              />
              <p className="text-xs text-muted-foreground">
                Tag or version (e.g., latest, v1.0.0, sha-abc123)
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="registry-url">Registry URL (optional)</Label>
              <Input
                id="registry-url"
                value={registryUrl}
                onChange={(e) => {
                  setRegistryUrl(e.target.value);
                  setIsDirty(true);
                }}
                placeholder="Leave empty for Docker Hub"
              />
              <p className="text-xs text-muted-foreground">
                Custom registry URL. Leave empty to use Docker Hub.
              </p>
            </div>

            <div className="pt-2 pb-2">
              <p className="text-sm font-medium mb-3">
                Registry Authentication (optional)
              </p>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="registry-username">Username</Label>
                  <Input
                    id="registry-username"
                    value={registryUsername}
                    onChange={(e) => {
                      setRegistryUsername(e.target.value);
                      setIsDirty(true);
                    }}
                    placeholder="Username or access token name"
                    autoComplete="off"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="registry-password">
                    {app.registry_username
                      ? "Password (leave blank to keep current)"
                      : "Password / Access Token"}
                  </Label>
                  <div className="relative">
                    <Input
                      id="registry-password"
                      type={showPassword ? "text" : "password"}
                      value={registryPassword}
                      onChange={(e) => {
                        setRegistryPassword(e.target.value);
                        setIsDirty(true);
                      }}
                      placeholder={
                        app.registry_username
                          ? "Leave blank to keep current"
                          : "Password or access token"
                      }
                      autoComplete="new-password"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
                      onClick={() => setShowPassword(!showPassword)}
                    >
                      {showPassword ? (
                        <EyeOff className="h-4 w-4 text-muted-foreground" />
                      ) : (
                        <Eye className="h-4 w-4 text-muted-foreground" />
                      )}
                    </Button>
                  </div>
                </div>
              </div>
              <p className="text-xs text-muted-foreground mt-2">
                Required for private registries. For GitHub Container Registry,
                use a personal access token with read:packages scope.
              </p>
            </div>

            <Button
              onClick={handleSave}
              disabled={isSaving || !isDirty}
              className="w-full sm:w-auto"
            >
              {isSaving ? "Saving..." : "Save Registry Settings"}
            </Button>
          </div>
        )}

        {/* Git mode indicator */}
        {!useRegistry && (
          <div className="pt-4 border-t">
            <p className="text-sm text-muted-foreground">
              Currently configured to build from Git repository:{" "}
              <code className="px-1.5 py-0.5 rounded bg-muted font-mono text-foreground">
                {app.git_url || "Not configured"}
              </code>
            </p>
            {isDirty && (
              <Button
                onClick={handleSave}
                disabled={isSaving}
                className="mt-4 w-full sm:w-auto"
              >
                {isSaving ? "Saving..." : "Switch to Git Mode"}
              </Button>
            )}
          </div>
        )}

        {/* Current registry status */}
        {app.docker_image && (
          <div className="pt-4 border-t">
            <p className="text-sm text-muted-foreground">
              Currently pulling image:{" "}
              <code className="px-1.5 py-0.5 rounded bg-muted font-mono text-foreground">
                {app.docker_image}:{app.docker_image_tag || "latest"}
              </code>
              {app.registry_username && (
                <span className="ml-2">
                  (authenticated as{" "}
                  <code className="px-1 py-0.5 rounded bg-muted font-mono text-foreground">
                    {app.registry_username}
                  </code>
                  )
                </span>
              )}
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
