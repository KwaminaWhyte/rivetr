import { useState, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { AlertTriangle, Eye, EyeOff, Shield } from "lucide-react";
import api from "@/lib/api";
import type { UpdateBasicAuthRequest } from "@/types/api";

interface BasicAuthCardProps {
  appId: string;
  token?: string;
}

export function BasicAuthCard({ appId, token }: BasicAuthCardProps) {
  const queryClient = useQueryClient();
  const [enabled, setEnabled] = useState(false);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [isDirty, setIsDirty] = useState(false);

  // Fetch current basic auth status
  const { data: basicAuth, isLoading } = useQuery({
    queryKey: ["basic-auth", appId],
    queryFn: () => api.getBasicAuth(appId, token),
  });

  // Initialize state from fetched data
  useEffect(() => {
    if (basicAuth) {
      setEnabled(basicAuth.enabled);
      setUsername(basicAuth.username || "");
      // Password is never returned from the API
      setPassword("");
      setIsDirty(false);
    }
  }, [basicAuth]);

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: (data: UpdateBasicAuthRequest) =>
      api.updateBasicAuth(appId, data, token),
    onSuccess: () => {
      toast.success("Basic auth settings updated");
      queryClient.invalidateQueries({ queryKey: ["basic-auth", appId] });
      queryClient.invalidateQueries({ queryKey: ["app", appId] });
      setPassword(""); // Clear password after save
      setIsDirty(false);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to update basic auth");
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: () => api.deleteBasicAuth(appId, token),
    onSuccess: () => {
      toast.success("Basic auth disabled");
      queryClient.invalidateQueries({ queryKey: ["basic-auth", appId] });
      queryClient.invalidateQueries({ queryKey: ["app", appId] });
      setEnabled(false);
      setUsername("");
      setPassword("");
      setIsDirty(false);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to disable basic auth");
    },
  });

  const handleEnabledChange = (checked: boolean) => {
    setEnabled(checked);
    setIsDirty(true);
    if (!checked) {
      // When disabling, we can save immediately
      deleteMutation.mutate();
    }
  };

  const handleSave = () => {
    if (!enabled) {
      deleteMutation.mutate();
      return;
    }

    if (!username) {
      toast.error("Username is required");
      return;
    }

    if (!password && !basicAuth?.enabled) {
      // Password required when first enabling
      toast.error("Password is required");
      return;
    }

    const data: UpdateBasicAuthRequest = {
      enabled: true,
      username,
    };

    // Only include password if it was changed
    if (password) {
      data.password = password;
    }

    updateMutation.mutate(data);
  };

  const isSaving = updateMutation.isPending || deleteMutation.isPending;

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            HTTP Basic Auth
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="animate-pulse h-32 bg-muted rounded" />
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Shield className="h-5 w-5" />
          HTTP Basic Auth
        </CardTitle>
        <CardDescription>
          Protect your application with username and password authentication.
          Users will be prompted for credentials before accessing your app.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Warning banner */}
        {!enabled && (
          <div className="flex items-start gap-3 p-3 rounded-lg bg-amber-50 dark:bg-amber-950/30 border border-amber-200 dark:border-amber-900">
            <AlertTriangle className="h-5 w-5 text-amber-600 dark:text-amber-500 shrink-0 mt-0.5" />
            <div className="text-sm text-amber-800 dark:text-amber-200">
              <p className="font-medium">Protect sensitive applications</p>
              <p className="mt-1 text-amber-700 dark:text-amber-300">
                Enable basic auth to add a layer of protection for staging
                environments, admin panels, or any application that should not
                be publicly accessible.
              </p>
            </div>
          </div>
        )}

        {/* Enable/Disable toggle */}
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <Label htmlFor="basic-auth-enabled" className="text-base">
              Enable Basic Auth
            </Label>
            <p className="text-sm text-muted-foreground">
              Require username and password to access this application
            </p>
          </div>
          <Switch
            id="basic-auth-enabled"
            checked={enabled}
            onCheckedChange={handleEnabledChange}
            disabled={isSaving}
          />
        </div>

        {/* Credentials form (shown when enabled) */}
        {enabled && (
          <div className="space-y-4 pt-4 border-t">
            <div className="space-y-2">
              <Label htmlFor="basic-auth-username">Username</Label>
              <Input
                id="basic-auth-username"
                value={username}
                onChange={(e) => {
                  setUsername(e.target.value);
                  setIsDirty(true);
                }}
                placeholder="admin"
                autoComplete="off"
              />
              <p className="text-xs text-muted-foreground">
                Letters, numbers, underscores, and dashes only
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="basic-auth-password">
                {basicAuth?.enabled
                  ? "New Password (leave blank to keep current)"
                  : "Password"}
              </Label>
              <div className="relative">
                <Input
                  id="basic-auth-password"
                  type={showPassword ? "text" : "password"}
                  value={password}
                  onChange={(e) => {
                    setPassword(e.target.value);
                    setIsDirty(true);
                  }}
                  placeholder={
                    basicAuth?.enabled
                      ? "Leave blank to keep current"
                      : "Enter password"
                  }
                  autoComplete="new-password"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
                  onClick={() => setShowPassword(!showPassword)}
                >
                  {showPassword ? (
                    <EyeOff className="h-4 w-4 text-muted-foreground" />
                  ) : (
                    <Eye className="h-4 w-4 text-muted-foreground" />
                  )}
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                Minimum 8 characters
              </p>
            </div>

            <Button
              onClick={handleSave}
              disabled={isSaving || !isDirty}
              className="w-full sm:w-auto"
            >
              {isSaving ? "Saving..." : "Save Credentials"}
            </Button>
          </div>
        )}

        {/* Current status */}
        {basicAuth?.enabled && (
          <div className="pt-4 border-t">
            <p className="text-sm text-muted-foreground">
              Currently protected with username:{" "}
              <code className="px-1.5 py-0.5 rounded bg-muted font-mono text-foreground">
                {basicAuth.username}
              </code>
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
