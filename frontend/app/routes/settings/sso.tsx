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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Loader2,
  Plus,
  Trash2,
  ExternalLink,
  Shield,
  CheckCircle,
  XCircle,
} from "lucide-react";
import {
  ssoApi,
  type OidcProvider,
  type CreateOidcProviderRequest,
  WELL_KNOWN_PROVIDERS,
} from "@/lib/api/sso";

export function meta() {
  return [
    { title: "SSO / OIDC - Rivetr" },
    {
      name: "description",
      content: "Configure OpenID Connect providers for enterprise single sign-on",
    },
  ];
}

function ProviderCard({
  provider,
  onDelete,
  onUpdate,
}: {
  provider: OidcProvider;
  onDelete: (id: string) => void;
  onUpdate: (id: string, data: Partial<CreateOidcProviderRequest>) => void;
}) {
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [editOpen, setEditOpen] = useState(false);
  const [name, setName] = useState(provider.name);
  const [clientId, setClientId] = useState(provider.client_id);
  const [clientSecret, setClientSecret] = useState("");
  const [discoveryUrl, setDiscoveryUrl] = useState(provider.discovery_url);
  const [scopes, setScopes] = useState(provider.scopes);
  const [enabled, setEnabled] = useState(provider.enabled);

  const handleSave = () => {
    if (!name.trim() || !clientId.trim() || !discoveryUrl.trim()) {
      toast.error("Name, Client ID, and Discovery URL are required");
      return;
    }
    onUpdate(provider.id, {
      name: name.trim(),
      client_id: clientId.trim(),
      client_secret: clientSecret.trim() || "unchanged",
      discovery_url: discoveryUrl.trim(),
      scopes: scopes.trim(),
      enabled,
    });
    setEditOpen(false);
    setClientSecret("");
  };

  return (
    <>
      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex items-center gap-3">
          <Shield className="h-5 w-5 text-muted-foreground" />
          <div>
            <div className="flex items-center gap-2">
              <span className="font-medium">{provider.name}</span>
              <Badge
                variant={provider.enabled ? "default" : "secondary"}
                className="gap-1"
              >
                {provider.enabled ? (
                  <>
                    <CheckCircle className="h-3 w-3" />
                    Enabled
                  </>
                ) : (
                  <>
                    <XCircle className="h-3 w-3" />
                    Disabled
                  </>
                )}
              </Badge>
            </div>
            <p className="text-sm text-muted-foreground truncate max-w-md">
              {provider.discovery_url}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() =>
              window.open(ssoApi.getLoginUrl(provider.id), "_blank")
            }
          >
            <ExternalLink className="h-4 w-4 mr-1" />
            Test Login
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setEditOpen(true)}
          >
            Edit
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setDeleteOpen(true)}
            className="text-destructive hover:text-destructive"
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Edit Dialog */}
      <Dialog open={editOpen} onOpenChange={setEditOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>Edit OIDC Provider</DialogTitle>
            <DialogDescription>
              Update the configuration for this SSO provider.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="edit-name">Provider Name</Label>
              <Input
                id="edit-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g. Acme Corp SSO"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-discovery-url">Discovery URL</Label>
              <Input
                id="edit-discovery-url"
                value={discoveryUrl}
                onChange={(e) => setDiscoveryUrl(e.target.value)}
                placeholder="https://example.com/.well-known/openid-configuration"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-client-id">Client ID</Label>
              <Input
                id="edit-client-id"
                value={clientId}
                onChange={(e) => setClientId(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-client-secret">
                Client Secret{" "}
                <span className="text-muted-foreground text-xs">
                  (leave blank to keep existing)
                </span>
              </Label>
              <Input
                id="edit-client-secret"
                type="password"
                value={clientSecret}
                onChange={(e) => setClientSecret(e.target.value)}
                placeholder="Leave blank to keep existing secret"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-scopes">Scopes</Label>
              <Input
                id="edit-scopes"
                value={scopes}
                onChange={(e) => setScopes(e.target.value)}
                placeholder="openid email profile"
              />
            </div>
            <div className="flex items-center gap-2">
              <Switch
                id="edit-enabled"
                checked={enabled}
                onCheckedChange={setEnabled}
              />
              <Label htmlFor="edit-enabled">Enable this provider</Label>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditOpen(false)}>
              Cancel
            </Button>
            <Button onClick={handleSave}>Save Changes</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Dialog */}
      <AlertDialog open={deleteOpen} onOpenChange={setDeleteOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove OIDC Provider</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to remove &ldquo;{provider.name}&rdquo;?
              Users will no longer be able to sign in using this SSO provider.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                onDelete(provider.id);
                setDeleteOpen(false);
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

function AddProviderDialog({
  open,
  onOpenChange,
  onCreate,
  isSaving,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreate: (data: CreateOidcProviderRequest) => void;
  isSaving: boolean;
}) {
  const [name, setName] = useState("");
  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const [discoveryUrl, setDiscoveryUrl] = useState("");
  const [scopes, setScopes] = useState("openid email profile");

  const handleQuickFill = (preset: (typeof WELL_KNOWN_PROVIDERS)[number]) => {
    setDiscoveryUrl(preset.discovery_url);
    setScopes(preset.scopes);
    if (!name) setName(preset.label);
  };

  const handleSubmit = () => {
    if (!name.trim()) {
      toast.error("Provider name is required");
      return;
    }
    if (!clientId.trim()) {
      toast.error("Client ID is required");
      return;
    }
    if (!clientSecret.trim()) {
      toast.error("Client Secret is required");
      return;
    }
    if (!discoveryUrl.trim()) {
      toast.error("Discovery URL is required");
      return;
    }
    onCreate({
      name: name.trim(),
      client_id: clientId.trim(),
      client_secret: clientSecret.trim(),
      discovery_url: discoveryUrl.trim(),
      scopes: scopes.trim() || "openid email profile",
      enabled: true,
    });
  };

  const handleClose = () => {
    setName("");
    setClientId("");
    setClientSecret("");
    setDiscoveryUrl("");
    setScopes("openid email profile");
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Add OIDC Provider</DialogTitle>
          <DialogDescription>
            Configure an OpenID Connect identity provider for enterprise SSO
            login.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Quick-fill buttons */}
          <div className="space-y-2">
            <Label>Quick Fill</Label>
            <div className="flex flex-wrap gap-2">
              {WELL_KNOWN_PROVIDERS.map((preset) => (
                <Button
                  key={preset.label}
                  variant="outline"
                  size="sm"
                  type="button"
                  onClick={() => handleQuickFill(preset)}
                >
                  {preset.label}
                </Button>
              ))}
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="new-name">Provider Name</Label>
            <Input
              id="new-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Acme Corp SSO"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="new-discovery-url">Discovery URL</Label>
            <Input
              id="new-discovery-url"
              value={discoveryUrl}
              onChange={(e) => setDiscoveryUrl(e.target.value)}
              placeholder="https://example.com/.well-known/openid-configuration"
            />
            <p className="text-xs text-muted-foreground">
              The OIDC discovery document URL (usually ends in
              /.well-known/openid-configuration)
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="new-client-id">Client ID</Label>
            <Input
              id="new-client-id"
              value={clientId}
              onChange={(e) => setClientId(e.target.value)}
              placeholder="Your OAuth Client ID"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="new-client-secret">Client Secret</Label>
            <Input
              id="new-client-secret"
              type="password"
              value={clientSecret}
              onChange={(e) => setClientSecret(e.target.value)}
              placeholder="Your OAuth Client Secret"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="new-scopes">Scopes</Label>
            <Input
              id="new-scopes"
              value={scopes}
              onChange={(e) => setScopes(e.target.value)}
              placeholder="openid email profile"
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={handleClose} disabled={isSaving}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={isSaving}>
            {isSaving && <Loader2 className="h-4 w-4 animate-spin mr-2" />}
            Add Provider
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default function SsoSettingsPage() {
  const queryClient = useQueryClient();
  const [addOpen, setAddOpen] = useState(false);

  const { data: providers = [], isLoading } = useQuery<OidcProvider[]>({
    queryKey: ["sso-providers"],
    queryFn: () => ssoApi.listProviders(),
  });

  const createMutation = useMutation({
    mutationFn: (data: CreateOidcProviderRequest) =>
      ssoApi.createProvider(data),
    onSuccess: () => {
      toast.success("OIDC provider added");
      queryClient.invalidateQueries({ queryKey: ["sso-providers"] });
      setAddOpen(false);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to add OIDC provider");
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({
      id,
      data,
    }: {
      id: string;
      data: Partial<CreateOidcProviderRequest>;
    }) => ssoApi.updateProvider(id, data),
    onSuccess: () => {
      toast.success("OIDC provider updated");
      queryClient.invalidateQueries({ queryKey: ["sso-providers"] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to update OIDC provider");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => ssoApi.deleteProvider(id),
    onSuccess: () => {
      toast.success("OIDC provider removed");
      queryClient.invalidateQueries({ queryKey: ["sso-providers"] });
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to remove OIDC provider");
    },
  });

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">SSO / OIDC</h1>
        <p className="text-muted-foreground">
          Configure OpenID Connect identity providers to allow enterprise users
          to sign in with their organization&apos;s SSO.
        </p>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Shield className="h-5 w-5" />
                OIDC Providers
              </CardTitle>
              <CardDescription>
                Each provider allows users to sign in using their identity from
                that organization. The callback URL for each provider will be
                shown after creation.
              </CardDescription>
            </div>
            <Button onClick={() => setAddOpen(true)}>
              <Plus className="h-4 w-4 mr-2" />
              Add Provider
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : providers.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground border-2 border-dashed rounded-lg">
              <Shield className="h-8 w-8 mx-auto mb-3 opacity-40" />
              <p className="font-medium">No OIDC providers configured</p>
              <p className="text-sm mt-1">
                Add an OpenID Connect provider to enable enterprise SSO login.
              </p>
              <Button
                variant="outline"
                className="mt-4"
                onClick={() => setAddOpen(true)}
              >
                <Plus className="h-4 w-4 mr-2" />
                Add Provider
              </Button>
            </div>
          ) : (
            <div className="space-y-3">
              {providers.map((provider) => (
                <ProviderCard
                  key={provider.id}
                  provider={provider}
                  onDelete={(id) => deleteMutation.mutate(id)}
                  onUpdate={(id, data) => updateMutation.mutate({ id, data })}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>How It Works</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm text-muted-foreground">
          <p>
            1. Add an OIDC provider with your identity provider&apos;s discovery
            URL, client ID, and client secret.
          </p>
          <p>
            2. Copy the callback URL shown for your provider (format:{" "}
            <code className="bg-muted px-1 rounded text-xs">
              /auth/sso/&#123;id&#125;/callback
            </code>
            ) and register it in your identity provider.
          </p>
          <p>
            3. Users can initiate login by visiting{" "}
            <code className="bg-muted px-1 rounded text-xs">
              /auth/sso/&#123;id&#125;/login
            </code>
            . After authentication, they are redirected to the dashboard.
          </p>
          <p>
            4. Existing users with matching email addresses will have their
            accounts automatically linked. New users will have accounts created.
          </p>
        </CardContent>
      </Card>

      <AddProviderDialog
        open={addOpen}
        onOpenChange={setAddOpen}
        onCreate={(data) => createMutation.mutate(data)}
        isSaving={createMutation.isPending}
      />
    </div>
  );
}
