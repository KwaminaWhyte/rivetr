import { Link, useLocation } from "react-router";
import { ShieldAlert } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { useTeamContext } from "@/lib/team-context";

/**
 * Shown while the current team requires 2FA and the user has neither an
 * authenticator app nor a passkey: the API refuses the team's resources
 * (`two_factor_required`) until they set one up.
 */
export function TwoFactorBanner() {
  const { currentTeam } = useTeamContext();
  const location = useLocation();
  if (!currentTeam?.two_factor_blocked || location.pathname.startsWith("/account/security")) {
    return null;
  }
  return (
    <Alert variant="destructive">
      <ShieldAlert className="h-4 w-4" />
      <AlertTitle>{currentTeam.name} requires two-factor authentication</AlertTitle>
      <AlertDescription>
        <span>
          You cannot open this team's apps, databases and services until you{" "}
          <Link to="/account/security" className="font-medium underline underline-offset-4">
            set up an authenticator app or a passkey
          </Link>
          .
        </span>
      </AlertDescription>
    </Alert>
  );
}
