import { useState, useEffect } from "react";
import { Link, Outlet, useLocation, useParams, useNavigate } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api";
import type { Service, ServiceStatus } from "@/types/api";
import { Play, Square, Circle, Layers } from "lucide-react";

export function meta() {
  return [
    { title: "Service - Rivetr" },
    { name: "description", content: "Manage and monitor service" },
  ];
}

// Status badge component
function StatusBadge({ status }: { status?: ServiceStatus }) {
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
      return (
        <Badge variant="outline" className="gap-1 text-blue-600 border-blue-300">
          <span className="relative flex h-2 w-2">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-blue-400 opacity-75"></span>
            <span className="relative inline-flex h-2 w-2 rounded-full bg-blue-500"></span>
          </span>
          Pending
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

const tabs = [
  { id: "general", label: "General", path: "" },
  { id: "network", label: "Network", path: "/network" },
  { id: "logs", label: "Logs", path: "/logs" },
  { id: "settings", label: "Settings", path: "/settings" },
];

export default function ServiceDetailLayout() {
  const { id } = useParams();
  const location = useLocation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const serviceId = id!;

  // Use React Query for client-side fetching
  const { data: service, isLoading } = useQuery<Service>({
    queryKey: ["service", serviceId],
    queryFn: () => api.getService(serviceId),
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!data) return 5000;
      const isTransitioning = data.status === "pending";
      return isTransitioning ? 2000 : 30000;
    },
  });

  // Mutations for start/stop
  const startMutation = useMutation({
    mutationFn: () => api.startService(serviceId),
    onSuccess: () => {
      toast.success("Service started");
      queryClient.invalidateQueries({ queryKey: ["service", serviceId] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to start service");
    },
  });

  const stopMutation = useMutation({
    mutationFn: () => api.stopService(serviceId),
    onSuccess: () => {
      toast.success("Service stopped");
      queryClient.invalidateQueries({ queryKey: ["service", serviceId] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to stop service");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => api.deleteService(serviceId),
    onSuccess: () => {
      toast.success("Service deleted");
      if (service?.project_id) {
        navigate(`/projects/${service.project_id}`);
      } else {
        navigate("/projects");
      }
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete service");
    },
  });

  const isSubmitting = startMutation.isPending || stopMutation.isPending || deleteMutation.isPending;

  // Determine active tab from path
  const basePath = `/services/${serviceId}`;
  const currentPath = location.pathname;
  const activeTab = tabs.find((tab) => {
    if (tab.path === "") {
      return currentPath === basePath || currentPath === basePath + "/";
    }
    return currentPath.startsWith(basePath + tab.path);
  })?.id || "general";

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div className="animate-pulse">
          <div className="h-8 w-48 bg-muted rounded mb-2" />
          <div className="h-4 w-64 bg-muted rounded" />
        </div>
      </div>
    );
  }

  if (!service) {
    return (
      <div className="space-y-6">
        <h1 className="text-3xl font-bold">Service Not Found</h1>
        <p className="text-muted-foreground">
          The service you're looking for doesn't exist or has been deleted.
        </p>
      </div>
    );
  }

  const isTransitioning = service.status === "pending";

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
            <Layers className="h-8 w-8 text-muted-foreground" />
            <h1 className="text-3xl font-bold">{service.name}</h1>
            <Badge variant="outline" className="bg-purple-50 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400 border-0">
              Docker Compose
            </Badge>
            <StatusBadge status={service.status} />
          </div>
          {service.project_id && (
            <p className="text-muted-foreground mt-1">
              <Link to={`/projects/${service.project_id}`} className="hover:underline">
                Back to Project
              </Link>
            </p>
          )}
        </div>
        <div className="flex gap-2">
          {/* Start/Stop buttons */}
          {service.status === "running" ? (
            <Button
              variant="outline"
              disabled={isSubmitting}
              className="gap-2"
              onClick={handleStop}
            >
              <Square className="h-4 w-4" />
              Stop
            </Button>
          ) : service.status === "stopped" || service.status === "failed" ? (
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
      <Outlet context={{ service }} />
    </div>
  );
}
