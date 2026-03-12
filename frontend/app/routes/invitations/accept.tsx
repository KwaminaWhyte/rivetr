import { useState, useEffect } from "react";
import { useSearchParams, useNavigate, Link } from "react-router";
import { useMutation } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Rocket, Users, Clock, AlertCircle, CheckCircle, Loader2 } from "lucide-react";
import { teamsApi } from "@/lib/api/teams";
import { getAuthToken, validateAuth, setAuthToken } from "@/lib/auth";
import type { TeamInvitation, TeamRole } from "@/types/api";

export function meta() {
  return [
    { title: "Accept Invitation - Rivetr" },
    { name: "description", content: "Accept a team invitation" },
  ];
}

type InvitationState =
  | { status: "loading" }
  | { status: "not_found" }
  | { status: "expired"; invitation: TeamInvitation }
  | { status: "already_accepted"; invitation: TeamInvitation }
  | { status: "valid"; invitation: TeamInvitation }
  | { status: "error"; message: string };

type AuthState =
  | { status: "loading" }
  | { status: "authenticated" }
  | { status: "unauthenticated" };

type SetupMode = "choose" | "login" | "register";

function getRoleBadgeVariant(role: TeamRole): "default" | "secondary" | "outline" {
  switch (role) {
    case "owner":
      return "default";
    case "admin":
      return "secondary";
    default:
      return "outline";
  }
}

function formatRoleName(role: TeamRole): string {
  return role.charAt(0).toUpperCase() + role.slice(1);
}

