import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
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
import { Github, GitBranch, Unlink, ExternalLink, Building2, User } from "lucide-react";
import { api } from "@/lib/api";
import type { App, GitHubAppInstallation } from "@/types/api";

interface GitHubSourceCardProps {
  app: App;
}

// Helper to extract owner/repo from git URL
function parseGitUrl(gitUrl: string): { owner: string; repo: string } | null {
  // Handle various formats:
  // https://github.com/owner/repo.git
  // https://github.com/owner/repo
  // git@github.com:owner/repo.git
  const httpsMatch = gitUrl.match(/github\.com\/([^/]+)\/([^/.]+)/);
  const sshMatch = gitUrl.match(/github\.com:([^/]+)\/([^/.]+)/);

  const match = httpsMatch || sshMatch;
  if (match) {
    return { owner: match[1], repo: match[2] };
  }
  return null;
}

export function GitHubSourceCard({ app }: GitHubSourceCardProps) {
  const queryClient = useQueryClient();
  const [showDisconnectDialog, setShowDisconnectDialog] = useState(false);
  const [isDisconnecting, setIsDisconnecting] = useState(false);

  // Fetch all installations to find the one this app is connected to
  const { data: installations = [] } = useQuery<GitHubAppInstallation[]>({
    queryKey: ["github-app-installations"],
    queryFn: () => api.getAllGitHubAppInstallations(),
    enabled: !!app.github_app_installation_id,
  });

  // Find the installation this app is connected to
  const connectedInstallation = installations.find(
    (inst) => inst.id === app.github_app_installation_id
  );

  // Parse repository info from git_url
  const repoInfo = parseGitUrl(app.git_url);

  // Handle disconnect
  const handleDisconnect = async () => {
    setIsDisconnecting(true);
    try {
      await api.updateApp(app.id, {
        github_app_installation_id: null,
      });
      toast.success("Disconnected from GitHub App");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
      setShowDisconnectDialog(false);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to disconnect");
    } finally {
      setIsDisconnecting(false);
    }
  };

  // Don't show if not connected via GitHub App
  if (!app.github_app_installation_id) {
    return null;
  }

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Github className="h-5 w-5" />
            GitHub App Connection
          </CardTitle>
          <CardDescription>
            This app is connected to GitHub via GitHub App integration.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Connection Status */}
          <div className="flex items-center gap-2">
            <Badge variant="outline" className="gap-1.5 text-green-600 border-green-200 bg-green-50">
              <span className="relative flex h-2 w-2">
                <span className="absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75 animate-pulse"></span>
                <span className="relative inline-flex h-2 w-2 rounded-full bg-green-500"></span>
              </span>
              Connected
            </Badge>
          </div>

          {/* Repository Info */}
          {repoInfo && (
            <div className="p-4 bg-muted/50 rounded-lg space-y-3">
              <div className="flex items-start justify-between">
                <div className="space-y-1">
                  <div className="flex items-center gap-2">
                    <Github className="h-4 w-4 text-muted-foreground" />
                    <span className="font-medium">{repoInfo.owner}/{repoInfo.repo}</span>
                  </div>
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <GitBranch className="h-3.5 w-3.5" />
                    <span>{app.branch}</span>
                  </div>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  asChild
                >
                  <a
                    href={`https://github.com/${repoInfo.owner}/${repoInfo.repo}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="gap-1.5"
                  >
                    <ExternalLink className="h-3.5 w-3.5" />
                    View
                  </a>
                </Button>
              </div>
            </div>
          )}

          {/* Installation Info */}
          {connectedInstallation && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              {connectedInstallation.account_type === "organization" ? (
                <Building2 className="h-4 w-4" />
              ) : (
                <User className="h-4 w-4" />
              )}
              <span>
                Installed on{" "}
                <span className="font-medium text-foreground">
                  {connectedInstallation.account_login}
                </span>
                {connectedInstallation.account_type === "organization" && " (organization)"}
              </span>
            </div>
          )}

          {/* Disconnect Button */}
          <div className="pt-2 border-t">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowDisconnectDialog(true)}
              className="gap-1.5 text-destructive hover:text-destructive hover:bg-destructive/10"
            >
              <Unlink className="h-3.5 w-3.5" />
              Disconnect
            </Button>
            <p className="mt-2 text-xs text-muted-foreground">
              Disconnecting will remove the GitHub App integration but keep the Git URL.
              You can still deploy using manual webhooks or URL-based git clone.
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Disconnect Confirmation Dialog */}
      <AlertDialog open={showDisconnectDialog} onOpenChange={setShowDisconnectDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Disconnect GitHub App?</AlertDialogTitle>
            <AlertDialogDescription>
              This will remove the GitHub App integration from this app.
              The Git URL will be preserved, and you can still deploy manually or
              via webhooks.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDisconnect}
              disabled={isDisconnecting}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {isDisconnecting ? "Disconnecting..." : "Disconnect"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
