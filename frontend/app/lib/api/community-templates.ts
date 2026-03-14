/**
 * Community template submissions API module.
 */

import { apiRequest } from "./core";

export interface CommunityTemplateSubmission {
  id: string;
  name: string;
  description: string;
  category: string;
  icon: string | null;
  compose_content: string;
  submitted_by: string;
  status: "pending" | "approved" | "rejected";
  admin_notes: string | null;
  reviewed_by: string | null;
  reviewed_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface SubmitTemplateRequest {
  name: string;
  description: string;
  category: string;
  icon?: string;
  compose_content: string;
}

export interface ReviewSubmissionRequest {
  action: "approve" | "reject";
  notes?: string;
}

export const communityTemplatesApi = {
  /** Submit a new template for admin review */
  submit: (data: SubmitTemplateRequest, token?: string) =>
    apiRequest<CommunityTemplateSubmission>("/templates/submit", {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** Admin: list all submissions */
  listAll: (token?: string) =>
    apiRequest<CommunityTemplateSubmission[]>("/templates/submissions", {}, token),

  /** Get a specific submission */
  get: (id: string, token?: string) =>
    apiRequest<CommunityTemplateSubmission>(`/templates/submissions/${id}`, {}, token),

  /** Admin: approve or reject a submission */
  review: (id: string, data: ReviewSubmissionRequest, token?: string) =>
    apiRequest<CommunityTemplateSubmission>(`/templates/submissions/${id}/review`, {
      method: "PUT",
      body: JSON.stringify(data),
    }, token),

  /** Get the current user's own submissions */
  mySubmissions: (token?: string) =>
    apiRequest<CommunityTemplateSubmission[]>("/templates/my-submissions", {}, token),

  /** Delete a pending submission */
  delete: (id: string, token?: string) =>
    apiRequest<void>(`/templates/submissions/${id}`, { method: "DELETE" }, token),
};
