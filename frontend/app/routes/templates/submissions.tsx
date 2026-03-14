import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import {
  CheckCircle,
  XCircle,
  Clock,
  Loader2,
  Trash2,
  ChevronRight,
  Send,
} from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { useBreadcrumb } from "@/lib/breadcrumb-context";
import type { CommunityTemplateSubmission } from "@/lib/api/community-templates";

export function meta() {
  return [
    { title: "Template Submissions - Rivetr" },
    {
      name: "description",
      content: "Manage community template submissions",
    },
  ];
}

function StatusBadge({ status }: { status: string }) {
  switch (status) {
    case "approved":
      return (
        <Badge className="bg-green-500/10 text-green-600 border-green-500/20">
          <CheckCircle className="h-3 w-3 mr-1" />
          Approved
        </Badge>
      );
    case "rejected":
      return (
        <Badge className="bg-red-500/10 text-red-600 border-red-500/20">
          <XCircle className="h-3 w-3 mr-1" />
          Rejected
        </Badge>
      );
    default:
      return (
        <Badge className="bg-yellow-500/10 text-yellow-600 border-yellow-500/20">
          <Clock className="h-3 w-3 mr-1" />
          Pending Review
        </Badge>
      );
  }
}

export default function TemplateSubmissionsPage() {
  useBreadcrumb([
    { label: "Templates", href: "/templates" },
    { label: "My Submissions" },
  ]);

  const queryClient = useQueryClient();
  const navigate = useNavigate();

  const [selectedSubmission, setSelectedSubmission] =
    useState<CommunityTemplateSubmission | null>(null);
  const [reviewAction, setReviewAction] = useState<"approve" | "reject" | null>(
    null
  );
  const [reviewNotes, setReviewNotes] = useState("");
  const [deletingId, setDeletingId] = useState<string | null>(null);

  // Try to get user info (to show admin controls)
  const { data: me } = useQuery({
    queryKey: ["me"],
    queryFn: () =>
      fetch("/api/auth/me", {
        credentials: "include",
      }).then((r) => r.json()),
  });

  const isAdmin = me?.role === "admin";

  // My submissions
  const { data: mySubmissions, isLoading: loadingMine } = useQuery({
    queryKey: ["my-template-submissions"],
    queryFn: () => api.myTemplateSubmissions(),
  });

  // Admin: all submissions
  const { data: allSubmissions, isLoading: loadingAll } = useQuery({
    queryKey: ["all-template-submissions"],
    queryFn: () => api.listAllSubmissions(),
    enabled: isAdmin,
  });

  const reviewMutation = useMutation({
    mutationFn: ({
      id,
      action,
      notes,
    }: {
      id: string;
      action: "approve" | "reject";
      notes?: string;
    }) => api.reviewSubmission(id, { action, notes }),
    onSuccess: () => {
      toast.success(
        reviewAction === "approve"
          ? "Template approved and added to the library!"
          : "Template submission rejected."
      );
      queryClient.invalidateQueries({
        queryKey: ["all-template-submissions"],
      });
      queryClient.invalidateQueries({
        queryKey: ["my-template-submissions"],
      });
      setSelectedSubmission(null);
      setReviewAction(null);
      setReviewNotes("");
    },
    onError: (err: Error) => {
      toast.error(err.message || "Failed to review submission");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deleteSubmission(id),
    onSuccess: () => {
      toast.success("Submission deleted.");
      queryClient.invalidateQueries({
        queryKey: ["my-template-submissions"],
      });
      queryClient.invalidateQueries({
        queryKey: ["all-template-submissions"],
      });
      setDeletingId(null);
    },
    onError: (err: Error) => {
      toast.error(err.message || "Failed to delete submission");
    },
  });

  const submissions = isAdmin ? allSubmissions : mySubmissions;
  const isLoading = isAdmin ? loadingAll : loadingMine;

  return (
    <div className="container max-w-4xl py-8 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">
            {isAdmin ? "All Template Submissions" : "My Template Submissions"}
          </h1>
          <p className="text-muted-foreground text-sm mt-1">
            {isAdmin
              ? "Review and approve community-submitted templates."
              : "Templates you have submitted for admin review."}
          </p>
        </div>
        <Button onClick={() => navigate("/templates")} variant="outline">
          <ChevronRight className="h-4 w-4 mr-1 rotate-180" />
          Back to Templates
        </Button>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      ) : !submissions || submissions.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center">
            <Send className="h-10 w-10 text-muted-foreground mx-auto mb-4" />
            <p className="font-medium">No submissions yet</p>
            <p className="text-sm text-muted-foreground mt-1">
              {isAdmin
                ? "No templates have been submitted for review."
                : "Submit a template from the templates page to share it with the community."}
            </p>
            {!isAdmin && (
              <Button
                className="mt-4"
                onClick={() => navigate("/templates")}
              >
                Browse Templates
              </Button>
            )}
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-3">
          {submissions.map((sub) => (
            <Card key={sub.id} className="hover:shadow-sm transition-shadow">
              <CardHeader className="pb-2">
                <div className="flex items-start justify-between gap-4">
                  <div className="space-y-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <CardTitle className="text-base">{sub.name}</CardTitle>
                      <Badge variant="outline" className="capitalize text-xs">
                        {sub.category}
                      </Badge>
                      <StatusBadge status={sub.status} />
                    </div>
                    <CardDescription className="line-clamp-2">
                      {sub.description}
                    </CardDescription>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    {isAdmin && sub.status === "pending" && (
                      <>
                        <Button
                          size="sm"
                          variant="outline"
                          className="text-green-600 border-green-300 hover:bg-green-50"
                          onClick={() => {
                            setSelectedSubmission(sub);
                            setReviewAction("approve");
                          }}
                        >
                          <CheckCircle className="h-4 w-4 mr-1" />
                          Approve
                        </Button>
                        <Button
                          size="sm"
                          variant="outline"
                          className="text-red-600 border-red-300 hover:bg-red-50"
                          onClick={() => {
                            setSelectedSubmission(sub);
                            setReviewAction("reject");
                          }}
                        >
                          <XCircle className="h-4 w-4 mr-1" />
                          Reject
                        </Button>
                      </>
                    )}
                    {(sub.status === "pending" || isAdmin) && (
                      <Button
                        size="sm"
                        variant="ghost"
                        className="text-muted-foreground hover:text-destructive"
                        onClick={() => setDeletingId(sub.id)}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    )}
                  </div>
                </div>
              </CardHeader>
              <CardContent className="pt-0 pb-3">
                <div className="text-xs text-muted-foreground flex items-center gap-4">
                  <span>
                    Submitted {new Date(sub.created_at).toLocaleDateString()}
                  </span>
                  {sub.admin_notes && (
                    <span className="italic">Note: {sub.admin_notes}</span>
                  )}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Review dialog */}
      <Dialog
        open={!!selectedSubmission && !!reviewAction}
        onOpenChange={(open) => {
          if (!open) {
            setSelectedSubmission(null);
            setReviewAction(null);
            setReviewNotes("");
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {reviewAction === "approve"
                ? "Approve Template"
                : "Reject Template"}
            </DialogTitle>
            <DialogDescription>
              {reviewAction === "approve"
                ? `Approving "${selectedSubmission?.name}" will add it to the service templates library.`
                : `Rejecting "${selectedSubmission?.name}" will notify the submitter.`}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3 py-2">
            <Label>Admin notes (optional)</Label>
            <Textarea
              placeholder="Leave a note for the submitter..."
              value={reviewNotes}
              onChange={(e) => setReviewNotes(e.target.value)}
              rows={3}
            />
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setSelectedSubmission(null);
                setReviewAction(null);
                setReviewNotes("");
              }}
            >
              Cancel
            </Button>
            <Button
              variant={reviewAction === "approve" ? "default" : "destructive"}
              disabled={reviewMutation.isPending}
              onClick={() => {
                if (selectedSubmission && reviewAction) {
                  reviewMutation.mutate({
                    id: selectedSubmission.id,
                    action: reviewAction,
                    notes: reviewNotes || undefined,
                  });
                }
              }}
            >
              {reviewMutation.isPending && (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              )}
              {reviewAction === "approve" ? "Approve" : "Reject"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete confirmation */}
      <AlertDialog
        open={!!deletingId}
        onOpenChange={(open) => {
          if (!open) setDeletingId(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Submission?</AlertDialogTitle>
            <AlertDialogDescription>
              This will permanently delete the template submission. This action
              cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive hover:bg-destructive/90"
              onClick={() => {
                if (deletingId) deleteMutation.mutate(deletingId);
              }}
            >
              {deleteMutation.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                "Delete"
              )}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
