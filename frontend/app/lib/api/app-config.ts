/**
 * App configuration API: structured healthcheck, container file browser (apps,
 * databases, services) and resource cloning.
 */

import { apiRawRequest, apiRequest } from "./core";
import type {
  CloneJob,
  CloneResourceRequest,
  CloneResourceResponse,
  CloneableResource,
  ContainerFileContent,
  ContainerFileList,
  HealthcheckConfig,
} from "@/types/app-config";

export type FileBrowserResource = "app" | "database" | "service";

const PLURAL: Record<FileBrowserResource, string> = {
  app: "apps",
  database: "databases",
  service: "services",
};

function filesBase(kind: FileBrowserResource, id: string): string {
  return `/${PLURAL[kind]}/${id}/files`;
}

function query(params: Record<string, string | undefined>): string {
  const qs = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== "") qs.set(k, v);
  }
  const s = qs.toString();
  return s ? `?${s}` : "";
}

export interface FileTarget {
  kind: FileBrowserResource;
  id: string;
  /** Service container name (services only) */
  container?: string;
}

export const appConfigApi = {
  getHealthcheck: (appId: string) =>
    apiRequest<HealthcheckConfig>(`/apps/${appId}/healthcheck`),

  updateHealthcheck: (appId: string, config: HealthcheckConfig) =>
    apiRequest<HealthcheckConfig>(`/apps/${appId}/healthcheck`, {
      method: "PUT",
      body: JSON.stringify(config),
    }),

  listFiles: (t: FileTarget, path: string) =>
    apiRequest<ContainerFileList>(
      `${filesBase(t.kind, t.id)}${query({ path, container: t.container })}`
    ),

  readFile: (t: FileTarget, path: string) =>
    apiRequest<ContainerFileContent>(
      `${filesBase(t.kind, t.id)}/content${query({ path, container: t.container })}`
    ),

  writeFile: (t: FileTarget, path: string, content: string) =>
    apiRequest<void>(`${filesBase(t.kind, t.id)}/content${query({ container: t.container })}`, {
      method: "PUT",
      body: JSON.stringify({ path, content }),
    }),

  createFolder: (t: FileTarget, path: string) =>
    apiRequest<void>(`${filesBase(t.kind, t.id)}/mkdir${query({ container: t.container })}`, {
      method: "POST",
      body: JSON.stringify({ path }),
    }),

  deletePath: (t: FileTarget, path: string) =>
    apiRequest<void>(`${filesBase(t.kind, t.id)}${query({ path, container: t.container })}`, {
      method: "DELETE",
    }),

  uploadFile: async (t: FileTarget, dir: string, file: File) => {
    const form = new FormData();
    form.append("file", file, file.name);
    await apiRawRequest(
      `${filesBase(t.kind, t.id)}/upload${query({ path: dir, container: t.container })}`,
      { method: "POST", body: form }
    );
  },

  /** Download a file and hand it to the browser as a save. */
  downloadFile: async (t: FileTarget, path: string, name: string) => {
    const res = await apiRawRequest(
      `${filesBase(t.kind, t.id)}/download${query({ path, container: t.container })}`
    );
    const blob = await res.blob();
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = name;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  },

  cloneResource: (kind: CloneableResource, id: string, req: CloneResourceRequest) =>
    apiRequest<CloneResourceResponse>(`/${PLURAL[kind]}/${id}/clone-resource`, {
      method: "POST",
      body: JSON.stringify(req),
    }),

  getCloneJob: (jobId: string) => apiRequest<CloneJob>(`/clone-jobs/${jobId}`),
};
