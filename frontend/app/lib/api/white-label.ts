/**
 * White Label API module.
 * Handles fetching and updating instance branding configuration.
 */

import { apiRequest } from "./core";

export interface WhiteLabel {
  id: number;
  app_name: string;
  app_description: string | null;
  logo_url: string | null;
  favicon_url: string | null;
  custom_css: string | null;
  footer_text: string | null;
  support_url: string | null;
  docs_url: string | null;
  login_page_message: string | null;
  updated_at: string;
}

export interface UpdateWhiteLabelRequest {
  app_name?: string;
  app_description?: string | null;
  logo_url?: string | null;
  favicon_url?: string | null;
  custom_css?: string | null;
  footer_text?: string | null;
  support_url?: string | null;
  docs_url?: string | null;
  login_page_message?: string | null;
}

export const whiteLabelApi = {
  /** Get the current white label configuration (public, no auth required) */
  get: (): Promise<WhiteLabel> =>
    apiRequest<WhiteLabel>("/white-label"),

  /** Update the white label configuration (requires auth) */
  update: (data: UpdateWhiteLabelRequest): Promise<WhiteLabel> =>
    apiRequest<WhiteLabel>("/white-label", {
      method: "PUT",
      body: JSON.stringify(data),
    }),
};
