/**
 * Community template submissions: users submit Docker Compose templates,
 * instance admins review them at /settings/templates. Approved ones appear in
 * the template catalogue marked "Community".
 */

import { apiRequest } from "./core";
import type { EnvSchemaEntry } from "@/types/api";

export type SubmissionStatus = "pending" | "approved" | "rejected";

export interface TemplateSubmission {
  id: string;
  name: string;
  description: string;
  category: string;
  icon: string | null;
  compose_content: string;
  env_hints: EnvSchemaEntry[];
  submitted_by: string;
  submitter_email: string | null;
  status: SubmissionStatus;
  admin_notes: string | null;
  rejection_reason: string | null;
  reviewed_by: string | null;
  reviewed_at: string | null;
  approved_template_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface SubmissionInput {
  name: string;
  description: string;
  category: string;
  icon?: string | null;
  compose_content: string;
  env_hints: EnvSchemaEntry[];
}

const BASE = "/templates/submissions";

export const templateSubmissionsApi = {
  submit: (data: SubmissionInput) =>
    apiRequest<TemplateSubmission>(BASE, { method: "POST", body: JSON.stringify(data) }),

  /** The signed-in user's own submissions */
  mine: () => apiRequest<TemplateSubmission[]>(`${BASE}/mine`),

  /** Admin review queue */
  list: (status?: SubmissionStatus) =>
    apiRequest<TemplateSubmission[]>(status ? `${BASE}?status=${status}` : BASE),

  /** Pending submissions (0 for non-admins) */
  pendingCount: () => apiRequest<{ count: number }>(`${BASE}/pending-count`),

  update: (id: string, data: SubmissionInput) =>
    apiRequest<TemplateSubmission>(`${BASE}/${id}`, { method: "PUT", body: JSON.stringify(data) }),

  approve: (id: string, edits?: SubmissionInput, adminNotes?: string) =>
    apiRequest<TemplateSubmission>(`${BASE}/${id}/approve`, {
      method: "POST",
      body: JSON.stringify({ edits: edits ?? null, admin_notes: adminNotes ?? null }),
    }),

  reject: (id: string, reason: string) =>
    apiRequest<TemplateSubmission>(`${BASE}/${id}/reject`, {
      method: "POST",
      body: JSON.stringify({ reason }),
    }),

  remove: (id: string) => apiRequest<void>(`${BASE}/${id}`, { method: "DELETE" }),
};
