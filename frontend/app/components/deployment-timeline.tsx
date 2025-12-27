import { useState } from "react";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  CheckCircle2,
  XCircle,
  Clock,
  Loader2,
  GitBranch,
  GitCommit,
  ChevronDown,
  FileText,
  RotateCcw,
  AlertCircle,
} from "lucide-react";
import type { Deployment, DeploymentStatus } from "@/types/api";

interface DeploymentTimelineProps {
  deployments: Deployment[];
  onViewLogs: (deploymentId: string) => void;
  onRollback: (deploymentId: string) => void;
  canRollback: (deployment: Deployment) => boolean;
  branch: string;
}

// Status configuration with colors and icons
const statusConfig: Record<
  DeploymentStatus,
  {
    color: string;
    bgColor: string;
    borderColor: string;
    icon: React.ComponentType<{ className?: string }>;
    label: string;
  }
> = {
  pending: {
    color: "text-yellow-600",
    bgColor: "bg-yellow-100 dark:bg-yellow-900/30",
    borderColor: "border-yellow-400",
    icon: Clock,
    label: "Pending",
  },
  cloning: {
    color: "text-blue-600",
    bgColor: "bg-blue-100 dark:bg-blue-900/30",
    borderColor: "border-blue-400",
    icon: Loader2,
    label: "Cloning",
  },
  building: {
    color: "text-blue-600",
    bgColor: "bg-blue-100 dark:bg-blue-900/30",
    borderColor: "border-blue-400",
    icon: Loader2,
    label: "Building",
  },
  starting: {
    color: "text-blue-600",
    bgColor: "bg-blue-100 dark:bg-blue-900/30",
    borderColor: "border-blue-400",
    icon: Loader2,
    label: "Starting",
  },
  checking: {
    color: "text-blue-600",
    bgColor: "bg-blue-100 dark:bg-blue-900/30",
    borderColor: "border-blue-400",
    icon: Loader2,
    label: "Health Check",
  },
  running: {
    color: "text-green-600",
    bgColor: "bg-green-100 dark:bg-green-900/30",
    borderColor: "border-green-500",
    icon: CheckCircle2,
    label: "Running",
  },
  failed: {
    color: "text-red-600",
    bgColor: "bg-red-100 dark:bg-red-900/30",
    borderColor: "border-red-500",
    icon: XCircle,
    label: "Failed",
  },
  stopped: {
    color: "text-gray-500",
    bgColor: "bg-gray-100 dark:bg-gray-800/50",
    borderColor: "border-gray-400",
    icon: CheckCircle2,
    label: "Stopped",
  },
  replaced: {
    color: "text-slate-500",
    bgColor: "bg-slate-100 dark:bg-slate-800/50",
    borderColor: "border-slate-400",
    icon: CheckCircle2,
    label: "Replaced",
  },
};

// Active statuses that show animation
const activeStatuses: DeploymentStatus[] = [
  "pending",
  "cloning",
  "building",
  "starting",
  "checking",
];

function isActiveStatus(status: DeploymentStatus): boolean {
  return activeStatuses.includes(status);
}

// Calculate duration between two timestamps
function calculateDuration(startedAt: string, finishedAt: string | null): string {
  const start = new Date(startedAt).getTime();
  const end = finishedAt ? new Date(finishedAt).getTime() : Date.now();
  const durationMs = end - start;

  if (durationMs < 1000) {
    return "< 1s";
  }

  const seconds = Math.floor(durationMs / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`;
  }
  if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`;
  }
  return `${seconds}s`;
}

// Format date for display
function formatDate(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const isToday = date.toDateString() === now.toDateString();

  if (isToday) {
    return `Today at ${date.toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
    })}`;
  }

  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (date.toDateString() === yesterday.toDateString()) {
    return `Yesterday at ${date.toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
    })}`;
  }

  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    year: date.getFullYear() !== now.getFullYear() ? "numeric" : undefined,
    hour: "2-digit",
    minute: "2-digit",
  });
}

