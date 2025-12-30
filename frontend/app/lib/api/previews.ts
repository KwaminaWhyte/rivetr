/**
 * Previews API module.
 * Handles preview deployments for pull requests.
 */

import { apiRequest } from "./core";
import type { PreviewDeployment } from "@/types/api";

export const previewsApi = {
  /** Get all preview deployments for an app */
  getAppPreviews: (appId: string, token?: string) =>
    apiRequest<PreviewDeployment[]>(`/apps/${appId}/previews`, {}, token),

  /** Get a single preview deployment */
  getPreview: (id: string, token?: string) =>
    apiRequest<PreviewDeployment>(`/previews/${id}`, {}, token),

  /** Delete a preview deployment */
  deletePreview: (id: string, token?: string) =>
    apiRequest<void>(`/previews/${id}`, { method: "DELETE" }, token),

  /** Redeploy a preview */
  redeployPreview: (id: string, token?: string) =>
    apiRequest<PreviewDeployment>(
      `/previews/${id}/redeploy`,
      { method: "POST" },
      token
    ),
};
