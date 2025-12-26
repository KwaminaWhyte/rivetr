import type {
  App,
  CreateAppRequest,
  CreateSshKeyRequest,
  Deployment,
  DeploymentLog,
  GitProvider,
  GitProviderType,
  GitRepository,
  OAuthAuthorizationResponse,
  SshKey,
  UpdateAppRequest,
  UpdateSshKeyRequest,
} from "@/types/api";

const API_BASE = "/api";

export interface LoginResponse {
  token: string;
  user: {
    id: string;
    email: string;
    name: string;
  };
}

export interface SetupRequest {
  name: string;
  email: string;
  password: string;
}

export interface SetupStatusResponse {
  needs_setup: boolean;
}

class ApiClient {
  private token: string | null = null;

  setToken(token: string | null) {
    this.token = token;
    if (token) {
      localStorage.setItem("rivetr_token", token);
    } else {
      localStorage.removeItem("rivetr_token");
    }
  }

  getToken(): string | null {
    if (!this.token) {
      this.token = localStorage.getItem("rivetr_token");
    }
    return this.token;
  }

  private async request<T>(
    path: string,
    options: RequestInit = {}
  ): Promise<T> {
    const token = this.getToken();
    const headers: HeadersInit = {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...options.headers,
    };

    const response = await fetch(`${API_BASE}${path}`, {
      ...options,
      headers,
    });

    if (response.status === 401) {
      this.setToken(null);
      window.location.href = "/login";
      throw new Error("Unauthorized");
    }

    if (!response.ok) {
      const error = await response.text();
      throw new Error(error || `HTTP ${response.status}`);
    }

    if (response.status === 204) {
      return undefined as T;
    }

    return response.json();
  }

  // Auth
  async login(email: string, password: string): Promise<LoginResponse> {
    const response = await fetch(`${API_BASE}/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, password }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(error || "Login failed");
    }

    const data = await response.json();
    this.setToken(data.token);
    return data;
  }

  async validateToken(): Promise<boolean> {
    const token = this.getToken();
    if (!token) return false;

    try {
      const response = await fetch(`${API_BASE}/auth/validate`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      return response.ok;
    } catch {
      return false;
    }
  }

  async checkSetupStatus(): Promise<SetupStatusResponse> {
    const response = await fetch(`${API_BASE}/auth/setup-status`);
    return response.json();
  }

  async setup(data: SetupRequest): Promise<LoginResponse> {
    const response = await fetch(`${API_BASE}/auth/setup`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(error || "Setup failed");
    }

    const result = await response.json();
    this.setToken(result.token);
    return result;
  }

  // Apps
  async getApps(): Promise<App[]> {
    return this.request<App[]>("/apps");
  }

  async getApp(id: string): Promise<App> {
    return this.request<App>(`/apps/${id}`);
  }

  async createApp(data: CreateAppRequest): Promise<App> {
    return this.request<App>("/apps", {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async updateApp(id: string, data: UpdateAppRequest): Promise<App> {
    return this.request<App>(`/apps/${id}`, {
      method: "PUT",
      body: JSON.stringify(data),
    });
  }

  async deleteApp(id: string): Promise<void> {
    return this.request<void>(`/apps/${id}`, {
      method: "DELETE",
    });
  }

  // Deployments
  async triggerDeploy(appId: string): Promise<Deployment> {
    return this.request<Deployment>(`/apps/${appId}/deploy`, {
      method: "POST",
    });
  }

  async getDeployments(appId: string): Promise<Deployment[]> {
    return this.request<Deployment[]>(`/apps/${appId}/deployments`);
  }

  async getDeployment(id: string): Promise<Deployment> {
    return this.request<Deployment>(`/deployments/${id}`);
  }

  async getDeploymentLogs(id: string): Promise<DeploymentLog[]> {
    return this.request<DeploymentLog[]>(`/deployments/${id}/logs`);
  }

  async rollbackDeployment(id: string): Promise<Deployment> {
    return this.request<Deployment>(`/deployments/${id}/rollback`, {
      method: "POST",
    });
  }

  // SSH Keys
  async getSshKeys(): Promise<SshKey[]> {
    return this.request<SshKey[]>("/ssh-keys");
  }

  async getSshKey(id: string): Promise<SshKey> {
    return this.request<SshKey>(`/ssh-keys/${id}`);
  }

  async createSshKey(data: CreateSshKeyRequest): Promise<SshKey> {
    return this.request<SshKey>("/ssh-keys", {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async updateSshKey(id: string, data: UpdateSshKeyRequest): Promise<SshKey> {
    return this.request<SshKey>(`/ssh-keys/${id}`, {
      method: "PUT",
      body: JSON.stringify(data),
    });
  }

  async deleteSshKey(id: string): Promise<void> {
    return this.request<void>(`/ssh-keys/${id}`, {
      method: "DELETE",
    });
  }

  // Runtime logs WebSocket URL
  getRuntimeLogsWsUrl(appId: string): string {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const token = this.getToken();
    return `${protocol}//${window.location.host}/api/apps/${appId}/logs/stream?token=${token}`;
  }

  // Git Providers (OAuth)
  async getGitProviders(): Promise<GitProvider[]> {
    return this.request<GitProvider[]>("/git-providers");
  }

  async getGitProvider(id: string): Promise<GitProvider> {
    return this.request<GitProvider>(`/git-providers/${id}`);
  }

  async deleteGitProvider(id: string): Promise<void> {
    return this.request<void>(`/git-providers/${id}`, {
      method: "DELETE",
    });
  }

  async getOAuthAuthorizationUrl(provider: GitProviderType): Promise<OAuthAuthorizationResponse> {
    return this.request<OAuthAuthorizationResponse>(`/auth/oauth/${provider}/authorize`);
  }

  async getProviderRepos(providerId: string): Promise<GitRepository[]> {
    return this.request<GitRepository[]>(`/git-providers/${providerId}/repos`);
  }
}

export const api = new ApiClient();
export default api;
