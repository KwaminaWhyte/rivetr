import { useState, useMemo } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { useQuery, useQueryClient, useMutation } from "@tanstack/react-query";
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
import { useTeamContext } from "@/lib/team-context";
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
  ServiceTemplate,
} from "@/types/api";
import { DATABASE_TYPES } from "@/types/api";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";

export function meta() {
  return [
    { title: "Project - Rivetr" },
    {
      name: "description",
      content: "Manage project and its applications",
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

export default function ProjectDetailPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { currentTeamId } = useTeamContext();
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

  // Form state for database creation
  const [dbName, setDbName] = useState("");
  const [dbVersion, setDbVersion] = useState("latest");
  const [dbPublicAccess, setDbPublicAccess] = useState(false);
  const [dbUsername, setDbUsername] = useState("");
  const [dbPassword, setDbPassword] = useState("");
  const [dbDatabase, setDbDatabase] = useState("");
  const [dbRootPassword, setDbRootPassword] = useState("");

  // Form state for service creation
  const [serviceName, setServiceName] = useState("");
  const [composeContent, setComposeContent] = useState(`version: "3.8"
services:
  app:
    image: nginx:alpine
    ports:
      - "80"
`);

  // Use React Query for data fetching
  const { data: project, isLoading: projectLoading } = useQuery<ProjectWithApps>({
    queryKey: ["project", id],
    queryFn: () => api.getProject(id!),
    refetchInterval: 5000, // Poll for database status updates
  });

  const { data: allApps = [] } = useQuery<App[]>({
    queryKey: ["apps", currentTeamId],
    queryFn: () => api.getApps({ teamId: currentTeamId ?? undefined }),
    enabled: currentTeamId !== null,
  });

  const { data: templates = [] } = useQuery<ServiceTemplate[]>({
    queryKey: ["service-templates"],
    queryFn: () => api.getTemplates(),
  });

  // Fetch app statuses for apps in this project
  const { data: appStatuses = {} } = useQuery<Record<string, string>>({
    queryKey: ["app-statuses", project?.apps?.map((a) => a.id)],
    queryFn: async () => {
      if (!project?.apps) return {};
      const statuses: Record<string, string> = {};
      await Promise.all(
        project.apps.map(async (app) => {
          try {
            const status = await api.getAppStatus(app.id);
            statuses[app.id] = status.status;
          } catch {
            statuses[app.id] = "stopped";
          }
        })
      );
      return statuses;
    },
    enabled: !!project?.apps?.length,
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

  const openEditDialog = () => {
    if (project) {
      setEditData({
        name: project.name,
        description: project.description || "",
      });
      setIsEditDialogOpen(true);
    }
  };

  // Mutations
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

  const assignAppMutation = useMutation({
    mutationFn: (appId: string) => api.assignAppToProject(appId, id!),
    onSuccess: () => {
      toast.success("App added to project");
      setIsAddAppDialogOpen(false);
      queryClient.invalidateQueries({ queryKey: ["project", id] });
      queryClient.invalidateQueries({ queryKey: ["apps"] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const removeAppMutation = useMutation({
    mutationFn: (appId: string) => api.assignAppToProject(appId, null),
    onSuccess: () => {
      toast.success("App removed from project");
      queryClient.invalidateQueries({ queryKey: ["project", id] });
      queryClient.invalidateQueries({ queryKey: ["apps"] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  // Database mutations
  const createDatabaseMutation = useMutation({
    mutationFn: async () => {
      if (!dbName.trim()) {
        throw new Error("Database name is required");
      }
      return api.createDatabase({
        name: dbName.trim(),
        db_type: selectedDbType,
        version: dbVersion,
        public_access: dbPublicAccess,
        project_id: id!,
        ...(dbUsername.trim() ? { username: dbUsername.trim() } : {}),
        ...(dbPassword.trim() ? { password: dbPassword.trim() } : {}),
        ...(dbDatabase.trim() ? { database: dbDatabase.trim() } : {}),
        ...(dbRootPassword.trim() ? { root_password: dbRootPassword.trim() } : {}),
      });
    },
    onSuccess: () => {
      toast.success("Database created");
      setIsCreateDbDialogOpen(false);
      resetDbForm();
      queryClient.invalidateQueries({ queryKey: ["project", id] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const deleteDatabaseMutation = useMutation({
    mutationFn: (databaseId: string) => api.deleteDatabase(databaseId),
    onSuccess: () => {
      toast.success("Database deleted");
      setIsDeleteDbDialogOpen(false);
      setSelectedDatabase(null);
      queryClient.invalidateQueries({ queryKey: ["project", id] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const startDatabaseMutation = useMutation({
    mutationFn: (databaseId: string) => api.startDatabase(databaseId),
    onSuccess: () => {
      toast.success("Database starting");
      queryClient.invalidateQueries({ queryKey: ["project", id] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const stopDatabaseMutation = useMutation({
    mutationFn: (databaseId: string) => api.stopDatabase(databaseId),
    onSuccess: () => {
      toast.success("Database stopped");
      queryClient.invalidateQueries({ queryKey: ["project", id] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  // Service mutations
  const createServiceMutation = useMutation({
    mutationFn: async () => {
      if (!serviceName.trim()) {
        throw new Error("Service name is required");
      }
      if (!composeContent.trim()) {
        throw new Error("Docker Compose content is required");
      }
      return api.createService({
        name: serviceName.trim(),
        compose_content: composeContent.trim(),
        project_id: id!,
      });
    },
    onSuccess: () => {
      toast.success("Service created");
      setIsCreateServiceDialogOpen(false);
      setServiceName("");
      setComposeContent(`version: "3.8"
services:
  app:
    image: nginx:alpine
    ports:
      - "80"
`);
      queryClient.invalidateQueries({ queryKey: ["project", id] });
      queryClient.invalidateQueries({ queryKey: ["services"] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const deleteServiceMutation = useMutation({
    mutationFn: (serviceId: string) => api.deleteService(serviceId),
    onSuccess: () => {
      toast.success("Service deleted");
      setIsDeleteServiceDialogOpen(false);
      setSelectedService(null);
      queryClient.invalidateQueries({ queryKey: ["project", id] });
      queryClient.invalidateQueries({ queryKey: ["services"] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const startServiceMutation = useMutation({
    mutationFn: (serviceId: string) => api.startService(serviceId),
    onSuccess: () => {
      toast.success("Service starting");
      queryClient.invalidateQueries({ queryKey: ["project", id] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const stopServiceMutation = useMutation({
    mutationFn: (serviceId: string) => api.stopService(serviceId),
    onSuccess: () => {
      toast.success("Service stopped");
      queryClient.invalidateQueries({ queryKey: ["project", id] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const deployTemplateMutation = useMutation({
    mutationFn: async () => {
      if (!selectedTemplate) {
        throw new Error("No template selected");
      }
      if (!templateServiceName.trim()) {
        throw new Error("Service name is required");
      }
      return api.deployTemplate(selectedTemplate.id, {
        name: templateServiceName.trim(),
        project_id: id!,
        env_vars: templateEnvVars,
      });
    },
    onSuccess: () => {
      toast.success("Service deployed from template");
      setIsTemplatesModalOpen(false);
      setSelectedTemplate(null);
      setTemplateServiceName("");
      setTemplateEnvVars({});
      setShowTemplateSecrets({});
      queryClient.invalidateQueries({ queryKey: ["project", id] });
      queryClient.invalidateQueries({ queryKey: ["services"] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const resetDbForm = () => {
    setDbName("");
    setDbVersion("latest");
    setDbPublicAccess(false);
    setDbUsername("");
    setDbPassword("");
    setDbDatabase("");
    setDbRootPassword("");
    setSelectedDbType("postgres");
    setShowCustomCredentials(false);
  };

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

  // Loading state
  if (projectLoading) {
    return (
      <div className="space-y-6">
        <div className="flex items-center gap-4">
          <Button variant="ghost" size="icon" asChild>
            <Link to="/projects">
              <ArrowLeft className="h-4 w-4" />
            </Link>
          </Button>
          <div className="flex-1">
            <div className="h-8 w-48 bg-muted animate-pulse rounded" />
            <div className="h-4 w-64 bg-muted animate-pulse rounded mt-2" />
          </div>
        </div>
        <Card>
          <CardContent className="py-8">
            <div className="h-6 w-32 bg-muted animate-pulse rounded mb-4" />
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {[1, 2, 3].map((i) => (
                <div key={i} className="h-32 bg-muted animate-pulse rounded" />
              ))}
            </div>
          </CardContent>
        </Card>
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
                const status = appStatuses?.[app.id] || "stopped";
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
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8 opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive transition-opacity relative z-10"
                          disabled={removeAppMutation.isPending}
                          title="Remove from project"
                          onClick={(e) => {
                            e.preventDefault();
                            removeAppMutation.mutate(app.id);
                          }}
                        >
                          <X className="h-4 w-4" />
                        </Button>
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
                            <Button
                              variant="outline"
                              size="sm"
                              className="h-7 px-2"
                              disabled={startDatabaseMutation.isPending}
                              onClick={(e) => {
                                e.preventDefault();
                                startDatabaseMutation.mutate(db.id);
                              }}
                            >
                              <Play className="h-3 w-3 mr-1" />
                              Start
                            </Button>
                          )}
                          {db.status === "running" && (
                            <Button
                              variant="outline"
                              size="sm"
                              className="h-7 px-2"
                              disabled={stopDatabaseMutation.isPending}
                              onClick={(e) => {
                                e.preventDefault();
                                stopDatabaseMutation.mutate(db.id);
                              }}
                            >
                              <Square className="h-3 w-3 mr-1" />
                              Stop
                            </Button>
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
                          <Button
                            variant="outline"
                            size="sm"
                            className="h-7 px-2"
                            disabled={startServiceMutation.isPending}
                            onClick={(e) => {
                              e.preventDefault();
                              startServiceMutation.mutate(service.id);
                            }}
                          >
                            <Play className="h-3 w-3 mr-1" />
                            Start
                          </Button>
                        )}
                        {service.status === "running" && (
                          <Button
                            variant="outline"
                            size="sm"
                            className="h-7 px-2"
                            disabled={stopServiceMutation.isPending}
                            onClick={(e) => {
                              e.preventDefault();
                              stopServiceMutation.mutate(service.id);
                            }}
                          >
                            <Square className="h-3 w-3 mr-1" />
                            Stop
                          </Button>
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
          <form onSubmit={(e) => {
            e.preventDefault();
            updateProjectMutation.mutate(editData);
          }}>
            <DialogHeader>
              <DialogTitle>Edit Project</DialogTitle>
              <DialogDescription>
                Update your project details.
              </DialogDescription>
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
                  <button
                    key={app.id}
                    type="button"
                    className="flex items-center justify-between w-full p-3 rounded-lg border hover:bg-muted/50 cursor-pointer text-left"
                    disabled={assignAppMutation.isPending}
                    onClick={() => assignAppMutation.mutate(app.id)}
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
        onOpenChange={(open) => {
          setIsCreateDbDialogOpen(open);
          if (!open) resetDbForm();
        }}
      >
        <DialogContent className="max-w-lg">
          <form onSubmit={(e) => {
            e.preventDefault();
            createDatabaseMutation.mutate();
          }}>
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
                  value={dbName}
                  onChange={(e) => setDbName(e.target.value)}
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
              </div>

              <div className="space-y-2">
                <Label htmlFor="db-version">Version</Label>
                <Select value={dbVersion} onValueChange={setDbVersion}>
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
                  checked={dbPublicAccess}
                  onCheckedChange={(checked) => setDbPublicAccess(checked === true)}
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
                      value={dbUsername}
                      onChange={(e) => setDbUsername(e.target.value)}
                      placeholder="Auto-generated if empty"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="db-password">Password</Label>
                    <Input
                      id="db-password"
                      value={dbPassword}
                      onChange={(e) => setDbPassword(e.target.value)}
                      type="password"
                      placeholder="Auto-generated if empty"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="db-database">Database Name</Label>
                    <Input
                      id="db-database"
                      value={dbDatabase}
                      onChange={(e) => setDbDatabase(e.target.value)}
                      placeholder="Defaults to username"
                    />
                  </div>
                  {selectedDbType === "mysql" && (
                    <div className="space-y-2">
                      <Label htmlFor="db-root-password">Root Password</Label>
                      <Input
                        id="db-root-password"
                        value={dbRootPassword}
                        onChange={(e) => setDbRootPassword(e.target.value)}
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
              <Button type="submit" disabled={createDatabaseMutation.isPending}>
                {createDatabaseMutation.isPending ? "Creating..." : "Create Database"}
              </Button>
            </DialogFooter>
          </form>
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
                          : "----------------"}
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
                            ":--------@"
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
                            ":--------@"
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
            <Button
              variant="destructive"
              disabled={deleteDatabaseMutation.isPending}
              onClick={() => {
                if (selectedDatabase) {
                  deleteDatabaseMutation.mutate(selectedDatabase.id);
                }
              }}
            >
              {deleteDatabaseMutation.isPending ? "Deleting..." : "Delete Database"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Create Service Dialog */}
      <Dialog
        open={isCreateServiceDialogOpen}
        onOpenChange={setIsCreateServiceDialogOpen}
      >
        <DialogContent className="max-w-2xl">
          <form onSubmit={(e) => {
            e.preventDefault();
            createServiceMutation.mutate();
          }}>
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
                  value={serviceName}
                  onChange={(e) => setServiceName(e.target.value)}
                  placeholder="my-service"
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="compose-content">Docker Compose Content</Label>
                <Textarea
                  id="compose-content"
                  value={composeContent}
                  onChange={(e) => setComposeContent(e.target.value)}
                  placeholder="Paste your docker-compose.yml content..."
                  className="font-mono text-sm"
                  rows={12}
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
              <Button type="submit" disabled={createServiceMutation.isPending}>
                {createServiceMutation.isPending ? "Creating..." : "Create Service"}
              </Button>
            </DialogFooter>
          </form>
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
            <Button
              variant="destructive"
              disabled={deleteServiceMutation.isPending}
              onClick={() => {
                if (selectedService) {
                  deleteServiceMutation.mutate(selectedService.id);
                }
              }}
            >
              {deleteServiceMutation.isPending ? "Deleting..." : "Delete Service"}
            </Button>
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
            <form onSubmit={(e) => {
              e.preventDefault();
              deployTemplateMutation.mutate();
            }}>
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
                <Button type="submit" disabled={deployTemplateMutation.isPending}>
                  <Rocket className="mr-2 h-4 w-4" />
                  {deployTemplateMutation.isPending ? "Deploying..." : "Deploy Service"}
                </Button>
              </DialogFooter>
            </form>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
