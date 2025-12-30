import { useState, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useSearchParams } from "react-router";
import { toast } from "sonner";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { api } from "@/lib/api";
import type { GitProvider, GitProviderType } from "@/types/api";
import { Github, ExternalLink, Trash2, Loader2, CheckCircle2, GitBranch } from "lucide-react";

// Provider configurations
const PROVIDERS: {
  type: GitProviderType;
  name: string;
  icon: React.ReactNode;
  color: string;
  description: string;
}[] = [
  {
    type: "github",
    name: "GitHub",
    icon: <Github className="h-5 w-5" />,
    color: "bg-gray-900 hover:bg-gray-800 text-white",
    description: "Connect to GitHub repositories",
  },
  {
    type: "gitlab",
    name: "GitLab",
    icon: (
      <svg className="h-5 w-5" viewBox="0 0 24 24" fill="currentColor">
        <path d="M22.65 14.39L12 22.13 1.35 14.39a.84.84 0 01-.3-.94l1.22-3.78 2.44-7.51A.42.42 0 014.82 2a.43.43 0 01.58 0 .42.42 0 01.11.18l2.44 7.49h8.1l2.44-7.51A.42.42 0 0118.6 2a.43.43 0 01.58 0 .42.42 0 01.11.18l2.44 7.51L23 13.45a.84.84 0 01-.35.94z" />
      </svg>
    ),
    color: "bg-orange-600 hover:bg-orange-500 text-white",
    description: "Connect to GitLab repositories",
  },
  {
    type: "bitbucket",
    name: "Bitbucket",
    icon: (
      <svg className="h-5 w-5" viewBox="0 0 24 24" fill="currentColor">
        <path d="M.778 1.213a.768.768 0 00-.768.892l3.263 19.81c.084.5.515.868 1.022.873H19.95a.772.772 0 00.77-.646l3.27-20.03a.768.768 0 00-.768-.889zM14.52 15.53H9.522L8.17 8.466h7.561z" />
      </svg>
    ),
    color: "bg-blue-600 hover:bg-blue-500 text-white",
    description: "Connect to Bitbucket repositories",
  },
];

function ProviderCard({
  provider,
  onDisconnect,
  isDisconnecting,
}: {
  provider: GitProvider;
  onDisconnect: () => void;
  isDisconnecting: boolean;
}) {
  const config = PROVIDERS.find((p) => p.type === provider.provider);
  if (!config) return null;

  return (
    <div className="flex items-center justify-between p-4 border rounded-lg">
      <div className="flex items-center gap-4">
        <Avatar className="h-12 w-12">
          <AvatarImage src={provider.avatar_url || undefined} alt={provider.username} />
          <AvatarFallback>{provider.username?.slice(0, 2).toUpperCase()}</AvatarFallback>
        </Avatar>
        <div>
          <div className="flex items-center gap-2">
            <span className="font-medium">{provider.display_name || provider.username}</span>
            <Badge variant="outline" className="gap-1 text-xs">
              {config.icon}
              {config.name}
            </Badge>
          </div>
          <p className="text-sm text-muted-foreground">@{provider.username}</p>
          {provider.email && (
            <p className="text-sm text-muted-foreground">{provider.email}</p>
          )}
          {provider.scopes && (
            <p className="text-xs text-muted-foreground mt-1">
              Scopes: {provider.scopes}
            </p>
          )}
        </div>
      </div>
      <Button
        variant="ghost"
        size="sm"
        onClick={onDisconnect}
        disabled={isDisconnecting}
        className="text-red-600 hover:text-red-700 hover:bg-red-50"
      >
        {isDisconnecting ? (
          <Loader2 className="h-4 w-4 animate-spin" />
        ) : (
          <Trash2 className="h-4 w-4" />
        )}
      </Button>
    </div>
  );
}

function ConnectProviderButton({
  providerType,
  name,
  icon,
  color,
  description,
  isConnected,
  isConnecting,
  onConnect,
}: {
  providerType: GitProviderType;
  name: string;
  icon: React.ReactNode;
  color: string;
  description: string;
  isConnected: boolean;
  isConnecting: boolean;
  onConnect: () => void;
}) {
  return (
    <div className="flex items-center justify-between p-4 border rounded-lg">
      <div className="flex items-center gap-3">
        <div className={`p-2 rounded-lg ${color}`}>{icon}</div>
        <div>
          <div className="flex items-center gap-2">
            <span className="font-medium">{name}</span>
            {isConnected && (
              <Badge variant="outline" className="text-green-600 border-green-300">
                <CheckCircle2 className="h-3 w-3 mr-1" />
                Connected
              </Badge>
            )}
          </div>
          <p className="text-sm text-muted-foreground">{description}</p>
        </div>
      </div>
      {!isConnected && (
        <Button
          variant="outline"
          onClick={onConnect}
          disabled={isConnecting}
          className="gap-2"
        >
          {isConnecting ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <ExternalLink className="h-4 w-4" />
          )}
          Connect
        </Button>
      )}
    </div>
  );
}

