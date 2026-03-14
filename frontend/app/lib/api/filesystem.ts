/**
 * Remote filesystem browser API module.
 */

import { apiRequest } from "./core";

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number | null;
  modified: string | null;
  permissions: string | null;
}

export const filesystemApi = {
  /** List directory contents on a remote server */
  browse: (serverId: string, path?: string, token?: string) => {
    const params = path ? `?path=${encodeURIComponent(path)}` : "";
    return apiRequest<FileEntry[]>(`/servers/${serverId}/files${params}`, {}, token);
  },

  /** Read a file's content from a remote server */
  readFile: (serverId: string, path: string, token?: string) =>
    apiRequest<{ content: string }>(
      `/servers/${serverId}/files/content?path=${encodeURIComponent(path)}`,
      {},
      token
    ),

  /** Write content to a remote file */
  writeFile: (serverId: string, path: string, content: string, token?: string) =>
    apiRequest<{ message: string }>(`/servers/${serverId}/files/content`, {
      method: "PUT",
      body: JSON.stringify({ path, content }),
    }, token),

  /** Delete a file or directory on a remote server */
  delete: (serverId: string, path: string, token?: string) =>
    apiRequest<void>(`/servers/${serverId}/files?path=${encodeURIComponent(path)}`, {
      method: "DELETE",
    }, token),
};
