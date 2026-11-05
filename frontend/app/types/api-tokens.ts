// -------------------------------------------------------------------------
// API token types (scoped, restrictable, pausable, rollable)
// -------------------------------------------------------------------------

export type ApiTokenScope = "read" | "deploy" | "write" | "delete" | "secrets" | "admin";

/** Scopes in display order, with what each one unlocks. */
export const API_TOKEN_SCOPES: { value: ApiTokenScope; description: string }[] = [
  { value: "read", description: "Read resources, deployments and logs" },
  { value: "deploy", description: "Deploy, start, stop, restart, roll back" },
  { value: "write", description: "Create and change resources, open terminals" },
  { value: "delete", description: "Delete resources" },
  { value: "secrets", description: "Reveal secret values, download backups" },
  { value: "admin", description: "Instance and team administration; implies all" },
];

export interface ApiTokenResource {
  type: "project" | "app" | "database" | "service";
  id: string;
}

export interface ApiToken {
  id: string;
  name: string;
  created_at: string;
  last_used_at: string | null;
  last_used_ip: string | null;
  expires_at: string | null;
  scopes: ApiTokenScope[];
  team_id: string | null;
  resources: ApiTokenResource[];
  disabled: boolean;
  token_hint: string | null;
  rolled_at: string | null;
  /** While set, the secret from before the last roll still works. */
  previous_expires_at: string | null;
  /** Only on create and roll. */
  token?: string;
}

export interface CreateApiTokenRequest {
  name: string;
  scopes: ApiTokenScope[];
  expires_at?: string;
  team_id?: string;
  resources?: ApiTokenResource[];
}
