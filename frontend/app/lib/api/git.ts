/**
 * Git API module.
 * Handles Git providers, SSH keys, and GitHub Apps.
 */

import { apiRequest } from "./core";
import type {
  GitProvider,
  GitRepository,
  OAuthAuthorizationResponse,
  SshKey,
  CreateSshKeyRequest,
  GitHubApp,
  GitHubAppInstallation,
  GitHubAppRepository,
  GitHubBranch,
  CreateGitHubAppManifestRequest,
  GitHubAppManifestResponse,
} from "@/types/api";

export const gitApi = {
  // -------------------------------------------------------------------------
  // SSH Keys
  // -------------------------------------------------------------------------

  /** List all SSH keys */
  getSshKeys: (token?: string) => apiRequest<SshKey[]>("/ssh-keys", {}, token),

  /** Create a new SSH key */
  createSshKey: (data: CreateSshKeyRequest, token?: string) =>
    apiRequest<SshKey>(
      "/ssh-keys",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Delete an SSH key */
  deleteSshKey: (id: string, token?: string) =>
    apiRequest<void>(
      `/ssh-keys/${id}`,
      {
        method: "DELETE",
      },
      token
    ),

  // -------------------------------------------------------------------------
  // Git Providers (PAT/Token-based)
  // -------------------------------------------------------------------------

  /** List all connected Git providers */
  getGitProviders: (token?: string) =>
    apiRequest<GitProvider[]>("/git-providers", {}, token),

  /** Get a single Git provider */
  getGitProvider: (id: string, token?: string) =>
    apiRequest<GitProvider>(`/git-providers/${id}`, {}, token),

  /** Add a Git provider via Personal Access Token (GitLab) or App Password (Bitbucket) */
  addGitProvider: (
    data: { provider: "gitlab" | "bitbucket"; token: string; username?: string },
    token?: string
  ) =>
    apiRequest<GitProvider>(
      "/git-providers",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Delete a Git provider connection */
  deleteGitProvider: (id: string, token?: string) =>
    apiRequest<void>(`/git-providers/${id}`, { method: "DELETE" }, token),

  /** Get repositories from a Git provider */
  getGitProviderRepos: (
    providerId: string,
    page = 1,
    perPage = 30,
    token?: string
  ) =>
    apiRequest<GitRepository[]>(
      `/git-providers/${providerId}/repos?page=${page}&per_page=${perPage}`,
      {},
      token
    ),

  // -------------------------------------------------------------------------
  // GitHub Apps
  // -------------------------------------------------------------------------

  /** List all GitHub Apps */
  getGitHubApps: (token?: string) =>
    apiRequest<GitHubApp[]>("/github-apps", {}, token),

  /** Get a single GitHub App */
  getGitHubApp: (id: string, token?: string) =>
    apiRequest<GitHubApp>(`/github-apps/${id}`, {}, token),

  /** Delete a GitHub App */
  deleteGitHubApp: (id: string, token?: string) =>
    apiRequest<void>(`/github-apps/${id}`, { method: "DELETE" }, token),

  /** Create a GitHub App via manifest flow */
  createGitHubAppManifest: (
    data: CreateGitHubAppManifestRequest,
    token?: string
  ) =>
    apiRequest<GitHubAppManifestResponse>(
      "/github-apps",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Get installations for a specific GitHub App */
  getGitHubAppInstallations: (appId: string, token?: string) =>
    apiRequest<GitHubAppInstallation[]>(
      `/github-apps/${appId}/installations`,
      {},
      token
    ),

  /** Get ALL installations across all GitHub Apps (for repository picker) */
  getAllGitHubAppInstallations: (token?: string) =>
    apiRequest<GitHubAppInstallation[]>("/github-apps/installations", {}, token),

  /** Get repositories from a GitHub App installation */
  getGitHubAppRepositories: (
    installationId: string,
    page = 1,
    perPage = 30,
    token?: string
  ) =>
    apiRequest<GitHubAppRepository[]>(
      `/github-apps/installations/${installationId}/repos?page=${page}&per_page=${perPage}`,
      {},
      token
    ),

  /** Get the install URL for a GitHub App */
  getGitHubAppInstallUrl: (appId: string, token?: string) =>
    apiRequest<{ install_url: string }>(
      `/github-apps/${appId}/install`,
      {},
      token
    ),

  /** Get branches for a repository via GitHub App installation */
  getGitHubAppRepoBranches: (
    installationId: string,
    owner: string,
    repo: string,
    token?: string
  ) =>
    apiRequest<GitHubBranch[]>(
      `/github-apps/installations/${installationId}/repos/${owner}/${repo}/branches`,
      {},
      token
    ),
};