// Individual timeline item component
function TimelineItem({
  deployment,
  isFirst,
  isLast,
  onViewLogs,
  onRollback,
  canRollback,
  branch,
}: {
  deployment: Deployment;
  isFirst: boolean;
  isLast: boolean;
  onViewLogs: (deploymentId: string) => void;
  onRollback: (deploymentId: string) => void;
  canRollback: boolean;
  branch: string;
}) {
  const [isOpen, setIsOpen] = useState(false);
  const config = statusConfig[deployment.status];
  const StatusIcon = config.icon;
  const isActive = isActiveStatus(deployment.status);
  const duration = calculateDuration(deployment.started_at, deployment.finished_at);

  return (
    <div className="relative flex gap-4">
      {/* Timeline line */}
      <div className="flex flex-col items-center">
        {/* Top connector */}
        {!isFirst && (
          <div className="w-0.5 h-4 bg-border" />
        )}

        {/* Status icon container */}
        <div
          className={cn(
            "relative z-10 flex items-center justify-center w-10 h-10 rounded-full border-2 transition-all duration-300",
            config.bgColor,
            config.borderColor,
            isActive && "ring-4 ring-blue-200 dark:ring-blue-900/50"
          )}
        >
          <StatusIcon
            className={cn(
              "w-5 h-5",
              config.color,
              isActive && "animate-spin"
            )}
          />
          {/* Active pulse effect */}
          {isActive && (
            <span className="absolute inset-0 rounded-full animate-ping opacity-30 bg-blue-400" />
          )}
        </div>

        {/* Bottom connector */}
        {!isLast && (
          <div className="w-0.5 flex-1 min-h-[2rem] bg-border" />
        )}
      </div>

      {/* Content */}
      <div className="flex-1 pb-6">
        <Collapsible open={isOpen} onOpenChange={setIsOpen}>
          <div
            className={cn(
              "rounded-lg border p-4 transition-all duration-200",
              "bg-card hover:shadow-md",
              isFirst && "border-2",
              isFirst && config.borderColor
            )}
          >
            {/* Header */}
            <div className="flex items-start justify-between gap-4">
              <div className="flex-1 min-w-0">
                {/* Status and timestamp */}
                <div className="flex items-center gap-2 flex-wrap">
                  <Badge
                    className={cn(
                      "font-medium",
                      deployment.status === "running" && "bg-green-500 text-white",
                      deployment.status === "failed" && "bg-red-500 text-white",
                      isActive && "bg-blue-500 text-white",
                      deployment.status === "stopped" && "bg-gray-500 text-white",
                      deployment.status === "pending" && "bg-yellow-500 text-white",
                      deployment.status === "replaced" && "bg-slate-400 text-white"
                    )}
                  >
                    {config.label}
                  </Badge>
                  <span className="text-sm text-muted-foreground">
                    {formatDate(deployment.started_at)}
                  </span>
                  {(deployment.finished_at || isActive) && (
                    <span className="text-sm text-muted-foreground flex items-center gap-1">
                      <Clock className="w-3 h-3" />
                      {duration}
                      {isActive && " (running)"}
                    </span>
                  )}
                </div>

                {/* Commit info */}
                {(deployment.commit_sha || deployment.commit_message) && (
                  <div className="mt-2 space-y-1">
                    {deployment.commit_message && (
                      <p className="text-sm font-medium text-foreground line-clamp-2">
                        {deployment.commit_message}
                      </p>
                    )}
                    <div className="flex items-center gap-3 text-xs text-muted-foreground">
                      {deployment.commit_sha && (
                        <span className="flex items-center gap-1 font-mono">
                          <GitCommit className="w-3 h-3" />
                          {deployment.commit_sha.slice(0, 7)}
                        </span>
                      )}
                      <span className="flex items-center gap-1">
                        <GitBranch className="w-3 h-3" />
                        {branch}
                      </span>
                    </div>
                  </div>
                )}

                {/* Error message - always show full error for failed deployments */}
                {deployment.status === "failed" && deployment.error_message && (
                  <div className="mt-3 p-3 rounded-md bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800">
                    <div className="flex items-start gap-2 text-sm text-red-700 dark:text-red-300">
                      <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                      <div>
                        <span className="font-medium">Error: </span>
                        <span className="break-words">{deployment.error_message}</span>
                      </div>
                    </div>
                  </div>
                )}
              </div>

              {/* Actions */}
              <div className="flex items-center gap-1 flex-shrink-0">
                <TooltipProvider>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => onViewLogs(deployment.id)}
                      >
                        <FileText className="w-4 h-4" />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>View build logs</TooltipContent>
                  </Tooltip>
                </TooltipProvider>

                {canRollback && (
                  <TooltipProvider>
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => onRollback(deployment.id)}
                        >
                          <RotateCcw className="w-4 h-4" />
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>Rollback to this deployment</TooltipContent>
                    </Tooltip>
                  </TooltipProvider>
                )}

                <CollapsibleTrigger asChild>
                  <Button variant="ghost" size="sm">
                    <ChevronDown
                      className={cn(
                        "w-4 h-4 transition-transform duration-200",
                        isOpen && "rotate-180"
                      )}
                    />
                  </Button>
                </CollapsibleTrigger>
              </div>
            </div>

            {/* Expandable details */}
            <CollapsibleContent>
              <div className="mt-4 pt-4 border-t space-y-3">
                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <span className="text-muted-foreground">Deployment ID</span>
                    <p className="font-mono text-xs mt-1">{deployment.id}</p>
                  </div>
                  {deployment.container_id && (
                    <div>
                      <span className="text-muted-foreground">Container ID</span>
                      <p className="font-mono text-xs mt-1">
                        {deployment.container_id.slice(0, 12)}
                      </p>
                    </div>
                  )}
                  <div>
                    <span className="text-muted-foreground">Started</span>
                    <p className="text-xs mt-1">
                      {new Date(deployment.started_at).toLocaleString()}
                    </p>
                  </div>
                  {deployment.finished_at && (
                    <div>
                      <span className="text-muted-foreground">Finished</span>
                      <p className="text-xs mt-1">
                        {new Date(deployment.finished_at).toLocaleString()}
                      </p>
                    </div>
                  )}
                </div>

                {deployment.error_message && (
                  <div className="mt-3">
                    <span className="text-muted-foreground text-sm">Error Details</span>
                    <pre className="mt-1 p-3 bg-red-50 dark:bg-red-900/20 rounded-lg text-xs text-red-700 dark:text-red-300 overflow-x-auto whitespace-pre-wrap">
                      {deployment.error_message}
                    </pre>
                  </div>
                )}

                <div className="flex gap-2 mt-4">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => onViewLogs(deployment.id)}
                    className="flex items-center gap-2"
                  >
                    <FileText className="w-4 h-4" />
                    View Full Logs
                  </Button>
                </div>
              </div>
            </CollapsibleContent>
          </div>
        </Collapsible>
      </div>
    </div>
  );
}

export function DeploymentTimeline({
  deployments,
  onViewLogs,
  onRollback,
  canRollback,
  branch,
}: DeploymentTimelineProps) {
  if (deployments.length === 0) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        <Clock className="w-12 h-12 mx-auto mb-4 opacity-50" />
        <p>No deployments yet.</p>
        <p className="text-sm mt-1">Click Deploy to start your first deployment.</p>
      </div>
    );
  }

  return (
    <div className="relative">
      {deployments.map((deployment, index) => (
        <TimelineItem
          key={deployment.id}
          deployment={deployment}
          isFirst={index === 0}
          isLast={index === deployments.length - 1}
          onViewLogs={onViewLogs}
          onRollback={onRollback}
          canRollback={canRollback(deployment)}
          branch={branch}
        />
      ))}
    </div>
  );
}
