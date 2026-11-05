import { QueryError } from "@/components/query-error";
import { useState, useEffect } from "react";
import { Link, Outlet, useLocation, useParams, useNavigate } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { HeaderSkeleton, PageHeader, ResourceStatus, SectionSkeleton } from "@/components/page";
import { TopBarPortal } from "@/components/shell/top-bar-slot";
import { api } from "@/lib/api";
import { useDeployPanel } from "@/lib/deploy-panel-context";
import { useBreadcrumb } from "@/lib/breadcrumb-context";
import type { Project, Service, ServiceStatus } from "@/types/api";
import { Play, Square, Circle, Layers, RotateCw, Activity, ExternalLink, Rocket } from "lucide-react";

export function meta() {
  return [
    { title: "Service - Rivetr" },
    { name: "description", content: "Manage and monitor service" },
  ];
}


const tabs = [
  { id: "general", label: "General", path: "" },
  { id: "deployments", label: "Deployments", path: "/deployments" },
  { id: "build", label: "Build & Hooks", path: "/build" },
  { id: "containers", label: "Containers", path: "/containers" },
  { id: "environment", label: "Environment", path: "/environment" },
  { id: "network", label: "Network", path: "/network" },
  { id: "logs", label: "Logs", path: "/logs" },
  { id: "settings", label: "Settings", path: "/settings" },
];

export default function ServiceDetailLayout() {
  const { id } = useParams();
  const location = useLocation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { show: showDeployPanel } = useDeployPanel();
  const serviceId = id!;

  // Use React Query for client-side fetching
  const { data: service, isLoading, error: serviceError, refetch: refetchService } = useQuery<Service>({
    queryKey: ["service", serviceId],
    queryFn: () => api.getService(serviceId),
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!data) return 5000;
      const isTransitioning = data.status === "pending";
      return isTransitioning ? 2000 : 30000;
    },
  });

  // Project name for the breadcrumb
  const { setItems } = useBreadcrumb();
  const { data: project } = useQuery<Project>({
    queryKey: ["project", service?.project_id],
    queryFn: () => api.getProject(service!.project_id!),
    enabled: !!service?.project_id,
  });

  useEffect(() => {
    if (service) {
      setItems([
        project
          ? { label: project.name, href: `/projects/${project.id}` }
          : { label: "Projects", href: "/projects" },
        { label: service.name },
      ]);
    }
  }, [service, project, setItems]);

  // Mutations for start/stop. The mutationFn opens the deploy side panel
  // immediately so the user can watch image-pull / container-start logs flow
  // while the (sometimes slow) docker compose up call resolves.
  const startMutation = useMutation({
    mutationFn: () => {
      showDeployPanel({
        kind: "service",
        id: serviceId,
        title: service?.name ?? "Service",
        subtitle: "Service start",
      });
      return api.startService(serviceId);
    },
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

  const restartMutation = useMutation({
    mutationFn: () => {
      showDeployPanel({
        kind: "service",
        id: serviceId,
        title: service?.name ?? "Service",
        subtitle: "Service restart",
      });
      return api.restartService(serviceId);
    },
    onSuccess: () => {
      toast.success("Service restarted");
      queryClient.invalidateQueries({ queryKey: ["service", serviceId] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to restart service");
    },
  });

  const deployMutation = useMutation({
    mutationFn: () => api.deployService(serviceId),
    onSuccess: (deployment) => {
      toast.success("Deployment started");
      queryClient.invalidateQueries({ queryKey: ["service-deployments", serviceId] });
      navigate(`/services/${serviceId}/deployments?deployment=${deployment.id}`);
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to start deployment");
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

  const isSubmitting = startMutation.isPending || stopMutation.isPending || restartMutation.isPending || deleteMutation.isPending || deployMutation.isPending;

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
        <HeaderSkeleton />
        <SectionSkeleton rows={4} />
      </div>
    );
  }

  if (serviceError && !service) {
    return (
      <div className="space-y-6">
        <QueryError what="service" error={serviceError} onRetry={() => refetchService()} />
      </div>
    );
  }

  if (!service) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-semibold tracking-tight">Service Not Found</h1>
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

  const handleRestart = () => {
    restartMutation.mutate();
  };

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Layers />}
        title={service.name}
        description={service.domain ? service.domain : `Port ${service.port}`}
        meta={
          <Badge variant="outline" className="bg-purple-50 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400 border-0">
            Docker Compose
          </Badge>
        }
      />

      <TopBarPortal slot="status" inlineClassName="-mt-2 flex lg:hidden">
        <ResourceStatus status={service.status} />
      </TopBarPortal>
      <TopBarPortal slot="actions" inlineClassName="-mt-2 flex flex-wrap gap-2 lg:hidden">
        {service.status === "running" && (service.domain || service.port > 0) && (() => {
          const url = service.domain
            ? `https://${service.domain}`
            : `http://${typeof window !== "undefined" ? window.location.hostname : "localhost"}:${service.port}`;
          return (
            <Button variant="outline" size="sm" asChild className="gap-1.5">
              <a href={url} target="_blank" rel="noopener noreferrer">
                <ExternalLink className="h-3.5 w-3.5" />
                Open
              </a>
            </Button>
          );
        })()}
        <Button
          size="sm"
          disabled={isSubmitting}
          className="gap-1.5"
          onClick={() => deployMutation.mutate()}
          title="Pull or build the images and recreate changed containers"
        >
          <Rocket className="h-3.5 w-3.5" />
          Deploy
        </Button>
        {service.status === "running" ? (
          <>
            <Button variant="outline" size="sm" disabled={isSubmitting} className="gap-1.5" onClick={handleStop}>
              <Square className="h-3.5 w-3.5" />
              Stop
            </Button>
            <Button variant="outline" size="sm" disabled={isSubmitting} className="gap-1.5" onClick={handleRestart}>
              <RotateCw className="h-3.5 w-3.5" />
              Restart
            </Button>
          </>
        ) : (
          <Button variant="outline" size="sm" disabled={isSubmitting || isTransitioning} className="gap-1.5" onClick={handleStart}>
            <Play className="h-3.5 w-3.5" />
            Start
          </Button>
        )}
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
      <Outlet context={{ service }} />
    </div>
  );
}
