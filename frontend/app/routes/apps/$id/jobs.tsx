import { useState } from "react";
import { useParams } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
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
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { jobsApi } from "@/lib/api/jobs";
import type {
  ScheduledJob,
  ScheduledJobRun,
  CreateScheduledJobRequest,
  UpdateScheduledJobRequest,
} from "@/types/api";
import {
  Plus,
  Play,
  Pencil,
  Trash2,
  MoreHorizontal,
  Clock,
  ChevronDown,
  ChevronRight,
  CheckCircle2,
  XCircle,
  Loader2,
  Calendar,
} from "lucide-react";

/** Common cron presets for the create/edit dialog.
 * Format: sec min hour dom month dow (6 fields required by cron crate)
 */
const CRON_PRESETS = [
  { label: "Every minute", value: "0 * * * * *" },
  { label: "Every 5 minutes", value: "0 */5 * * * *" },
  { label: "Every 15 minutes", value: "0 */15 * * * *" },
  { label: "Every hour", value: "0 0 * * * *" },
  { label: "Every 6 hours", value: "0 0 */6 * * *" },
  { label: "Daily at midnight", value: "0 0 0 * * *" },
  { label: "Daily at 3 AM", value: "0 0 3 * * *" },
  { label: "Weekly (Sunday midnight)", value: "0 0 0 * * 0" },
  { label: "Monthly (1st at midnight)", value: "0 0 0 1 * *" },
];

function formatDuration(ms: number | null): string {
  if (ms === null || ms === undefined) return "-";
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  const minutes = Math.floor(ms / 60000);
  const seconds = Math.floor((ms % 60000) / 1000);
  return `${minutes}m ${seconds}s`;
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "Never";
  try {
    const date = new Date(dateStr);
    return date.toLocaleString();
  } catch {
    return dateStr;
  }
}

