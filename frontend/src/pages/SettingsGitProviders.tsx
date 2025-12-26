import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
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
import { Github, GitlabIcon, ExternalLink, Loader2 } from "lucide-react";
import { api } from "@/lib/api";
import type { GitProvider, GitProviderType } from "@/types/api";

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

function getProviderIcon(provider: GitProviderType) {
  switch (provider) {
    case "github":
      return <Github className="h-5 w-5" />;
    case "gitlab":
      return <GitlabIcon className="h-5 w-5" />;
    case "bitbucket":
      return (
        <svg className="h-5 w-5" viewBox="0 0 24 24" fill="currentColor">
          <path d="M.778 1.213a.768.768 0 00-.768.892l3.263 19.81c.084.5.515.868 1.022.873H19.95a.772.772 0 00.77-.646l3.27-20.03a.768.768 0 00-.768-.891zM14.52 15.53H9.522L8.17 8.466h7.561z" />
        </svg>
      );
    default:
      return null;
  }
}

function getProviderLabel(provider: GitProviderType) {
  switch (provider) {
    case "github":
      return "GitHub";
    case "gitlab":
      return "GitLab";
    case "bitbucket":
      return "Bitbucket";
    default:
      return provider;
  }
}

export function SettingsGitProvidersPage() {
  const queryClient = useQueryClient();
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [selectedProvider, setSelectedProvider] = useState<GitProvider | null>(null);
  const [connectingProvider, setConnectingProvider] = useState<GitProviderType | null>(null);

  const { data: providers = [], isLoading } = useQuery<GitProvider[]>({
    queryKey: ["git-providers"],
    queryFn: () => api.getGitProviders(),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deleteGitProvider(id),
    onSuccess: () => {
      toast.success("Git provider disconnected");
      queryClient.invalidateQueries({ queryKey: ["git-providers"] });
      setShowDeleteDialog(false);
      setSelectedProvider(null);
    },
    onError: (error: Error) => {
      toast.error(`Failed to disconnect: ${error.message}`);
    },
  });

  const handleConnect = async (provider: GitProviderType) => {
    setConnectingProvider(provider);
    try {
      const { authorization_url } = await api.getOAuthAuthorizationUrl(provider);
      // Redirect to OAuth provider
      window.location.href = authorization_url;
    } catch (error) {
      toast.error(`Failed to connect ${getProviderLabel(provider)}: ${error instanceof Error ? error.message : "Unknown error"}`);
      setConnectingProvider(null);
    }
  };

  const connectedProviders = new Set(providers.map((p) => p.provider));

  const availableProviders: { type: GitProviderType; label: string; description: string }[] = [
    {
      type: "github",
      label: "GitHub",
      description: "Connect to GitHub to deploy from your repositories",
    },
    {
      type: "gitlab",
      label: "GitLab",
      description: "Connect to GitLab to deploy from your repositories",
    },
    {
      type: "bitbucket",
      label: "Bitbucket",
      description: "Connect to Bitbucket to deploy from your repositories",
    },
  ];

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Git Providers</h1>
        <p className="text-muted-foreground">
          Connect your Git accounts to deploy applications directly from your repositories
        </p>
      </div>

      {/* Available Providers */}
      <Card>
        <CardHeader>
          <CardTitle>Connect a Provider</CardTitle>
          <CardDescription>
            Link your Git hosting accounts to easily deploy from your repositories and enable CI/CD.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-3">
            {availableProviders.map(({ type, label, description }) => {
              const isConnected = connectedProviders.has(type);
              const isConnecting = connectingProvider === type;

              return (
                <Card key={type} className="relative">
                  <CardContent className="pt-6">
                    <div className="flex flex-col items-center text-center space-y-4">
                      <div className="p-3 rounded-full bg-muted">
                        {getProviderIcon(type)}
                      </div>
                      <div>
                        <h3 className="font-semibold">{label}</h3>
                        <p className="text-sm text-muted-foreground mt-1">
                          {description}
                        </p>
                      </div>
                      {isConnected ? (
                        <Badge variant="secondary">Connected</Badge>
                      ) : (
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => handleConnect(type)}
                          disabled={isConnecting}
                        >
                          {isConnecting ? (
                            <>
                              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                              Connecting...
                            </>
                          ) : (
                            "Connect"
                          )}
                        </Button>
                      )}
                    </div>
                  </CardContent>
                </Card>
              );
            })}
          </div>
        </CardContent>
      </Card>

      {/* Connected Providers */}
      <Card>
        <CardHeader>
          <CardTitle>Connected Accounts</CardTitle>
          <CardDescription>
            Manage your connected Git provider accounts.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {[1, 2].map((i) => (
                <Skeleton key={i} className="h-16 w-full" />
              ))}
            </div>
          ) : providers.length === 0 ? (
            <p className="text-muted-foreground py-4 text-center">
              No Git providers connected. Connect one above to get started.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Provider</TableHead>
                  <TableHead>Account</TableHead>
                  <TableHead>Connected</TableHead>
                  <TableHead className="w-24">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {providers.map((provider) => (
                  <TableRow key={provider.id}>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        {getProviderIcon(provider.provider as GitProviderType)}
                        <span className="font-medium">
                          {getProviderLabel(provider.provider as GitProviderType)}
                        </span>
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-3">
                        <Avatar className="h-8 w-8">
                          <AvatarImage src={provider.avatar_url || undefined} />
                          <AvatarFallback>
                            {provider.username.charAt(0).toUpperCase()}
                          </AvatarFallback>
                        </Avatar>
                        <div>
                          <div className="font-medium">
                            {provider.display_name || provider.username}
                          </div>
                          <div className="text-sm text-muted-foreground">
                            @{provider.username}
                          </div>
                        </div>
                        <a
                          href={
                            provider.provider === "github"
                              ? `https://github.com/${provider.username}`
                              : provider.provider === "gitlab"
                              ? `https://gitlab.com/${provider.username}`
                              : `https://bitbucket.org/${provider.username}`
                          }
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-muted-foreground hover:text-foreground"
                        >
                          <ExternalLink className="h-4 w-4" />
                        </a>
                      </div>
                    </TableCell>
                    <TableCell>{formatDate(provider.created_at)}</TableCell>
                    <TableCell>
                      <Button
                        variant="destructive"
                        size="sm"
                        onClick={() => {
                          setSelectedProvider(provider);
                          setShowDeleteDialog(true);
                        }}
                      >
                        Disconnect
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Delete Confirmation Dialog */}
      <Dialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Disconnect Git Provider</DialogTitle>
            <DialogDescription>
              Are you sure you want to disconnect{" "}
              {selectedProvider && getProviderLabel(selectedProvider.provider as GitProviderType)}?
              You will need to re-authorize to use it again.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowDeleteDialog(false);
                setSelectedProvider(null);
              }}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                if (selectedProvider) {
                  deleteMutation.mutate(selectedProvider.id);
                }
              }}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? "Disconnecting..." : "Disconnect"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
