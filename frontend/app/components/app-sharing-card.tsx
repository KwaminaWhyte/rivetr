import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Share2, Plus, Trash2, Users, Eye } from "lucide-react";
import { api } from "@/lib/api";
import { useTeamContext } from "@/lib/team-context";
import type { App, AppShare, TeamWithMemberCount } from "@/types/api";

interface AppSharingCardProps {
  app: App;
}

/** Format timestamp to relative or absolute date */
function formatDate(timestamp: string): string {
  const date = new Date(timestamp);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));

  if (days === 0) {
    return "Today";
  } else if (days === 1) {
    return "Yesterday";
  } else if (days < 7) {
    return `${days} days ago`;
  } else {
    return date.toLocaleDateString();
  }
}

export function AppSharingCard({ app }: AppSharingCardProps) {
  const queryClient = useQueryClient();
  const { currentTeamId } = useTeamContext();
  const [selectedTeamId, setSelectedTeamId] = useState<string>("");
  const [teamToRemove, setTeamToRemove] = useState<AppShare | null>(null);

  // Fetch current shares for this app
  const { data: shares = [], isLoading: sharesLoading } = useQuery<AppShare[]>({
    queryKey: ["app-shares", app.id],
    queryFn: () => api.getAppShares(app.id),
  });

  // Fetch all teams user belongs to (for team selection dropdown)
  const { data: teams = [] } = useQuery<TeamWithMemberCount[]>({
    queryKey: ["teams"],
    queryFn: () => api.getTeams(),
  });

  // Filter out teams that:
  // 1. Already have this app shared with them
  // 2. Are the current (owner) team
  const availableTeams = teams.filter((team) => {
    // Don't show the owner team
    if (team.id === app.team_id) return false;
    // Don't show teams that already have the share
    if (shares.some((s) => s.shared_with_team_id === team.id)) return false;
    return true;
  });

  // Create share mutation
  const createShareMutation = useMutation({
    mutationFn: (teamId: string) =>
      api.createAppShare(app.id, { team_id: teamId }),
    onSuccess: () => {
      toast.success("App shared successfully");
      queryClient.invalidateQueries({ queryKey: ["app-shares", app.id] });
      setSelectedTeamId("");
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to share app");
    },
  });

  // Delete share mutation
  const deleteShareMutation = useMutation({
    mutationFn: (teamId: string) => api.deleteAppShare(app.id, teamId),
    onSuccess: () => {
      toast.success("Sharing removed");
      queryClient.invalidateQueries({ queryKey: ["app-shares", app.id] });
      setTeamToRemove(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to remove sharing");
    },
  });

  const handleShare = () => {
    if (selectedTeamId) {
      createShareMutation.mutate(selectedTeamId);
    }
  };

  const handleRemove = () => {
    if (teamToRemove) {
      deleteShareMutation.mutate(teamToRemove.shared_with_team_id);
    }
  };

  // Only show if this app belongs to the current team (not a shared app)
  const isOwner = app.team_id === currentTeamId;

  if (!isOwner) {
    return null;
  }

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Share2 className="h-5 w-5" />
            App Sharing
          </CardTitle>
          <CardDescription>
            Share this app with other teams. Shared teams can view the app but cannot modify it.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Share with team section */}
          <div className="flex items-end gap-3">
            <div className="flex-1 space-y-2">
              <label className="text-sm font-medium">Share with team</label>
              <Select
                value={selectedTeamId}
                onValueChange={setSelectedTeamId}
                disabled={availableTeams.length === 0}
              >
                <SelectTrigger>
                  <SelectValue placeholder={
                    availableTeams.length === 0
                      ? "No teams available"
                      : "Select a team..."
                  } />
                </SelectTrigger>
                <SelectContent>
                  {availableTeams.map((team) => (
                    <SelectItem key={team.id} value={team.id}>
                      <div className="flex items-center gap-2">
                        <Users className="h-4 w-4 text-muted-foreground" />
                        <span>{team.name}</span>
                        <Badge variant="outline" className="ml-2 text-xs">
                          {team.member_count} {team.member_count === 1 ? "member" : "members"}
                        </Badge>
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <Button
              onClick={handleShare}
              disabled={!selectedTeamId || createShareMutation.isPending}
            >
              <Plus className="h-4 w-4 mr-2" />
              {createShareMutation.isPending ? "Sharing..." : "Share"}
            </Button>
          </div>

          {/* Current shares list */}
          {sharesLoading ? (
            <div className="text-sm text-muted-foreground">Loading shares...</div>
          ) : shares.length === 0 ? (
            <div className="py-6 text-center text-muted-foreground">
              <Share2 className="h-8 w-8 mx-auto mb-2 opacity-50" />
              <p>This app is not shared with any other teams.</p>
            </div>
          ) : (
            <div className="border rounded-md">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Team</TableHead>
                    <TableHead>Permission</TableHead>
                    <TableHead>Shared</TableHead>
                    <TableHead>Shared By</TableHead>
                    <TableHead className="w-[80px]"></TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {shares.map((share) => (
                    <TableRow key={share.id}>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <Users className="h-4 w-4 text-muted-foreground" />
                          <span className="font-medium">{share.shared_with_team_name}</span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <Badge variant="secondary" className="gap-1">
                          <Eye className="h-3 w-3" />
                          View
                        </Badge>
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {formatDate(share.created_at)}
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {share.created_by_name || "Unknown"}
                      </TableCell>
                      <TableCell>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => setTeamToRemove(share)}
                          className="text-destructive hover:text-destructive hover:bg-destructive/10"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Remove share confirmation dialog */}
      <Dialog open={teamToRemove !== null} onOpenChange={(open) => !open && setTeamToRemove(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Remove Sharing</DialogTitle>
            <DialogDescription>
              Are you sure you want to stop sharing this app with{" "}
              <span className="font-medium text-foreground">{teamToRemove?.shared_with_team_name}</span>?
              They will no longer be able to view this app.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setTeamToRemove(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleRemove}
              disabled={deleteShareMutation.isPending}
            >
              {deleteShareMutation.isPending ? "Removing..." : "Remove Sharing"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
