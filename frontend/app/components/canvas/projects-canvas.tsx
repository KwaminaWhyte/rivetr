import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Background,
  BackgroundVariant,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  useStore,
  type Connection,
  type Node,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import {
  ExternalLink,
  List,
  Maximize,
  Minus,
  Plus,
  RotateCcw,
  FolderKanban,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { EmptyState, ResourceStatus } from "@/components/page";
import { QueryError } from "@/components/query-error";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import { useServers } from "@/lib/resource-queries";
import { useTeamContext } from "@/lib/team-context";
import { cn } from "@/lib/utils";
import { KIND_ICON, KIND_LABEL, nodeTypes, type ResourceNodeData } from "./canvas-nodes";
import { layoutCanvas, layoutEdges } from "./layout";
import {
  SHARED_ENV_ID,
  useCanvasData,
  type CanvasProject,
  type CanvasResource,
} from "./use-canvas-data";

const ALL = "__all__";

function ZoomLabel() {
  const zoom = useStore((s) => s.transform[2]);
  return <span className="w-11 text-center text-xs tabular-nums text-muted-foreground">{Math.round(zoom * 100)}%</span>;
}

function Toolbar({
  projects,
  projectFilter,
  envFilter,
  envNames,
  onProjectFilter,
  onEnvFilter,
  onListView,
}: {
  projects: CanvasProject[];
  projectFilter: string;
  envFilter: string;
  envNames: string[];
  onProjectFilter: (v: string) => void;
  onEnvFilter: (v: string) => void;
  onListView?: () => void;
}) {
  const { zoomIn, zoomOut, fitView, setViewport } = useReactFlow();
  return (
    <div className="pointer-events-none absolute inset-x-0 bottom-3 z-10 flex justify-center px-3">
      <div className="pointer-events-auto flex max-w-full flex-wrap items-center justify-center gap-1 rounded-xl border bg-card/95 p-1 shadow-lg backdrop-blur">
        <div className="flex items-center">
          <Button variant="ghost" size="icon-sm" aria-label="Zoom out" onClick={() => zoomOut({ duration: 150 })}>
            <Minus />
          </Button>
          <ZoomLabel />
          <Button variant="ghost" size="icon-sm" aria-label="Zoom in" onClick={() => zoomIn({ duration: 150 })}>
            <Plus />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label="Fit to screen"
            title="Fit to screen"
            onClick={() => fitView({ padding: 0.15, duration: 250 })}
          >
            <Maximize />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label="Reset zoom"
            title="Reset to 100%"
            onClick={() => setViewport({ x: 24, y: 24, zoom: 1 }, { duration: 250 })}
          >
            <RotateCcw />
          </Button>
        </div>
        <span className="mx-0.5 hidden h-5 w-px bg-border sm:block" aria-hidden="true" />
        <Select value={projectFilter} onValueChange={onProjectFilter}>
          <SelectTrigger size="sm" className="w-36 border-0 bg-transparent" aria-label="Filter projects">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={ALL}>All projects</SelectItem>
            {projects.map((p) => (
              <SelectItem key={p.id} value={p.id}>
                {p.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select value={envFilter} onValueChange={onEnvFilter}>
          <SelectTrigger size="sm" className="w-40 border-0 bg-transparent" aria-label="Filter environments">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={ALL}>All environments</SelectItem>
            {envNames.map((n) => (
              <SelectItem key={n} value={n}>
                {n}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <span className="mx-0.5 hidden h-5 w-px bg-border sm:block" aria-hidden="true" />
        {onListView && (
          <Button variant="ghost" size="sm" onClick={onListView}>
            <List />
            List view
          </Button>
        )}
        <Button size="sm" asChild>
          <Link to="/projects?new=1">
            <Plus />
            New Project
          </Link>
        </Button>
      </div>
    </div>
  );
}

function DetailRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-4 py-2.5 text-sm">
      <span className="shrink-0 text-muted-foreground">{label}</span>
      <span className="min-w-0 text-right">{children}</span>
    </div>
  );
}

function ResourceDrawer({
  resource,
  project,
  envName,
  onClose,
}: {
  resource: CanvasResource | null;
  project?: CanvasProject;
  envName?: string;
  onClose: () => void;
}) {
  const { data: servers = [] } = useServers(!!resource);
  const Icon = resource ? KIND_ICON[resource.kind] : FolderKanban;
  const server = resource?.serverId ? servers.find((s) => s.id === resource.serverId) : null;

  return (
    <Sheet open={!!resource} onOpenChange={(open) => !open && onClose()}>
      <SheetContent side="right" className="w-full gap-0 bg-card sm:max-w-md">
        {resource && (
          <>
            <SheetHeader className="border-b p-5">
              <div className="flex items-center gap-3 pr-6">
                <span className="inline-flex size-10 shrink-0 items-center justify-center rounded-lg border bg-muted/50 text-muted-foreground">
                  <Icon className="size-5" />
                </span>
                <div className="min-w-0">
                  <SheetTitle className="truncate text-base">{resource.name}</SheetTitle>
                  <SheetDescription className="flex items-center gap-2">
                    {KIND_LABEL[resource.kind]}
                    <ResourceStatus status={resource.status} size="sm" />
                  </SheetDescription>
                </div>
              </div>
            </SheetHeader>
            <div className="flex-1 overflow-y-auto p-5">
              <h3 className="text-xs font-medium text-muted-foreground">Details</h3>
              <div className="mt-1 divide-y">
                <DetailRow label="Source">{resource.source}</DetailRow>
                <DetailRow label="URL">
                  {resource.url ? (
                    <a
                      href={resource.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="inline-flex max-w-full items-center gap-1 text-primary hover:underline"
                    >
                      <span className="truncate">{resource.url.replace(/^https?:\/\//, "")}</span>
                      <ExternalLink className="size-3.5 shrink-0" />
                    </a>
                  ) : (
                    <span className="text-muted-foreground">No domain</span>
                  )}
                </DetailRow>
                <DetailRow label="Server">
                  {server ? (
                    <Link to={`/servers/${server.id}`} className="hover:underline">
                      {server.name}
                    </Link>
                  ) : (
                    "localhost"
                  )}
                </DetailRow>
                {project && (
                  <DetailRow label="Project">
                    {project.real ? (
                      <Link to={`/projects/${project.id}`} className="hover:underline">
                        {project.name}
                      </Link>
                    ) : (
                      project.name
                    )}
                  </DetailRow>
                )}
                {envName && <DetailRow label="Environment">{envName}</DetailRow>}
              </div>
              {resource.kind === "app" && (
                <div className="mt-5 rounded-lg border border-dashed p-3 text-xs text-muted-foreground">
                  <span className="font-medium text-foreground">Links.</span> Drag the dot on the right edge
                  of an app card onto another card to record a dependency.
                </div>
              )}
            </div>
            <div className="border-t p-4">
              <Button asChild size="lg" className="w-full">
                <Link to={resource.href}>Open</Link>
              </Button>
            </div>
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}

function CanvasInner({
  className,
  onListView,
}: {
  className?: string;
  onListView?: () => void;
}) {
  const { currentTeamId } = useTeamContext();
  const queryClient = useQueryClient();
  const { projects, edges: links, isLoading, error, refetch } = useCanvasData(currentTeamId);
  const { fitView } = useReactFlow();
  const [projectFilter, setProjectFilter] = useState(ALL);
  const [envFilter, setEnvFilter] = useState(ALL);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  // Wrap rows to the container so narrow screens stack projects vertically.
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(1200);
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => setContainerWidth(entry.contentRect.width));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const rowMaxWidth = Math.max(700, Math.round(containerWidth * 1.6));

  const envNames = useMemo(() => {
    const names = new Set<string>();
    for (const p of projects) for (const e of p.environments) if (e.id !== SHARED_ENV_ID && p.real) names.add(e.name);
    return [...names].sort();
  }, [projects]);

  const visibleProjects = useMemo(
    () =>
      projects
        .filter((p) => projectFilter === ALL || p.id === projectFilter)
        .map((p) =>
          envFilter === ALL
            ? p
            : { ...p, environments: p.environments.filter((e) => e.name === envFilter) }
        )
        .filter((p) => envFilter === ALL || p.environments.length > 0),
    [projects, projectFilter, envFilter]
  );

  const { nodes, ids } = useMemo(
    () => layoutCanvas(visibleProjects, selectedId, rowMaxWidth),
    [visibleProjects, selectedId, rowMaxWidth]
  );
  const edges = useMemo(() => layoutEdges(links, ids), [links, ids]);

  // Refit when the filters change the set of projects on screen.
  const layoutKey = visibleProjects
    .map((p) => `${p.id}:${p.environments.map((e) => e.resources.length).join(",")}`)
    .join("|");
  useEffect(() => {
    if (!layoutKey) return;
    const t = setTimeout(() => fitView({ padding: 0.15, maxZoom: 1, duration: 200 }), 150);
    return () => clearTimeout(t);
  }, [layoutKey, fitView]);

  const lookup = useMemo(() => {
    const map = new Map<string, { resource: CanvasResource; project: CanvasProject; env: string }>();
    for (const p of projects) for (const e of p.environments) for (const r of e.resources) map.set(r.id, { resource: r, project: p, env: e.name });
    return map;
  }, [projects]);
  const selected = selectedId ? lookup.get(selectedId) : undefined;

  const onNodeClick = useCallback((_: unknown, node: Node) => {
    if (node.type === "resource") setSelectedId((node.data as ResourceNodeData).resource.id);
  }, []);

  const onConnect = useCallback(
    async (conn: Connection) => {
      const from = lookup.get(conn.source);
      const to = lookup.get(conn.target);
      if (!from || !to || from.resource.kind !== "app" || conn.source === conn.target) return;
      const body =
        to.resource.kind === "app"
          ? { depends_on_app_id: to.resource.id }
          : to.resource.kind === "database"
            ? { depends_on_database_id: to.resource.id }
            : { depends_on_service_id: to.resource.id };
      try {
        await api.addDependency(from.resource.id, body);
        toast.success(`${from.resource.name} now depends on ${to.resource.name}`);
        queryClient.invalidateQueries({ queryKey: ["dependency-graph"] });
      } catch (err) {
        toast.error(err instanceof Error ? err.message : "Could not link resources");
      }
    },
    [lookup, queryClient]
  );

  return (
    <div
      ref={containerRef}
      className={cn(
        "rivetr-canvas relative overflow-hidden rounded-xl border bg-background shadow-xs",
        className
      )}
    >
      {isLoading ? (
        <div className="grid h-full grid-cols-1 gap-4 p-6 md:grid-cols-2">
          <Skeleton className="h-56 rounded-2xl" />
          <Skeleton className="hidden h-56 rounded-2xl md:block" />
        </div>
      ) : error && projects.length === 0 ? (
        <div className="p-6">
          <QueryError what="projects" error={error} onRetry={() => refetch()} />
        </div>
      ) : projects.length === 0 ? (
        <EmptyState
          className="h-full"
          icon={<FolderKanban />}
          title="No projects yet"
          description="Create a project to group your apps, databases and services."
          action={
            <Button asChild>
              <Link to="/projects?new=1">
                <Plus />
                New Project
              </Link>
            </Button>
          }
        />
      ) : (
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          onNodeClick={onNodeClick}
          onConnect={onConnect}
          nodesDraggable={false}
          nodesConnectable
          elementsSelectable
          fitView
          fitViewOptions={{ padding: 0.15, maxZoom: 1 }}
          minZoom={0.2}
          maxZoom={1.75}
          panOnScroll
          zoomOnPinch
        >
          <Background variant={BackgroundVariant.Dots} gap={18} size={1.2} color="var(--border)" className="!bg-background" />
        </ReactFlow>
      )}
      {!isLoading && projects.length > 0 && (
        <Toolbar
          projects={projects.filter((p) => p.real)}
          projectFilter={projectFilter}
          envFilter={envFilter}
          envNames={envNames}
          onProjectFilter={setProjectFilter}
          onEnvFilter={setEnvFilter}
          onListView={onListView}
        />
      )}
      <ResourceDrawer
        resource={selected?.resource ?? null}
        project={selected?.project}
        envName={selected?.env}
        onClose={() => setSelectedId(null)}
      />
    </div>
  );
}

/**
 * Projects canvas: project boxes containing environment boxes containing
 * resource cards, with pan/zoom, filters and a details drawer.
 */
export function ProjectsCanvas(props: { className?: string; onListView?: () => void }) {
  return (
    <ReactFlowProvider>
      <CanvasInner {...props} />
    </ReactFlowProvider>
  );
}
