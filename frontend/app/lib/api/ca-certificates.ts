/**
 * CA Certificates API module.
 * Handles custom CA certificate management for servers using private CAs.
 */

import { apiRequest } from "./core";

export interface CaCertificate {
  id: string;
  name: string;
  certificate: string;
  team_id?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateCaCertificateRequest {
  name: string;
  certificate: string;
  team_id?: string;
}

export const caCertificatesApi = {
  /** List all CA certificates */
  list: (token?: string) =>
    apiRequest<CaCertificate[]>("/ca-certificates", {}, token),

  /** Create a new CA certificate */
  create: (data: CreateCaCertificateRequest, token?: string) =>
    apiRequest<CaCertificate>("/ca-certificates", {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** Delete a CA certificate by ID */
  delete: (id: string, token?: string) =>
    apiRequest<void>(`/ca-certificates/${id}`, { method: "DELETE" }, token),
};
