/**
 * Two-Factor Authentication API module.
 * Handles 2FA setup, verification, validation, and disabling.
 */

import { apiRequest } from "./core";

export interface TwoFactorSetupResponse {
  secret: string;
  qr_code_url: string;
  qr_code_svg: string;
}

export interface TwoFactorVerifyResponse {
  recovery_codes: string[];
}

export interface TwoFactorStatusResponse {
  enabled: boolean;
}

export interface TwoFactorValidateResponse {
  token: string;
  user: {
    id: string;
    email: string;
    name: string;
    role: string;
    totp_enabled: boolean;
  };
}

export const twoFactorApi = {
  /**
   * Start 2FA setup - generates TOTP secret and QR code.
   * Requires authentication.
   */
  setup: (): Promise<TwoFactorSetupResponse> =>
    apiRequest<TwoFactorSetupResponse>("/auth/2fa/setup", {
      method: "POST",
    }),

  /**
   * Verify a TOTP code during setup to confirm authenticator is configured.
   * Returns one-time recovery codes on success.
   * Requires authentication.
   */
  verify: (code: string): Promise<TwoFactorVerifyResponse> =>
    apiRequest<TwoFactorVerifyResponse>("/auth/2fa/verify", {
      method: "POST",
      body: JSON.stringify({ code }),
    }),

  /**
   * Disable 2FA for the current user.
   * Requires a valid TOTP code or the user's password.
   * Requires authentication.
   */
  disable: (code: string): Promise<void> =>
    apiRequest<void>("/auth/2fa/disable", {
      method: "POST",
      body: JSON.stringify({ code }),
    }),

  /**
   * Get 2FA status for the current user.
   * Requires authentication.
   */
  getStatus: (): Promise<TwoFactorStatusResponse> =>
    apiRequest<TwoFactorStatusResponse>("/auth/2fa/status"),

  /**
   * Validate a TOTP code during login (for users with 2FA enabled).
   * Uses a temporary session token from the login response.
   * This is a public endpoint (no auth required, uses session_token in body).
   */
  validate: (
    sessionToken: string,
    code: string
  ): Promise<TwoFactorValidateResponse> => {
    // This endpoint is on /api/auth/2fa/validate which is a public auth route
    // We need to make a raw fetch since it doesn't use the normal auth header
    return fetch("/api/auth/2fa/validate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ session_token: sessionToken, code }),
    }).then(async (response) => {
      if (!response.ok) {
        const error = await response.text();
        throw new Error(error || `Validation failed: ${response.status}`);
      }
      return response.json();
    });
  },
};
