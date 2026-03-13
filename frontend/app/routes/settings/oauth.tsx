import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
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
  Github,
  Loader2,
  Trash2,
  Shield,
  CheckCircle,
  ExternalLink,
  Unlink,
} from "lucide-react";
import {
  oauthApi,
  type OAuthProviderResponse,
  type UserOAuthConnection,
} from "@/lib/api/oauth";

export function meta() {
  return [
    { title: "Authentication - Rivetr" },
    {
      name: "description",
      content: "Configure OAuth login providers for your Rivetr instance",
    },
  ];
}

// Google icon SVG
const GoogleIcon = ({ className }: { className?: string }) => (
  <svg className={className} viewBox="0 0 24 24">
    <path
      d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92a5.06 5.06 0 01-2.2 3.32v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.1z"
      fill="#4285F4"
    />
    <path
      d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
      fill="#34A853"
    />
    <path
      d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
      fill="#FBBC05"
    />
    <path
      d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
      fill="#EA4335"
    />
  </svg>
);

// GitLab icon SVG
const GitLabIcon = ({ className }: { className?: string }) => (
  <svg className={className} viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
    <path d="M22.65 14.39L12 22.13 1.35 14.39a.84.84 0 0 1-.3-.94l1.22-3.78 2.44-7.51A.42.42 0 0 1 4.82 2a.43.43 0 0 1 .58 0 .42.42 0 0 1 .11.18l2.44 7.49h8.1l2.44-7.51A.42.42 0 0 1 18.6 2a.43.43 0 0 1 .58 0 .42.42 0 0 1 .11.18l2.44 7.51 1.22 3.78a.84.84 0 0 1-.3.92z" fill="#E24329"/>
    <path d="M12 22.13L7.26 7.67h9.48L12 22.13z" fill="#FC6D26"/>
    <path d="M12 22.13l-4.74-14.46H1.05L12 22.13z" fill="#FCA326"/>
    <path d="M1.05 7.67L-.17 11.45a.84.84 0 0 0 .3.94L12 22.13z" fill="#E24329"/>
    <path d="M1.05 7.67h6.21L4.82 0a.43.43 0 0 0-.82 0z" fill="#FC6D26"/>
    <path d="M12 22.13l4.74-14.46h6.21L12 22.13z" fill="#FCA326"/>
    <path d="M22.95 7.67l1.22 3.78a.84.84 0 0 1-.3.94L12 22.13z" fill="#E24329"/>
    <path d="M22.95 7.67h-6.21L19.18 0a.43.43 0 0 1 .82 0z" fill="#FC6D26"/>
  </svg>
);

// Microsoft icon SVG
const MicrosoftIcon = ({ className }: { className?: string }) => (
  <svg className={className} viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
    <path d="M0 0h11.5v11.5H0z" fill="#F25022"/>
    <path d="M12.5 0H24v11.5H12.5z" fill="#7FBA00"/>
    <path d="M0 12.5h11.5V24H0z" fill="#00A4EF"/>
    <path d="M12.5 12.5H24V24H12.5z" fill="#FFB900"/>
  </svg>
);

// Bitbucket icon SVG
const BitbucketIcon = ({ className }: { className?: string }) => (
  <svg className={className} viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
    <path
      d="M.778 1.213a.768.768 0 00-.768.892l3.263 19.81c.084.5.515.868 1.022.873H19.95a.772.772 0 00.77-.646l3.27-20.03a.768.768 0 00-.768-.891L.778 1.213zM14.52 15.53H9.522L8.17 8.471h7.561l-1.211 7.06z"
      fill="#2684FF"
    />
  </svg>
);

