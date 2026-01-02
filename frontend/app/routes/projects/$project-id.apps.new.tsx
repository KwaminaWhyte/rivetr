import { useState } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Switch } from "@/components/ui/switch";
import { Eye, EyeOff, GitBranch, Package, Sparkles, FileCode, Github, Link2, Upload, Zap, Cloud } from "lucide-react";
import { CPU_OPTIONS, MEMORY_OPTIONS } from "@/components/resource-limits-card";
import { GitHubRepoPicker, type SelectedRepo } from "@/components/github-repo-picker";
import { ZipUploadZone } from "@/components/zip-upload-zone";
import { api } from "@/lib/api";
import type { AppEnvironment, BuildType, BuildDetectionResult, NixpacksConfig, Project, ProjectWithApps, CreateAppRequest } from "@/types/api";

const ENVIRONMENT_OPTIONS: { value: AppEnvironment; label: string }[] = [
  { value: "development", label: "Development" },
  { value: "staging", label: "Staging" },
  { value: "production", label: "Production" },
];

export function meta() {
  return [
    { title: "New Application - Rivetr" },
    { name: "description", content: "Create a new application" },
  ];
}

export default function NewAppPage() {
  const navigate = useNavigate();
  const { projectId } = useParams();
  const queryClient = useQueryClient();
  const [deploymentSource, setDeploymentSource] = useState<"git" | "registry" | "upload">("git");
  const [gitSourceType, setGitSourceType] = useState<"github" | "manual">("github");
  const [buildType, setBuildType] = useState<BuildType>("nixpacks");
  const [previewEnabled, setPreviewEnabled] = useState(false);
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Upload state
  const [uploadFile, setUploadFile] = useState<File | null>(null);
  const [detectionResult, setDetectionResult] = useState<BuildDetectionResult | null>(null);
  const [isDetecting, setIsDetecting] = useState(false);

  // GitHub repo picker state
  const [selectedRepo, setSelectedRepo] = useState<SelectedRepo | null>(null);
  const [manualGitUrl, setManualGitUrl] = useState("");
  const [manualBranch, setManualBranch] = useState("main");

  // Nixpacks config state
  const [nixpacksConfig, setNixpacksConfig] = useState<NixpacksConfig>({
    install_cmd: undefined,
    build_cmd: undefined,
    start_cmd: undefined,
    packages: undefined,
    apt_packages: undefined,
  });

  // Use React Query for data fetching
  const { data: projects = [] } = useQuery<Project[]>({
    queryKey: ["projects"],
    queryFn: () => api.getProjects(),
  });

  const { data: project } = useQuery<ProjectWithApps>({
    queryKey: ["project", projectId],
    queryFn: () => api.getProject(projectId!),
    enabled: !!projectId,
  });

  // Mutation for creating app
  const createAppMutation = useMutation({
    mutationFn: (data: CreateAppRequest) => api.createApp(data),
    onSuccess: (app) => {
      queryClient.invalidateQueries({ queryKey: ["apps"] });
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
      navigate(`/apps/${app.id}`);
    },
    onError: (err: Error) => {
      setError(err.message);
    },
  });

  // Mutation for creating app from upload
  const uploadCreateMutation = useMutation({
    mutationFn: async (data: {
      file: File;
      config: {
        name: string;
        port?: number;
        domain?: string;
        healthcheck?: string;
        cpu_limit?: string;
        memory_limit?: string;
        environment?: string;
        build_type?: string;
        publish_directory?: string;
      };
    }) => {
      return api.uploadCreateApp(projectId!, data.file, data.config);
    },
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: ["apps"] });
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
      navigate(`/apps/${result.app.id}`);
    },
    onError: (err: Error) => {
      setError(err.message);
    },
  });

  // Handle file selection for upload
  const handleFileSelect = async (file: File) => {
    setUploadFile(file);
    setDetectionResult(null);
    setIsDetecting(true);

    try {
      const result = await api.detectBuildType(file);
      setDetectionResult(result);
      // Auto-set build type based on detection
      if (result.build_type === "dockerfile" || result.build_type === "nixpacks" || result.build_type === "staticsite") {
        setBuildType(result.build_type as BuildType);
      }
    } catch (err) {
      console.warn("Build detection failed:", err);
    } finally {
      setIsDetecting(false);
    }
  };

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    const formData = new FormData(event.currentTarget);

    const name = formData.get("name") as string;
    const port = parseInt(formData.get("port") as string) || 3000;
    const domain = (formData.get("domain") as string) || undefined;
    const healthcheck = (formData.get("healthcheck") as string) || undefined;
    const cpu_limit = (formData.get("cpu_limit") as string) || "1";
    const memory_limit = (formData.get("memory_limit") as string) || "512m";
    const environment = (formData.get("environment") || "development") as AppEnvironment;

    if (!name?.trim()) {
      setError("Name is required");
      return;
    }

    if (deploymentSource === "upload") {
      // Upload-based deployment
      if (!uploadFile) {
        setError("Please select a ZIP file to upload");
        return;
      }

      const publish_directory = (formData.get("publish_directory") as string) || detectionResult?.publish_directory || undefined;

      uploadCreateMutation.mutate({
        file: uploadFile,
        config: {
          name: name.trim(),
          port,
          domain,
          healthcheck,
          cpu_limit,
          memory_limit,
          environment,
          build_type: buildType,
          publish_directory,
        },
      });
    } else if (deploymentSource === "registry") {
      // Registry-based deployment
      const docker_image = formData.get("docker_image") as string;
      const docker_image_tag = (formData.get("docker_image_tag") as string) || "latest";
      const registry_url = (formData.get("registry_url") as string) || undefined;
      const registry_username = (formData.get("registry_username") as string) || undefined;
      const registry_password = (formData.get("registry_password") as string) || undefined;

      if (!docker_image?.trim()) {
        setError("Docker image is required");
        return;
      }

      createAppMutation.mutate({
        name: name.trim(),
        docker_image: docker_image.trim(),
        docker_image_tag,
        registry_url,
        registry_username,
        registry_password,
        port,
        domain,
        healthcheck,
        cpu_limit,
        memory_limit,
        environment,
        project_id: projectId,
      });
    } else {
      // Git-based deployment
      let git_url: string;
      let branch: string;

      if (gitSourceType === "github" && selectedRepo) {
        // Use GitHub repo picker selection
        git_url = selectedRepo.gitUrl;
        branch = selectedRepo.branch;
      } else {
        // Manual URL entry
        git_url = manualGitUrl;
        branch = manualBranch || "main";
      }

      const dockerfile = (formData.get("dockerfile") as string) || "Dockerfile";
      const publish_directory = (formData.get("publish_directory") as string) || undefined;

      if (!git_url?.trim()) {
        setError(gitSourceType === "github" ? "Please select a repository" : "Git URL is required");
        return;
      }

      // Build Nixpacks config if build type is nixpacks
      let nixpacksConfigToSend: NixpacksConfig | undefined = undefined;
      if (buildType === "nixpacks") {
        // Only include non-empty values
        nixpacksConfigToSend = {};
        if (nixpacksConfig.install_cmd) nixpacksConfigToSend.install_cmd = nixpacksConfig.install_cmd;
        if (nixpacksConfig.build_cmd) nixpacksConfigToSend.build_cmd = nixpacksConfig.build_cmd;
        if (nixpacksConfig.start_cmd) nixpacksConfigToSend.start_cmd = nixpacksConfig.start_cmd;
        if (nixpacksConfig.packages?.length) nixpacksConfigToSend.packages = nixpacksConfig.packages;
        if (nixpacksConfig.apt_packages?.length) nixpacksConfigToSend.apt_packages = nixpacksConfig.apt_packages;
        // If empty, don't send
        if (Object.keys(nixpacksConfigToSend).length === 0) {
          nixpacksConfigToSend = undefined;
        }
      }

      createAppMutation.mutate({
        name: name.trim(),
        git_url: git_url.trim(),
        branch,
        dockerfile: buildType === "dockerfile" ? dockerfile : undefined,
        port,
        domain,
        healthcheck,
        cpu_limit,
        memory_limit,
        environment,
        project_id: projectId,
        build_type: buildType,
        nixpacks_config: nixpacksConfigToSend,
        publish_directory: buildType === "staticsite" ? publish_directory : undefined,
        preview_enabled: previewEnabled,
        github_app_installation_id: gitSourceType === "github" && selectedRepo ? selectedRepo.installationId : undefined,
      });
    }
  }

  const isSubmitting = createAppMutation.isPending || uploadCreateMutation.isPending;

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">New Application</h1>

      <Card>
        <CardHeader>
          <CardTitle>Application Details</CardTitle>
          <CardDescription>
            Create a new application by building from Git, uploading a ZIP file, or deploying a pre-built Docker image.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {error && (
            <div className="mb-4 p-3 rounded-md bg-destructive/10 text-destructive text-sm">
              {error}
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-6">
            <input type="hidden" name="deployment_source" value={deploymentSource} />

            {/* Two column layout for basic info */}
            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="name">Name *</Label>
                <Input
                  id="name"
                  name="name"
                  placeholder="my-app"
                  required
                />
                <p className="text-xs text-muted-foreground">
                  A unique name for your application
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="environment">Environment</Label>
                <Select name="environment" defaultValue="development">
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select environment" />
                  </SelectTrigger>
                  <SelectContent>
                    {ENVIRONMENT_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  The deployment environment for this application
                </p>
              </div>
            </div>

            {/* Deployment Source Tabs */}
            <div className="space-y-4">
              <Label>Deployment Source</Label>
              <Tabs value={deploymentSource} onValueChange={(v) => setDeploymentSource(v as "git" | "registry" | "upload")}>
                <TabsList className="grid w-full grid-cols-3">
                  <TabsTrigger value="git" className="flex items-center gap-2">
                    <GitBranch className="h-4 w-4" />
                    Build from Git
                  </TabsTrigger>
                  <TabsTrigger value="upload" className="flex items-center gap-2">
                    <Upload className="h-4 w-4" />
                    Upload ZIP
                  </TabsTrigger>
                  <TabsTrigger value="registry" className="flex items-center gap-2">
                    <Package className="h-4 w-4" />
                    Docker Registry
                  </TabsTrigger>
                </TabsList>

                {/* Git Source Tab */}
                <TabsContent value="git" className="space-y-4 pt-4">
                  {/* Git Source Type Toggle */}
                  <div className="space-y-3">
                    <Label>Source</Label>
                    <div className="grid grid-cols-2 gap-3">
                      <button
                        type="button"
                        onClick={() => setGitSourceType("github")}
                        className={`flex items-center gap-2 p-3 rounded-lg border-2 transition-colors ${
                          gitSourceType === "github"
                            ? "border-primary bg-primary/5"
                            : "border-border hover:border-muted-foreground/50"
                        }`}
                      >
                        <Github className="h-5 w-5" />
                        <div className="text-left">
                          <span className="text-sm font-medium block">GitHub</span>
                          <span className="text-xs text-muted-foreground">Select from your repos</span>
                        </div>
                      </button>
                      <button
                        type="button"
                        onClick={() => setGitSourceType("manual")}
                        className={`flex items-center gap-2 p-3 rounded-lg border-2 transition-colors ${
                          gitSourceType === "manual"
                            ? "border-primary bg-primary/5"
                            : "border-border hover:border-muted-foreground/50"
                        }`}
                      >
                        <Link2 className="h-5 w-5" />
                        <div className="text-left">
                          <span className="text-sm font-medium block">Manual URL</span>
                          <span className="text-xs text-muted-foreground">Enter any Git URL</span>
                        </div>
                      </button>
                    </div>
                  </div>

                  {/* GitHub Repository Picker */}
                  {gitSourceType === "github" && (
                    <GitHubRepoPicker
                      onSelect={(selection) => setSelectedRepo(selection)}
                      selectedInstallationId={selectedRepo?.installationId}
                      selectedRepoFullName={selectedRepo?.repository.full_name}
                    />
                  )}

                  {/* Manual Git URL Input */}
                  {gitSourceType === "manual" && (
                    <>
                      <div className="space-y-2">
                        <Label htmlFor="git_url">Git Repository URL *</Label>
                        <Input
                          id="git_url"
                          name="git_url"
                          placeholder="https://github.com/user/repo.git"
                          value={manualGitUrl}
                          onChange={(e) => setManualGitUrl(e.target.value)}
                          required={deploymentSource === "git" && gitSourceType === "manual"}
                        />
                        <p className="text-xs text-muted-foreground">
                          The Git repository URL to clone
                        </p>
                      </div>

                      <div className="space-y-2">
                        <Label htmlFor="branch">Branch</Label>
                        <Input
                          id="branch"
                          name="branch"
                          placeholder="main"
                          value={manualBranch}
                          onChange={(e) => setManualBranch(e.target.value)}
                        />
                      </div>
                    </>
                  )}

                  {/* Show selected branch from GitHub picker */}
                  {gitSourceType === "github" && selectedRepo && (
                    <div className="space-y-2">
                      <Label htmlFor="branch">Branch</Label>
                      <Input
                        id="branch"
                        name="branch"
                        value={selectedRepo.branch}
                        readOnly
                        className="bg-muted"
                      />
                      <p className="text-xs text-muted-foreground">
                        Default branch from the repository
                      </p>
                    </div>
                  )}

                  {/* Build Type Selection */}
                  <div className="space-y-3">
                    <Label>Build Type</Label>
                    <div className="grid grid-cols-5 gap-3">
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
                          CNB
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

                  {/* Dockerfile options */}
                  {buildType === "dockerfile" && (
                    <div className="space-y-2">
                      <Label htmlFor="dockerfile">Dockerfile</Label>
                      <Input
                        id="dockerfile"
                        name="dockerfile"
                        placeholder="Dockerfile"
                        defaultValue="Dockerfile"
                      />
                      <p className="text-xs text-muted-foreground">
                        Path to your Dockerfile in the repository
                      </p>
                    </div>
                  )}

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
                        Railpack is Railway's next-generation builder with faster builds and better caching.
                        Requires Docker BuildKit. Not available on Windows.
                      </p>
                      <div className="grid gap-4 md:grid-cols-3">
                        <div className="space-y-2">
                          <Label htmlFor="railpack_install_cmd">Install Command</Label>
                          <Input
                            id="railpack_install_cmd"
                            placeholder="npm install"
                            value={nixpacksConfig.install_cmd || ""}
                            onChange={(e) => setNixpacksConfig({ ...nixpacksConfig, install_cmd: e.target.value || undefined })}
                          />
                        </div>
                        <div className="space-y-2">
                          <Label htmlFor="railpack_build_cmd">Build Command</Label>
                          <Input
                            id="railpack_build_cmd"
                            placeholder="npm run build"
                            value={nixpacksConfig.build_cmd || ""}
                            onChange={(e) => setNixpacksConfig({ ...nixpacksConfig, build_cmd: e.target.value || undefined })}
                          />
                        </div>
                        <div className="space-y-2">
                          <Label htmlFor="railpack_start_cmd">Start Command</Label>
                          <Input
                            id="railpack_start_cmd"
                            placeholder="npm start"
                            value={nixpacksConfig.start_cmd || ""}
                            onChange={(e) => setNixpacksConfig({ ...nixpacksConfig, start_cmd: e.target.value || undefined })}
                          />
                        </div>
                      </div>
                    </div>
                  )}

                  {/* CNB/Buildpacks options */}
                  {buildType === "cnb" && (
                    <div className="space-y-4 p-4 bg-muted/50 rounded-lg">
                      <p className="text-sm text-muted-foreground">
                        Cloud Native Buildpacks (CNB) automatically detect and build your application
                        using Paketo or Heroku buildpacks. No Dockerfile required.
                      </p>
                      <p className="text-xs text-muted-foreground">
                        Supports: Node.js, Python, Go, Java, Ruby, PHP, .NET, and more.
                        Uses the pack CLI with optimized builder images.
                      </p>
                    </div>
                  )}

                  {/* Static options */}
                  {buildType === "staticsite" && (
                    <div className="space-y-2">
                      <Label htmlFor="publish_directory">Publish Directory</Label>
                      <Input
                        id="publish_directory"
                        name="publish_directory"
                        placeholder="dist, build, public"
                        defaultValue="dist"
                      />
                      <p className="text-xs text-muted-foreground">
                        Directory containing your built static files
                      </p>
                    </div>
                  )}

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
                </TabsContent>

                {/* Upload Source Tab */}
                <TabsContent value="upload" className="space-y-4 pt-4">
                  <ZipUploadZone
                    onFileSelect={handleFileSelect}
                    isUploading={isDetecting || uploadCreateMutation.isPending}
                    detectionResult={detectionResult}
                    disabled={isDetecting || uploadCreateMutation.isPending}
                  />

                  {/* Build Type Selection (for override) */}
                  {uploadFile && (
                    <>
                      <div className="space-y-3">
                        <Label>Build Type {detectionResult && "(Auto-detected, can override)"}</Label>
                        <div className="grid grid-cols-5 gap-3">
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
                              CNB
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

                      {/* Static options */}
                      {buildType === "staticsite" && (
                        <div className="space-y-2">
                          <Label htmlFor="publish_directory_upload">Publish Directory</Label>
                          <Input
                            id="publish_directory_upload"
                            name="publish_directory"
                            placeholder="dist, build, public"
                            defaultValue={detectionResult?.publish_directory || "dist"}
                          />
                          <p className="text-xs text-muted-foreground">
                            Directory containing your built static files
                          </p>
                        </div>
                      )}
                    </>
                  )}
                </TabsContent>

                {/* Registry Source Tab */}
                <TabsContent value="registry" className="space-y-4 pt-4">
                  <div className="space-y-2">
                    <Label htmlFor="docker_image">Docker Image *</Label>
                    <Input
                      id="docker_image"
                      name="docker_image"
                      placeholder="nginx, ghcr.io/user/app, registry.example.com/image"
                      required={deploymentSource === "registry"}
                    />
                    <p className="text-xs text-muted-foreground">
                      Image name with optional registry prefix
                    </p>
                  </div>

                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="space-y-2">
                      <Label htmlFor="docker_image_tag">Image Tag</Label>
                      <Input
                        id="docker_image_tag"
                        name="docker_image_tag"
                        placeholder="latest"
                        defaultValue="latest"
                      />
                      <p className="text-xs text-muted-foreground">
                        Tag or version (e.g., latest, v1.0.0)
                      </p>
                    </div>

                    <div className="space-y-2">
                      <Label htmlFor="registry_url">Registry URL</Label>
                      <Input
                        id="registry_url"
                        name="registry_url"
                        placeholder="Leave empty for Docker Hub"
                      />
                      <p className="text-xs text-muted-foreground">
                        Custom registry URL (optional)
                      </p>
                    </div>
                  </div>

                  <div className="space-y-2">
                    <p className="text-sm font-medium">Registry Authentication (optional)</p>
                    <div className="grid gap-4 md:grid-cols-2">
                      <div className="space-y-2">
                        <Label htmlFor="registry_username">Username</Label>
                        <Input
                          id="registry_username"
                          name="registry_username"
                          placeholder="Username or token name"
                          autoComplete="off"
                        />
                      </div>

                      <div className="space-y-2">
                        <Label htmlFor="registry_password">Password / Access Token</Label>
                        <div className="relative">
                          <Input
                            id="registry_password"
                            name="registry_password"
                            type={showPassword ? "text" : "password"}
                            placeholder="Password or access token"
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
                    <p className="text-xs text-muted-foreground">
                      Required for private registries. For GitHub Container Registry, use a personal access token.
                    </p>
                  </div>
                </TabsContent>
              </Tabs>
            </div>

            {/* Project and Healthcheck row */}
            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="project">Project</Label>
                <Select name="project_id" defaultValue={projectId} disabled>
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select a project" />
                  </SelectTrigger>
                  <SelectContent>
                    {projects.map((project) => (
                      <SelectItem key={project.id} value={project.id}>
                        {project.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  This app will be added to the selected project
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="healthcheck">Healthcheck Path</Label>
                <Input
                  id="healthcheck"
                  name="healthcheck"
                  placeholder="/health"
                />
                <p className="text-xs text-muted-foreground">
                  Optional endpoint to check if the app is healthy
                </p>
              </div>
            </div>

            {/* Port, Domain row */}
            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="port">Port</Label>
                <Input
                  id="port"
                  name="port"
                  type="number"
                  placeholder="3000"
                  defaultValue="3000"
                />
                <p className="text-xs text-muted-foreground">
                  Container port to expose
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="domain">Domain</Label>
                <Input
                  id="domain"
                  name="domain"
                  placeholder="app.example.com"
                />
                <p className="text-xs text-muted-foreground">
                  Optional custom domain for your application
                </p>
              </div>
            </div>

            {/* Resource Limits row */}
            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="cpu_limit">CPU Limit</Label>
                <Select name="cpu_limit" defaultValue="1">
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select CPU limit" />
                  </SelectTrigger>
                  <SelectContent>
                    {CPU_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  Maximum CPU cores for this container
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="memory_limit">Memory Limit</Label>
                <Select name="memory_limit" defaultValue="512m">
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select memory limit" />
                  </SelectTrigger>
                  <SelectContent>
                    {MEMORY_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  Maximum memory for this container
                </p>
              </div>
            </div>

            <div className="flex gap-4">
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Creating..." : "Create Application"}
              </Button>
              <Button type="button" variant="outline" asChild>
                <Link to={`/projects/${projectId}`}>Cancel</Link>
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
