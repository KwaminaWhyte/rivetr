import { useState, useMemo, useEffect } from "react";
import { Form, Link, redirect, useNavigation } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { Route } from "./+types/$id";
import { toast } from "sonner";
import {
  ArrowLeft,
  ChevronDown,
  Copy,
  Database,
  Edit2,
  ExternalLink,
  Eye,
  EyeOff,
  Layers,
  MoreVertical,
  Play,
  Plus,
  Square,
  Trash2,
  X,
  AlertCircle,
  Search,
  Rocket,
} from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { EnvironmentBadge } from "@/components/environment-badge";
import { Badge } from "@/components/ui/badge";
import type {
  App,
  ProjectWithApps,
  UpdateProjectRequest,
  ManagedDatabase,
  DatabaseType,
  Service,
  ServiceStatus,
  ServiceTemplate,
} from "@/types/api";
import { DATABASE_TYPES } from "@/types/api";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";

export function meta({ data }: Route.MetaArgs) {
  const projectName = data?.project?.name || "Project";
  return [
    { title: `${projectName} - Rivetr` },
    {
      name: "description",
      content: `Manage ${projectName} and its applications`,
    },
  ];
}

// Status badge component
function StatusBadge({ status }: { status: string }) {
  const variants: Record<string, { className: string; label: string }> = {
    running: { className: "bg-green-500 text-white", label: "Running" },
    stopped: { className: "bg-gray-500 text-white", label: "Stopped" },
    not_deployed: {
      className: "bg-gray-400 text-white",
      label: "Not Deployed",
    },
    failed: { className: "bg-red-500 text-white", label: "Failed" },
    building: { className: "bg-blue-500 text-white", label: "Building" },
    pending: { className: "bg-yellow-500 text-white", label: "Pending" },
  };
  const variant = variants[status] || variants.stopped;
  return <Badge className={variant.className}>{variant.label}</Badge>;
}