function ProviderConfigCard({
  provider,
  icon,
  title,
  description,
  docsUrl,
  docsLabel,
  existingConfig,
  onSave,
  onDelete,
  isSaving,
  extraFields,
  onGetExtraConfig,
}: {
  provider: string;
  icon: React.ReactNode;
  title: string;
  description: string;
  docsUrl: string;
  docsLabel: string;
  existingConfig: OAuthProviderResponse | undefined;
  onSave: (data: {
    provider: string;
    client_id: string;
    client_secret: string;
    enabled: boolean;
    extra_config?: string;
  }) => void;
  onDelete: (id: string) => void;
  isSaving: boolean;
  extraFields?: React.ReactNode;
  onGetExtraConfig?: () => string | undefined;
}) {
  const [clientId, setClientId] = useState(existingConfig?.client_id || "");
  const [clientSecret, setClientSecret] = useState("");
  const [enabled, setEnabled] = useState(existingConfig?.enabled ?? true);
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);

  const isConfigured = !!existingConfig;

  const handleSave = () => {
    if (!clientId.trim()) {
      toast.error("Client ID is required");
      return;
    }
    if (!clientSecret.trim() && !isConfigured) {
      toast.error("Client Secret is required");
      return;
    }
    onSave({
      provider,
      client_id: clientId.trim(),
      client_secret: clientSecret.trim() || "unchanged",
      enabled,
      extra_config: onGetExtraConfig?.(),
    });
    setClientSecret("");
  };

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                {icon}
                {title}
                {isConfigured && (
                  <Badge
                    variant={existingConfig.enabled ? "default" : "secondary"}
                    className="ml-2"
                  >
                    {existingConfig.enabled ? "Enabled" : "Disabled"}
                  </Badge>
                )}
              </CardTitle>
              <CardDescription>{description}</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor={`${provider}-client-id`}>Client ID</Label>
            <Input
              id={`${provider}-client-id`}
              placeholder="Your OAuth Client ID"
              value={clientId}
              onChange={(e) => setClientId(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor={`${provider}-client-secret`}>Client Secret</Label>
            <Input
              id={`${provider}-client-secret`}
              type="password"
              placeholder={
                isConfigured
                  ? "Leave blank to keep existing secret"
                  : "Your OAuth Client Secret"
              }
              value={clientSecret}
              onChange={(e) => setClientSecret(e.target.value)}
            />
          </div>
          {extraFields}
          <div className="flex items-center gap-2">
            <Switch
              id={`${provider}-enabled`}
              checked={enabled}
              onCheckedChange={setEnabled}
            />
            <Label htmlFor={`${provider}-enabled`}>
              Enable {title} login
            </Label>
          </div>
          <div className="flex items-center justify-between pt-2">
            <a
              href={docsUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-sm text-primary hover:underline"
            >
              <ExternalLink className="h-3 w-3" />
              {docsLabel}
            </a>
            <div className="flex items-center gap-2">
              {isConfigured && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setDeleteConfirmOpen(true)}
                  className="text-destructive hover:text-destructive"
                >
                  <Trash2 className="h-4 w-4 mr-1" />
                  Remove
                </Button>
              )}
              <Button onClick={handleSave} disabled={isSaving}>
                {isSaving ? (
                  <Loader2 className="h-4 w-4 animate-spin mr-2" />
                ) : null}
                {isConfigured ? "Update" : "Save"}
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      <AlertDialog
        open={deleteConfirmOpen}
        onOpenChange={setDeleteConfirmOpen}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove {title} OAuth</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to remove {title} OAuth configuration? Users
              will no longer be able to sign in with {title}.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (existingConfig) onDelete(existingConfig.id);
                setDeleteConfirmOpen(false);
                setClientId("");
                setClientSecret("");
              }}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Remove
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

