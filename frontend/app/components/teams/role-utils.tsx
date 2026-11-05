/**
 * Shared helpers for team roles: presets, custom roles and the role picker.
 * Roles come from GET /api/teams/:id/roles; the backend decides who may assign
 * what (owners only for the owner role, never a role with abilities the
 * assigner lacks), the UI only hides options that can never work.
 */
import { useQuery } from "@tanstack/react-query";
import { Code, Crown, Eye, Rocket, Shield, UserCog } from "lucide-react";
import { SelectItem, SelectSeparator } from "@/components/ui/select";
import { teamsApi } from "@/lib/api/teams";
import type { TeamRole, TeamRolesResponse } from "@/types/api";

export function useTeamRoles(teamId: string | null | undefined) {
  return useQuery<TeamRolesResponse>({
    queryKey: ["team-roles", teamId],
    queryFn: () => teamsApi.getRoles(teamId!),
    enabled: !!teamId,
    staleTime: 30_000,
  });
}

const PRESET_NAMES: Record<string, string> = {
  owner: "Owner",
  admin: "Admin",
  developer: "Developer",
  deployer: "Deployer",
  viewer: "Viewer",
};

/** Display name of a role value (preset or `custom:<id>`). */
export function roleName(role: string, roles?: TeamRolesResponse): string {
  if (PRESET_NAMES[role]) return PRESET_NAMES[role];
  const custom = roles?.custom.find((c) => c.key === role);
  if (custom) return custom.name;
  return role.startsWith("custom:") ? "Custom role" : role;
}

export function roleBadgeVariant(role: string): "default" | "secondary" | "outline" {
  if (role === "owner") return "default";
  if (role === "admin") return "secondary";
  return "outline";
}

export function RoleIcon({ role }: { role: string }) {
  switch (role) {
    case "owner":
      return <Crown className="h-4 w-4" />;
    case "admin":
      return <Shield className="h-4 w-4" />;
    case "developer":
      return <Code className="h-4 w-4" />;
    case "deployer":
      return <Rocket className="h-4 w-4" />;
    case "viewer":
      return <Eye className="h-4 w-4" />;
    default:
      return <UserCog className="h-4 w-4" />;
  }
}

/**
 * Options for a role `<Select>`: presets with their description, then the
 * team's custom roles. Only owners see the owner preset.
 */
export function RoleSelectItems({
  roles,
  currentUserRole,
  withDescriptions = true,
}: {
  roles?: TeamRolesResponse;
  currentUserRole: TeamRole | null;
  withDescriptions?: boolean;
}) {
  const presets = (roles?.presets ?? fallbackPresets).filter(
    (p) => currentUserRole === "owner" || p.key !== "owner",
  );
  const custom = roles?.custom ?? [];
  return (
    <>
      {presets.map((p) => (
        <SelectItem key={p.key} value={p.key}>
          {withDescriptions ? (
            <div className="flex flex-col">
              <span>{p.name}</span>
              <span className="text-xs text-muted-foreground">{p.description}</span>
            </div>
          ) : (
            p.name
          )}
        </SelectItem>
      ))}
      {custom.length > 0 && <SelectSeparator />}
      {custom.map((c) => (
        <SelectItem key={c.key} value={c.key}>
          {withDescriptions ? (
            <div className="flex flex-col">
              <span>{c.name}</span>
              <span className="text-xs text-muted-foreground">
                {c.description || `${c.abilities.length} abilities`}
              </span>
            </div>
          ) : (
            c.name
          )}
        </SelectItem>
      ))}
    </>
  );
}

/** Used until the roles request finishes. */
const fallbackPresets: TeamRolesResponse["presets"] = [
  { key: "owner", name: "Owner", description: "Everything", abilities: [] },
  { key: "admin", name: "Admin", description: "Everything except deleting the team", abilities: [] },
  { key: "developer", name: "Developer", description: "Create, change, deploy and delete resources", abilities: [] },
  { key: "deployer", name: "Deployer", description: "Read and deploy; no edits or deletes", abilities: [] },
  { key: "viewer", name: "Viewer", description: "Read-only", abilities: [] },
];
