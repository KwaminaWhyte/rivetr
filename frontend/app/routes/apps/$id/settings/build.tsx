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
import { Sparkles, FileCode, Package, Zap, Cloud, Lock, Plus, Trash2, AlertTriangle, Github, Cpu, Wand2, Copy, RefreshCw } from "lucide-react";
import { Checkbox } from "@/components/ui/checkbox";
import { DockerRegistryCard } from "@/components/docker-registry-card";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api";
import { aiApi } from "@/lib/api/ai";
import { buildServersApi, type BuildServer } from "@/lib/api/build-servers";
import type { App, BuildType, BuildSecret, NixpacksConfig, UpdateAppRequest } from "@/types/api";

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
    custom_container_name: app.custom_container_name || "",
  });

  // Git clone options
  const [gitSubmodules, setGitSubmodules] = useState(app.git_submodules || false);
  const [gitLfs, setGitLfs] = useState(app.git_lfs || false);
  const [shallowClone, setShallowClone] = useState(app.shallow_clone !== false);

  // Build options
  const [disableBuildCache, setDisableBuildCache] = useState(app.disable_build_cache || false);
  const [includeSourceCommit, setIncludeSourceCommit] = useState(app.include_source_commit || false);
  const [isStaticSite, setIsStaticSite] = useState(app.is_static_site || false);
  const [inlineDockerfile, setInlineDockerfile] = useState(app.inline_dockerfile || "");

  const [buildType, setBuildType] = useState<BuildType>(app.build_type || "dockerfile");
  const [previewEnabled, setPreviewEnabled] = useState(app.preview_enabled || false);
  const [publishDirectory, setPublishDirectory] = useState(app.publish_directory || "dist");
  const [buildServerId, setBuildServerId] = useState<string>(app.build_server_id || "");

  // Target platforms state: parse comma-separated string into a Set
  const parsePlatforms = (val: string | null): Set<string> => {
    if (!val) return new Set(["linux/amd64"]);
    const parts = val.split(",").map(s => s.trim()).filter(Boolean);
    return parts.length > 0 ? new Set(parts) : new Set(["linux/amd64"]);
  };
  const [selectedPlatforms, setSelectedPlatforms] = useState<Set<string>>(
    parsePlatforms(app.build_platforms)
  );

  const togglePlatform = (platform: string) => {
    setSelectedPlatforms(prev => {
      const next = new Set(prev);
      if (next.has(platform)) {
        next.delete(platform);
        // Always keep at least one platform selected
        if (next.size === 0) next.add("linux/amd64");
      } else {
        next.add(platform);
      }
      return next;
    });
  };

  const { data: buildServers = [] } = useQuery<BuildServer[]>({
    queryKey: ["build-servers"],
    queryFn: () => buildServersApi.list(),
  });

  // Build Secrets state
  const parseBuildSecrets = (json: string | null): BuildSecret[] => {
    if (!json) return [];
    try {
      return JSON.parse(json);
    } catch {
      return [];
    }
  };

  const [buildSecrets, setBuildSecrets] = useState<BuildSecret[]>(
    parseBuildSecrets(app.build_secrets)
  );
  const [isSavingSecrets, setIsSavingSecrets] = useState(false);

  // GitHub Actions workflow state
  const [workflowYaml, setWorkflowYaml] = useState<string | null>(null);
  const [isLoadingWorkflow, setIsLoadingWorkflow] = useState(false);

  // AI Dockerfile Optimizer state
  const [dockerfileOptResult, setDockerfileOptResult] = useState<{ original: string; suggested: string; improvements: string[] } | null>(null);
  const [dockerfileOptLoading, setDockerfileOptLoading] = useState(false);
  const [dockerfileOptUnavailable, setDockerfileOptUnavailable] = useState(false);

  useEffect(() => {
    setBuildSecrets(parseBuildSecrets(app.build_secrets));
  }, [app.build_secrets]);

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
    setSelectedPlatforms(parsePlatforms(app.build_platforms));
    setGitSubmodules(app.git_submodules || false);
    setGitLfs(app.git_lfs || false);
    setShallowClone(app.shallow_clone !== false);
    setDisableBuildCache(app.disable_build_cache || false);
    setIncludeSourceCommit(app.include_source_commit || false);
    setIsStaticSite(app.is_static_site || false);
    setInlineDockerfile(app.inline_dockerfile || "");
    setBuildForm(prev => ({
      ...prev,
      custom_container_name: app.custom_container_name || "",
    }));
  }, [app.build_type, app.preview_enabled, app.publish_directory, app.nixpacks_config, app.build_server_id, app.build_platforms, app.git_submodules, app.git_lfs, app.shallow_clone, app.disable_build_cache, app.include_source_commit, app.custom_container_name, app.is_static_site, app.inline_dockerfile]);

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

      // Serialize platforms: if only linux/amd64 selected (default), send null to clear
      const platformsArray = Array.from(selectedPlatforms).sort();
      const buildPlatformsValue =
        platformsArray.length === 1 && platformsArray[0] === "linux/amd64"
          ? ""
          : platformsArray.join(",");

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
        build_platforms: buildPlatformsValue || undefined,
        // Git clone options
        git_submodules: gitSubmodules,
        git_lfs: gitLfs,
        shallow_clone: shallowClone,
        // Build options
        disable_build_cache: disableBuildCache,
        include_source_commit: includeSourceCommit,
        // Container naming — empty string clears it
        custom_container_name: buildForm.custom_container_name || "",
        // Static site flag
        is_static_site: isStaticSite,
        // Inline Dockerfile — empty string clears it
        inline_dockerfile: inlineDockerfile || undefined,
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

  const handleSaveSecrets = async () => {
    setIsSavingSecrets(true);
    try {
      await api.updateApp(app.id, { build_secrets: buildSecrets });
      toast.success("Build secrets saved");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to save secrets");
    } finally {
      setIsSavingSecrets(false);
    }
  };

  const handleLoadWorkflow = async () => {
    setIsLoadingWorkflow(true);
    try {
      const yaml = await api.getGithubActionsWorkflow(app.id);
      setWorkflowYaml(yaml);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to load workflow");
    } finally {
      setIsLoadingWorkflow(false);
    }
  };

  const handleDownloadWorkflow = () => {
    if (!workflowYaml) return;
    const blob = new Blob([workflowYaml], { type: "text/yaml" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `deploy-${app.name}.yml`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleOptimizeDockerfile = async () => {
    setDockerfileOptLoading(true);
    setDockerfileOptResult(null);
    setDockerfileOptUnavailable(false);
    try {
      const result = await aiApi.suggestDockerfile(app.id);
      setDockerfileOptResult(result);
    } catch (error) {
      const msg = error instanceof Error ? error.message : "";
      if (
        msg.includes("503") ||
        msg.toLowerCase().includes("not configured") ||
        msg.toLowerCase().includes("unavailable")
      ) {
        setDockerfileOptUnavailable(true);
      } else {
        toast.error(msg || "Failed to optimize Dockerfile");
      }
    } finally {
      setDockerfileOptLoading(false);
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
                <div className="space-y-2">
                  <Label>Inline Dockerfile (optional)</Label>
                  <Textarea
                    placeholder={"FROM node:18-alpine\nWORKDIR /app\nCOPY . .\nRUN npm install\nCMD [\"node\", \"server.js\"]"}
                    value={inlineDockerfile}
                    onChange={(e) => setInlineDockerfile(e.target.value)}
                    className="font-mono text-sm min-h-[200px]"
                  />
                  <p className="text-xs text-muted-foreground">
                    If set, this Dockerfile is used directly — no git repository needed. Leave blank to build from your git repo.
                  </p>
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

            {/* Target Platforms */}
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <Cpu className="h-4 w-4 text-muted-foreground" />
                <Label className="text-base">Target Platforms</Label>
              </div>
              <p className="text-sm text-muted-foreground">
                Select which CPU architectures to build for. Multi-platform builds use{" "}
                <code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">docker buildx</code>{" "}
                and may take longer. Requires BuildKit.
              </p>
              <div className="flex flex-col gap-2">
                {[
                  { value: "linux/amd64", label: "linux/amd64", description: "x86-64 (Intel/AMD) — default" },
                  { value: "linux/arm64", label: "linux/arm64", description: "ARM 64-bit (Apple Silicon, AWS Graviton)" },
                  { value: "linux/arm/v7", label: "linux/arm/v7", description: "ARM 32-bit v7 (Raspberry Pi)" },
                ].map(({ value, label, description }) => (
                  <div key={value} className="flex items-center gap-3">
                    <Checkbox
                      id={`platform-${value.replace(/\//g, "-")}`}
                      checked={selectedPlatforms.has(value)}
                      onCheckedChange={() => togglePlatform(value)}
                    />
                    <label
                      htmlFor={`platform-${value.replace(/\//g, "-")}`}
                      className="flex flex-col cursor-pointer"
                    >
                      <span className="text-sm font-mono font-medium">{label}</span>
                      <span className="text-xs text-muted-foreground">{description}</span>
                    </label>
                  </div>
                ))}
              </div>
              {selectedPlatforms.size > 1 && (
                <p className="text-xs text-amber-600 dark:text-amber-400">
                  Multi-platform builds require <code className="font-mono">docker buildx</code> with QEMU or a multi-platform builder.
                </p>
              )}
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

            {/* Git Clone Options */}
            <div className="space-y-3">
              <Label className="text-base">Git Clone Options</Label>
              <div className="space-y-3">
                <div className="flex items-center justify-between p-4 rounded-lg border">
                  <div className="space-y-0.5">
                    <Label htmlFor="shallow-clone" className="text-base">Shallow Clone</Label>
                    <p className="text-sm text-muted-foreground">
                      Use <code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">--depth 1</code> for faster clones (default on). Disable for full git history.
                    </p>
                  </div>
                  <Switch
                    id="shallow-clone"
                    checked={shallowClone}
                    onCheckedChange={setShallowClone}
                  />
                </div>
                <div className="flex items-center justify-between p-4 rounded-lg border">
                  <div className="space-y-0.5">
                    <Label htmlFor="git-submodules" className="text-base">Git Submodules</Label>
                    <p className="text-sm text-muted-foreground">
                      Pass <code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">--recurse-submodules</code> to clone all submodules
                    </p>
                  </div>
                  <Switch
                    id="git-submodules"
                    checked={gitSubmodules}
                    onCheckedChange={setGitSubmodules}
                  />
                </div>
                <div className="flex items-center justify-between p-4 rounded-lg border">
                  <div className="space-y-0.5">
                    <Label htmlFor="git-lfs" className="text-base">Git LFS</Label>
                    <p className="text-sm text-muted-foreground">
                      Run <code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">git lfs pull</code> after clone (requires git-lfs installed on the server)
                    </p>
                  </div>
                  <Switch
                    id="git-lfs"
                    checked={gitLfs}
                    onCheckedChange={setGitLfs}
                  />
                </div>
              </div>
            </div>

            {/* Build Options */}
            <div className="space-y-3">
              <Label className="text-base">Build Options</Label>
              <div className="space-y-3">
                <div className="flex items-center justify-between p-4 rounded-lg border">
                  <div className="space-y-0.5">
                    <Label htmlFor="disable-build-cache" className="text-base">Disable Build Cache</Label>
                    <p className="text-sm text-muted-foreground">
                      Pass <code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">--no-cache</code> to force a clean build every time
                    </p>
                  </div>
                  <Switch
                    id="disable-build-cache"
                    checked={disableBuildCache}
                    onCheckedChange={setDisableBuildCache}
                  />
                </div>
                <div className="flex items-center justify-between p-4 rounded-lg border">
                  <div className="space-y-0.5">
                    <Label htmlFor="include-source-commit" className="text-base">Inject SOURCE_COMMIT</Label>
                    <p className="text-sm text-muted-foreground">
                      Inject the current git SHA as the <code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">SOURCE_COMMIT</code> build argument
                    </p>
                  </div>
                  <Switch
                    id="include-source-commit"
                    checked={includeSourceCommit}
                    onCheckedChange={setIncludeSourceCommit}
                  />
                </div>
                <div className="flex items-center justify-between p-4 rounded-lg border">
                  <div className="space-y-0.5">
                    <Label htmlFor="is-static-site" className="text-base">Static Site</Label>
                    <p className="text-sm text-muted-foreground">
                      Serve built files as a static site. The output directory is served directly without a runtime container.
                    </p>
                  </div>
                  <Switch
                    id="is-static-site"
                    checked={isStaticSite}
                    onCheckedChange={setIsStaticSite}
                  />
                </div>
              </div>
            </div>

            {/* Custom Container Name */}
            <div className="space-y-2">
              <Label htmlFor="custom_container_name">Custom Container Name</Label>
              <Input
                id="custom_container_name"
                placeholder={`rivetr-${app.name} (default)`}
                value={buildForm.custom_container_name}
                onChange={(e) => setBuildForm({ ...buildForm, custom_container_name: e.target.value })}
              />
              <p className="text-xs text-muted-foreground">
                Override the Docker container name. Leave blank to use the default <code className="font-mono">rivetr-{app.name}</code> pattern.
              </p>
            </div>

            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? "Saving..." : "Save Changes"}
            </Button>
          </form>
        </CardContent>
      </Card>

      {/* Docker Registry / Deployment Source */}
      <DockerRegistryCard app={app} />

      {/* Build Secrets */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Lock className="h-5 w-5 text-muted-foreground" />
            <CardTitle>Build Secrets</CardTitle>
          </div>
          <CardDescription>
            Injected during <code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">docker build</code> via BuildKit{" "}
            <code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">--secret</code>. Not stored in image layers. Use{" "}
            <code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">RUN --mount=type=secret,id=KEY</code> in your Dockerfile.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 dark:border-amber-800 dark:bg-amber-950/30 p-3 text-sm text-amber-800 dark:text-amber-200">
            <AlertTriangle className="h-4 w-4 mt-0.5 shrink-0" />
            <span>Only works with Dockerfile builds. Requires BuildKit (enabled by default in Docker 23+).</span>
          </div>

          <div className="space-y-2">
            {buildSecrets.map((secret, index) => (
              <div key={index} className="flex items-center gap-2">
                <Input
                  placeholder="KEY"
                  value={secret.key}
                  onChange={(e) => {
                    const updated = [...buildSecrets];
                    updated[index] = { ...updated[index], key: e.target.value };
                    setBuildSecrets(updated);
                  }}
                  className="font-mono text-sm"
                />
                <Input
                  type="password"
                  placeholder="value"
                  value={secret.value}
                  onChange={(e) => {
                    const updated = [...buildSecrets];
                    updated[index] = { ...updated[index], value: e.target.value };
                    setBuildSecrets(updated);
                  }}
                  className="font-mono text-sm"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => setBuildSecrets(buildSecrets.filter((_, i) => i !== index))}
                >
                  <Trash2 className="h-4 w-4 text-destructive" />
                </Button>
              </div>
            ))}
          </div>

          <div className="flex items-center gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => setBuildSecrets([...buildSecrets, { key: "", value: "" }])}
            >
              <Plus className="h-4 w-4 mr-2" />
              Add Secret
            </Button>
          </div>

          <div className="flex gap-2 pt-2">
            <Button
              type="button"
              onClick={handleSaveSecrets}
              disabled={isSavingSecrets}
            >
              {isSavingSecrets ? "Saving..." : "Save Secrets"}
            </Button>
            <Button
              type="button"
              variant="outline"
              onClick={() => setBuildSecrets(parseBuildSecrets(app.build_secrets))}
              disabled={isSavingSecrets}
            >
              Cancel
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* GitHub Actions Integration */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Github className="h-5 w-5 text-muted-foreground" />
            <CardTitle>GitHub Actions</CardTitle>
          </div>
          <CardDescription>
            Automatically trigger deployments from your GitHub Actions pipeline.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-sm text-muted-foreground">
            After downloading, add the workflow file to{" "}
            <code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">.github/workflows/</code>{" "}
            in your repository and set the{" "}
            <code className="font-mono text-xs bg-muted px-1 py-0.5 rounded">RIVETR_API_TOKEN</code>{" "}
            secret in your GitHub repository settings (Settings → Secrets and variables → Actions).
          </p>

          <div className="flex items-center gap-2">
            <Button
              type="button"
              variant="outline"
              onClick={handleLoadWorkflow}
              disabled={isLoadingWorkflow}
            >
              {isLoadingWorkflow ? "Loading..." : "Preview Workflow"}
            </Button>
            {workflowYaml && (
              <Button
                type="button"
                onClick={handleDownloadWorkflow}
              >
                Download Workflow
              </Button>
            )}
          </div>

          {workflowYaml && (
            <pre className="rounded-md border bg-muted p-4 text-xs font-mono overflow-x-auto whitespace-pre">
              {workflowYaml}
            </pre>
          )}
        </CardContent>
      </Card>

      {/* AI Dockerfile Optimizer */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Wand2 className="h-5 w-5 text-purple-500" />
                AI Dockerfile Optimizer
              </CardTitle>
              <CardDescription>
                Get AI-powered suggestions to improve your Dockerfile for smaller images, faster builds, and better security.
              </CardDescription>
            </div>
            <Button
              type="button"
              variant="outline"
              onClick={handleOptimizeDockerfile}
              disabled={dockerfileOptLoading}
              className="gap-2"
            >
              <Sparkles className="h-4 w-4" />
              {dockerfileOptLoading ? "Optimizing..." : "Optimize Dockerfile"}
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {dockerfileOptLoading && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <RefreshCw className="h-4 w-4 animate-spin" />
              Analyzing your Dockerfile...
            </div>
          )}
          {dockerfileOptUnavailable && (
            <p className="text-sm text-muted-foreground">
              Configure AI provider in instance settings to use this feature.
            </p>
          )}
          {dockerfileOptResult && !dockerfileOptLoading && (
            <div className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div className="space-y-2">
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Current</p>
                  <pre className="rounded-md border bg-muted p-4 text-xs font-mono overflow-x-auto whitespace-pre max-h-80">
                    {dockerfileOptResult.original}
                  </pre>
                </div>
                <div className="space-y-2">
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Suggested</p>
                  <pre className="rounded-md border border-purple-200 bg-purple-50/40 dark:bg-purple-950/10 p-4 text-xs font-mono overflow-x-auto whitespace-pre max-h-80">
                    {dockerfileOptResult.suggested}
                  </pre>
                </div>
              </div>

              {dockerfileOptResult.improvements.length > 0 && (
                <div className="space-y-2">
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Improvements</p>
                  <ul className="space-y-1.5">
                    {dockerfileOptResult.improvements.map((imp, i) => (
                      <li key={i} className="flex items-start gap-2 text-sm">
                        <span className="text-purple-500 mt-0.5 shrink-0">•</span>
                        <span>{imp}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}

              <div className="flex gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="gap-2"
                  onClick={() => {
                    navigator.clipboard.writeText(dockerfileOptResult.suggested);
                    toast.success("Suggested Dockerfile copied to clipboard");
                  }}
                >
                  <Copy className="h-4 w-4" />
                  Copy Suggested
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="gap-2"
                  onClick={handleOptimizeDockerfile}
                  disabled={dockerfileOptLoading}
                >
                  <RefreshCw className="h-4 w-4" />
                  Re-analyze
                </Button>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
