/**
 * Database ↔ App link API.
 *
 * A "link" makes a managed database's connection details (DATABASE_URL,
 * REDIS_URL, MONGODB_URL, plus host/port/user/password/db) auto-inject into
 * the linked app's container env at deploy time.
 */

import { apiRequest } from "./core";

export interface DatabaseAppLink {
  id: string;
  database_id: string;
  app_id: string;
  env_prefix: string;
  created_at: string;
  database_name: string;
  database_type: string;
  database_status: string;
}

export interface CreateDatabaseLinkRequest {
  database_id: string;
  env_prefix?: string;
}

export interface LinkedEnvVarPreview {
  key: string;
  /** True if the app already defines this key, so injection is suppressed. */
  overridden: boolean;
}

export interface LinkedEnvVarsForDatabase {
  link_id: string;
  database_id: string;
  database_name: string;
  env_prefix: string;
  vars: LinkedEnvVarPreview[];
}

export const databaseLinksApi = {
  /** List all database→app links for an app. */
  listLinks: (appId: string, token?: string) =>
    apiRequest<DatabaseAppLink[]>(`/apps/${appId}/links`, {}, token),

  /** Create a new link. */
  createLink: (
    appId: string,
    data: CreateDatabaseLinkRequest,
    token?: string,
  ) =>
    apiRequest<DatabaseAppLink>(
      `/apps/${appId}/links`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token,
    ),

  /** Remove a link. */
  deleteLink: (appId: string, linkId: string, token?: string) =>
    apiRequest<void>(
      `/apps/${appId}/links/${linkId}`,
      { method: "DELETE" },
      token,
    ),

  /** Preview the env var keys that would be injected from currently-linked DBs. */
  previewLinkedEnvVars: (appId: string, token?: string) =>
    apiRequest<LinkedEnvVarsForDatabase[]>(
      `/apps/${appId}/linked-env-vars`,
      {},
      token,
    ),
};