// Database status badge
function DatabaseStatusBadge({ status }: { status: string }) {
  switch (status) {
    case "running":
      return <Badge className="bg-green-500 hover:bg-green-600">Running</Badge>;
    case "stopped":
      return <Badge variant="secondary">Stopped</Badge>;
    case "pending":
      return <Badge variant="outline">Pending</Badge>;
    case "pulling":
      return <Badge className="bg-blue-500 hover:bg-blue-600">Pulling</Badge>;
    case "starting":
      return (
        <Badge className="bg-yellow-500 hover:bg-yellow-600">Starting</Badge>
      );
    case "failed":
      return <Badge variant="destructive">Failed</Badge>;
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
}

// Service status badge
function ServiceStatusBadge({ status }: { status: string }) {
  switch (status) {
    case "running":
      return <Badge className="bg-green-500 hover:bg-green-600">Running</Badge>;
    case "stopped":
      return <Badge variant="secondary">Stopped</Badge>;
    case "pending":
      return <Badge variant="outline">Pending</Badge>;
    case "failed":
      return <Badge variant="destructive">Failed</Badge>;
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
}

export async function loader({ request, params }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const [project, allApps, templates] = await Promise.all([
    api.getProject(token, params.id!),
    api.getApps(token).catch(() => []),
    api.getTemplates(token).catch(() => []),
  ]);

  // Get app statuses for apps in this project
  const appStatuses: Record<string, string> = {};
  await Promise.all(
    project.apps.map(async (app) => {
      try {
        const status = await api.getAppStatus(token, app.id);
        appStatuses[app.id] = status.status;
      } catch {
        appStatuses[app.id] = "stopped";
      }
    })
  );

  return { project, allApps, appStatuses, templates };
}

export async function action({ request, params }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "delete") {
    await api.deleteProject(token, params.id!);
    return redirect("/projects");
  }

  if (intent === "update") {
    const name = formData.get("name");
    const description = formData.get("description");

    if (typeof name !== "string" || !name.trim()) {
      return { error: "Project name is required" };
    }

    try {
      await api.updateProject(token, params.id!, {
        name: name.trim(),
        description: typeof description === "string" ? description : undefined,
      });
      return { success: true, action: "update" };
    } catch (error) {
      return {
        error:
          error instanceof Error ? error.message : "Failed to update project",
      };
    }
  }

  if (intent === "assign-app") {
    const appId = formData.get("appId");
    if (typeof appId !== "string") {
      return { error: "App ID is required" };
    }
    try {
      await api.assignAppToProject(token, appId, params.id!);
      return { success: true, action: "assign-app" };
    } catch (error) {
      return {
        error: error instanceof Error ? error.message : "Failed to add app",
      };
    }
  }

  if (intent === "remove-app") {
    const appId = formData.get("appId");
    if (typeof appId !== "string") {
      return { error: "App ID is required" };
    }
    try {
      await api.assignAppToProject(token, appId, null);
      return { success: true, action: "remove-app" };
    } catch (error) {
      return {
        error: error instanceof Error ? error.message : "Failed to remove app",
      };
    }
  }

  // Database operations
  if (intent === "create-database") {
    const name = formData.get("name");
    const db_type = formData.get("db_type") as DatabaseType;
    const version = formData.get("version") || "latest";
    const public_access = formData.get("public_access") === "true";

    // Optional credentials
    const username = formData.get("username");
    const password = formData.get("password");
    const database = formData.get("database");
    const root_password = formData.get("root_password");

    if (typeof name !== "string" || !name.trim()) {
      return { error: "Database name is required" };
    }
    if (!db_type) {
      return { error: "Database type is required" };
    }

    try {
      await api.createDatabase(token, {
        name: name.trim(),
        db_type,
        version: version as string,
        public_access,
        project_id: params.id!,
        // Only include credentials if provided
        ...(username && typeof username === "string" && username.trim()
          ? { username: username.trim() }
          : {}),
        ...(password && typeof password === "string" && password.trim()
          ? { password: password.trim() }
          : {}),
        ...(database && typeof database === "string" && database.trim()
          ? { database: database.trim() }
          : {}),
        ...(root_password &&
        typeof root_password === "string" &&
        root_password.trim()
          ? { root_password: root_password.trim() }
          : {}),
      });
      return { success: true, action: "create-database" };
    } catch (error) {
      return {
        error:
          error instanceof Error ? error.message : "Failed to create database",
      };
    }
  }

  if (intent === "delete-database") {
    const databaseId = formData.get("databaseId");
    if (typeof databaseId !== "string") {
      return { error: "Database ID is required" };
    }
    try {
      await api.deleteDatabase(token, databaseId);
      return { success: true, action: "delete-database" };
    } catch (error) {
      return {
        error:
          error instanceof Error ? error.message : "Failed to delete database",
      };
    }
  }

  if (intent === "start-database") {
    const databaseId = formData.get("databaseId");
    if (typeof databaseId !== "string") {
      return { error: "Database ID is required" };
    }
    try {
      await api.startDatabase(token, databaseId);
      return { success: true, action: "start-database" };
    } catch (error) {
      return {
        error:
          error instanceof Error ? error.message : "Failed to start database",
      };
    }
  }

  if (intent === "stop-database") {
    const databaseId = formData.get("databaseId");
    if (typeof databaseId !== "string") {
      return { error: "Database ID is required" };
    }
    try {
      await api.stopDatabase(token, databaseId);
      return { success: true, action: "stop-database" };
    } catch (error) {
      return {
        error:
          error instanceof Error ? error.message : "Failed to stop database",
      };
    }
  }

  // Service operations
  if (intent === "create-service") {
    const name = formData.get("name");
    const compose_content = formData.get("compose_content");

    if (typeof name !== "string" || !name.trim()) {
      return { error: "Service name is required" };
    }
    if (typeof compose_content !== "string" || !compose_content.trim()) {
      return { error: "Docker Compose content is required" };
    }

    try {
      await api.createService(token, {
        name: name.trim(),
        compose_content: compose_content.trim(),
        project_id: params.id!,
      });
      return { success: true, action: "create-service" };
    } catch (error) {
      return {
        error:
          error instanceof Error ? error.message : "Failed to create service",
      };
    }
  }

  if (intent === "delete-service") {
    const serviceId = formData.get("serviceId");
    if (typeof serviceId !== "string") {
      return { error: "Service ID is required" };
    }
    try {
      await api.deleteService(token, serviceId);
      return { success: true, action: "delete-service" };
    } catch (error) {
      return {
        error:
          error instanceof Error ? error.message : "Failed to delete service",
      };
    }
  }

  if (intent === "start-service") {
    const serviceId = formData.get("serviceId");
    if (typeof serviceId !== "string") {
      return { error: "Service ID is required" };
    }
    try {
      await api.startService(token, serviceId);
      return { success: true, action: "start-service" };
    } catch (error) {
      return {
        error:
          error instanceof Error ? error.message : "Failed to start service",
      };
    }
  }

  if (intent === "stop-service") {
    const serviceId = formData.get("serviceId");
    if (typeof serviceId !== "string") {
      return { error: "Service ID is required" };
    }
    try {
      await api.stopService(token, serviceId);
      return { success: true, action: "stop-service" };
    } catch (error) {
      return {
        error:
          error instanceof Error ? error.message : "Failed to stop service",
      };
    }
  }

  if (intent === "deploy-from-template") {
    const templateId = formData.get("templateId");
    const serviceName = formData.get("serviceName");
    const envVarsJson = formData.get("envVars");

    if (typeof templateId !== "string") {
      return { error: "Template ID is required" };
    }
    if (typeof serviceName !== "string" || !serviceName.trim()) {
      return { error: "Service name is required" };
    }

    // Parse env vars from JSON
    let envVars: Record<string, string> = {};
    if (typeof envVarsJson === "string" && envVarsJson.trim()) {
      try {
        envVars = JSON.parse(envVarsJson);
      } catch {
        return { error: "Invalid environment variables format" };
      }
    }

    try {
      await api.deployTemplate(token, templateId, {
        name: serviceName.trim(),
        project_id: params.id!,
        env_vars: envVars,
      });
      return { success: true, action: "deploy-from-template" };
    } catch (error) {
      return {
        error:
          error instanceof Error ? error.message : "Failed to deploy template",
      };
    }
  }

  return { error: "Unknown action" };
}

export default function ProjectDetailPage({
  loaderData,
  actionData,
}: Route.ComponentProps) {
  const queryClient = useQueryClient();
  const navigation = useNavigation();
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [isAddAppDialogOpen, setIsAddAppDialogOpen] = useState(false);
  const [editData, setEditData] = useState<UpdateProjectRequest>({});

  // Database state
  const [isCreateDbDialogOpen, setIsCreateDbDialogOpen] = useState(false);
  const [isDeleteDbDialogOpen, setIsDeleteDbDialogOpen] = useState(false);
  const [isCredentialsDialogOpen, setIsCredentialsDialogOpen] = useState(false);
  const [selectedDatabase, setSelectedDatabase] =
    useState<ManagedDatabase | null>(null);
  const [selectedDbType, setSelectedDbType] =
    useState<DatabaseType>("postgres");
  const [showCustomCredentials, setShowCustomCredentials] = useState(false);

  // Service state
  const [isCreateServiceDialogOpen, setIsCreateServiceDialogOpen] =
    useState(false);
  const [isDeleteServiceDialogOpen, setIsDeleteServiceDialogOpen] =
    useState(false);
  const [selectedService, setSelectedService] = useState<Service | null>(null);
  const [showPasswords, setShowPasswords] = useState(false);
  const [revealedDatabase, setRevealedDatabase] =
    useState<ManagedDatabase | null>(null);

  // Templates state
  const [isTemplatesModalOpen, setIsTemplatesModalOpen] = useState(false);
  const [templateSearch, setTemplateSearch] = useState("");
  const [selectedCategory, setSelectedCategory] = useState<string>("all");
  const [selectedTemplate, setSelectedTemplate] =
    useState<ServiceTemplate | null>(null);
  const [templateServiceName, setTemplateServiceName] = useState("");
  const [templateEnvVars, setTemplateEnvVars] = useState<Record<string, string>>({});
  const [showTemplateSecrets, setShowTemplateSecrets] = useState<Record<string, boolean>>({});

  // Use React Query with SSR initial data
  const { data: project, refetch } = useQuery<ProjectWithApps>({
    queryKey: ["project", loaderData.project.id],
    queryFn: () => api.getProject(loaderData.project.id),
    initialData: loaderData.project,
    refetchInterval: 5000, // Poll for database status updates
  });

  const { data: allApps = [] } = useQuery<App[]>({
    queryKey: ["apps"],
    queryFn: () => api.getApps(),
    initialData: loaderData.allApps,
  });

  const { data: templates = [] } = useQuery<ServiceTemplate[]>({
    queryKey: ["service-templates"],
    queryFn: () => api.getTemplates(),
    initialData: loaderData.templates,
  });

  // Get unique categories from templates
  const categories = useMemo(() => {
    const cats = new Set(templates.map((t) => t.category));
    return ["all", ...Array.from(cats).sort()];
  }, [templates]);

  // Filter templates by search and category
  const filteredTemplates = useMemo(() => {
    return templates.filter((t) => {
      const matchesSearch =
        !templateSearch ||
        t.name.toLowerCase().includes(templateSearch.toLowerCase()) ||
        (t.description && t.description.toLowerCase().includes(templateSearch.toLowerCase()));
      const matchesCategory =
        selectedCategory === "all" || t.category === selectedCategory;
      return matchesSearch && matchesCategory;
    });
  }, [templates, templateSearch, selectedCategory]);

  // Apps available to add (not in this project)
  const availableApps = useMemo(() => {
    if (!project) return [];
    const projectAppIds = new Set(project.apps.map((a) => a.id));
    return allApps.filter(
      (app) => !app.project_id && !projectAppIds.has(app.id)
    );
  }, [allApps, project]);

  const isSubmitting = navigation.state === "submitting";

  const openEditDialog = () => {
    if (project) {
      setEditData({
        name: project.name,
        description: project.description || "",
      });
      setIsEditDialogOpen(true);
    }
  };

  // Handle success actions
  useEffect(() => {
    if (actionData?.success) {
      if (actionData.action === "update") {
        toast.success("Project updated");
        setIsEditDialogOpen(false);
      } else if (actionData.action === "assign-app") {
        toast.success("App added to project");
        setIsAddAppDialogOpen(false);
      } else if (actionData.action === "remove-app") {
        toast.success("App removed from project");
      } else if (actionData.action === "create-database") {
        toast.success("Database created");
        setIsCreateDbDialogOpen(false);
      } else if (actionData.action === "delete-database") {
        toast.success("Database deleted");
        setIsDeleteDbDialogOpen(false);
        setSelectedDatabase(null);
      } else if (actionData.action === "start-database") {
        toast.success("Database starting");
      } else if (actionData.action === "stop-database") {
        toast.success("Database stopped");
      } else if (actionData.action === "create-service") {
        toast.success("Service created");
        setIsCreateServiceDialogOpen(false);
      } else if (actionData.action === "delete-service") {
        toast.success("Service deleted");
        setIsDeleteServiceDialogOpen(false);
        setSelectedService(null);
      } else if (actionData.action === "start-service") {
        toast.success("Service starting");
      } else if (actionData.action === "stop-service") {
        toast.success("Service stopped");
      } else if (actionData.action === "deploy-from-template") {
        toast.success("Service deployed from template");
        setIsTemplatesModalOpen(false);
        setSelectedTemplate(null);
        setTemplateServiceName("");
        setTemplateEnvVars({});
        setShowTemplateSecrets({});
      }
      queryClient.invalidateQueries({ queryKey: ["project", project?.id] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["apps"] });
      queryClient.invalidateQueries({ queryKey: ["services"] });
    }

    if (actionData?.error) {
      toast.error(actionData.error);
    }
  }, [actionData, queryClient, project?.id]);

  const handleViewCredentials = async (database: ManagedDatabase) => {
    try {
      const revealed = await api.getDatabase(database.id, true);
      setRevealedDatabase(revealed);
      setIsCredentialsDialogOpen(true);
    } catch {
      toast.error("Failed to fetch credentials");
    }
  };

  const copyToClipboard = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    toast.success(`${label} copied to clipboard`);
  };

  const dbTypeConfig = DATABASE_TYPES.find((t) => t.type === selectedDbType);

  if (!project) {
    return (
      <div className="space-y-6">
        <Button variant="ghost" asChild>
          <Link to="/projects">
            <ArrowLeft className="mr-2 h-4 w-4" />
            Back to Projects
          </Link>
        </Button>
        <Card>
          <CardContent className="py-8 text-center text-destructive">
            Failed to load project. It may have been deleted.
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to="/projects">
            <ArrowLeft className="h-4 w-4" />
          </Link>
        </Button>
        <div className="flex-1">
          <h1 className="text-3xl font-bold">{project.name}</h1>
          {project.description && (
            <p className="text-muted-foreground">{project.description}</p>
          )}
        </div>
        <Button variant="outline" onClick={openEditDialog}>
          <Edit2 className="mr-2 h-4 w-4" />
          Edit
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon">
              <MoreVertical className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={openEditDialog}>
              <Edit2 className="mr-2 h-4 w-4" />
              Edit Project
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
      </div>

      {/* Apps Table */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Applications</CardTitle>
          <div className="flex gap-2">
            <Button
              variant="outline"
              onClick={() => setIsAddAppDialogOpen(true)}
            >
              <Plus className="mr-2 h-4 w-4" />
              Add Existing App
            </Button>
            <Button asChild>
              <Link to={`/projects/${project.id}/apps/new`}>
                <Plus className="mr-2 h-4 w-4" />
                Create New App
              </Link>
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {project.apps.length === 0 ? (
            <div className="py-8 text-center">
              <p className="text-muted-foreground mb-4">
                No applications in this project yet.
              </p>
              <div className="flex gap-2 justify-center">
                <Button
                  variant="outline"
                  onClick={() => setIsAddAppDialogOpen(true)}
                >
                  <Plus className="mr-2 h-4 w-4" />
                  Add Existing
                </Button>
                <Button asChild>
                  <Link to={`/projects/${project.id}/apps/new`}>
                    <Plus className="mr-2 h-4 w-4" />
                    Create New App
                  </Link>
                </Button>
              </div>
            </div>
          ) : (
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {project.apps.map((app) => {
                const status = loaderData.appStatuses?.[app.id] || "stopped";
                return (
                  <Card
                    key={app.id}
                    className="group relative hover:shadow-md transition-shadow"
                  >
                    <Link
                      to={`/apps/${app.id}`}
                      className="absolute inset-0 z-0"
                    />
                    <CardHeader className="pb-2">
                      <div className="flex items-start justify-between">
                        <div className="space-y-1">
                          <CardTitle className="text-base font-semibold">
                            {app.name}
                          </CardTitle>
                          <div className="flex items-center gap-2">
                            <StatusBadge status={status} />
                            <EnvironmentBadge environment={app.environment} />
                          </div>
                        </div>
                        <Form method="post" className="relative z-10">
                          <input
                            type="hidden"
                            name="intent"
                            value="remove-app"
                          />
                          <input type="hidden" name="appId" value={app.id} />
                          <Button
                            type="submit"
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8 opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive transition-opacity"
                            disabled={isSubmitting}
                            title="Remove from project"
                          >
                            <X className="h-4 w-4" />
                          </Button>
                        </Form>
                      </div>
                    </CardHeader>
                    <CardContent className="pt-0 pb-4">
                      <div className="space-y-2 text-sm text-muted-foreground">
                        {app.domain && (
                          <div className="flex items-center gap-2 truncate">
                            <ExternalLink className="h-3 w-3 flex-shrink-0" />
                            <span className="truncate">{app.domain}</span>
                          </div>
                        )}
                        {app.git_url && (
                          <div className="truncate text-xs opacity-75">
                            {app.git_url
                              .replace(/^https?:\/\//, "")
                              .replace(/\.git$/, "")}
                          </div>
                        )}
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Databases Table */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Databases</CardTitle>
          <Button onClick={() => setIsCreateDbDialogOpen(true)}>
            <Database className="mr-2 h-4 w-4" />
            Create Database
          </Button>
        </CardHeader>
        <CardContent>
          {!project.databases || project.databases.length === 0 ? (
            <div className="py-8 text-center">
              <Database className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
              <p className="text-muted-foreground mb-4">
                No databases in this project yet.
              </p>
              <Button onClick={() => setIsCreateDbDialogOpen(true)}>
                <Database className="mr-2 h-4 w-4" />
                Create Database
              </Button>
            </div>
          ) : (
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {project.databases.map((db) => {
                const dbTypeInfo = DATABASE_TYPES.find((t) => t.type === db.db_type);
                return (
                  <Card
                    key={db.id}
                    className="group relative hover:shadow-md transition-shadow"
                  >
                    <Link
                      to={`/databases/${db.id}`}
                      className="absolute inset-0 z-0"
                    />
                    <CardHeader className="pb-2">
                      <div className="flex items-start justify-between">
                        <div className="space-y-1">
                          <div className="flex items-center gap-2">
                            <CardTitle className="text-base font-semibold">
                              {db.name}
                            </CardTitle>
                            {db.status === "failed" && db.error_message && (
                              <TooltipProvider>
                                <Tooltip>
                                  <TooltipTrigger>
                                    <AlertCircle className="h-4 w-4 text-destructive" />
                                  </TooltipTrigger>
                                  <TooltipContent className="max-w-xs">
                                    <p className="text-sm">
                                      {db.error_message}
                                    </p>
                                  </TooltipContent>
                                </Tooltip>
                              </TooltipProvider>
                            )}
                          </div>
                          <div className="flex items-center gap-2">
                            <DatabaseStatusBadge status={db.status} />
                            <Badge
                              variant="outline"
                              className="capitalize text-xs"
                            >
                              {dbTypeInfo?.name || db.db_type} {db.version}
                            </Badge>
                          </div>
                        </div>
                        <div className="flex items-center gap-1 relative z-10 opacity-0 group-hover:opacity-100 transition-opacity">
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-7 w-7"
                            title="View Credentials"
                            onClick={(e) => {
                              e.preventDefault();
                              handleViewCredentials(db);
                            }}
                          >
                            <Eye className="h-3.5 w-3.5" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-7 w-7 text-destructive"
                            title="Delete Database"
                            onClick={(e) => {
                              e.preventDefault();
                              setSelectedDatabase(db);
                              setIsDeleteDbDialogOpen(true);
                            }}
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                          </Button>
                        </div>
                      </div>
                    </CardHeader>
                    <CardContent className="pt-0 pb-4">
                      <div className="flex items-center justify-between text-sm">
                        <span className="text-muted-foreground">
                          {db.public_access && db.external_port > 0 ? (
                            <span className="font-mono">
                              Port {db.external_port}
                            </span>
                          ) : (
                            "Internal only"
                          )}
                        </span>
                        <div className="relative z-10 flex items-center gap-1">
                          {db.status === "stopped" && (
                            <Form method="post">
                              <input
                                type="hidden"
                                name="intent"
                                value="start-database"
                              />
                              <input
                                type="hidden"
                                name="databaseId"
                                value={db.id}
                              />
                              <Button
                                variant="outline"
                                size="sm"
                                type="submit"
                                className="h-7 px-2"
                                disabled={isSubmitting}
                              >
                                <Play className="h-3 w-3 mr-1" />
                                Start
                              </Button>
                            </Form>
                          )}
                          {db.status === "running" && (
                            <Form method="post">
                              <input
                                type="hidden"
                                name="intent"
                                value="stop-database"
                              />
                              <input
                                type="hidden"
                                name="databaseId"
                                value={db.id}
                              />
                              <Button
                                variant="outline"
                                size="sm"
                                type="submit"
                                className="h-7 px-2"
                                disabled={isSubmitting}
                              >
                                <Square className="h-3 w-3 mr-1" />
                                Stop
                              </Button>
                            </Form>
                          )}
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Services Section */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Services</CardTitle>
          <div className="flex gap-2">
            <Button
              variant="outline"
              onClick={() => setIsTemplatesModalOpen(true)}
            >
              <Rocket className="mr-2 h-4 w-4" />
              Deploy Template
            </Button>
            <Button onClick={() => setIsCreateServiceDialogOpen(true)}>
              <Layers className="mr-2 h-4 w-4" />
              Custom Service
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {!project.services || project.services.length === 0 ? (
            <div className="py-8 text-center">
              <Layers className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
              <p className="text-muted-foreground mb-4">
                No services in this project yet.
              </p>
              <div className="flex justify-center gap-2">
                <Button
                  variant="outline"
                  onClick={() => setIsTemplatesModalOpen(true)}
                >
                  <Rocket className="mr-2 h-4 w-4" />
                  Deploy Template
                </Button>
                <Button onClick={() => setIsCreateServiceDialogOpen(true)}>
                  <Layers className="mr-2 h-4 w-4" />
                  Custom Service
                </Button>
              </div>
            </div>
          ) : (
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {project.services.map((service) => (
                <Card
                  key={service.id}
                  className="group relative hover:shadow-md transition-shadow"
                >
                  <Link
                    to={`/services/${service.id}`}
                    className="absolute inset-0 z-0"
                  />
                  <CardHeader className="pb-2">
                    <div className="flex items-start justify-between">
                      <div className="space-y-1">
                        <div className="flex items-center gap-2">
                          <CardTitle className="text-base font-semibold">
                            {service.name}
                          </CardTitle>
                          {service.status === "failed" &&
                            service.error_message && (
                              <TooltipProvider>
                                <Tooltip>
                                  <TooltipTrigger>
                                    <AlertCircle className="h-4 w-4 text-destructive" />
                                  </TooltipTrigger>
                                  <TooltipContent className="max-w-xs">
                                    <p className="text-sm">
                                      {service.error_message}
                                    </p>
                                  </TooltipContent>
                                </Tooltip>
                              </TooltipProvider>
                            )}
                        </div>
                        <ServiceStatusBadge status={service.status} />
                      </div>
                      <div className="flex items-center gap-1 relative z-10 opacity-0 group-hover:opacity-100 transition-opacity">
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-7 w-7 text-destructive"
                          title="Delete Service"
                          onClick={(e) => {
                            e.preventDefault();
                            setSelectedService(service);
                            setIsDeleteServiceDialogOpen(true);
                          }}
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    </div>
                  </CardHeader>
                  <CardContent className="pt-0 pb-4">
                    <div className="flex items-center justify-end text-sm">
                      <div className="relative z-10 flex items-center gap-1">
                        {service.status === "stopped" && (
                          <Form method="post">
                            <input
                              type="hidden"
                              name="intent"
                              value="start-service"
                            />
                            <input
                              type="hidden"
                              name="serviceId"
                              value={service.id}
                            />
                            <Button
                              variant="outline"
                              size="sm"
                              type="submit"
                              className="h-7 px-2"
                              disabled={isSubmitting}
                            >
                              <Play className="h-3 w-3 mr-1" />
                              Start
                            </Button>
                          </Form>
                        )}
                        {service.status === "running" && (
                          <Form method="post">
                            <input
                              type="hidden"
                              name="intent"
                              value="stop-service"
                            />
                            <input
                              type="hidden"
                              name="serviceId"
                              value={service.id}
                            />
                            <Button
                              variant="outline"
                              size="sm"
                              type="submit"
                              className="h-7 px-2"
                              disabled={isSubmitting}
                            >
                              <Square className="h-3 w-3 mr-1" />
                              Stop
                            </Button>
                          </Form>
                        )}
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Edit Dialog */}
      <Dialog open={isEditDialogOpen} onOpenChange={setIsEditDialogOpen}>
        <DialogContent>
          <Form method="post">
            <input type="hidden" name="intent" value="update" />
            <DialogHeader>
              <DialogTitle>Edit Project</DialogTitle>
              <DialogDescription>
                Update your project details.
              </DialogDescription>
            </DialogHeader>
            {actionData?.error && (
              <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
                {actionData.error}
              </div>
            )}
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="edit-name">Name</Label>
                <Input
                  id="edit-name"
                  name="name"
                  defaultValue={editData.name || ""}
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="edit-description">Description</Label>
                <Textarea
                  id="edit-description"
                  name="description"
                  defaultValue={editData.description || ""}
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
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Saving..." : "Save Changes"}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <AlertDialog
        open={isDeleteDialogOpen}
        onOpenChange={setIsDeleteDialogOpen}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Project</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete "{project.name}"? This will not
              delete the apps, but they will be unassigned from this project.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <Form method="post">
              <input type="hidden" name="intent" value="delete" />
              <AlertDialogAction
                type="submit"
                className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              >
                {isSubmitting ? "Deleting..." : "Delete"}
              </AlertDialogAction>
            </Form>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Add App Dialog */}
      <Dialog open={isAddAppDialogOpen} onOpenChange={setIsAddAppDialogOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Add App to Project</DialogTitle>
            <DialogDescription>
              Select an unassigned app to add to this project.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            {availableApps.length === 0 ? (
              <div className="text-center py-8">
                <p className="text-muted-foreground mb-4">
                  No unassigned apps available.
                </p>
                <Button asChild>
                  <Link to={`/projects/${project.id}/apps/new`}>
                    Create New App
                  </Link>
                </Button>
              </div>
            ) : (
              <div className="space-y-2 max-h-80 overflow-y-auto">
                {availableApps.map((app) => (
                  <Form method="post" key={app.id}>
                    <input type="hidden" name="intent" value="assign-app" />
                    <input type="hidden" name="appId" value={app.id} />
                    <button
                      type="submit"
                      className="flex items-center justify-between w-full p-3 rounded-lg border hover:bg-muted/50 cursor-pointer text-left"
                      disabled={isSubmitting}
                    >
                      <div className="flex items-center gap-3">
                        <div>
                          <p className="font-medium">{app.name}</p>
                          <p className="text-sm text-muted-foreground truncate max-w-md">
                            {app.git_url}
                          </p>
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <EnvironmentBadge environment={app.environment} />
                        <Plus className="h-4 w-4" />
                      </div>
                    </button>
                  </Form>
                ))}
              </div>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setIsAddAppDialogOpen(false)}
            >
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Create Database Dialog */}
      <Dialog
        open={isCreateDbDialogOpen}
        onOpenChange={setIsCreateDbDialogOpen}
      >
        <DialogContent className="max-w-lg">
          <Form method="post">
            <input type="hidden" name="intent" value="create-database" />
            <DialogHeader>
              <DialogTitle>Create Database</DialogTitle>
              <DialogDescription>
                Deploy a new managed database with auto-generated credentials.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="db-name">Name</Label>
                <Input
                  id="db-name"
                  name="name"
                  placeholder="e.g., my-postgres-db"
                  pattern="[a-zA-Z0-9-]+"
                  title="Only alphanumeric characters and hyphens are allowed"
                  required
                />
                <p className="text-xs text-muted-foreground">
                  Only letters, numbers, and hyphens allowed
                </p>
              </div>

              <div className="space-y-2">
                <Label>Database Type</Label>
                <div className="grid grid-cols-2 gap-2">
                  {DATABASE_TYPES.map((config) => (
                    <button
                      key={config.type}
                      type="button"
                      className={`p-3 border rounded-lg text-left transition-colors ${
                        selectedDbType === config.type
                          ? "border-primary bg-primary/5"
                          : "border-border hover:border-primary/50"
                      }`}
                      onClick={() => setSelectedDbType(config.type)}
                    >
                      <div className="font-medium">{config.name}</div>
                      <div className="text-xs text-muted-foreground">
                        Port {config.defaultPort}
                      </div>
                    </button>
                  ))}
                </div>
                <input type="hidden" name="db_type" value={selectedDbType} />
              </div>

              <div className="space-y-2">
                <Label htmlFor="db-version">Version</Label>
                <Select name="version" defaultValue="latest">
                  <SelectTrigger>
                    <SelectValue placeholder="Select version" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="latest">
                      Latest ({dbTypeConfig?.defaultVersion})
                    </SelectItem>
                    {dbTypeConfig?.versions.map((v) => (
                      <SelectItem key={v} value={v}>
                        {v}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="flex items-center space-x-2">
                <Checkbox
                  id="public_access"
                  name="public_access"
                  value="true"
                />
                <Label htmlFor="public_access" className="text-sm font-normal">
                  Enable public access (expose port to host)
                </Label>
              </div>

              {/* Optional Credentials */}
              <Collapsible
                open={showCustomCredentials}
                onOpenChange={setShowCustomCredentials}
              >
                <CollapsibleTrigger asChild>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="flex items-center gap-1 p-0 h-auto hover:bg-transparent"
                  >
                    <ChevronDown
                      className={`h-4 w-4 transition-transform ${
                        showCustomCredentials ? "rotate-180" : ""
                      }`}
                    />
                    <span className="text-sm text-muted-foreground">
                      Custom credentials (optional)
                    </span>
                  </Button>
                </CollapsibleTrigger>
                <CollapsibleContent className="space-y-3 pt-3">
                  <p className="text-xs text-muted-foreground">
                    Leave fields empty to auto-generate secure credentials.
                  </p>
                  <div className="space-y-2">
                    <Label htmlFor="db-username">Username</Label>
                    <Input
                      id="db-username"
                      name="username"
                      placeholder="Auto-generated if empty"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="db-password">Password</Label>
                    <Input
                      id="db-password"
                      name="password"
                      type="password"
                      placeholder="Auto-generated if empty"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="db-database">Database Name</Label>
                    <Input
                      id="db-database"
                      name="database"
                      placeholder="Defaults to username"
                    />
                  </div>
                  {selectedDbType === "mysql" && (
                    <div className="space-y-2">
                      <Label htmlFor="db-root-password">Root Password</Label>
                      <Input
                        id="db-root-password"
                        name="root_password"
                        type="password"
                        placeholder="Auto-generated if empty"
                      />
                      <p className="text-xs text-muted-foreground">
                        MySQL root password for administrative access
                      </p>
                    </div>
                  )}
                </CollapsibleContent>
              </Collapsible>
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setIsCreateDbDialogOpen(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Creating..." : "Create Database"}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>

      {/* Database Credentials Dialog */}
      <Dialog
        open={isCredentialsDialogOpen}
        onOpenChange={setIsCredentialsDialogOpen}
      >
        <DialogContent className="max-w-xl">
          <DialogHeader>
            <DialogTitle>Database Credentials</DialogTitle>
            <DialogDescription>
              Connection details for {revealedDatabase?.name}
            </DialogDescription>
          </DialogHeader>
          {revealedDatabase && (
            <div className="space-y-4 py-4">
              <div className="flex items-center justify-end">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setShowPasswords(!showPasswords)}
                >
                  {showPasswords ? (
                    <>
                      <EyeOff className="h-4 w-4 mr-2" /> Hide Passwords
                    </>
                  ) : (
                    <>
                      <Eye className="h-4 w-4 mr-2" /> Show Passwords
                    </>
                  )}
                </Button>
              </div>

              <div className="space-y-3">
                {revealedDatabase.credentials?.username && (
                  <div className="flex items-center justify-between p-2 bg-muted rounded">
                    <div>
                      <div className="text-xs text-muted-foreground">
                        Username
                      </div>
                      <code className="text-sm">
                        {revealedDatabase.credentials.username}
                      </code>
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() =>
                        copyToClipboard(
                          revealedDatabase.credentials!.username,
                          "Username"
                        )
                      }
                    >
                      <Copy className="h-4 w-4" />
                    </Button>
                  </div>
                )}

                {revealedDatabase.credentials?.password && (
                  <div className="flex items-center justify-between p-2 bg-muted rounded">
                    <div>
                      <div className="text-xs text-muted-foreground">
                        Password
                      </div>
                      <code className="text-sm">
                        {showPasswords
                          ? revealedDatabase.credentials.password
                          : "••••••••••••••••"}
                      </code>
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() =>
                        copyToClipboard(
                          revealedDatabase.credentials!.password,
                          "Password"
                        )
                      }
                    >
                      <Copy className="h-4 w-4" />
                    </Button>
                  </div>
                )}

                {revealedDatabase.credentials?.database && (
                  <div className="flex items-center justify-between p-2 bg-muted rounded">
                    <div>
                      <div className="text-xs text-muted-foreground">
                        Database
                      </div>
                      <code className="text-sm">
                        {revealedDatabase.credentials.database}
                      </code>
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() =>
                        copyToClipboard(
                          revealedDatabase.credentials!.database!,
                          "Database"
                        )
                      }
                    >
                      <Copy className="h-4 w-4" />
                    </Button>
                  </div>
                )}

                {revealedDatabase.internal_connection_string && (
                  <div className="p-2 bg-muted rounded">
                    <div className="flex items-center justify-between mb-1">
                      <div className="text-xs text-muted-foreground">
                        Internal Connection String
                      </div>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() =>
                          copyToClipboard(
                            revealedDatabase.internal_connection_string!,
                            "Internal connection string"
                          )
                        }
                      >
                        <Copy className="h-4 w-4" />
                      </Button>
                    </div>
                    <code className="text-xs break-all">
                      {showPasswords
                        ? revealedDatabase.internal_connection_string
                        : revealedDatabase.internal_connection_string.replace(
                            /:[^:@]+@/,
                            ":••••••••@"
                          )}
                    </code>
                  </div>
                )}

                {revealedDatabase.external_connection_string && (
                  <div className="p-2 bg-muted rounded">
                    <div className="flex items-center justify-between mb-1">
                      <div className="text-xs text-muted-foreground">
                        External Connection String
                      </div>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() =>
                          copyToClipboard(
                            revealedDatabase.external_connection_string!,
                            "External connection string"
                          )
                        }
                      >
                        <Copy className="h-4 w-4" />
                      </Button>
                    </div>
                    <code className="text-xs break-all">
                      {showPasswords
                        ? revealedDatabase.external_connection_string
                        : revealedDatabase.external_connection_string.replace(
                            /:[^:@]+@/,
                            ":••••••••@"
                          )}
                    </code>
                  </div>
                )}
              </div>
            </div>
          )}
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setIsCredentialsDialogOpen(false)}
            >
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Database Dialog */}
      <Dialog
        open={isDeleteDbDialogOpen}
        onOpenChange={setIsDeleteDbDialogOpen}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Database</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete "{selectedDatabase?.name}"? This
              will stop the container and delete all data. This action cannot be
              undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setIsDeleteDbDialogOpen(false);
                setSelectedDatabase(null);
              }}
            >
              Cancel
            </Button>
            <Form method="post">
              <input type="hidden" name="intent" value="delete-database" />
              <input
                type="hidden"
                name="databaseId"
                value={selectedDatabase?.id || ""}
              />
              <Button
                type="submit"
                variant="destructive"
                disabled={isSubmitting}
              >
                {isSubmitting ? "Deleting..." : "Delete Database"}
              </Button>
            </Form>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Create Service Dialog */}
      <Dialog
        open={isCreateServiceDialogOpen}
        onOpenChange={setIsCreateServiceDialogOpen}
      >
        <DialogContent className="max-w-2xl">
          <Form method="post">
            <input type="hidden" name="intent" value="create-service" />
            <DialogHeader>
              <DialogTitle>Create Docker Compose Service</DialogTitle>
              <DialogDescription>
                Deploy a multi-container application using Docker Compose.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="service-name">Service Name</Label>
                <Input
                  id="service-name"
                  name="name"
                  placeholder="my-service"
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="compose-content">Docker Compose Content</Label>
                <Textarea
                  id="compose-content"
                  name="compose_content"
                  placeholder="Paste your docker-compose.yml content..."
                  className="font-mono text-sm"
                  rows={12}
                  defaultValue={`version: "3.8"
services:
  app:
    image: nginx:alpine
    ports:
      - "80"
`}
                  required
                />
              </div>
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setIsCreateServiceDialogOpen(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Creating..." : "Create Service"}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>

      {/* Delete Service Dialog */}
      <Dialog
        open={isDeleteServiceDialogOpen}
        onOpenChange={setIsDeleteServiceDialogOpen}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Service</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete "{selectedService?.name}"? This
              will stop all containers and remove all data. This action cannot
              be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setIsDeleteServiceDialogOpen(false);
                setSelectedService(null);
              }}
            >
              Cancel
            </Button>
            <Form method="post">
              <input type="hidden" name="intent" value="delete-service" />
              <input
                type="hidden"
                name="serviceId"
                value={selectedService?.id || ""}
              />
              <Button
                type="submit"
                variant="destructive"
                disabled={isSubmitting}
              >
                {isSubmitting ? "Deleting..." : "Delete Service"}
              </Button>
            </Form>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Templates Modal */}
      <Dialog
        open={isTemplatesModalOpen}
        onOpenChange={(open) => {
          setIsTemplatesModalOpen(open);
          if (!open) {
            setSelectedTemplate(null);
            setTemplateServiceName("");
            setTemplateSearch("");
            setSelectedCategory("all");
            setTemplateEnvVars({});
            setShowTemplateSecrets({});
          }
        }}
      >
        <DialogContent className="min-w-4xl max-h-[85vh]">
          {!selectedTemplate ? (
            <>
              <DialogHeader>
                <DialogTitle>Deploy Service from Template</DialogTitle>
                <DialogDescription>
                  Choose a pre-configured service template to deploy to this
                  project.
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4">
                {/* Search and Filter */}
                <div className="flex items-center gap-4">
                  <div className="relative flex-1">
                    <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                    <Input
                      placeholder="Search templates..."
                      value={templateSearch}
                      onChange={(e) => setTemplateSearch(e.target.value)}
                      className="pl-9"
                    />
                  </div>
                  <Tabs
                    value={selectedCategory}
                    onValueChange={setSelectedCategory}
                  >
                    <TabsList>
                      {categories.slice(0, 5).map((cat) => (
                        <TabsTrigger
                          key={cat}
                          value={cat}
                          className="capitalize"
                        >
                          {cat}
                        </TabsTrigger>
                      ))}
                    </TabsList>
                  </Tabs>
                </div>

                {/* Templates Grid */}
                <ScrollArea className="h-[400px] pr-4">
                  {filteredTemplates.length === 0 ? (
                    <div className="py-8 text-center text-muted-foreground">
                      No templates found matching your search.
                    </div>
                  ) : (
                    <div className="grid gap-3 sm:grid-cols-2">
                      {filteredTemplates.map((template) => (
                        <button
                          key={template.id}
                          type="button"
                          className="p-4 border rounded-lg text-left hover:border-primary hover:bg-muted/50 transition-colors"
                          onClick={() => {
                            setSelectedTemplate(template);
                            setTemplateServiceName(
                              template.name
                                .toLowerCase()
                                .replace(/[^a-z0-9]/g, "-")
                            );
                            // Initialize env vars with defaults from template
                            const defaults: Record<string, string> = {};
                            // Add PORT with default value
                            defaults["PORT"] = "8080";
                            // Add template-defined env vars
                            if (template.env_schema) {
                              for (const entry of template.env_schema) {
                                defaults[entry.name] = entry.default || "";
                              }
                            }
                            setTemplateEnvVars(defaults);
                            setShowTemplateSecrets({});
                          }}
                        >
                          <div className="flex items-start justify-between gap-2">
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2">
                                <span className="font-semibold truncate">
                                  {template.name}
                                </span>
                                {template.is_builtin && (
                                  <Badge
                                    variant="secondary"
                                    className="text-xs"
                                  >
                                    Built-in
                                  </Badge>
                                )}
                              </div>
                              <p className="text-sm text-muted-foreground line-clamp-2 mt-1">
                                {template.description}
                              </p>
                            </div>
                          </div>
                          <div className="flex items-center justify-between mt-3">
                            <Badge
                              variant="outline"
                              className="text-xs capitalize"
                            >
                              {template.category}
                            </Badge>
                            <Rocket className="h-4 w-4 text-muted-foreground" />
                          </div>
                        </button>
                      ))}
                    </div>
                  )}
                </ScrollArea>
              </div>
              <DialogFooter>
                <Button
                  variant="outline"
                  onClick={() => setIsTemplatesModalOpen(false)}
                >
                  Cancel
                </Button>
              </DialogFooter>
            </>
          ) : (
            /* Template Configuration */
            <Form method="post">
              <input type="hidden" name="intent" value="deploy-from-template" />
              <input
                type="hidden"
                name="templateId"
                value={selectedTemplate.id}
              />
              <input
                type="hidden"
                name="envVars"
                value={JSON.stringify(templateEnvVars)}
              />
              <DialogHeader>
                <DialogTitle>Deploy {selectedTemplate.name}</DialogTitle>
                <DialogDescription>
                  {selectedTemplate.description}
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4 py-4">
                <div className="space-y-2">
                  <Label htmlFor="template-service-name">Service Name</Label>
                  <Input
                    id="template-service-name"
                    name="serviceName"
                    value={templateServiceName}
                    onChange={(e) => setTemplateServiceName(e.target.value)}
                    placeholder="my-service"
                    pattern="[a-z0-9-]+"
                    required
                  />
                  <p className="text-xs text-muted-foreground">
                    Lowercase letters, numbers, and hyphens only
                  </p>
                </div>

                {/* PORT configuration */}
                <div className="space-y-2">
                  <Label htmlFor="template-port">
                    Port
                    <span className="text-destructive ml-1">*</span>
                  </Label>
                  <Input
                    id="template-port"
                    type="number"
                    value={templateEnvVars["PORT"] || "8080"}
                    onChange={(e) =>
                      setTemplateEnvVars((prev) => ({ ...prev, PORT: e.target.value }))
                    }
                    placeholder="8080"
                    required
                  />
                  <p className="text-xs text-muted-foreground">
                    Container port to expose (use unique ports to avoid conflicts)
                  </p>
                </div>

                {/* Template-defined environment variables */}
                {selectedTemplate.env_schema && selectedTemplate.env_schema.length > 0 && (
                  <div className="space-y-4 pt-2">
                    <Label className="text-base">Configuration</Label>
                    {selectedTemplate.env_schema.map((entry) => (
                      <div key={entry.name} className="space-y-1">
                        <Label htmlFor={`template-env-${entry.name}`} className="text-sm">
                          {entry.label}
                          {entry.required && <span className="text-destructive ml-1">*</span>}
                        </Label>
                        <div className="relative">
                          <Input
                            id={`template-env-${entry.name}`}
                            type={entry.secret && !showTemplateSecrets[entry.name] ? "password" : "text"}
                            value={templateEnvVars[entry.name] || ""}
                            onChange={(e) =>
                              setTemplateEnvVars((prev) => ({ ...prev, [entry.name]: e.target.value }))
                            }
                            placeholder={entry.default || `Enter ${entry.label.toLowerCase()}`}
                            required={entry.required}
                            className={entry.secret ? "pr-10" : ""}
                          />
                          {entry.secret && (
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon"
                              className="absolute right-0 top-0 h-full px-3"
                              onClick={() =>
                                setShowTemplateSecrets((prev) => ({
                                  ...prev,
                                  [entry.name]: !prev[entry.name],
                                }))
                              }
                            >
                              {showTemplateSecrets[entry.name] ? (
                                <EyeOff className="h-4 w-4" />
                              ) : (
                                <Eye className="h-4 w-4" />
                              )}
                            </Button>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => {
                    setSelectedTemplate(null);
                    setTemplateServiceName("");
                    setTemplateEnvVars({});
                    setShowTemplateSecrets({});
                  }}
                >
                  Back
                </Button>
                <Button type="submit" disabled={isSubmitting}>
                  <Rocket className="mr-2 h-4 w-4" />
                  {isSubmitting ? "Deploying..." : "Deploy Service"}
                </Button>
              </DialogFooter>
            </Form>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
