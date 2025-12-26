import { useState, useMemo, useEffect } from "react";
import { Link, Outlet, useLocation, useNavigation, Form, redirect } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { Route } from "./+types/_layout";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { EnvironmentBadge } from "@/components/environment-badge";
import { api } from "@/lib/api";
import type { App, Deployment, DeploymentStatus } from "@/types/api";

const ACTIVE_STATUSES: DeploymentStatus[] = ["pending", "cloning", "building", "starting", "checking"];

function isActiveDeployment(status: DeploymentStatus): boolean {
  return ACTIVE_STATUSES.includes(status);
}

export async function loader({ request, params }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const [app, deployments] = await Promise.all([
    api.getApp(token, params.id!),
    api.getDeployments(token, params.id!).catch(() => []),
  ]);
  return { app, deployments, token };
}

export async function action({ request, params }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "delete") {
    await api.deleteApp(token, params.id!);
    return redirect("/projects");
  }

  if (intent === "deploy") {
    try {
      await api.triggerDeploy(token, params.id!);
      return { success: true, action: "deploy" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Deployment failed" };
    }
  }

  return { error: "Unknown action" };
}

const tabs = [
  { id: "general", label: "General", path: "" },
  { id: "settings", label: "Settings", path: "/settings" },
  { id: "deployments", label: "Deployments", path: "/deployments" },
  { id: "logs", label: "Logs", path: "/logs" },
  { id: "terminal", label: "Terminal", path: "/terminal" },
];

export default function AppDetailLayout({ loaderData, actionData, params }: Route.ComponentProps) {
  const location = useLocation();
  const navigation = useNavigation();
  const queryClient = useQueryClient();

  // Use React Query with SSR initial data
  const { data: app } = useQuery<App>({
    queryKey: ["app", loaderData.app.id],
    queryFn: () => api.getApp(loaderData.app.id, loaderData.token),
    initialData: loaderData.app,
  });

  const { data: deployments = [] } = useQuery<Deployment[]>({
    queryKey: ["deployments", loaderData.app.id],
    queryFn: () => api.getDeployments(loaderData.app.id, loaderData.token),
    initialData: loaderData.deployments,
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!data || data.length === 0) return 5000;
      const hasActive = data.some((d: Deployment) => isActiveDeployment(d.status));
      return hasActive ? 2000 : 30000;
    },
    refetchIntervalInBackground: false,
  });

  const hasActiveDeployment = useMemo(() => {
    return deployments.some((d) => isActiveDeployment(d.status));
  }, [deployments]);

  const runningDeployment = useMemo(() => {
    return deployments.find((d) => d.status === "running");
  }, [deployments]);

  const isSubmitting = navigation.state === "submitting";

  // Handle successful actions
  useEffect(() => {
    if (actionData?.success) {
      if (actionData.action === "deploy") {
        toast.success("Deployment started");
      }
      queryClient.invalidateQueries({ queryKey: ["app", app?.id] });
      queryClient.invalidateQueries({ queryKey: ["deployments", app?.id] });
    }
    if (actionData?.error) {
      toast.error(actionData.error);
    }
  }, [actionData, app?.id, queryClient]);

  // Determine active tab from path
  const basePath = `/apps/${params.id}`;
  const currentPath = location.pathname;
  const activeTab = tabs.find((tab) => {
    if (tab.path === "") {
      return currentPath === basePath || currentPath === basePath + "/";
    }
    return currentPath.startsWith(basePath + tab.path);
  })?.id || "general";

  if (!app) {
    return (
      <div className="space-y-6">
        <h1 className="text-3xl font-bold">Application Not Found</h1>
        <p className="text-muted-foreground">
          The application you're looking for doesn't exist or has been deleted.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-3">
            <h1 className="text-3xl font-bold">{app.name}</h1>
            <EnvironmentBadge environment={app.environment} />
            {hasActiveDeployment && (
              <span className="flex items-center gap-1.5 text-sm font-normal text-blue-600">
                <span className="relative flex h-2 w-2">
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-blue-400 opacity-75"></span>
                  <span className="relative inline-flex h-2 w-2 rounded-full bg-blue-500"></span>
                </span>
                Deploying
              </span>
            )}
          </div>
          <p className="text-muted-foreground">{app.git_url}</p>
        </div>
        <div className="flex gap-2">
          <Form method="post">
            <input type="hidden" name="intent" value="deploy" />
            <Button type="submit" disabled={isSubmitting || hasActiveDeployment}>
              {isSubmitting ? "Deploying..." : "Deploy"}
            </Button>
          </Form>
          {runningDeployment && app.domain && (
            <Button variant="outline" asChild>
              <a href={`https://${app.domain}`} target="_blank" rel="noopener noreferrer">
                Open Site
              </a>
            </Button>
          )}
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
      <Outlet context={{ app, deployments, token: loaderData.token }} />
    </div>
  );
}
