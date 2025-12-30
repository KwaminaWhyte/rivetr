import { useState, useEffect, useRef } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useSearchParams } from "react-router";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Github, Plus, MoreVertical, Trash2, ExternalLink, Users, Globe, Loader2, CheckCircle, Download } from "lucide-react";
import { api } from "@/lib/api";
import type { GitHubApp, GitHubAppInstallation } from "@/types/api";

export function meta() {
  return [
    { title: "GitHub Apps - Rivetr" },
    { name: "description", content: "Manage GitHub App integrations" },
  ];
}

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleDateString();
}

export default function GitHubAppsPage() {
  const queryClient = useQueryClient();
  const [searchParams] = useSearchParams();
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [expandedApp, setExpandedApp] = useState<string | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const formRef = useRef<HTMLFormElement>(null);

  // Check for query params from GitHub callback
  const registered = searchParams.get("registered");
  const installed = searchParams.get("installed");
  const appId = searchParams.get("app_id");
  const action = searchParams.get("action");

  // Show success messages from callback
  useEffect(() => {
    if (registered === "true") {
      toast.success("GitHub App registered successfully!");
      // If we just registered an app, show prompt to install it
      if (appId) {
        setExpandedApp(appId);
      }
    }
    if (installed === "true") {
      toast.success("GitHub App installed successfully!");
      queryClient.invalidateQueries({ queryKey: ["github-apps"] });
      queryClient.invalidateQueries({ queryKey: ["github-app-installations-all"] });
    }
  }, [registered, installed, appId, queryClient]);

  // Auto-open create dialog if action=create
  useEffect(() => {
    if (action === "create") {
      setCreateDialogOpen(true);
    }
  }, [action]);

  const { data: apps = [], isLoading } = useQuery<GitHubApp[]>({
    queryKey: ["github-apps"],
    queryFn: () => api.getGitHubApps(),
  });

  const { data: installations = [] } = useQuery<GitHubAppInstallation[]>({
    queryKey: ["github-app-installations", expandedApp],
    queryFn: () => api.getGitHubAppInstallations(expandedApp!),
    enabled: !!expandedApp,
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deleteGitHubApp(id),
    onSuccess: () => {
      toast.success("GitHub App deleted");
      queryClient.invalidateQueries({ queryKey: ["github-apps"] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to delete GitHub App");
    },
  });

  const handleCreateApp = async () => {
    setIsCreating(true);
    try {
      // Get the manifest from the backend
      const response = await api.createGitHubAppManifest({
        name: "rivetr",
        is_system_wide: true,
      });

      // Submit the form to GitHub via POST
      if (formRef.current && response.manifest) {
        const manifestInput = formRef.current.querySelector('input[name="manifest"]') as HTMLInputElement;
        if (manifestInput) {
          manifestInput.value = response.manifest;
          formRef.current.submit();
        }
      }
    } catch (error) {
      toast.error("Failed to create GitHub App");
      setIsCreating(false);
    }
  };

  const handleInstallApp = async (app: GitHubApp) => {
    try {
      const { install_url } = await api.getGitHubAppInstallUrl(app.id);
      window.location.href = install_url;
    } catch (error) {
      toast.error("Failed to get installation URL");
    }
  };

  // Show newly registered app card with install prompt
  const newlyRegisteredApp = apps.find(app => app.id === appId);
  const hasNoInstallations = expandedApp && installations.length === 0;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">GitHub Apps</h1>
          <p className="text-muted-foreground">
            Connect GitHub to deploy from your repositories automatically.
          </p>
        </div>
        <Button className="gap-2" onClick={() => setCreateDialogOpen(true)}>
          <Plus className="h-4 w-4" />
          Connect GitHub
        </Button>
      </div>

      {/* Success message for newly registered app */}
      {registered === "true" && newlyRegisteredApp && (
        <Card className="border-green-500/50 bg-green-500/5">
          <CardContent className="pt-6">
            <div className="flex items-start gap-4">
              <CheckCircle className="h-6 w-6 text-green-500 mt-0.5" />
              <div className="flex-1 space-y-2">
                <h3 className="font-semibold">GitHub App Created Successfully!</h3>
                <p className="text-sm text-muted-foreground">
                  Your GitHub App "{newlyRegisteredApp.name}" has been registered.
                  Now install it on your GitHub account to start deploying from your repositories.
                </p>
                <Button onClick={() => handleInstallApp(newlyRegisteredApp)} className="gap-2">
                  <Download className="h-4 w-4" />
                  Install on GitHub
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Github className="h-5 w-5" />
            Connected Apps
          </CardTitle>
          <CardDescription>
            GitHub Apps connected for automatic deployments and PR previews.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : apps.length === 0 ? (
            <div className="text-center py-12 space-y-4">
              <Github className="h-12 w-12 mx-auto text-muted-foreground/50" />
              <div>
                <p className="text-lg font-medium">No GitHub Apps Connected</p>
                <p className="text-sm text-muted-foreground">
                  Connect a GitHub App to deploy from your repositories.
                </p>
              </div>
              <Button onClick={() => setCreateDialogOpen(true)} className="gap-2">
                <Github className="h-4 w-4" />
                Connect GitHub
              </Button>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>App Name</TableHead>
                  <TableHead>App ID</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="w-[100px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {apps.map((app) => (
                  <TableRow
                    key={app.id}
                    className="cursor-pointer"
                    onClick={() => setExpandedApp(expandedApp === app.id ? null : app.id)}
                  >
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Github className="h-4 w-4" />
                        <span className="font-medium">{app.name}</span>
                        {app.slug && (
                          <a
                            href={`https://github.com/apps/${app.slug}`}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-muted-foreground hover:text-foreground"
                            onClick={(e) => e.stopPropagation()}
                          >
                            <ExternalLink className="h-3 w-3" />
                          </a>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <code className="text-xs bg-muted px-1.5 py-0.5 rounded">
                        {app.app_id}
                      </code>
                    </TableCell>
                    <TableCell>
                      {app.is_system_wide ? (
                        <Badge variant="secondary" className="gap-1">
                          <Globe className="h-3 w-3" />
                          System-wide
                        </Badge>
                      ) : (
                        <Badge variant="outline" className="gap-1">
                          <Users className="h-3 w-3" />
                          Team
                        </Badge>
                      )}
                    </TableCell>
                    <TableCell className="text-muted-foreground">
                      {formatDate(app.created_at)}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleInstallApp(app);
                          }}
                        >
                          Install
                        </Button>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
                            <Button variant="ghost" size="icon">
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem
                              onClick={(e) => {
                                e.stopPropagation();
                                setDeleteId(app.id);
                              }}
                              className="text-destructive"
                            >
                              <Trash2 className="h-4 w-4 mr-2" />
                              Delete
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Installations panel */}
      {expandedApp && (
        <Card>
          <CardHeader>
            <CardTitle>Installations</CardTitle>
            <CardDescription>
              GitHub accounts where this app is installed.
            </CardDescription>
          </CardHeader>
          <CardContent>
            {installations.length === 0 ? (
              <div className="text-center py-8 space-y-3">
                <p className="text-muted-foreground">No installations yet.</p>
                <Button
                  variant="outline"
                  onClick={() => {
                    const app = apps.find(a => a.id === expandedApp);
                    if (app) handleInstallApp(app);
                  }}
                >
                  Install on GitHub
                </Button>
              </div>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Account</TableHead>
                    <TableHead>Type</TableHead>
                    <TableHead>Repository Access</TableHead>
                    <TableHead>Installed</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {installations.map((installation) => (
                    <TableRow key={installation.id}>
                      <TableCell className="font-medium">
                        {installation.account_login}
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline">
                          {installation.account_type === "organization" ? "Organization" : "User"}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <Badge variant="secondary">
                          {installation.repository_selection === "all" ? "All repositories" : "Selected"}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {formatDate(installation.created_at)}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </CardContent>
        </Card>
      )}

      {/* Create GitHub App Dialog */}
      <Dialog open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Github className="h-5 w-5" />
              Connect GitHub
            </DialogTitle>
            <DialogDescription>
              Create a GitHub App to enable automatic deployments from your repositories.
              You'll be redirected to GitHub to complete the setup.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <div className="p-4 bg-muted/50 rounded-lg space-y-2">
              <h4 className="font-medium">What happens next:</h4>
              <ol className="text-sm text-muted-foreground space-y-1 list-decimal list-inside">
                <li>You'll be redirected to GitHub</li>
                <li>Click "Create GitHub App" (settings are pre-filled)</li>
                <li>Choose which account to install it on</li>
                <li>Select which repositories to grant access</li>
                <li>You'll be redirected back here</li>
              </ol>
            </div>
            <p className="text-xs text-muted-foreground">
              The GitHub App will have permissions to read your code and create deployment status checks.
            </p>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateDialogOpen(false)}>
              Cancel
            </Button>
            <Button onClick={handleCreateApp} disabled={isCreating} className="gap-2">
              {isCreating ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Redirecting...
                </>
              ) : (
                <>
                  <Github className="h-4 w-4" />
                  Continue to GitHub
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Hidden form for GitHub manifest POST */}
      <form ref={formRef} method="post" action="https://github.com/settings/apps/new" style={{ display: 'none' }}>
        <input type="hidden" name="manifest" />
      </form>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={!!deleteId} onOpenChange={() => setDeleteId(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete GitHub App</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete this GitHub App? This will disconnect all installations.
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