function RunStatusBadge({ status }: { status: string }) {
  switch (status) {
    case "success":
      return (
        <Badge className="bg-green-500 text-white gap-1">
          <CheckCircle2 className="h-3 w-3" />
          Success
        </Badge>
      );
    case "failed":
      return (
        <Badge variant="destructive" className="gap-1">
          <XCircle className="h-3 w-3" />
          Failed
        </Badge>
      );
    case "running":
      return (
        <Badge variant="secondary" className="gap-1">
          <Loader2 className="h-3 w-3 animate-spin" />
          Running
        </Badge>
      );
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
}

function JobRunHistory({ appId, jobId }: { appId: string; jobId: string }) {
  const { data: runs, isLoading } = useQuery<ScheduledJobRun[]>({
    queryKey: ["jobRuns", appId, jobId],
    queryFn: () => jobsApi.getJobRuns(appId, jobId, { limit: 20 }),
    refetchInterval: 10000,
  });

  if (isLoading) {
    return <p className="text-sm text-muted-foreground py-2">Loading run history...</p>;
  }

  if (!runs || runs.length === 0) {
    return <p className="text-sm text-muted-foreground py-2">No runs yet</p>;
  }

  return (
    <div className="border rounded-md mt-2">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Status</TableHead>
            <TableHead>Started</TableHead>
            <TableHead>Duration</TableHead>
            <TableHead>Output</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {runs.map((run) => (
            <TableRow key={run.id}>
              <TableCell>
                <RunStatusBadge status={run.status} />
              </TableCell>
              <TableCell className="text-sm">{formatDate(run.started_at)}</TableCell>
              <TableCell className="text-sm">{formatDuration(run.duration_ms)}</TableCell>
              <TableCell className="max-w-md">
                {run.error_message && (
                  <p className="text-sm text-destructive truncate" title={run.error_message}>
                    {run.error_message}
                  </p>
                )}
                {run.output && (
                  <details className="text-sm">
                    <summary className="cursor-pointer text-muted-foreground hover:text-foreground">
                      View output
                    </summary>
                    <pre className="mt-1 p-2 bg-muted rounded text-xs max-h-40 overflow-auto whitespace-pre-wrap">
                      {run.output}
                    </pre>
                  </details>
                )}
                {!run.error_message && !run.output && run.status !== "running" && (
                  <span className="text-sm text-muted-foreground">No output</span>
                )}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

export default function JobsPage() {
  const { id: appId } = useParams();
  const queryClient = useQueryClient();

  // Dialog state
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [editingJob, setEditingJob] = useState<ScheduledJob | null>(null);
  const [deleteConfirmJob, setDeleteConfirmJob] = useState<ScheduledJob | null>(null);

  // Form state
  const [formName, setFormName] = useState("");
  const [formCommand, setFormCommand] = useState("");
  const [formCron, setFormCron] = useState("0 0 * * * *");
  const [formEnabled, setFormEnabled] = useState(true);

  // Expanded job runs
  const [expandedJobs, setExpandedJobs] = useState<Set<string>>(new Set());

  // Fetch jobs
  const { data: jobs, isLoading } = useQuery<ScheduledJob[]>({
    queryKey: ["jobs", appId],
    queryFn: () => jobsApi.getJobs(appId!),
    enabled: !!appId,
    refetchInterval: 30000,
  });

  // Mutations
  const createMutation = useMutation({
    mutationFn: (data: CreateScheduledJobRequest) => jobsApi.createJob(appId!, data),
    onSuccess: () => {
      toast.success("Scheduled job created");
      queryClient.invalidateQueries({ queryKey: ["jobs", appId] });
      setShowCreateDialog(false);
      resetForm();
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to create job");
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ jobId, data }: { jobId: string; data: UpdateScheduledJobRequest }) =>
      jobsApi.updateJob(appId!, jobId, data),
    onSuccess: () => {
      toast.success("Scheduled job updated");
      queryClient.invalidateQueries({ queryKey: ["jobs", appId] });
      setEditingJob(null);
      resetForm();
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to update job");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (jobId: string) => jobsApi.deleteJob(appId!, jobId),
    onSuccess: () => {
      toast.success("Scheduled job deleted");
      queryClient.invalidateQueries({ queryKey: ["jobs", appId] });
      setDeleteConfirmJob(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to delete job");
    },
  });

  const triggerMutation = useMutation({
    mutationFn: (jobId: string) => jobsApi.triggerJobRun(appId!, jobId),
    onSuccess: (_, jobId) => {
      toast.success("Job run triggered");
      queryClient.invalidateQueries({ queryKey: ["jobRuns", appId, jobId] });
      queryClient.invalidateQueries({ queryKey: ["jobs", appId] });
      // Auto-expand the job to show the run
      setExpandedJobs((prev) => new Set([...prev, jobId]));
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to trigger job run");
    },
  });

  const toggleMutation = useMutation({
    mutationFn: ({ jobId, enabled }: { jobId: string; enabled: boolean }) =>
      jobsApi.updateJob(appId!, jobId, { enabled }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["jobs", appId] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to toggle job");
    },
  });

  function resetForm() {
    setFormName("");
    setFormCommand("");
    setFormCron("0 0 * * * *");
    setFormEnabled(true);
  }

  function openEditDialog(job: ScheduledJob) {
    setFormName(job.name);
    setFormCommand(job.command);
    setFormCron(job.cron_expression);
    setFormEnabled(job.enabled);
    setEditingJob(job);
  }

  function handleCreate() {
    createMutation.mutate({
      name: formName,
      command: formCommand,
      cron_expression: formCron,
      enabled: formEnabled,
    });
  }

  function handleUpdate() {
    if (!editingJob) return;
    updateMutation.mutate({
      jobId: editingJob.id,
      data: {
        name: formName,
        command: formCommand,
        cron_expression: formCron,
        enabled: formEnabled,
      },
    });
  }

  function toggleExpanded(jobId: string) {
    setExpandedJobs((prev) => {
      const next = new Set(prev);
      if (next.has(jobId)) {
        next.delete(jobId);
      } else {
        next.add(jobId);
      }
      return next;
    });
  }

  if (isLoading) {
    return (
      <div className="space-y-4">
        <div className="animate-pulse">
          <div className="h-8 bg-muted rounded w-1/3 mb-4"></div>
          <div className="h-32 bg-muted rounded"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold">Scheduled Jobs</h2>
          <p className="text-sm text-muted-foreground">
            Run commands inside your app&apos;s container on a cron schedule
          </p>
        </div>
        <Button
          onClick={() => {
            resetForm();
            setShowCreateDialog(true);
          }}
          className="gap-2"
        >
          <Plus className="h-4 w-4" />
          New Job
        </Button>
      </div>

      {/* Jobs list */}
      {!jobs || jobs.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12">
            <Calendar className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-medium mb-1">No scheduled jobs</h3>
            <p className="text-sm text-muted-foreground mb-4">
              Create a scheduled job to run commands in your app&apos;s container automatically.
            </p>
            <Button
              onClick={() => {
                resetForm();
                setShowCreateDialog(true);
              }}
              className="gap-2"
            >
              <Plus className="h-4 w-4" />
              Create your first job
            </Button>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-3">
          {jobs.map((job) => (
            <Collapsible
              key={job.id}
              open={expandedJobs.has(job.id)}
              onOpenChange={() => toggleExpanded(job.id)}
            >
              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <CollapsibleTrigger className="flex items-center gap-1 hover:text-foreground text-muted-foreground">
                        {expandedJobs.has(job.id) ? (
                          <ChevronDown className="h-4 w-4" />
                        ) : (
                          <ChevronRight className="h-4 w-4" />
                        )}
                      </CollapsibleTrigger>
                      <div>
                        <CardTitle className="text-base">{job.name}</CardTitle>
                        <CardDescription className="font-mono text-xs mt-0.5">
                          {job.command}
                        </CardDescription>
                      </div>
                    </div>
                    <div className="flex items-center gap-3">
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <Clock className="h-3.5 w-3.5" />
                        <code className="text-xs bg-muted px-1.5 py-0.5 rounded">
                          {job.cron_expression}
                        </code>
                      </div>
                      {job.enabled ? (
                        <Badge className="bg-green-500 text-white">Enabled</Badge>
                      ) : (
                        <Badge variant="secondary">Disabled</Badge>
                      )}
                      <Switch
                        checked={job.enabled}
                        onCheckedChange={(checked) =>
                          toggleMutation.mutate({ jobId: job.id, enabled: checked })
                        }
                      />
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => triggerMutation.mutate(job.id)}
                        disabled={triggerMutation.isPending}
                        className="gap-1"
                      >
                        <Play className="h-3.5 w-3.5" />
                        Run Now
                      </Button>
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="sm">
                            <MoreHorizontal className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onClick={() => openEditDialog(job)}>
                            <Pencil className="h-4 w-4 mr-2" />
                            Edit
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={() => setDeleteConfirmJob(job)}
                            className="text-destructive"
                          >
                            <Trash2 className="h-4 w-4 mr-2" />
                            Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                  </div>
                  <div className="flex gap-6 text-xs text-muted-foreground mt-2 ml-8">
                    <span>Last run: {formatDate(job.last_run_at)}</span>
                    <span>Next run: {formatDate(job.next_run_at)}</span>
                  </div>
                </CardHeader>
                <CollapsibleContent>
                  <CardContent className="pt-0 pl-12">
                    <h4 className="text-sm font-medium mb-2">Run History</h4>
                    <JobRunHistory appId={appId!} jobId={job.id} />
                  </CardContent>
                </CollapsibleContent>
              </Card>
            </Collapsible>
          ))}
        </div>
      )}

      {/* Create Dialog */}
      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>Create Scheduled Job</DialogTitle>
            <DialogDescription>
              Schedule a command to run inside your app&apos;s container on a cron schedule.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
                placeholder="e.g., Database cleanup"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="command">Command</Label>
              <Input
                id="command"
                value={formCommand}
                onChange={(e) => setFormCommand(e.target.value)}
                placeholder="e.g., python manage.py clearsessions"
                className="font-mono text-sm"
              />
              <p className="text-xs text-muted-foreground">
                Executed via <code>/bin/sh -c</code> inside the container.
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="cron">Cron Expression</Label>
              <Input
                id="cron"
                value={formCron}
                onChange={(e) => setFormCron(e.target.value)}
                placeholder="0 0 * * * *"
                className="font-mono text-sm"
              />
              <div className="flex flex-wrap gap-1.5 mt-1">
                {CRON_PRESETS.map((preset) => (
                  <button
                    key={preset.value}
                    type="button"
                    onClick={() => setFormCron(preset.value)}
                    className="text-xs px-2 py-0.5 rounded border hover:bg-muted transition-colors"
                  >
                    {preset.label}
                  </button>
                ))}
              </div>
            </div>
            <div className="flex items-center space-x-2">
              <Switch
                id="enabled"
                checked={formEnabled}
                onCheckedChange={setFormEnabled}
              />
              <Label htmlFor="enabled">Enabled</Label>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCreateDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleCreate}
              disabled={!formName || !formCommand || !formCron || createMutation.isPending}
            >
              {createMutation.isPending ? "Creating..." : "Create Job"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Dialog */}
      <Dialog open={!!editingJob} onOpenChange={(open) => !open && setEditingJob(null)}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>Edit Scheduled Job</DialogTitle>
            <DialogDescription>
              Update the schedule and command for this job.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="edit-name">Name</Label>
              <Input
                id="edit-name"
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-command">Command</Label>
              <Input
                id="edit-command"
                value={formCommand}
                onChange={(e) => setFormCommand(e.target.value)}
                className="font-mono text-sm"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-cron">Cron Expression</Label>
              <Input
                id="edit-cron"
                value={formCron}
                onChange={(e) => setFormCron(e.target.value)}
                className="font-mono text-sm"
              />
              <div className="flex flex-wrap gap-1.5 mt-1">
                {CRON_PRESETS.map((preset) => (
                  <button
                    key={preset.value}
                    type="button"
                    onClick={() => setFormCron(preset.value)}
                    className="text-xs px-2 py-0.5 rounded border hover:bg-muted transition-colors"
                  >
                    {preset.label}
                  </button>
                ))}
              </div>
            </div>
            <div className="flex items-center space-x-2">
              <Switch
                id="edit-enabled"
                checked={formEnabled}
                onCheckedChange={setFormEnabled}
              />
              <Label htmlFor="edit-enabled">Enabled</Label>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditingJob(null)}>
              Cancel
            </Button>
            <Button
              onClick={handleUpdate}
              disabled={!formName || !formCommand || !formCron || updateMutation.isPending}
            >
              {updateMutation.isPending ? "Saving..." : "Save Changes"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <Dialog open={!!deleteConfirmJob} onOpenChange={(open) => !open && setDeleteConfirmJob(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Scheduled Job</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete the job &quot;{deleteConfirmJob?.name}&quot;? This
              will also delete all run history. This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteConfirmJob(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => deleteConfirmJob && deleteMutation.mutate(deleteConfirmJob.id)}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