export default function AcceptInvitationPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const token = searchParams.get("token");

  const [invitationState, setInvitationState] = useState<InvitationState>({ status: "loading" });
  const [authState, setAuthState] = useState<AuthState>({ status: "loading" });
  const [setupMode, setSetupMode] = useState<SetupMode>("choose");
  const [registerName, setRegisterName] = useState("");
  const [registerPassword, setRegisterPassword] = useState("");

  // Check authentication status
  useEffect(() => {
    async function checkAuth() {
      const isAuthenticated = await validateAuth();
      setAuthState({ status: isAuthenticated ? "authenticated" : "unauthenticated" });
    }
    checkAuth();
  }, []);

  // Validate invitation token
  useEffect(() => {
    async function validateInvitation() {
      if (!token) {
        setInvitationState({ status: "not_found" });
        return;
      }

      try {
        const invitation = await teamsApi.validateInvitation(token);

        // Check if already accepted (shouldn't happen since API returns error, but just in case)
        if (invitation.accepted_at) {
          setInvitationState({ status: "already_accepted", invitation });
          return;
        }

        // Check if expired
        const expiresAt = new Date(invitation.expires_at);
        if (expiresAt < new Date()) {
          setInvitationState({ status: "expired", invitation });
          return;
        }

        setInvitationState({ status: "valid", invitation });
      } catch (error) {
        const message = error instanceof Error ? error.message : "Failed to validate invitation";

        // Check for specific error messages from the API
        if (message.includes("already been accepted")) {
          setInvitationState({ status: "error", message: "This invitation has already been accepted." });
        } else if (message.includes("expired")) {
          setInvitationState({ status: "error", message: "This invitation has expired." });
        } else if (message.includes("not found")) {
          setInvitationState({ status: "not_found" });
        } else {
          setInvitationState({ status: "error", message });
        }
      }
    }

    validateInvitation();
  }, [token]);

  // Accept invitation mutation
  const acceptMutation = useMutation({
    mutationFn: () => {
      const authToken = getAuthToken();
      return teamsApi.acceptInvitation(token!, authToken ?? undefined);
    },
    onSuccess: (member) => {
      // Navigate to the team settings page
      navigate(`/teams/${member.team_id}`, { replace: true });
    },
    onError: (error: Error) => {
      // Handle specific error cases
      if (error.message.includes("already a member")) {
        setInvitationState({
          status: "error",
          message: "You are already a member of this team.",
        });
      } else if (error.message.includes("different email")) {
        setInvitationState({
          status: "error",
          message: "This invitation was sent to a different email address. Please log in with the correct account.",
        });
      } else {
        setInvitationState({
          status: "error",
          message: error.message || "Failed to accept invitation",
        });
      }
    },
  });

  // Register + accept in one step
  const registerMutation = useMutation({
    mutationFn: ({ name, password }: { name: string; password: string }) =>
      teamsApi.registerWithInvitation({ token: token!, name, password }),
    onSuccess: (data) => {
      setAuthToken(data.token);
      navigate(`/teams/${data.member.team_id}`, { replace: true });
    },
    onError: (error: Error) => {
      setInvitationState({
        status: "error",
        message: error.message || "Failed to create account",
      });
    },
  });

  const handleRegisterSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!registerName.trim() || !registerPassword) return;
    registerMutation.mutate({ name: registerName.trim(), password: registerPassword });
  };

  // Handle login redirect
  const handleLoginRedirect = () => {
    // Store the invitation URL to return to after login
    const returnUrl = `/invitations/accept?token=${token}`;
    localStorage.setItem("rivetr_return_url", returnUrl);
    navigate("/login");
  };

  // Render loading state
  if (invitationState.status === "loading" || authState.status === "loading") {
    return (
      <div className="min-h-svh flex items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          <p className="text-muted-foreground">Validating invitation...</p>
        </div>
      </div>
    );
  }

  // Render not found state
  if (invitationState.status === "not_found") {
    return (
      <div className="min-h-svh flex items-center justify-center bg-background p-4">
        <Card className="w-full max-w-md">
          <CardHeader className="text-center">
            <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-destructive/10">
              <AlertCircle className="h-6 w-6 text-destructive" />
            </div>
            <CardTitle>Invitation Not Found</CardTitle>
            <CardDescription>
              This invitation link is invalid or has been removed.
            </CardDescription>
          </CardHeader>
          <CardFooter className="flex justify-center">
            <Button asChild>
              <Link to="/login">Go to Login</Link>
            </Button>
          </CardFooter>
        </Card>
      </div>
    );
  }

  // Render error state
  if (invitationState.status === "error") {
    return (
      <div className="min-h-svh flex items-center justify-center bg-background p-4">
        <Card className="w-full max-w-md">
          <CardHeader className="text-center">
            <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-destructive/10">
              <AlertCircle className="h-6 w-6 text-destructive" />
            </div>
            <CardTitle>Unable to Accept Invitation</CardTitle>
            <CardDescription>{invitationState.message}</CardDescription>
          </CardHeader>
          <CardFooter className="flex justify-center gap-2">
            <Button variant="outline" asChild>
              <Link to="/login">Go to Login</Link>
            </Button>
            {authState.status === "authenticated" && (
              <Button asChild>
                <Link to="/">Go to Dashboard</Link>
              </Button>
            )}
          </CardFooter>
        </Card>
      </div>
    );
  }

  // Render expired state
  if (invitationState.status === "expired") {
    return (
      <div className="min-h-svh flex items-center justify-center bg-background p-4">
        <Card className="w-full max-w-md">
          <CardHeader className="text-center">
            <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-amber-500/10">
              <Clock className="h-6 w-6 text-amber-500" />
            </div>
            <CardTitle>Invitation Expired</CardTitle>
            <CardDescription>
              This invitation to join{" "}
              <span className="font-medium text-foreground">
                {invitationState.invitation.team_name || "the team"}
              </span>{" "}
              has expired. Please ask the team administrator to send you a new invitation.
            </CardDescription>
          </CardHeader>
          <CardFooter className="flex justify-center">
            <Button asChild>
              <Link to="/login">Go to Login</Link>
            </Button>
          </CardFooter>
        </Card>
      </div>
    );
  }

  // Render already accepted state
  if (invitationState.status === "already_accepted") {
    return (
      <div className="min-h-svh flex items-center justify-center bg-background p-4">
        <Card className="w-full max-w-md">
          <CardHeader className="text-center">
            <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-green-500/10">
              <CheckCircle className="h-6 w-6 text-green-500" />
            </div>
            <CardTitle>Already Accepted</CardTitle>
            <CardDescription>
              This invitation to join{" "}
              <span className="font-medium text-foreground">
                {invitationState.invitation.team_name || "the team"}
              </span>{" "}
              has already been accepted.
            </CardDescription>
          </CardHeader>
          <CardFooter className="flex justify-center">
            <Button asChild>
              <Link to="/">Go to Dashboard</Link>
            </Button>
          </CardFooter>
        </Card>
      </div>
    );
  }

  // Render valid invitation state
  const { invitation } = invitationState;

  return (
    <div className="min-h-svh flex items-center justify-center bg-background p-4">
      <div className="w-full max-w-md">
        {/* Logo */}
        <div className="flex justify-center mb-8">
          <Link to="/" className="flex items-center gap-2 font-medium">
            <div className="bg-primary text-primary-foreground flex size-8 items-center justify-center rounded-md">
              <Rocket className="size-5" />
            </div>
            <span className="text-xl font-bold">Rivetr</span>
          </Link>
        </div>

        <Card>
          <CardHeader className="text-center">
            <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
              <Users className="h-6 w-6 text-primary" />
            </div>
            <CardTitle>You&apos;re Invited!</CardTitle>
            <CardDescription>
              {invitation.inviter_name || "A team administrator"} has invited you to join a team on
              Rivetr.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* Team Info */}
            <div className="rounded-lg border p-4 space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Team</span>
                <span className="font-medium">{invitation.team_name || "Team"}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Your Role</span>
                <Badge variant={getRoleBadgeVariant(invitation.role)}>
                  {formatRoleName(invitation.role)}
                </Badge>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Invited By</span>
                <span className="text-sm">{invitation.inviter_name || "Unknown"}</span>
              </div>
            </div>

            {/* Action based on auth state */}
            {authState.status === "authenticated" ? (
              <Button
                className="w-full"
                onClick={() => acceptMutation.mutate()}
                disabled={acceptMutation.isPending}
              >
                {acceptMutation.isPending ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Accepting...
                  </>
                ) : (
                  "Accept Invitation"
                )}
              </Button>
            ) : setupMode === "choose" ? (
              <div className="space-y-3">
                <Button className="w-full" onClick={() => setSetupMode("register")}>
                  Create Account &amp; Accept
                </Button>
                <Button variant="outline" className="w-full" onClick={handleLoginRedirect}>
                  Log In to Accept
                </Button>
              </div>
            ) : setupMode === "register" ? (
              <form onSubmit={handleRegisterSubmit} className="space-y-3">
                <div className="space-y-1">
                  <Label htmlFor="reg-name">Your Name</Label>
                  <Input
                    id="reg-name"
                    placeholder="Jane Smith"
                    value={registerName}
                    onChange={(e) => setRegisterName(e.target.value)}
                    required
                  />
                </div>
                <div className="space-y-1">
                  <Label htmlFor="reg-password">Password</Label>
                  <Input
                    id="reg-password"
                    type="password"
                    placeholder="Min 12 chars, upper, lower, digit, special"
                    value={registerPassword}
                    onChange={(e) => setRegisterPassword(e.target.value)}
                    required
                  />
                </div>
                <Button type="submit" className="w-full" disabled={registerMutation.isPending}>
                  {registerMutation.isPending ? (
                    <><Loader2 className="mr-2 h-4 w-4 animate-spin" />Creating account...</>
                  ) : (
                    "Create Account & Accept"
                  )}
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  className="w-full"
                  onClick={() => setSetupMode("choose")}
                >
                  Back
                </Button>
              </form>
            ) : null}
          </CardContent>
          <CardFooter className="flex justify-center">
            <p className="text-xs text-muted-foreground">
              This invitation will expire on{" "}
              {new Date(invitation.expires_at).toLocaleDateString()}
            </p>
          </CardFooter>
        </Card>
      </div>
    </div>
  );
}
