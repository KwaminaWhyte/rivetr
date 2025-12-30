import { useState, useEffect, useRef } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useSearchParams } from "react-router";
import { toast } from "sonner";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
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
import { api } from "@/lib/api";
import type { GitProvider, GitHubApp, GitHubAppInstallation } from "@/types/api";
import {
  Github,
  Plus,
  MoreVertical,
  Trash2,
  ExternalLink,
  Users,
  Globe,
  Loader2,
  CheckCircle,
  Download,
  Key,
  Link2,
  GitBranch,
} from "lucide-react";

export function meta() {
  return [
    { title: "Git Integrations - Rivetr" },
    { name: "description", content: "Connect Git providers to deploy from repositories" },
  ];
}

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleDateString();
}

// GitLab icon SVG
const GitLabIcon = ({ className }: { className?: string }) => (
  <svg className={className} viewBox="0 0 24 24" fill="currentColor">
    <path d="M22.65 14.39L12 22.13 1.35 14.39a.84.84 0 01-.3-.94l1.22-3.78 2.44-7.51A.42.42 0 014.82 2a.43.43 0 01.58 0 .42.42 0 01.11.18l2.44 7.49h8.1l2.44-7.51A.42.42 0 0118.6 2a.43.43 0 01.58 0 .42.42 0 01.11.18l2.44 7.51L23 13.45a.84.84 0 01-.35.94z" />
  </svg>
);

// Bitbucket icon SVG
const BitbucketIcon = ({ className }: { className?: string }) => (
  <svg className={className} viewBox="0 0 24 24" fill="currentColor">
    <path d="M.778 1.213a.768.768 0 00-.768.892l3.263 19.81c.084.5.515.868 1.022.873H19.95a.772.772 0 00.77-.646l3.27-20.03a.768.768 0 00-.768-.889zM14.52 15.53H9.522L8.17 8.466h7.561z" />
  </svg>
);

