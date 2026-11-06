import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Check, ChevronDown, ChevronRight, ClipboardList, Loader2, X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { EmptyState, PageHeader } from "@/components/page";
import { QueryError } from "@/components/query-error";
import { templateSubmissionsApi } from "@/lib/api";
import type { SubmissionStatus, TemplateSubmission } from "@/lib/api";

export function meta() {
  return [
    { title: "Template submissions - Settings" },
    { name: "description", content: "Review community template submissions" },
  ];
}

function SubmissionRow({ submission }: { submission: TemplateSubmission }) {
  const [open, setOpen] = useState(false);
  const [rejecting, setRejecting] = useState(false);
  const [reason, setReason] = useState("");
  const queryClient = useQueryClient();
  const refresh = () => {
    void queryClient.invalidateQueries({ queryKey: ["template-submissions"] });
  };
  const approve = useMutation({
    mutationFn: () => templateSubmissionsApi.approve(submission.id),
    onSuccess: () => {
      toast.success(`${submission.name} added to the catalogue`);
      refresh();
    },
    onError: (e: Error) => toast.error(e.message || "Could not approve the submission"),
  });
  const reject = useMutation({
    mutationFn: () => templateSubmissionsApi.reject(submission.id, reason.trim()),
    onSuccess: () => {
      toast.success("Submission rejected");
      setRejecting(false);
      refresh();
    },
    onError: (e: Error) => toast.error(e.message || "Could not reject the submission"),
  });

  return (
    <li className="p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <button
          type="button"
          className="flex min-w-0 items-start gap-2 text-left"
          onClick={() => setOpen((o) => !o)}
          aria-expanded={open}
        >
          {open ? (
            <ChevronDown className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
          ) : (
            <ChevronRight className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
          )}
          <span className="min-w-0 space-y-1">
            <span className="flex flex-wrap items-center gap-2">
              <span className="font-medium">{submission.name}</span>
              <Badge variant="outline">{submission.category}</Badge>
              <Badge
                className={`border-0 ${
                  submission.status === "pending"
                    ? "bg-amber-500/15 text-amber-700 dark:text-amber-400"
                    : submission.status === "approved"
                      ? "bg-green-500/15 text-green-700 dark:text-green-400"
                      : "bg-destructive/15 text-destructive"
                }`}
              >
                {submission.status}
              </Badge>
            </span>
            <span className="block truncate text-sm text-muted-foreground">
              {submission.description}
            </span>
            <span className="block text-xs text-muted-foreground">
              {submission.submitter_email ?? "unknown user"} ·{" "}
              {new Date(submission.created_at).toLocaleDateString()}
              {submission.admin_notes ? ` · ${submission.admin_notes}` : ""}
              {submission.rejection_reason ? ` · ${submission.rejection_reason}` : ""}
            </span>
          </span>
        </button>
        {submission.status === "pending" && (
          <div className="flex shrink-0 gap-2">
            <Button
              size="sm"
              variant="outline"
              className="border-destructive/50 text-destructive hover:text-destructive"
              disabled={reject.isPending}
              onClick={() => setRejecting((r) => !r)}
            >
              <X className="h-3.5 w-3.5" />
              Reject
            </Button>
            <Button size="sm" disabled={approve.isPending} onClick={() => approve.mutate()}>
              {approve.isPending ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <Check className="h-3.5 w-3.5" />
              )}
              Approve
            </Button>
          </div>
        )}
      </div>

      {rejecting && submission.status === "pending" && (
        <div className="mt-3 space-y-2 rounded-md border p-3">
          <Label htmlFor={`reason-${submission.id}`}>Reason (shown to the submitter)</Label>
          <Textarea
            id={`reason-${submission.id}`}
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            placeholder="The compose file needs pinned image tags."
            rows={2}
          />
          <div className="flex justify-end gap-2">
            <Button size="sm" variant="ghost" onClick={() => setRejecting(false)}>
              Cancel
            </Button>
            <Button
              size="sm"
              variant="destructive"
              disabled={reject.isPending || !reason.trim()}
              onClick={() => reject.mutate()}
            >
              Reject submission
            </Button>
          </div>
        </div>
      )}

      {open && (
        <div className="mt-3 space-y-3">
          <pre className="max-h-72 overflow-auto rounded-md bg-muted/60 p-3 font-mono text-xs leading-relaxed">
            {submission.compose_content}
          </pre>
          {submission.env_hints.length > 0 && (
            <div className="text-sm">
              <p className="mb-1 font-medium">Environment variables</p>
              <ul className="list-inside list-disc text-muted-foreground">
                {submission.env_hints.map((e) => (
                  <li key={e.name}>
                    <code className="text-xs">{e.name}</code> {e.label}
                    {e.required ? " (required)" : ""}
                    {e.secret ? " (secret)" : ""}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}
    </li>
  );
}

export default function TemplateSubmissionsSettingsPage() {
  const [status, setStatus] = useState<SubmissionStatus>("pending");
  const query = useQuery({
    queryKey: ["template-submissions", "queue", status],
    queryFn: () => templateSubmissionsApi.list(status),
  });

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<ClipboardList />}
        title="Template submissions"
        description="Community templates wait here until an admin approves or rejects them. Approved templates join the catalogue marked Community."
      />
      <Card>
        <CardHeader>
          <CardTitle>Review queue</CardTitle>
          <CardDescription>Click a submission to see its compose file.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <Tabs value={status} onValueChange={(v) => setStatus(v as SubmissionStatus)}>
            <TabsList>
              <TabsTrigger value="pending">Pending</TabsTrigger>
              <TabsTrigger value="approved">Approved</TabsTrigger>
              <TabsTrigger value="rejected">Rejected</TabsTrigger>
            </TabsList>
          </Tabs>
          {query.error ? (
            <QueryError what="submissions" error={query.error} onRetry={() => query.refetch()} />
          ) : query.isLoading ? (
            <div className="flex justify-center py-8">
              <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
            </div>
          ) : (query.data ?? []).length === 0 ? (
            <EmptyState
              icon={<ClipboardList />}
              title={status === "pending" ? "Nothing to review" : `No ${status} submissions`}
              description="Submissions from the Templates page land here."
              compact
              bordered
            />
          ) : (
            <ul className="divide-y rounded-lg border bg-card">
              {(query.data ?? []).map((s) => (
                <SubmissionRow key={s.id} submission={s} />
              ))}
            </ul>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
