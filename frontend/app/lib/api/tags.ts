/**
 * Resource tags API (E1): team-scoped labels on apps, databases and services.
 */

import { apiRequest } from "./core";

export type TaggableKind = "app" | "database" | "service";

export interface Tag {
  id: string;
  team_id: string | null;
  name: string;
  color: string;
  created_at: string;
  updated_at: string;
}

export interface TagWithCount extends Tag {
  resource_count: number;
}

export interface TagAssignment {
  tag_id: string;
  resource_type: TaggableKind;
  resource_id: string;
}

export interface TaggedResource {
  resource_type: TaggableKind;
  id: string;
  name: string;
  status: string | null;
  project_id: string | null;
  team_id: string | null;
}

export interface TagDeployResult {
  resource_type: TaggableKind;
  id: string;
  name: string;
  status: "queued" | "pending_approval" | "scheduled" | "skipped" | "frozen" | "failed";
  message: string | null;
  deployment_id: string | null;
}

export interface TagDeployResponse {
  tag: Tag;
  results: TagDeployResult[];
}

const PLURAL: Record<TaggableKind, string> = {
  app: "apps",
  database: "databases",
  service: "services",
};

function teamQuery(teamId?: string | null): string {
  return teamId ? `?team_id=${encodeURIComponent(teamId)}` : "";
}

export const tagsApi = {
  list: (teamId?: string | null) => apiRequest<TagWithCount[]>(`/tags${teamQuery(teamId)}`),

  create: (body: { name: string; color?: string; team_id?: string | null }) =>
    apiRequest<Tag>("/tags", { method: "POST", body: JSON.stringify(body) }),

  update: (id: string, body: { name?: string; color?: string }) =>
    apiRequest<Tag>(`/tags/${id}`, { method: "PUT", body: JSON.stringify(body) }),

  remove: (id: string) => apiRequest<void>(`/tags/${id}`, { method: "DELETE" }),

  resources: (id: string) => apiRequest<TaggedResource[]>(`/tags/${id}/resources`),

  deploy: (id: string) => apiRequest<TagDeployResponse>(`/tags/${id}/deploy`, { method: "POST" }),

  assignments: (params: {
    teamId?: string | null;
    resourceType?: TaggableKind;
    resourceId?: string;
  } = {}) => {
    const q = new URLSearchParams();
    if (params.teamId) q.set("team_id", params.teamId);
    if (params.resourceType) q.set("resource_type", params.resourceType);
    if (params.resourceId) q.set("resource_id", params.resourceId);
    const s = q.toString();
    return apiRequest<TagAssignment[]>(`/tags/assignments${s ? `?${s}` : ""}`);
  },

  /** Replace a resource's tags with exactly `tagIds`. */
  setForResource: (kind: TaggableKind, id: string, tagIds: string[]) =>
    apiRequest<Tag[]>(`/${PLURAL[kind]}/${id}/tags`, {
      method: "PUT",
      body: JSON.stringify({ tag_ids: tagIds }),
    }),

  attach: (kind: TaggableKind, id: string, tagId: string) =>
    apiRequest<Tag[]>(`/${PLURAL[kind]}/${id}/tags`, {
      method: "POST",
      body: JSON.stringify({ tag_id: tagId }),
    }),

  detach: (kind: TaggableKind, id: string, tagId: string) =>
    apiRequest<Tag[]>(`/${PLURAL[kind]}/${id}/tags/${tagId}`, { method: "DELETE" }),
};
