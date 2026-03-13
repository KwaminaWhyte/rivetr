/**
 * OAuth API module.
 * Handles OAuth login providers and user OAuth connections.
 */

import { apiRequest } from "./core";

/** Public OAuth provider info (for login page) */
export interface OAuthProviderPublic {
  provider: string;
  enabled: boolean;
}

/** OAuth provider config (admin view) */
export interface OAuthProviderResponse {
  id: string;
  provider: string;
  client_id: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
  /** Provider-specific extra config JSON (e.g. `{"tenant_id": "..."}` for Azure AD) */
  extra_config?: string | null;
}

/** OAuth authorization URL response */
export interface OAuthAuthorizeResponse {
  authorization_url: string;
  state: string;
}

/** User's linked OAuth connection */
export interface UserOAuthConnection {
  id: string;
  provider: string;
  provider_user_id: string;
  provider_email: string | null;
  provider_name: string | null;
  created_at: string;
}

/** Request to create/update an OAuth provider */
export interface CreateOAuthProviderRequest {
  provider: string;
  client_id: string;
  client_secret: string;
  enabled?: boolean;
  /** Provider-specific extra config JSON (e.g. `{"tenant_id": "..."}` for Azure AD) */
  extra_config?: string;
}

export const oauthApi = {
  // -------------------------------------------------------------------------
  // Public endpoints (no auth required)
  // -------------------------------------------------------------------------

  /** List enabled OAuth providers (for login page) */
  getEnabledProviders: () =>
    fetch("/api/auth/oauth/providers").then((res) => {
      if (!res.ok) return [];
      return res.json() as Promise<OAuthProviderPublic[]>;
    }),

  /** Get OAuth authorization URL for login */
  getLoginAuthorizeUrl: (provider: string) =>
    fetch(`/api/auth/oauth-login/${provider}/authorize`).then((res) => {
      if (!res.ok) throw new Error("Failed to get authorization URL");
      return res.json() as Promise<OAuthAuthorizeResponse>;
    }),

  // -------------------------------------------------------------------------
  // Admin endpoints (auth required)
  // -------------------------------------------------------------------------

  /** List all OAuth providers (admin) */
  getOAuthProviders: (token?: string) =>
    apiRequest<OAuthProviderResponse[]>("/settings/oauth-providers", {}, token),

  /** Create/update an OAuth provider (admin) */
  createOAuthProvider: (data: CreateOAuthProviderRequest, token?: string) =>
    apiRequest<OAuthProviderResponse>(
      "/settings/oauth-providers",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Delete an OAuth provider (admin) */
  deleteOAuthProvider: (id: string, token?: string) =>
    apiRequest<void>(
      `/settings/oauth-providers/${id}`,
      { method: "DELETE" },
      token
    ),

  // -------------------------------------------------------------------------
  // User OAuth connections (auth required)
  // -------------------------------------------------------------------------

  /** List current user's OAuth connections */
  getOAuthConnections: (token?: string) =>
    apiRequest<UserOAuthConnection[]>("/settings/oauth-connections", {}, token),

  /** Unlink an OAuth connection */
  deleteOAuthConnection: (id: string, token?: string) =>
    apiRequest<void>(
      `/settings/oauth-connections/${id}`,
      { method: "DELETE" },
      token
    ),
};
