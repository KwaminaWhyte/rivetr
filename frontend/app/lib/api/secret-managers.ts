/**
 * External secret managers (HashiCorp Vault, OpenBao, Infisical, Doppler,
 * AWS Secrets Manager). Env var values reference secrets as
 * `secret://<connection>/<path>#<key>`; Rivetr resolves them at deploy time.
 * Credentials are write-only and secret values never reach the browser.
 */

import { apiRequest } from "./core";

export type SecretProviderKind =
  | "vault"
  | "openbao"
  | "infisical"
  | "doppler"
  | "aws_secrets_manager";

export interface SecretConnectionConfig {
  auth_method?: string | null;
  address?: string | null;
  mount?: string | null;
  namespace?: string | null;
  role_id?: string | null;
  approle_mount?: string | null;
  site_url?: string | null;
  project_id?: string | null;
  environment?: string | null;
  client_id?: string | null;
  api_url?: string | null;
  project?: string | null;
  config?: string | null;
  region?: string | null;
  access_key_id?: string | null;
  endpoint?: string | null;
}

export interface SecretConnectionCredentials {
  token?: string;
  secret_id?: string;
  client_secret?: string;
  secret_access_key?: string;
  session_token?: string;
}

export interface SecretConnection {
  id: string;
  team_id: string | null;
  name: string;
  provider: SecretProviderKind;
  config: SecretConnectionConfig;
  /** Names of the stored credential fields (values are never returned). */
  credentials_set: string[];
  last_tested_at: string | null;
  last_test_ok: boolean | null;
  last_test_message: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateSecretConnectionRequest {
  team_id?: string | null;
  name: string;
  provider: SecretProviderKind;
  config: SecretConnectionConfig;
  credentials: SecretConnectionCredentials;
}

export interface UpdateSecretConnectionRequest {
  name?: string;
  config?: SecretConnectionConfig;
  /** Blank fields keep the stored value. */
  credentials?: SecretConnectionCredentials;
}

export interface SecretTestResult {
  ok: boolean;
  message: string;
}

export interface SecretListingEntry {
  name: string;
  path: string;
  kind: "folder" | "secret";
}

export interface SecretListing {
  entries: SecretListingEntry[];
  /** Key names at the browsed path (no values). */
  keys: string[];
}

export type SecretOwnerType = "app" | "service" | "project";

export interface SecretImport {
  id: string;
  owner_type: SecretOwnerType;
  owner_id: string;
  connection_id: string;
  path: string;
  prefix: string;
  created_at: string;
  connection_name: string | null;
  provider: SecretProviderKind | null;
}

export type SecretFailurePolicy = "fail" | "last_known";

export interface SecretPolicy {
  failure_policy: SecretFailurePolicy;
  last_resolved_at: string | null;
}

const ownerBase = (type: SecretOwnerType, id: string) =>
  `/${type === "app" ? "apps" : type === "service" ? "services" : "projects"}/${id}`;

export const secretManagersApi = {
  list: (teamId?: string) =>
    apiRequest<SecretConnection[]>(
      `/secret-connections${teamId ? `?team_id=${encodeURIComponent(teamId)}` : ""}`,
    ),

  create: (data: CreateSecretConnectionRequest) =>
    apiRequest<SecretConnection>("/secret-connections", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  update: (id: string, data: UpdateSecretConnectionRequest) =>
    apiRequest<SecretConnection>(`/secret-connections/${id}`, {
      method: "PUT",
      body: JSON.stringify(data),
    }),

  delete: (id: string) =>
    apiRequest<void>(`/secret-connections/${id}`, { method: "DELETE" }),

  test: (id: string) =>
    apiRequest<SecretTestResult>(`/secret-connections/${id}/test`, {
      method: "POST",
    }),

  testUnsaved: (data: Omit<CreateSecretConnectionRequest, "name" | "team_id">) =>
    apiRequest<SecretTestResult>("/secret-connections/test", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  browse: (id: string, path: string) =>
    apiRequest<SecretListing>(
      `/secret-connections/${id}/browse?path=${encodeURIComponent(path)}`,
    ),

  listImports: (type: SecretOwnerType, id: string) =>
    apiRequest<SecretImport[]>(`${ownerBase(type, id)}/secret-imports`),

  createImport: (
    type: SecretOwnerType,
    id: string,
    data: { connection_id: string; path: string; prefix: string },
  ) =>
    apiRequest<SecretImport>(`${ownerBase(type, id)}/secret-imports`, {
      method: "POST",
      body: JSON.stringify(data),
    }),

  deleteImport: (type: SecretOwnerType, id: string, importId: string) =>
    apiRequest<void>(`${ownerBase(type, id)}/secret-imports/${importId}`, {
      method: "DELETE",
    }),

  getPolicy: (type: "app" | "service", id: string) =>
    apiRequest<SecretPolicy>(`${ownerBase(type, id)}/secret-policy`),

  setPolicy: (type: "app" | "service", id: string, failure_policy: SecretFailurePolicy) =>
    apiRequest<SecretPolicy>(`${ownerBase(type, id)}/secret-policy`, {
      method: "PUT",
      body: JSON.stringify({ failure_policy }),
    }),
};

/** Parsed `secret://<connection>/<path>#<key>` reference, or null. */
export interface SecretReference {
  connection: string;
  path: string;
  key: string | null;
}

const REF_RE = /^secret:\/\/([a-z0-9][a-z0-9_-]{0,62})(?:\/([^#\s]*))?(?:#(\S+))?$/;

export function parseSecretReference(value: string | null | undefined): SecretReference | null {
  if (!value || !value.startsWith("secret://")) return null;
  const m = REF_RE.exec(value);
  if (!m) return null;
  return {
    connection: m[1],
    path: (m[2] ?? "").replace(/^\/+|\/+$/g, ""),
    key: m[3] ?? null,
  };
}

export function formatSecretReference(connection: string, path: string, key: string | null): string {
  const clean = path.replace(/^\/+|\/+$/g, "");
  return `secret://${connection}/${clean}${key ? `#${key}` : ""}`;
}

export const PROVIDER_LABELS: Record<SecretProviderKind, string> = {
  vault: "HashiCorp Vault",
  openbao: "OpenBao",
  infisical: "Infisical",
  doppler: "Doppler",
  aws_secrets_manager: "AWS Secrets Manager",
};
