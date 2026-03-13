import { useState, useEffect } from "react";
import { useOutletContext } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Sparkles, FileCode, Package, Zap, Cloud } from "lucide-react";
import { DockerRegistryCard } from "@/components/docker-registry-card";
import { api } from "@/lib/api";
import { buildServersApi, type BuildServer } from "@/lib/api/build-servers";
import type { App, BuildType, NixpacksConfig, UpdateAppRequest } from "@/types/api";

export default function AppSettingsBuild() {
  const { app } = useOutletContext<{ app: App }>();
  const queryClient = useQueryClient();
  const [isSubmitting, setIsSubmitting] = useState(false);

  const [buildForm, setBuildForm] = useState({
    dockerfile: app.dockerfile,
    dockerfile_path: app.dockerfile_path || "",
    base_directory: app.base_directory || "",
    build_target: app.build_target || "",
    watch_paths: app.watch_paths || "",
    custom_docker_options: app.custom_docker_options || "",
  });

  const [buildType, setBuildType] = useState<BuildType>(app.build_type || "dockerfile");
  const [previewEnabled, setPreviewEnabled] = useState(app.preview_enabled || false);
  const [publishDirectory, setPublishDirectory] = useState(app.publish_directory || "dist");
  const [buildServerId, setBuildServerId] = useState<string>(app.build_server_id || "");

  const { data: buildServers = [] } = useQuery<BuildServer[]>({
    queryKey: ["build-servers"],
    queryFn: () => buildServersApi.list(),
  });

  const parseNixpacksConfig = (json: string | null): NixpacksConfig => {
    if (!json) return {};
    try {
      return JSON.parse(json);
    } catch {
      return {};
    }
  };

  const [nixpacksConfig, setNixpacksConfig] = useState<NixpacksConfig>(
    parseNixpacksConfig(app.nixpacks_config)
  );

  useEffect(() => {
    setBuildType(app.build_type || "dockerfile");
    setPreviewEnabled(app.preview_enabled || false);
    setPublishDirectory(app.publish_directory || "dist");
    setNixpacksConfig(parseNixpacksConfig(app.nixpacks_config));
    setBuildServerId(app.build_server_id || "");
  }, [app.build_type, app.preview_enabled, app.publish_directory, app.nixpacks_config, app.build_server_id]);

  const handleBuildSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      let nixpacksConfigToSend: NixpacksConfig | undefined = undefined;
      if (buildType === "nixpacks") {
        nixpacksConfigToSend = {};
        if (nixpacksConfig.install_cmd) nixpacksConfigToSend.install_cmd = nixpacksConfig.install_cmd;
        if (nixpacksConfig.build_cmd) nixpacksConfigToSend.build_cmd = nixpacksConfig.build_cmd;
        if (nixpacksConfig.start_cmd) nixpacksConfigToSend.start_cmd = nixpacksConfig.start_cmd;
        if (nixpacksConfig.packages?.length) nixpacksConfigToSend.packages = nixpacksConfig.packages;
        if (nixpacksConfig.apt_packages?.length) nixpacksConfigToSend.apt_packages = nixpacksConfig.apt_packages;
        if (Object.keys(nixpacksConfigToSend).length === 0) {
          nixpacksConfigToSend = undefined;
        }
      }

      const updates: UpdateAppRequest = {
        dockerfile: buildType === "dockerfile" ? buildForm.dockerfile : undefined,
        dockerfile_path: buildType === "dockerfile" ? buildForm.dockerfile_path : undefined,
        base_directory: buildForm.base_directory,
        build_target: buildType === "dockerfile" ? buildForm.build_target : undefined,
        watch_paths: buildForm.watch_paths,
        custom_docker_options: buildType === "dockerfile" ? buildForm.custom_docker_options : undefined,
        build_type: buildType,
        nixpacks_config: nixpacksConfigToSend,
        publish_directory: buildType === "staticsite" ? publishDirectory : undefined,
        preview_enabled: previewEnabled,
        // Empty string clears the build server assignment on the backend
        build_server_id: buildServerId || "",
      };
      await api.updateApp(app.id, updates);
      toast.success("Settings saved");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Update failed");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Build Configuration</CardTitle>
          <CardDescription>
            Configure how your application is built.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleBuildSubmit} className="space-y-6">
            {/* Build Type Selection */}
            <div className="space-y-3">
              <Label>Build Type</Label>
              <div className="grid grid-cols-3 md:grid-cols-5 gap-3">
                <button
                  type="button"
                  onClick={() => setBuildType("nixpacks")}
                  className={`flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-colors ${
                    buildType === "nixpacks"
                      ? "border-primary bg-primary/5"
                      : "border-border hover:border-muted-foreground/50"
                  }`}
                >
                  <Sparkles className="h-6 w-6" />
                  <span className="text-sm font-medium">Nixpacks</span>
                  <span className="text-xs text-muted-foreground text-center">
                    Auto-detect
                  </span>
                </button>
                <button
                  type="button"
                  onClick={() => setBuildType("railpack")}
                  className={`flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-colors ${
                    buildType === "railpack"
                      ? "border-primary bg-primary/5"
                      : "border-border hover:border-muted-foreground/50"
                  }`}
                >
                  <Zap className="h-6 w-6" />
                  <span className="text-sm font-medium">Railpack</span>
                  <span className="text-xs text-muted-foreground text-center">
                    Fast builds
                  </span>
                </button>
                <button
                  type="button"
                  onClick={() => setBuildType("cnb")}
                  className={`flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-colors ${
                    buildType === "cnb"
                      ? "border-primary bg-primary/5"
                      : "border-border hover:border-muted-foreground/50"
                  }`}
                >
                  <Cloud className="h-6 w-6" />
                  <span className="text-sm font-medium">Buildpacks</span>
                  <span className="text-xs text-muted-foreground text-center">
                    Paketo/Heroku
                  </span>
                </button>
                <button
                  type="button"
                  onClick={() => setBuildType("dockerfile")}
                  className={`flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-colors ${
                    buildType === "dockerfile"
                      ? "border-primary bg-primary/5"
                      : "border-border hover:border-muted-foreground/50"
                  }`}
                >
                  <FileCode className="h-6 w-6" />
                  <span className="text-sm font-medium">Dockerfile</span>
                  <span className="text-xs text-muted-foreground text-center">
                    Custom build
                  </span>
                </button>
                <button
                  type="button"
                  onClick={() => setBuildType("staticsite")}
                  className={`flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-colors ${
                    buildType === "staticsite"
                      ? "border-primary bg-primary/5"
                      : "border-border hover:border-muted-foreground/50"
                  }`}
                >
                  <Package className="h-6 w-6" />
                  <span className="text-sm font-medium">Static</span>
                  <span className="text-xs text-muted-foreground text-center">
                    HTML/CSS/JS
                  </span>
                </button>
              </div>
            </div>

            {/* Nixpacks options */}
            {buildType === "nixpacks" && (
              <div className="space-y-4 p-4 bg-muted/50 rounded-lg">
                <p className="text-sm text-muted-foreground">
                  Nixpacks will automatically detect your project type and build it.
                  You can optionally override the default commands.
                </p>
                <div className="grid gap-4 md:grid-cols-3">
                  <div className="space-y-2">
                    <Label htmlFor="install_cmd">Install Command</Label>
                    <Input
                      id="install_cmd"
                      placeholder="npm install"
                      value={nixpacksConfig.install_cmd || ""}
                      onChange={(e) => setNixpacksConfig({ ...nixpacksConfig, install_cmd: e.target.value || undefined })}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="build_cmd">Build Command</Label>
                    <Input
                      id="build_cmd"
                      placeholder="npm run build"
                      value={nixpacksConfig.build_cmd || ""}
                      onChange={(e) => setNixpacksConfig({ ...nixpacksConfig, build_cmd: e.target.value || undefined })}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="start_cmd">Start Command</Label>
                    <Input
                      id="start_cmd"
                      placeholder="npm start"
                      value={nixpacksConfig.start_cmd || ""}
                      onChange={(e) => setNixpacksConfig({ ...nixpacksConfig, start_cmd: e.target.value || undefined })}
                    />
                  </div>
                </div>
              </div>
            )}

            {/* Railpack options */}
            {buildType === "railpack" && (
              <div className="space-y-4 p-4 bg-muted/50 rounded-lg">
                <p className="text-sm text-muted-foreground">
                  Railpack (Railway's Nixpacks successor) offers faster builds with better caching.
                  38% faster for Node.js, 77% faster for Python compared to Nixpacks.
                </p>
                <p className="text-xs text-muted-foreground">
                  <strong>Note:</strong> Requires Railpack CLI and BuildKit. Linux/macOS only (Windows not supported).
                </p>
              </div>
            )}

            {/* Cloud Native Buildpacks options */}
            {buildType === "cnb" && (
              <div className="space-y-4 p-4 bg-muted/50 rounded-lg">
                <p className="text-sm text-muted-foreground">
                  Cloud Native Buildpacks (Paketo/Heroku) create production-ready, security-focused container images
                  without requiring a Dockerfile. Auto-selects the best builder for your project.
                </p>
                <p className="text-xs text-muted-foreground">
                  <strong>Note:</strong> Requires Pack CLI installed. Supports Java, Node.js, Python, Go, Ruby, .NET, and more.
                </p>
              </div>
            )}

            {/* Static options */}
            {buildType === "staticsite" && (
              <div className="space-y-2">
                <Label htmlFor="publish_directory">Publish Directory</Label>
                <Input
                  id="publish_directory"
                  placeholder="dist, build, public"
                  value={publishDirectory}
                  onChange={(e) => setPublishDirectory(e.target.value)}
                />
                <p className="text-xs text-muted-foreground">
                  Directory containing your built static files
                </p>
              </div>
            )}

            {/* Dockerfile options */}
            {buildType === "dockerfile" && (
              <>
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="dockerfile">Dockerfile</Label>
                    <Input
                      id="dockerfile"
                      value={buildForm.dockerfile}
                      onChange={(e) => setBuildForm({ ...buildForm, dockerfile: e.target.value })}
                    />
                    <p className="text-xs text-muted-foreground">
                      Dockerfile name (e.g., Dockerfile)
                    </p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="dockerfile_path">Dockerfile Path</Label>
                    <Input
                      id="dockerfile_path"
                      placeholder="Dockerfile.prod"
                      value={buildForm.dockerfile_path}
                      onChange={(e) => setBuildForm({ ...buildForm, dockerfile_path: e.target.value })}
                    />
                    <p className="text-xs text-muted-foreground">
                      Custom Dockerfile location (relative to base directory)
                    </p>
                  </div>
                </div>

                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="build_target">Build Target</Label>
                    <Input
                      id="build_target"
                      placeholder="production"
                      value={buildForm.build_target}
                      onChange={(e) => setBuildForm({ ...buildForm, build_target: e.target.value })}
                    />
                    <p className="text-xs text-muted-foreground">
                      Multi-stage build target (--target flag)
                    </p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="custom_docker_options">Custom Docker Options</Label>
                    <Textarea
                      id="custom_docker_options"
                      placeholder="--no-cache --build-arg FOO=bar"
                      rows={2}
                      value={buildForm.custom_docker_options}
                      onChange={(e) => setBuildForm({ ...buildForm, custom_docker_options: e.target.value })}
                    />
                    <p className="text-xs text-muted-foreground">
                      Extra Docker build arguments
                    </p>
                  </div>
                </div>
              </>
            )}

            {/* Common settings */}
            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="base_directory">Base Directory</Label>
                <Input
                  id="base_directory"
                  placeholder="backend/"
                  value={buildForm.base_directory}
                  onChange={(e) => setBuildForm({ ...buildForm, base_directory: e.target.value })}
                />
                <p className="text-xs text-muted-foreground">
                  Subdirectory to use as build context
                </p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="watch_paths">Watch Paths</Label>
                <Input
                  id="watch_paths"
                  placeholder='["src/", "package.json"]'
                  value={buildForm.watch_paths}
                  onChange={(e) => setBuildForm({ ...buildForm, watch_paths: e.target.value })}
                />
                <p className="text-xs text-muted-foreground">
                  JSON array of paths to trigger auto-deploy
                </p>
              </div>
            </div>

            {/* Build Server selection */}
            <div className="space-y-2">
              <Label htmlFor="build-server">Build Server</Label>
              <Select
                value={buildServerId || "__local__"}
                onValueChange={(v) => setBuildServerId(v === "__local__" ? "" : v)}
              >
                <SelectTrigger id="build-server">
                  <SelectValue placeholder="Local (default)" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__local__">Local (default)</SelectItem>
                  {buildServers.map((bs) => (
                    <SelectItem key={bs.id} value={bs.id}>
                      {bs.name} ({bs.host}:{bs.port})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                Offload Docker builds to a dedicated remote build server. Leave blank to build locally.
              </p>
            </div>

            {/* Preview deployments toggle */}
            <div className="flex items-center justify-between p-4 rounded-lg border">
              <div className="space-y-0.5">
                <Label htmlFor="preview-enabled" className="text-base">Enable PR Previews</Label>
                <p className="text-sm text-muted-foreground">
                  Automatically deploy preview environments for pull requests
                </p>
              </div>
              <Switch
                id="preview-enabled"
                checked={previewEnabled}
                onCheckedChange={setPreviewEnabled}
              />
            </div>

            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? "Saving..." : "Save Changes"}
            </Button>
          </form>
        </CardContent>
      </Card>

      {/* Docker Registry / Deployment Source */}
      <DockerRegistryCard app={app} />
    </div>
  );
}
