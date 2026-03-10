/**
 * SSO / OIDC API module.
 * Handles OIDC provider management for enterprise single sign-on.
 */

import { apiRequest } from "./core";

/** OIDC provider configuration (admin view, secret excluded) */
export interface OidcProvider {
  id: string;
  name: string;
  client_id: string;
  discovery_url: string;
  redirect_uri: string;
  scopes: string;
  enabled: boolean;
  team_id?: string;
  created_at: string;
  updated_at: string;
}

/** Request to create or update an OIDC provider */
export interface CreateOidcProviderRequest {
  name: string;
  client_id: string;
  client_secret: string;
  discovery_url: string;
  redirect_uri?: string;
  scopes?: string;
  team_id?: string;
  enabled?: boolean;
}

/** Well-known discovery URLs for common OIDC providers */
export const WELL_KNOWN_PROVIDERS = [
  {
    label: "Google",
    discovery_url:
      "https://accounts.google.com/.well-known/openid-configuration",
    scopes: "openid email profile",
  },
  {
    label: "Microsoft / Azure AD",
    discovery_url:
      "https://login.microsoftonline.com/common/v2.0/.well-known/openid-configuration",
    scopes: "openid email profile",
  },
  {
    label: "Okta",
    discovery_url: "https://{your-domain}.okta.com/.well-known/openid-configuration",
    scopes: "openid email profile",
  },
  {
    label: "Auth0",
    discovery_url: "https://{your-tenant}.auth0.com/.well-known/openid-configuration",
    scopes: "openid email profile",
  },
  {
    label: "Keycloak",
    discovery_url:
      "https://{your-host}/realms/{your-realm}/.well-known/openid-configuration",
    scopes: "openid email profile",
  },
] as const;

export const ssoApi = {
  /** List all OIDC providers (admin) */
  listProviders: (token?: string) =>
    apiRequest<OidcProvider[]>("/sso/providers", {}, token),

  /** Create an OIDC provider (admin) */
  createProvider: (data: CreateOidcProviderRequest, token?: string) =>
    apiRequest<OidcProvider>(
      "/sso/providers",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Get an OIDC provider (admin) */
  getProvider: (id: string, token?: string) =>
    apiRequest<OidcProvider>(`/sso/providers/${id}`, {}, token),

  /** Update an OIDC provider (admin) */
  updateProvider: (
    id: string,
    data: Partial<CreateOidcProviderRequest>,
    token?: string
  ) =>
    apiRequest<OidcProvider>(
      `/sso/providers/${id}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Delete an OIDC provider (admin) */
  deleteProvider: (id: string, token?: string) =>
    apiRequest<void>(`/sso/providers/${id}`, { method: "DELETE" }, token),

  /** Get the URL to initiate SSO login for a provider */
  getLoginUrl: (providerId: string) => `/auth/sso/${providerId}/login`,
};