function AzureProviderCard({
  existingConfig,
  onSave,
  onDelete,
  isSaving,
}: {
  existingConfig: OAuthProviderResponse | undefined;
  onSave: (data: {
    provider: string;
    client_id: string;
    client_secret: string;
    enabled: boolean;
    extra_config?: string;
  }) => void;
  onDelete: (id: string) => void;
  isSaving: boolean;
}) {
  // Parse existing tenant_id from extra_config
  const existingTenantId = (() => {
    if (!existingConfig?.extra_config) return "";
    try {
      const parsed = JSON.parse(existingConfig.extra_config);
      return parsed.tenant_id ?? "";
    } catch {
      return "";
    }
  })();

  const [tenantId, setTenantId] = useState(existingTenantId);

  const getExtraConfig = () => {
    if (!tenantId.trim()) return undefined;
    return JSON.stringify({ tenant_id: tenantId.trim() });
  };

  return (
    <ProviderConfigCard
      provider="azure"
      icon={<MicrosoftIcon className="h-5 w-5" />}
      title="Microsoft Entra"
      description="Allow users to sign in with their Microsoft / Azure AD accounts."
      docsUrl="https://learn.microsoft.com/en-us/entra/identity-platform/quickstart-register-app"
      docsLabel="Microsoft Entra App Registration Docs"
      existingConfig={existingConfig}
      onSave={onSave}
      onDelete={onDelete}
      isSaving={isSaving}
      onGetExtraConfig={getExtraConfig}
      extraFields={
        <div className="space-y-2">
          <Label htmlFor="azure-tenant-id">Tenant ID (optional)</Label>
          <Input
            id="azure-tenant-id"
            placeholder="common (leave blank for multi-tenant)"
            value={tenantId}
            onChange={(e) => setTenantId(e.target.value)}
          />
          <p className="text-xs text-muted-foreground">
            Leave blank to allow any Microsoft account. Enter a specific tenant
            ID to restrict to a single Azure AD directory.
          </p>
        </div>
      }
    />
  );
}

