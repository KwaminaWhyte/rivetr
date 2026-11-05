import { QueryError } from "@/components/query-error";
import { useEffect } from "react";
import { Link, Outlet, useLocation, useParams, useNavigate } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { HeaderSkeleton, PageHeader, ResourceStatus, SectionSkeleton } from "@/components/page";
import { TopBarPortal } from "@/components/shell/top-bar-slot";
import { api } from "@/lib/api";
import { useBreadcrumb } from "@/lib/breadcrumb-context";
import { useDeployPanel } from "@/lib/deploy-panel-context";
import type { ManagedDatabase, DatabaseStatus, Project } from "@/types/api";
import { Play, Square, Circle, Database } from "lucide-react";

export function meta() {
  return [
    { title: "Database - Rivetr" },
    { name: "description", content: "Manage and monitor database instance" },
  ];
}


// Database type badge
function DatabaseTypeBadge({ dbType }: { dbType: string }) {
  const typeConfig: Record<string, { bg: string; text: string; label: string }> = {
    postgres: { bg: "bg-blue-100 dark:bg-blue-500/20", text: "text-blue-800 dark:text-blue-300", label: "PostgreSQL" },
    mysql: { bg: "bg-orange-100 dark:bg-orange-500/20", text: "text-orange-800 dark:text-orange-300", label: "MySQL" },
    mariadb: { bg: "bg-amber-100 dark:bg-amber-500/20", text: "text-amber-800 dark:text-amber-300", label: "MariaDB" },
    mongodb: { bg: "bg-green-100 dark:bg-green-500/20", text: "text-green-800 dark:text-green-300", label: "MongoDB" },
    redis: { bg: "bg-red-100 dark:bg-red-500/20", text: "text-red-800 dark:text-red-300", label: "Redis" },
  };

  const config = typeConfig[dbType] || { bg: "bg-gray-100 dark:bg-gray-500/20", text: "text-gray-800 dark:text-gray-300", label: dbType };

  return (
    <Badge variant="outline" className={`${config.bg} ${config.text} border-0`}>
      {config.label}
    </Badge>
  );
}

const tabs = [
  { id: "general", label: "General", path: "" },
  { id: "extensions", label: "Extensions", path: "/extensions" },
  { id: "network", label: "Network", path: "/network" },
  { id: "storage", label: "Storage", path: "/storage" },
  { id: "backups", label: "Backups", path: "/backups" },
  { id: "import", label: "Import", path: "/import" },
  { id: "logs", label: "Logs", path: "/logs" },
  { id: "settings", label: "Settings", path: "/settings" },
];

export default function DatabaseDetailLayout() {
  const { id } = useParams();
  const location = useLocation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { setItems } = useBreadcrumb();
  const { show: showDeployPanel } = useDeployPanel();
  const databaseId = id!;

  // Fetch database data client-side
  const { data: database, isLoading, error, refetch } = useQuery<ManagedDatabase>({
    queryKey: ["database", databaseId],
    queryFn: () => api.getDatabase(databaseId, true),
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!data) return 5000;
      const isTransitioning = ["pending", "pulling", "starting"].includes(data.status);
      return isTransitioning ? 2000 : 30000;
    },
  });

  // Fetch project for breadcrumb
  const { data: project } = useQuery<Project>({
    queryKey: ["project", database?.project_id],
    queryFn: () => api.getProject(database!.project_id!),
    enabled: !!database?.project_id,
  });

  // Set breadcrumbs when database and project are loaded
  useEffect(() => {
    if (database) {
      const breadcrumbs = [];
      if (project) {
        breadcrumbs.push({ label: project.name, href: `/projects/${project.id}` });
      } else {
        breadcrumbs.push({ label: "Projects", href: "/projects" });
      }
      breadcrumbs.push({ label: database.name });
      setItems(breadcrumbs);
    }
  }, [database, project, setItems]);

  // Mutations. Open the deploy side panel before firing the request so the
  // user can watch image-pull / container-create logs live.
  const startMutation = useMutation({
    mutationFn: () => {
      showDeployPanel({
        kind: "database",
        id: databaseId,
        title: database?.name ?? "Database",
        subtitle: "Database start",
      });
      return api.startDatabase(databaseId);
    },
    onSuccess: () => {
      toast.success("Database started");
      queryClient.invalidateQueries({ queryKey: ["database", databaseId] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to start database");
    },
  });

  const stopMutation = useMutation({
    mutationFn: () => api.stopDatabase(databaseId),
    onSuccess: () => {
      toast.success("Database stopped");
      queryClient.invalidateQueries({ queryKey: ["database", databaseId] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to stop database");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => api.deleteDatabase(databaseId),
    onSuccess: () => {
      toast.success("Database deleted");
      navigate("/projects");
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete database");
    },
  });

  const isSubmitting = startMutation.isPending || stopMutation.isPending || deleteMutation.isPending;

  // Determine active tab from path
  const basePath = `/databases/${databaseId}`;
  const currentPath = location.pathname;
  const activeTab = tabs.find((tab) => {
    if (tab.path === "") {
      return currentPath === basePath || currentPath === basePath + "/";
    }
    return currentPath.startsWith(basePath + tab.path);
  })?.id || "general";

  // Loading state
  if (isLoading) {
    return (
      <div className="space-y-6">
        <HeaderSkeleton />
        <SectionSkeleton rows={4} />
      </div>
    );
  }

  // Request failed (not a 404-style missing database)
  if (error && !database) {
    return (
      <div className="space-y-6">
        <QueryError what="database" error={error} onRetry={() => refetch()} />
      </div>
    );
  }

  // Not found state
  if (!database) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-semibold tracking-tight">Database Not Found</h1>
        <p className="text-muted-foreground">
          The database you're looking for doesn't exist or has been deleted.
        </p>
      </div>
    );
  }

  const isTransitioning = ["pending", "pulling", "starting"].includes(database.status);

  const handleStart = () => {
    startMutation.mutate();
  };

  const handleStop = () => {
    stopMutation.mutate();
  };

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Database />}
        title={database.name}
        description={database.public_access ? `Public on port ${database.external_port}` : "Internal only"}
        meta={
          <>
            <DatabaseTypeBadge dbType={database.db_type} />
            <Badge variant="outline">{database.version}</Badge>
          </>
        }
      />

      <TopBarPortal slot="status" inlineClassName="-mt-2 flex lg:hidden">
        <ResourceStatus status={database.status} />
      </TopBarPortal>
      <TopBarPortal slot="actions" inlineClassName="-mt-2 flex flex-wrap gap-2 lg:hidden">
        {database.status === "running" ? (
          <Button variant="outline" size="sm" disabled={isSubmitting} className="gap-1.5" onClick={handleStop}>
            <Square className="h-3.5 w-3.5" />
            Stop
          </Button>
        ) : database.status === "stopped" || database.status === "failed" ? (
          <Button size="sm" disabled={isSubmitting || isTransitioning} className="gap-1.5" onClick={handleStart}>
            <Play className="h-3.5 w-3.5" />
            Start
          </Button>
        ) : null}
      </TopBarPortal>

      {/* Tabs Navigation: the contextual sidebar replaces it from md up */}
      <Tabs value={activeTab} className="w-full overflow-x-auto md:hidden">
        <TabsList className="w-max justify-start">
          {tabs.map((tab) => (
            <TabsTrigger key={tab.id} value={tab.id} asChild>
              <Link to={`${basePath}${tab.path}`}>{tab.label}</Link>
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>

      {/* Tab Content via Outlet */}
      <Outlet context={{ database }} />
    </div>
  );
}
