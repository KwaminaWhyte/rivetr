import { useState, useEffect } from "react";
import { Form, useNavigation, Link } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { Route } from "./+types/teams";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { api } from "@/lib/api";
import type { TeamWithMemberCount, TeamRole } from "@/types/api";
import { Users, Settings, ChevronRight } from "lucide-react";

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString();
}

function getRoleBadgeVariant(role: TeamRole | null): "default" | "secondary" | "outline" {
  switch (role) {
    case "owner":
      return "default";
    case "admin":
      return "secondary";
    default:
      return "outline";
  }
}

function formatRole(role: TeamRole | null): string {
  if (!role) return "";
  return role.charAt(0).toUpperCase() + role.slice(1);
}

export async function loader({ request }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const teams = await api.getTeams(token).catch(() => []);
  return { teams, token };
}

export async function action({ request }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "create") {
    const name = formData.get("name");
    const slug = formData.get("slug");

    if (typeof name !== "string" || !name.trim()) {
      return { error: "Team name is required" };
    }

    try {
      await api.createTeam(token, {
        name: name.trim(),
        slug: typeof slug === "string" && slug.trim() ? slug.trim() : undefined,
      });
      return { success: true, action: "create" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to create team" };
    }
  }

  if (intent === "delete") {
    const teamId = formData.get("teamId");
    if (typeof teamId !== "string") {
      return { error: "Team ID is required" };
    }
    try {
      await api.deleteTeam(token, teamId);
      return { success: true, action: "delete" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to delete team" };
    }
  }

  return { error: "Unknown action" };
}

export default function SettingsTeamsPage({ loaderData, actionData }: Route.ComponentProps) {
  const queryClient = useQueryClient();
  const navigation = useNavigation();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [selectedTeamId, setSelectedTeamId] = useState<string | null>(null);
  const [selectedTeamName, setSelectedTeamName] = useState<string>("");

  const { data: teams = [] } = useQuery<TeamWithMemberCount[]>({
    queryKey: ["teams"],
    queryFn: () => api.getTeams(loaderData.token),
    initialData: loaderData.teams,
  });

  const isSubmitting = navigation.state === "submitting";

  // Handle success actions
  useEffect(() => {
    if (actionData?.success) {
      if (actionData.action === "create") {
        toast.success("Team created successfully");
        setShowCreateDialog(false);
      } else if (actionData.action === "delete") {
        toast.success("Team deleted successfully");
        setShowDeleteDialog(false);
        setSelectedTeamId(null);
        setSelectedTeamName("");
      }
      queryClient.invalidateQueries({ queryKey: ["teams"] });
    }

    if (actionData?.error) {
      toast.error(actionData.error);
    }
  }, [actionData, queryClient]);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Teams</h1>
          <p className="text-muted-foreground">
            Manage teams and collaborate with other users
          </p>
        </div>
        <Button onClick={() => setShowCreateDialog(true)}>Create Team</Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Your Teams</CardTitle>
          <CardDescription>
            Teams allow you to collaborate with others on apps and projects.
            Each team member has a role that determines their permissions.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {teams.length === 0 ? (
            <div className="text-center py-8">
              <Users className="mx-auto h-12 w-12 text-muted-foreground/50" />
              <p className="mt-4 text-muted-foreground">
                You're not a member of any teams yet. Create one to get started.
              </p>
              <Button className="mt-4" onClick={() => setShowCreateDialog(true)}>
                Create Your First Team
              </Button>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Team Name</TableHead>
                  <TableHead>Your Role</TableHead>
                  <TableHead>Members</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="w-24">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {teams.map((team) => (
                  <TableRow key={team.id}>
                    <TableCell>
                      <div className="flex flex-col">
                        <span className="font-medium">{team.name}</span>
                        <span className="text-xs text-muted-foreground">
                          /{team.slug}
                        </span>
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge variant={getRoleBadgeVariant(team.user_role)}>
                        {formatRole(team.user_role)}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <Users className="h-4 w-4 text-muted-foreground" />
                        {team.member_count}
                      </div>
                    </TableCell>
                    <TableCell>{formatDate(team.created_at)}</TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Button variant="ghost" size="sm" asChild>
                          <Link to={`/settings/teams/${team.id}`}>
                            <Settings className="h-4 w-4 mr-1" />
                            Manage
                            <ChevronRight className="h-4 w-4 ml-1" />
                          </Link>
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Create Team Dialog */}
      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent>
          <Form method="post">
            <input type="hidden" name="intent" value="create" />
            <DialogHeader>
              <DialogTitle>Create Team</DialogTitle>
              <DialogDescription>
                Create a new team to collaborate with others. You'll be the owner
                of this team and can invite members after creation.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="name">Team Name</Label>
                <Input
                  id="name"
                  name="name"
                  placeholder="e.g., My Awesome Team"
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="slug">URL Slug (optional)</Label>
                <Input
                  id="slug"
                  name="slug"
                  placeholder="e.g., my-awesome-team"
                />
                <p className="text-xs text-muted-foreground">
                  A URL-friendly identifier. Will be auto-generated from the name
                  if not provided.
                </p>
              </div>
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setShowCreateDialog(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Creating..." : "Create Team"}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Team</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete "{selectedTeamName}"? This action
              cannot be undone. All team memberships will be removed, but apps
              and projects will remain.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowDeleteDialog(false);
                setSelectedTeamId(null);
                setSelectedTeamName("");
              }}
            >
              Cancel
            </Button>
            <Form method="post">
              <input type="hidden" name="intent" value="delete" />
              <input type="hidden" name="teamId" value={selectedTeamId || ""} />
              <Button type="submit" variant="destructive" disabled={isSubmitting}>
                {isSubmitting ? "Deleting..." : "Delete Team"}
              </Button>
            </Form>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