function OAuthConnectionsCard() {
  const queryClient = useQueryClient();
  const [unlinkId, setUnlinkId] = useState<string | null>(null);

  const { data: connections = [], isLoading } = useQuery<UserOAuthConnection[]>(
    {
      queryKey: ["oauth-connections"],
      queryFn: () => oauthApi.getOAuthConnections(),
    }
  );

  const unlinkMutation = useMutation({
    mutationFn: (id: string) => oauthApi.deleteOAuthConnection(id),
    onSuccess: () => {
      toast.success("OAuth account unlinked");
      queryClient.invalidateQueries({ queryKey: ["oauth-connections"] });
      setUnlinkId(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to unlink account");
    },
  });

  const getProviderIcon = (provider: string) => {
    switch (provider) {
      case "github":
        return <Github className="h-5 w-5" />;
      case "google":
        return <GoogleIcon className="h-5 w-5" />;
      case "gitlab":
        return <GitLabIcon className="h-5 w-5" />;
      case "azure":
        return <MicrosoftIcon className="h-5 w-5" />;
      case "bitbucket":
        return <BitbucketIcon className="h-5 w-5" />;
      default:
        return <Shield className="h-5 w-5" />;
    }
  };

  const getProviderLabel = (provider: string) => {
    switch (provider) {
      case "github":
        return "GitHub";
      case "google":
        return "Google";
      case "gitlab":
        return "GitLab";
      case "azure":
        return "Microsoft";
      case "bitbucket":
        return "Bitbucket";
      default:
        return provider;
    }
  };

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            Connected Accounts
          </CardTitle>
          <CardDescription>
            OAuth accounts linked to your Rivetr account. You can use these to
            sign in.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : connections.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <p>No OAuth accounts connected.</p>
              <p className="text-sm mt-1">
                Sign in with an OAuth provider to automatically link it to your
                account.
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {connections.map((conn) => (
                <div
                  key={conn.id}
                  className="flex items-center justify-between p-3 border rounded-lg"
                >
                  <div className="flex items-center gap-3">
                    {getProviderIcon(conn.provider)}
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-medium">
                          {getProviderLabel(conn.provider)}
                        </span>
                        <Badge
                          variant="outline"
                          className="gap-1 text-green-600 border-green-300"
                        >
                          <CheckCircle className="h-3 w-3" />
                          Connected
                        </Badge>
                      </div>
                      <p className="text-sm text-muted-foreground">
                        {conn.provider_email || conn.provider_name || conn.provider_user_id}
                      </p>
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setUnlinkId(conn.id)}
                    className="text-muted-foreground hover:text-destructive"
                  >
                    <Unlink className="h-4 w-4" />
                  </Button>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <AlertDialog open={!!unlinkId} onOpenChange={() => setUnlinkId(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Unlink OAuth Account</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to unlink this OAuth account? You can always
              reconnect it later by signing in with the provider.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => unlinkId && unlinkMutation.mutate(unlinkId)}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Unlink
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

export default function OAuthSettingsPage() {
  const queryClient = useQueryClient();

  const { data: providers = [], isLoading } = useQuery<OAuthProviderResponse[]>(
    {
      queryKey: ["oauth-providers"],
      queryFn: () => oauthApi.getOAuthProviders(),
    }
  );

  const providerNames: Record<string, string> = {
    github: "GitHub",
    google: "Google",
    gitlab: "GitLab",
    azure: "Microsoft Entra",
    bitbucket: "Bitbucket",
  };

  const saveMutation = useMutation({
    mutationFn: (data: {
      provider: string;
      client_id: string;
      client_secret: string;
      enabled: boolean;
      extra_config?: string;
    }) => oauthApi.createOAuthProvider(data),
    onSuccess: (_, variables) => {
      toast.success(
        `${providerNames[variables.provider] ?? variables.provider} OAuth configuration saved`
      );
      queryClient.invalidateQueries({ queryKey: ["oauth-providers"] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to save OAuth configuration");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => oauthApi.deleteOAuthProvider(id),
    onSuccess: () => {
      toast.success("OAuth provider removed");
      queryClient.invalidateQueries({ queryKey: ["oauth-providers"] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to remove OAuth provider");
    },
  });

  const githubConfig = providers.find((p) => p.provider === "github");
  const googleConfig = providers.find((p) => p.provider === "google");
  const gitlabConfig = providers.find((p) => p.provider === "gitlab");
  const azureConfig = providers.find((p) => p.provider === "azure");
  const bitbucketConfig = providers.find((p) => p.provider === "bitbucket");

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Authentication</h1>
        <p className="text-muted-foreground">
          Configure OAuth providers to allow users to sign in with their GitHub,
          Google, GitLab, Microsoft, or Bitbucket accounts.
        </p>
      </div>

      {/* OAuth Provider Configuration (Admin) */}
      <ProviderConfigCard
        provider="github"
        icon={<Github className="h-5 w-5" />}
        title="GitHub"
        description="Allow users to sign in with their GitHub accounts."
        docsUrl="https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/creating-an-oauth-app"
        docsLabel="GitHub OAuth App Docs"
        existingConfig={githubConfig}
        onSave={(data) => saveMutation.mutate(data)}
        onDelete={(id) => deleteMutation.mutate(id)}
        isSaving={saveMutation.isPending}
      />

      <ProviderConfigCard
        provider="google"
        icon={<GoogleIcon className="h-5 w-5" />}
        title="Google"
        description="Allow users to sign in with their Google accounts."
        docsUrl="https://developers.google.com/identity/protocols/oauth2/web-server#creatingcred"
        docsLabel="Google OAuth Credentials Docs"
        existingConfig={googleConfig}
        onSave={(data) => saveMutation.mutate(data)}
        onDelete={(id) => deleteMutation.mutate(id)}
        isSaving={saveMutation.isPending}
      />

      <ProviderConfigCard
        provider="gitlab"
        icon={<GitLabIcon className="h-5 w-5" />}
        title="GitLab"
        description="Allow users to sign in with their GitLab.com accounts."
        docsUrl="https://docs.gitlab.com/ee/integration/oauth_provider.html"
        docsLabel="GitLab OAuth Application Docs"
        existingConfig={gitlabConfig}
        onSave={(data) => saveMutation.mutate(data)}
        onDelete={(id) => deleteMutation.mutate(id)}
        isSaving={saveMutation.isPending}
      />

      <AzureProviderCard
        existingConfig={azureConfig}
        onSave={(data) => saveMutation.mutate(data)}
        onDelete={(id) => deleteMutation.mutate(id)}
        isSaving={saveMutation.isPending}
      />

      <ProviderConfigCard
        provider="bitbucket"
        icon={<BitbucketIcon className="h-5 w-5" />}
        title="Bitbucket"
        description="Allow users to sign in with their Bitbucket accounts."
        docsUrl="https://support.atlassian.com/bitbucket-cloud/docs/use-oauth-on-bitbucket-cloud/"
        docsLabel="Bitbucket OAuth Consumer Docs"
        existingConfig={bitbucketConfig}
        onSave={(data) => saveMutation.mutate(data)}
        onDelete={(id) => deleteMutation.mutate(id)}
        isSaving={saveMutation.isPending}
      />

      {/* Connected OAuth Accounts (User) */}
      <OAuthConnectionsCard />
    </div>
  );
}
