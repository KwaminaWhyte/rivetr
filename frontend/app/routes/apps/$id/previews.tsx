import { useOutletContext } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
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
import { useState } from "react";
import {
  ExternalLink,
  GitPullRequest,
  GitBranch,
  MoreVertical,
  Trash2,
  RotateCw,
  AlertCircle,
} from "lucide-react";
import { api } from "@/lib/api";
import type { App, PreviewDeployment, PreviewDeploymentStatus } from "@/types/api";

interface OutletContext {
  app: App;
}

function getStatusBadge(status: PreviewDeploymentStatus) {
  switch (status) {
    case "running":
      return <Badge className="bg-green-500">Running</Badge>;
    case "building":
      return <Badge className="bg-blue-500">Building</Badge>;
    case "pending":
    case "cloning":
    case "starting":
      return <Badge className="bg-yellow-500">Deploying</Badge>;
    case "failed":
      return <Badge variant="destructive">Failed</Badge>;
    case "closed":
      return <Badge variant="secondary">Closed</Badge>;
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
}

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleString();
}

export default function AppPreviewsTab() {
  const { app } = useOutletContext<OutletContext>();
  const queryClient = useQueryClient();
  const [deleteId, setDeleteId] = useState<string | null>(null);

  const { data: previews = [], isLoading } = useQuery<PreviewDeployment[]>({
    queryKey: ["app-previews", app.id],
    queryFn: () => api.getAppPreviews(app.id),
    refetchInterval: 10000, // Refresh every 10 seconds
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deletePreview(id),
    onSuccess: () => {
      toast.success("Preview deployment deleted");
      queryClient.invalidateQueries({ queryKey: ["app-previews", app.id] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to delete preview");
    },
  });

  const redeployMutation = useMutation({
    mutationFn: (id: string) => api.redeployPreview(id),
    onSuccess: () => {
      toast.success("Preview redeployment started");
      queryClient.invalidateQueries({ queryKey: ["app-previews", app.id] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to redeploy preview");
    },
  });

  if (!app.preview_enabled) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <GitPullRequest className="h-5 w-5" />
            PR Preview Deployments
          </CardTitle>
          <CardDescription>
            Preview deployments are not enabled for this application.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-4 p-6 bg-muted/50 rounded-lg">
            <AlertCircle className="h-8 w-8 text-muted-foreground" />
            <div>
              <p className="font-medium">Enable PR Previews</p>
              <p className="text-sm text-muted-foreground">
                Go to Settings &gt; Build to enable automatic preview deployments for pull requests.
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <GitPullRequest className="h-5 w-5" />
            PR Preview Deployments
          </CardTitle>
          <CardDescription>
            Automatic preview environments for pull requests. Each PR gets its own isolated deployment.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-3">
              {[1, 2, 3].map((i) => (
                <div key={i} className="h-16 bg-muted animate-pulse rounded" />
              ))}
            </div>
          ) : previews.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <GitPullRequest className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p className="text-lg font-medium">No Preview Deployments</p>
              <p className="text-sm">
                Open a pull request to create a preview deployment.
              </p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Pull Request</TableHead>
                  <TableHead>Branch</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Preview URL</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="w-[50px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {previews.map((preview) => (
                  <TableRow key={preview.id}>
                    <TableCell>
                      <div className="flex flex-col">
                        <a
                          href={preview.pr_url || "#"}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="font-medium hover:underline flex items-center gap-1"
                        >
                          #{preview.pr_number}
                          <ExternalLink className="h-3 w-3" />
                        </a>
                        <span className="text-sm text-muted-foreground truncate max-w-[200px]">
                          {preview.pr_title || "No title"}
                        </span>
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1.5">
                        <GitBranch className="h-4 w-4 text-muted-foreground" />
                        <code className="text-xs bg-muted px-1.5 py-0.5 rounded">
                          {preview.pr_source_branch}
                        </code>
                      </div>
                    </TableCell>
                    <TableCell>
                      {getStatusBadge(preview.status)}
                      {preview.error_message && (
                        <p className="text-xs text-destructive mt-1 truncate max-w-[150px]" title={preview.error_message}>
                          {preview.error_message}
                        </p>
                      )}
                    </TableCell>
                    <TableCell>
                      {preview.status === "running" && preview.preview_domain ? (
                        <a
                          href={`https://${preview.preview_domain}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-blue-600 hover:underline flex items-center gap-1"
                        >
                          {preview.preview_domain}
                          <ExternalLink className="h-3 w-3" />
                        </a>
                      ) : (
                        <span className="text-muted-foreground">-</span>
                      )}
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatDate(preview.created_at)}
                    </TableCell>
                    <TableCell>
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon">
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem
                            onClick={() => redeployMutation.mutate(preview.id)}
                            disabled={redeployMutation.isPending}
                          >
                            <RotateCw className="h-4 w-4 mr-2" />
                            Redeploy
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={() => setDeleteId(preview.id)}
                            className="text-destructive"
                          >
                            <Trash2 className="h-4 w-4 mr-2" />
                            Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={!!deleteId} onOpenChange={() => setDeleteId(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Preview Deployment</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete this preview deployment? This will stop the container and remove all associated resources.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (deleteId) {
                  deleteMutation.mutate(deleteId);
                  setDeleteId(null);
                }
              }}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
