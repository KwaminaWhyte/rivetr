import { useEffect } from "react";
import { Link, Outlet, useLocation, useParams, useNavigate } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import type { ManagedDatabase, DatabaseStatus } from "@/types/api";
import { Play, Square, Circle, Database } from "lucide-react";

// Status badge component
function StatusBadge({ status }: { status?: DatabaseStatus }) {
  if (!status) return null;

  switch (status) {
    case "running":
      return (
        <Badge className="bg-green-500 text-white gap-1">
          <Circle className="h-2 w-2 fill-current" />
          Running
        </Badge>
      );
    case "stopped":
      return (
        <Badge variant="secondary" className="gap-1">
          <Circle className="h-2 w-2" />
          Stopped
        </Badge>
      );
    case "pending":
    case "pulling":
    case "starting":
      return (
        <Badge variant="outline" className="gap-1 text-blue-600 border-blue-300">
          <span className="relative flex h-2 w-2">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-blue-400 opacity-75"></span>
            <span className="relative inline-flex h-2 w-2 rounded-full bg-blue-500"></span>
          </span>
          {status === "pending" ? "Pending" : status === "pulling" ? "Pulling" : "Starting"}
        </Badge>
      );
    case "failed":
      return (
        <Badge variant="destructive" className="gap-1">
          <Circle className="h-2 w-2 fill-current" />
          Failed
        </Badge>
      );
    default:
      return null;
  }
}

// Database type badge
function DatabaseTypeBadge({ dbType }: { dbType: string }) {
  const typeConfig: Record<string, { bg: string; text: string; label: string }> = {
    postgres: { bg: "bg-blue-100", text: "text-blue-800", label: "PostgreSQL" },
    mysql: { bg: "bg-orange-100", text: "text-orange-800", label: "MySQL" },
    mongodb: { bg: "bg-green-100", text: "text-green-800", label: "MongoDB" },
    redis: { bg: "bg-red-100", text: "text-red-800", label: "Redis" },
  };

  const config = typeConfig[dbType] || { bg: "bg-gray-100", text: "text-gray-800", label: dbType };

  return (
    <Badge variant="outline" className={`${config.bg} ${config.text} border-0`}>
      {config.label}
    </Badge>
  );
}

const tabs = [
  { id: "general", label: "General", path: "" },
  { id: "network", label: "Network", path: "/network" },
  { id: "storage", label: "Storage", path: "/storage" },
  { id: "backups", label: "Backups", path: "/backups" },
  { id: "logs", label: "Logs", path: "/logs" },
  { id: "settings", label: "Settings", path: "/settings" },
];

export default function DatabaseDetailLayout() {
  const { id } = useParams();
  const location = useLocation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const databaseId = id!;

  // Fetch database data client-side
  const { data: database, isLoading, error } = useQuery<ManagedDatabase>({
    queryKey: ["database", databaseId],
    queryFn: () => api.getDatabase(databaseId, true),
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!data) return 5000;
      const isTransitioning = ["pending", "pulling", "starting"].includes(data.status);
      return isTransitioning ? 2000 : 30000;
    },
  });

  // Mutations
  const startMutation = useMutation({
    mutationFn: () => api.startDatabase(databaseId),
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
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Database className="h-8 w-8 text-muted-foreground" />
            <Skeleton className="h-9 w-48" />
            <Skeleton className="h-6 w-24" />
          </div>
        </div>
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  // Error or not found state
  if (error || !database) {
    return (
      <div className="space-y-6">
        <h1 className="text-3xl font-bold">Database Not Found</h1>
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
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-3">
            <Database className="h-8 w-8 text-muted-foreground" />
            <h1 className="text-3xl font-bold">{database.name}</h1>
            <DatabaseTypeBadge dbType={database.db_type} />
            <Badge variant="outline">{database.version}</Badge>
            <StatusBadge status={database.status} />
          </div>
          {database.project_id && (
            <p className="text-muted-foreground mt-1">
              <Link to={`/projects/${database.project_id}`} className="hover:underline">
                Back to Project
              </Link>
            </p>
          )}
        </div>
        <div className="flex gap-2">
          {/* Start/Stop buttons */}
          {database.status === "running" ? (
            <Button
              variant="outline"
              disabled={isSubmitting}
              className="gap-2"
              onClick={handleStop}
            >
              <Square className="h-4 w-4" />
              Stop
            </Button>
          ) : database.status === "stopped" || database.status === "failed" ? (
            <Button
              variant="outline"
              disabled={isSubmitting || isTransitioning}
              className="gap-2"
              onClick={handleStart}
            >
              <Play className="h-4 w-4" />
              Start
            </Button>
          ) : null}
        </div>
      </div>

      {/* Tabs Navigation */}
      <Tabs value={activeTab} className="w-full">
        <TabsList className="w-full justify-start">
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
