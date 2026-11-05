import { memo } from "react";
import { Link } from "react-router";
import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";
import { Box, Database, Globe, Layers, Plus, Settings } from "lucide-react";
import { ResourceStatus } from "@/components/page";
import { cn } from "@/lib/utils";
import type { CanvasKind, CanvasProject, CanvasResource } from "./use-canvas-data";

export const KIND_ICON: Record<CanvasKind, typeof Box> = {
  app: Box,
  database: Database,
  service: Layers,
};

export const KIND_LABEL: Record<CanvasKind, string> = {
  app: "Application",
  database: "Database",
  service: "Service",
};

export type ProjectNodeData = { project: CanvasProject };
export type EnvNodeData = { name: string; isDefault: boolean; empty: boolean; shared: boolean };
export type ResourceNodeData = { resource: CanvasResource; selected: boolean };

export type ProjectNode = Node<ProjectNodeData, "project">;
export type EnvNode = Node<EnvNodeData, "environment">;
export type ResourceNode = Node<ResourceNodeData, "resource">;

function ProjectNodeView({ data }: NodeProps<ProjectNode>) {
  const { project } = data;
  const envCount = project.environments.filter((e) => e.id !== "__shared__").length;
  return (
    <div className="h-full w-full rounded-2xl border border-dashed border-foreground/15 bg-card/40 p-4 backdrop-blur-[1px]">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          {project.real ? (
            <Link
              to={`/projects/${project.id}`}
              className="nodrag nopan block truncate text-[15px] font-semibold tracking-tight hover:underline"
            >
              {project.name}
            </Link>
          ) : (
            <span className="block truncate text-[15px] font-semibold tracking-tight text-muted-foreground">
              {project.name}
            </span>
          )}
          <p className="truncate text-xs text-muted-foreground">
            {project.real
              ? `${project.description ? `${project.description} · ` : ""}${envCount} environment${envCount === 1 ? "" : "s"} · ${project.resourceCount} resource${project.resourceCount === 1 ? "" : "s"}`
              : project.description}
          </p>
        </div>
        {project.real && (
          <div className="nodrag nopan flex shrink-0 items-center gap-1">
            <Link
              to={`/projects/${project.id}/apps/new`}
              title="Add a resource"
              aria-label={`Add a resource to ${project.name}`}
              className="inline-flex size-7 items-center justify-center rounded-md border bg-card text-muted-foreground shadow-xs hover:text-foreground"
            >
              <Plus className="size-3.5" />
            </Link>
            <Link
              to={`/projects/${project.id}`}
              title="Project settings"
              aria-label={`Open ${project.name}`}
              className="inline-flex size-7 items-center justify-center rounded-md border bg-card text-muted-foreground shadow-xs hover:text-foreground"
            >
              <Settings className="size-3.5" />
            </Link>
          </div>
        )}
      </div>
    </div>
  );
}

function EnvNodeView({ data }: NodeProps<EnvNode>) {
  return (
    <div className="h-full w-full rounded-xl border bg-frame/80 px-3.5 pt-2">
      <div className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
        <span className={cn("size-1.5 rounded-full", data.shared ? "bg-muted-foreground/50" : "bg-primary/70")} />
        {data.name}
        {data.isDefault && <span className="font-normal opacity-70">default</span>}
      </div>
      {data.empty && (
        <p className="mt-3 text-center text-xs text-muted-foreground/80">Nothing deployed here yet</p>
      )}
    </div>
  );
}

function ResourceNodeView({ data }: NodeProps<ResourceNode>) {
  const { resource, selected } = data;
  const Icon = KIND_ICON[resource.kind];
  return (
    <div
      className={cn(
        "group relative h-full w-full cursor-pointer rounded-xl border bg-card p-3 shadow-xs transition-shadow hover:shadow-md",
        selected && "ring-2 ring-primary/60"
      )}
    >
      <Handle type="target" position={Position.Left} className="!-left-[6px] opacity-0 group-hover:opacity-100" />
      {resource.kind === "app" && (
        <Handle type="source" position={Position.Right} className="!-right-[6px]" />
      )}
      <div className="flex items-start gap-2.5">
        <span className="inline-flex size-8 shrink-0 items-center justify-center rounded-lg border bg-muted/50 text-muted-foreground">
          <Icon className="size-4" />
        </span>
        <div className="min-w-0 flex-1">
          <div className="truncate text-sm font-medium leading-5">{resource.name}</div>
          <div className="truncate text-[11px] text-muted-foreground">{resource.source}</div>
        </div>
      </div>
      <div className="mt-2.5 flex items-center justify-between gap-2">
        <span className="flex min-w-0 items-center gap-1 text-[11px] text-muted-foreground">
          {resource.url ? (
            <>
              <Globe className="size-3 shrink-0" />
              <span className="truncate">{resource.url.replace(/^https?:\/\//, "")}</span>
            </>
          ) : (
            <span className="opacity-70">{KIND_LABEL[resource.kind]}</span>
          )}
        </span>
        <ResourceStatus status={resource.status} size="sm" />
      </div>
    </div>
  );
}

export const nodeTypes = {
  project: memo(ProjectNodeView),
  environment: memo(EnvNodeView),
  resource: memo(ResourceNodeView),
};
