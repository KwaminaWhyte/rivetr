import { useEffect } from "react";
import { Link, Outlet, useLocation, useNavigation, Form, redirect } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { Route } from "./+types/_layout";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api";
import type { Service, ServiceStatus } from "@/types/api";
import { Play, Square, Circle, Layers } from "lucide-react";

export function meta({ data }: Route.MetaArgs) {
  const serviceName = data?.service?.name || "Service";
  return [
    { title: `${serviceName} - Rivetr` },
    { name: "description", content: `Manage and monitor ${serviceName}` },
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
    case "starting":
      return (
        <Badge variant="outline" className="gap-1 text-blue-600 border-blue-300">
          <span className="relative flex h-2 w-2">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-blue-400 opacity-75"></span>
            <span className="relative inline-flex h-2 w-2 rounded-full bg-blue-500"></span>
          </span>
          {status === "pending" ? "Pending" : "Starting"}
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

export async function loader({ request, params }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const service = await api.getService(token, params.id!);
  return { service, token };
}

export async function action({ request, params }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "start") {
    try {
      await api.startService(token, params.id!);
      return { success: true, action: "start" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to start service" };
    }
  }

  if (intent === "stop") {
    try {
      await api.stopService(token, params.id!);
      return { success: true, action: "stop" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to stop service" };
    }
  }

  if (intent === "delete") {
    const projectId = formData.get("projectId");
    try {
      await api.deleteService(token, params.id!);
      if (projectId) {
        return redirect(`/projects/${projectId}`);
      }
      return redirect("/projects");
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to delete service" };
    }
  }

  return { error: "Unknown action" };
}

const tabs = [
  { id: "general", label: "General", path: "" },
  { id: "network", label: "Network", path: "/network" },
  { id: "logs", label: "Logs", path: "/logs" },
  { id: "settings", label: "Settings", path: "/settings" },
];

export default function ServiceDetailLayout({ loaderData, actionData, params }: Route.ComponentProps) {
  const location = useLocation();
  const navigation = useNavigation();
  const queryClient = useQueryClient();

  // Use React Query with SSR initial data
  const { data: service } = useQuery<Service>({
    queryKey: ["service", loaderData.service.id],
    queryFn: () => api.getService(loaderData.service.id, loaderData.token),
    initialData: loaderData.service,
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!data) return 5000;
      const isTransitioning = ["pending", "starting"].includes(data.status);
      return isTransitioning ? 2000 : 30000;
    },
  });

  const isSubmitting = navigation.state === "submitting";

  // Handle successful actions
  useEffect(() => {
    if (actionData?.success) {
      if (actionData.action === "start") {
        toast.success("Service started");
      } else if (actionData.action === "stop") {
        toast.success("Service stopped");
      }
      queryClient.invalidateQueries({ queryKey: ["service", service?.id] });
    }
    if (actionData?.error) {
      toast.error(actionData.error);
    }
  }, [actionData, service?.id, queryClient]);

  // Determine active tab from path
  const basePath = `/services/${params.id}`;
  const currentPath = location.pathname;
  const activeTab = tabs.find((tab) => {
    if (tab.path === "") {
      return currentPath === basePath || currentPath === basePath + "/";
    }
    return currentPath.startsWith(basePath + tab.path);
  })?.id || "general";

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

  const isTransitioning = ["pending", "starting"].includes(service.status);

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
            <Form method="post">
              <input type="hidden" name="intent" value="stop" />
              <Button
                type="submit"
                variant="outline"
                disabled={isSubmitting}
                className="gap-2"
              >
                <Square className="h-4 w-4" />
                Stop
              </Button>
            </Form>
          ) : service.status === "stopped" || service.status === "failed" ? (
            <Form method="post">
              <input type="hidden" name="intent" value="start" />
              <Button
                type="submit"
                variant="outline"
                disabled={isSubmitting || isTransitioning}
                className="gap-2"
              >
                <Play className="h-4 w-4" />
                Start
              </Button>
            </Form>
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
      <Outlet context={{ service, token: loaderData.token }} />
    </div>
  );
}