export default function SettingsGitProvidersPage() {
  const queryClient = useQueryClient();
  const [searchParams, setSearchParams] = useSearchParams();
  const [connectingProvider, setConnectingProvider] = useState<GitProviderType | null>(null);
  const [disconnectDialog, setDisconnectDialog] = useState<GitProvider | null>(null);

  // Check for success message from OAuth callback
  useEffect(() => {
    if (searchParams.get("connected") === "true") {
      toast.success("Git provider connected successfully!");
      // Remove the query param
      setSearchParams({}, { replace: true });
      // Refresh providers
      queryClient.invalidateQueries({ queryKey: ["gitProviders"] });
    }
  }, [searchParams, setSearchParams, queryClient]);

  // Fetch connected providers
  const { data: providers = [], isLoading } = useQuery<GitProvider[]>({
    queryKey: ["gitProviders"],
    queryFn: () => api.getGitProviders(),
  });

  // Disconnect mutation
  const disconnectMutation = useMutation({
    mutationFn: (id: string) => api.deleteGitProvider(id),
    onSuccess: () => {
      toast.success("Git provider disconnected");
      queryClient.invalidateQueries({ queryKey: ["gitProviders"] });
      setDisconnectDialog(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to disconnect provider");
    },
  });

  // Handle connect
  const handleConnect = async (providerType: GitProviderType) => {
    setConnectingProvider(providerType);
    try {
      const response = await api.getGitProviderAuthUrl(providerType);
      // Redirect to OAuth authorization URL
      window.location.href = response.authorization_url;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to initiate OAuth");
      setConnectingProvider(null);
    }
  };

  // Check which providers are connected
  const connectedTypes = new Set(providers.map((p) => p.provider));

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Git Providers</h1>
          <p className="text-muted-foreground">
            Connect Git providers for OAuth authentication to access repositories
          </p>
        </div>
      </div>

      {/* Available Providers */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <GitBranch className="h-5 w-5" />
            Available Providers
          </CardTitle>
          <CardDescription>
            Connect your Git provider accounts to access private repositories without SSH keys.
            OAuth connections also enable features like automatic webhook setup.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {PROVIDERS.map((provider) => (
            <ConnectProviderButton
              key={provider.type}
              providerType={provider.type}
              name={provider.name}
              icon={provider.icon}
              color={provider.color}
              description={provider.description}
              isConnected={connectedTypes.has(provider.type)}
              isConnecting={connectingProvider === provider.type}
              onConnect={() => handleConnect(provider.type)}
            />
          ))}
        </CardContent>
      </Card>

      {/* Connected Providers */}
      <Card>
        <CardHeader>
          <CardTitle>Connected Accounts</CardTitle>
          <CardDescription>
            Your connected Git provider accounts and their permissions.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : providers.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <GitBranch className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>No Git providers connected yet.</p>
              <p className="text-sm">Connect a provider above to get started.</p>
            </div>
          ) : (
            <div className="space-y-3">
              {providers.map((provider) => (
                <ProviderCard
                  key={provider.id}
                  provider={provider}
                  onDisconnect={() => setDisconnectDialog(provider)}
                  isDisconnecting={disconnectMutation.isPending}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* OAuth Configuration Note */}
      <Card>
        <CardHeader>
          <CardTitle>OAuth Configuration</CardTitle>
          <CardDescription>
            To enable Git provider connections, OAuth must be configured in your Rivetr server configuration.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-sm text-muted-foreground space-y-2">
            <p>Add the following to your <code className="bg-muted px-1 rounded">rivetr.toml</code>:</p>
            <pre className="bg-muted p-3 rounded-lg overflow-x-auto text-xs">
{`[oauth.github]
client_id = "your-github-client-id"
client_secret = "your-github-client-secret"
redirect_uri = "https://your-domain.com/api/auth/oauth/github/callback"

[oauth.gitlab]
client_id = "your-gitlab-client-id"
client_secret = "your-gitlab-client-secret"
redirect_uri = "https://your-domain.com/api/auth/oauth/gitlab/callback"

[oauth.bitbucket]
client_id = "your-bitbucket-client-id"
client_secret = "your-bitbucket-client-secret"
redirect_uri = "https://your-domain.com/api/auth/oauth/bitbucket/callback"`}
            </pre>
          </div>
        </CardContent>
      </Card>

      {/* Disconnect Confirmation Dialog */}
      <Dialog open={!!disconnectDialog} onOpenChange={() => setDisconnectDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Disconnect Git Provider</DialogTitle>
            <DialogDescription>
              Are you sure you want to disconnect {disconnectDialog?.display_name || disconnectDialog?.username} ({disconnectDialog?.provider})?
              You will lose access to private repositories linked through this provider.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDisconnectDialog(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => disconnectDialog && disconnectMutation.mutate(disconnectDialog.id)}
              disabled={disconnectMutation.isPending}
            >
              {disconnectMutation.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin mr-2" />
              ) : null}
              Disconnect
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
