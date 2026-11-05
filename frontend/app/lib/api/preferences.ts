/**
 * Per-user UI preferences (theme) and the instance default theme.
 * The theme is stored as an opaque JSON object; the dashboard validates it.
 */

import { apiRequest } from "./core";

export interface ThemePreferencesResponse {
  /** The user's saved theme, null when they never customised it. */
  theme: unknown | null;
  /** Instance default set by an admin, null when unset. */
  instance_default: unknown | null;
}

export const preferencesApi = {
  getThemePreferences: () => apiRequest<ThemePreferencesResponse>("/preferences/theme"),

  /** Save the user's theme; pass null to clear it and use the instance default. */
  updateThemePreferences: (theme: unknown | null) =>
    apiRequest<ThemePreferencesResponse>("/preferences/theme", {
      method: "PUT",
      body: JSON.stringify({ theme }),
    }),

  /** Admin only: set (or clear with null) the instance default theme. */
  updateDefaultTheme: (theme: unknown | null) =>
    apiRequest<ThemePreferencesResponse>("/settings/default-theme", {
      method: "PUT",
      body: JSON.stringify({ theme }),
    }),
};