// ============================================================================
// GitHub Apps Tab Component
// ============================================================================
function GitHubAppsTab() {
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

  useEffect(() => {
    if (registered === "true") {
      toast.success("GitHub App registered successfully!");
      if (appId) setExpandedApp(appId);
    }
    if (installed === "true") {
      toast.success("GitHub App installed successfully!");
      queryClient.invalidateQueries({ queryKey: ["github-apps"] });
      queryClient.invalidateQueries({ queryKey: ["github-app-installations-all"] });
    }
    if (action === "create") {
      setCreateDialogOpen(true);
    }
  }, [registered, installed, appId, action, queryClient]);

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
      const response = await api.createGitHubAppManifest({
        name: "rivetr",
        is_system_wide: true,
      });
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

  const newlyRegisteredApp = apps.find(app => app.id === appId);

  return (
    <div className="space-y-6">
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
                  Now install it on your GitHub account to start deploying.
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
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Github className="h-5 w-5" />
                GitHub Apps
              </CardTitle>
              <CardDescription>
                Connect GitHub to deploy from your repositories with automatic webhooks.
              </CardDescription>
            </div>
            <Button onClick={() => setCreateDialogOpen(true)} className="gap-2">
              <Plus className="h-4 w-4" />
              Connect GitHub
            </Button>
          </div>
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
                      <code className="text-xs bg-muted px-1.5 py-0.5 rounded">{app.app_id}</code>
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
                    <TableCell className="text-muted-foreground">{formatDate(app.created_at)}</TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={(e) => { e.stopPropagation(); handleInstallApp(app); }}
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
                              onClick={(e) => { e.stopPropagation(); setDeleteId(app.id); }}
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
            <CardDescription>GitHub accounts where this app is installed.</CardDescription>
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
                      <TableCell className="font-medium">{installation.account_login}</TableCell>
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
                      <TableCell className="text-muted-foreground">{formatDate(installation.created_at)}</TableCell>
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
              Create a GitHub App for automatic deployments from your repositories.
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
            <Button variant="outline" onClick={() => setCreateDialogOpen(false)}>Cancel</Button>
            <Button onClick={handleCreateApp} disabled={isCreating} className="gap-2">
              {isCreating ? (
                <><Loader2 className="h-4 w-4 animate-spin" />Redirecting...</>
              ) : (
                <><Github className="h-4 w-4" />Continue to GitHub</>
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
              onClick={() => { if (deleteId) { deleteMutation.mutate(deleteId); setDeleteId(null); } }}
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

// ============================================================================
// GitLab Tab Component
// ============================================================================
function GitLabTab() {
  const queryClient = useQueryClient();
  const [addDialogOpen, setAddDialogOpen] = useState(false);
  const [deleteProvider, setDeleteProvider] = useState<GitProvider | null>(null);
  const [token, setToken] = useState("");
  const [isAdding, setIsAdding] = useState(false);

  const { data: providers = [], isLoading } = useQuery<GitProvider[]>({
    queryKey: ["gitProviders"],
    queryFn: () => api.getGitProviders(),
  });

  const gitlabProvider = providers.find(p => p.provider === "gitlab");

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deleteGitProvider(id),
    onSuccess: () => {
      toast.success("GitLab disconnected");
      queryClient.invalidateQueries({ queryKey: ["gitProviders"] });
      setDeleteProvider(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to disconnect GitLab");
    },
  });

  const handleAddProvider = async () => {
    if (!token.trim()) {
      toast.error("Personal Access Token is required");
      return;
    }
    setIsAdding(true);
    try {
      await api.addGitProvider({ provider: "gitlab", token: token.trim() });
      toast.success("GitLab connected successfully!");
      queryClient.invalidateQueries({ queryKey: ["gitProviders"] });
      setAddDialogOpen(false);
      setToken("");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to connect GitLab");
    } finally {
      setIsAdding(false);
    }
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <GitLabIcon className="h-5 w-5 text-orange-500" />
                GitLab
              </CardTitle>
              <CardDescription>
                Connect GitLab using a Personal Access Token to access your repositories.
              </CardDescription>
            </div>
            {!gitlabProvider && (
              <Button onClick={() => setAddDialogOpen(true)} className="gap-2">
                <Plus className="h-4 w-4" />
                Connect GitLab
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : gitlabProvider ? (
            <div className="flex items-center justify-between p-4 border rounded-lg">
              <div className="flex items-center gap-4">
                <Avatar className="h-12 w-12">
                  <AvatarImage src={gitlabProvider.avatar_url || undefined} />
                  <AvatarFallback>{gitlabProvider.username?.slice(0, 2).toUpperCase()}</AvatarFallback>
                </Avatar>
                <div>
                  <div className="flex items-center gap-2">
                    <span className="font-medium">{gitlabProvider.display_name || gitlabProvider.username}</span>
                    <Badge variant="outline" className="gap-1 text-green-600 border-green-300">
                      <CheckCircle className="h-3 w-3" />
                      Connected
                    </Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">@{gitlabProvider.username}</p>
                </div>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setDeleteProvider(gitlabProvider)}
                className="text-red-600 hover:text-red-700 hover:bg-red-50"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          ) : (
            <div className="text-center py-12 space-y-4">
              <GitLabIcon className="h-12 w-12 mx-auto text-muted-foreground/50" />
              <div>
                <p className="text-lg font-medium">GitLab Not Connected</p>
                <p className="text-sm text-muted-foreground">
                  Add a Personal Access Token to deploy from GitLab repositories.
                </p>
              </div>
              <Button onClick={() => setAddDialogOpen(true)} className="gap-2">
                <Key className="h-4 w-4" />
                Add Personal Access Token
              </Button>
            </div>
          )}
        </CardContent>
      </Card>

      {/* How to create PAT */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">How to Create a GitLab Personal Access Token</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm text-muted-foreground">
          <ol className="list-decimal list-inside space-y-2">
            <li>Go to GitLab → Settings → Access Tokens</li>
            <li>Click "Add new token"</li>
            <li>Give it a name like "Rivetr"</li>
            <li>Select scopes: <code className="bg-muted px-1 rounded">api</code>, <code className="bg-muted px-1 rounded">read_repository</code></li>
            <li>Click "Create personal access token"</li>
            <li>Copy the token and paste it here</li>
          </ol>
          <a
            href="https://gitlab.com/-/user_settings/personal_access_tokens"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 text-primary hover:underline"
          >
            <ExternalLink className="h-3 w-3" />
            Open GitLab Token Settings
          </a>
        </CardContent>
      </Card>

      {/* Add GitLab Dialog */}
      <Dialog open={addDialogOpen} onOpenChange={setAddDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <GitLabIcon className="h-5 w-5 text-orange-500" />
              Connect GitLab
            </DialogTitle>
            <DialogDescription>
              Enter your GitLab Personal Access Token to connect your repositories.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <div className="space-y-2">
              <Label htmlFor="gitlab-token">Personal Access Token</Label>
              <Input
                id="gitlab-token"
                type="password"
                placeholder="glpat-xxxxxxxxxxxxxxxxxxxx"
                value={token}
                onChange={(e) => setToken(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                Required scopes: <code>api</code>, <code>read_repository</code>
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAddDialogOpen(false)}>Cancel</Button>
            <Button onClick={handleAddProvider} disabled={isAdding || !token.trim()} className="gap-2">
              {isAdding ? <Loader2 className="h-4 w-4 animate-spin" /> : <Link2 className="h-4 w-4" />}
              Connect
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <AlertDialog open={!!deleteProvider} onOpenChange={() => setDeleteProvider(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Disconnect GitLab</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to disconnect {deleteProvider?.username}? You will lose access to private repositories.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => deleteProvider && deleteMutation.mutate(deleteProvider.id)}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Disconnect
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

// ============================================================================
// Bitbucket Tab Component
// ============================================================================
function BitbucketTab() {
  const queryClient = useQueryClient();
  const [addDialogOpen, setAddDialogOpen] = useState(false);
  const [deleteProvider, setDeleteProvider] = useState<GitProvider | null>(null);
  const [username, setUsername] = useState("");
  const [appPassword, setAppPassword] = useState("");
  const [isAdding, setIsAdding] = useState(false);

  const { data: providers = [], isLoading } = useQuery<GitProvider[]>({
    queryKey: ["gitProviders"],
    queryFn: () => api.getGitProviders(),
  });

  const bitbucketProvider = providers.find(p => p.provider === "bitbucket");

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deleteGitProvider(id),
    onSuccess: () => {
      toast.success("Bitbucket disconnected");
      queryClient.invalidateQueries({ queryKey: ["gitProviders"] });
      setDeleteProvider(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to disconnect Bitbucket");
    },
  });

  const handleAddProvider = async () => {
    if (!username.trim() || !appPassword.trim()) {
      toast.error("Username and App Password are required");
      return;
    }
    setIsAdding(true);
    try {
      await api.addGitProvider({
        provider: "bitbucket",
        token: appPassword.trim(),
        username: username.trim(),
      });
      toast.success("Bitbucket connected successfully!");
      queryClient.invalidateQueries({ queryKey: ["gitProviders"] });
      setAddDialogOpen(false);
      setUsername("");
      setAppPassword("");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to connect Bitbucket");
    } finally {
      setIsAdding(false);
    }
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <BitbucketIcon className="h-5 w-5 text-blue-600" />
                Bitbucket
              </CardTitle>
              <CardDescription>
                Connect Bitbucket using an App Password to access your repositories.
              </CardDescription>
            </div>
            {!bitbucketProvider && (
              <Button onClick={() => setAddDialogOpen(true)} className="gap-2">
                <Plus className="h-4 w-4" />
                Connect Bitbucket
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : bitbucketProvider ? (
            <div className="flex items-center justify-between p-4 border rounded-lg">
              <div className="flex items-center gap-4">
                <Avatar className="h-12 w-12">
                  <AvatarImage src={bitbucketProvider.avatar_url || undefined} />
                  <AvatarFallback>{bitbucketProvider.username?.slice(0, 2).toUpperCase()}</AvatarFallback>
                </Avatar>
                <div>
                  <div className="flex items-center gap-2">
                    <span className="font-medium">{bitbucketProvider.display_name || bitbucketProvider.username}</span>
                    <Badge variant="outline" className="gap-1 text-green-600 border-green-300">
                      <CheckCircle className="h-3 w-3" />
                      Connected
                    </Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">@{bitbucketProvider.username}</p>
                </div>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setDeleteProvider(bitbucketProvider)}
                className="text-red-600 hover:text-red-700 hover:bg-red-50"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          ) : (
            <div className="text-center py-12 space-y-4">
              <BitbucketIcon className="h-12 w-12 mx-auto text-muted-foreground/50" />
              <div>
                <p className="text-lg font-medium">Bitbucket Not Connected</p>
                <p className="text-sm text-muted-foreground">
                  Add an App Password to deploy from Bitbucket repositories.
                </p>
              </div>
              <Button onClick={() => setAddDialogOpen(true)} className="gap-2">
                <Key className="h-4 w-4" />
                Add App Password
              </Button>
            </div>
          )}
        </CardContent>
      </Card>

      {/* How to create App Password */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">How to Create a Bitbucket App Password</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm text-muted-foreground">
          <ol className="list-decimal list-inside space-y-2">
            <li>Go to Bitbucket → Personal Settings → App passwords</li>
            <li>Click "Create app password"</li>
            <li>Give it a label like "Rivetr"</li>
            <li>Select permissions: <code className="bg-muted px-1 rounded">Repositories: Read</code>, <code className="bg-muted px-1 rounded">Account: Read</code></li>
            <li>Click "Create"</li>
            <li>Copy the password and paste it here</li>
          </ol>
          <a
            href="https://bitbucket.org/account/settings/app-passwords/"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 text-primary hover:underline"
          >
            <ExternalLink className="h-3 w-3" />
            Open Bitbucket App Passwords
          </a>
        </CardContent>
      </Card>

      {/* Add Bitbucket Dialog */}
      <Dialog open={addDialogOpen} onOpenChange={setAddDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <BitbucketIcon className="h-5 w-5 text-blue-600" />
              Connect Bitbucket
            </DialogTitle>
            <DialogDescription>
              Enter your Bitbucket username and App Password to connect your repositories.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <div className="space-y-2">
              <Label htmlFor="bitbucket-username">Username</Label>
              <Input
                id="bitbucket-username"
                placeholder="your-username"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="bitbucket-password">App Password</Label>
              <Input
                id="bitbucket-password"
                type="password"
                placeholder="xxxx-xxxx-xxxx-xxxx"
                value={appPassword}
                onChange={(e) => setAppPassword(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                Required permissions: Repository Read, Account Read
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAddDialogOpen(false)}>Cancel</Button>
            <Button onClick={handleAddProvider} disabled={isAdding || !username.trim() || !appPassword.trim()} className="gap-2">
              {isAdding ? <Loader2 className="h-4 w-4 animate-spin" /> : <Link2 className="h-4 w-4" />}
              Connect
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <AlertDialog open={!!deleteProvider} onOpenChange={() => setDeleteProvider(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Disconnect Bitbucket</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to disconnect {deleteProvider?.username}? You will lose access to private repositories.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => deleteProvider && deleteMutation.mutate(deleteProvider.id)}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Disconnect
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

// ============================================================================
// Main Page Component
// ============================================================================
export default function GitIntegrationsPage() {
  const [searchParams] = useSearchParams();
  const defaultTab = searchParams.get("tab") || "github";

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Git Integrations</h1>
        <p className="text-muted-foreground">
          Connect Git providers to deploy from your repositories automatically.
        </p>
      </div>

      <Tabs defaultValue={defaultTab} className="w-full">
        <TabsList className="grid w-full grid-cols-3">
          <TabsTrigger value="github" className="gap-2">
            <Github className="h-4 w-4" />
            GitHub
          </TabsTrigger>
          <TabsTrigger value="gitlab" className="gap-2">
            <GitLabIcon className="h-4 w-4" />
            GitLab
          </TabsTrigger>
          <TabsTrigger value="bitbucket" className="gap-2">
            <BitbucketIcon className="h-4 w-4" />
            Bitbucket
          </TabsTrigger>
        </TabsList>

        <TabsContent value="github" className="mt-6">
          <GitHubAppsTab />
        </TabsContent>

        <TabsContent value="gitlab" className="mt-6">
          <GitLabTab />
        </TabsContent>

        <TabsContent value="bitbucket" className="mt-6">
          <BitbucketTab />
        </TabsContent>
      </Tabs>
    </div>
  );
}
