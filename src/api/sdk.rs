//! TypeScript SDK download endpoint.
//!
//! GET /api/sdk  — returns a TypeScript SDK file as a downloadable attachment.
//! Public, no auth required.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::AppState;

/// GET /api/sdk
///
/// Returns a complete TypeScript SDK as a downloadable `.ts` file.
/// The SDK is pre-configured with the server's external URL.
pub async fn get_sdk(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let base_url = state
        .config
        .server
        .external_url
        .clone()
        .unwrap_or_else(|| format!("http://localhost:{}", state.config.server.api_port));

    let sdk_content = generate_sdk(&base_url);

    match Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/typescript")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"rivetr-sdk.ts\"",
        )
        .body(sdk_content)
    {
        Ok(response) => response.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Generate the TypeScript SDK content.
fn generate_sdk(base_url: &str) -> String {
    format!(
        r#"/**
 * Rivetr TypeScript SDK
 * Auto-generated — download from GET /api/sdk
 *
 * Usage:
 *   const client = new RivetrClient("YOUR_API_TOKEN");
 *   const apps = await client.listApps();
 */

// -------------------------------------------------------------------------
// Types
// -------------------------------------------------------------------------

export interface App {{
  id: string;
  name: string;
  status: string;
  repo_url: string | null;
  branch: string | null;
  domain: string | null;
  created_at: string;
  updated_at: string;
}}

export interface Deployment {{
  id: string;
  app_id: string;
  status: string;
  started_at: string;
  finished_at: string | null;
  error_message: string | null;
  git_commit: string | null;
}}

export interface Database {{
  id: string;
  name: string;
  db_type: string;
  status: string;
  host: string | null;
  port: number | null;
  created_at: string;
}}

export interface Service {{
  id: string;
  name: string;
  status: string;
  image: string;
  port: number | null;
  created_at: string;
}}

export interface CreateAppRequest {{
  name: string;
  repo_url?: string;
  branch?: string;
  domain?: string;
  project_id?: string;
}}

export interface UpdateAppRequest {{
  name?: string;
  repo_url?: string;
  branch?: string;
  domain?: string;
}}

export interface CreateDatabaseRequest {{
  name: string;
  db_type: string;
  version?: string;
  project_id?: string;
}}

// -------------------------------------------------------------------------
// Client
// -------------------------------------------------------------------------

export class RivetrClient {{
  private baseUrl: string;
  private token: string;

  constructor(token: string, baseUrl: string = "{base_url}") {{
    this.token = token;
    this.baseUrl = baseUrl.replace(/\/$/, "");
  }}

  private async request<T>(
    method: string,
    path: string,
    body?: unknown
  ): Promise<T> {{
    const response = await fetch(`${{this.baseUrl}}/api${{path}}`, {{
      method,
      headers: {{
        "Authorization": `Bearer ${{this.token}}`,
        "Content-Type": "application/json",
      }},
      body: body !== undefined ? JSON.stringify(body) : undefined,
    }});

    if (!response.ok) {{
      const text = await response.text();
      let message = text;
      try {{
        const json = JSON.parse(text);
        message = json?.error?.message || json?.message || text;
      }} catch {{
        // use raw text
      }}
      throw new Error(`Rivetr API error ${{response.status}}: ${{message}}`);
    }}

    if (response.status === 204) return undefined as T;
    return response.json() as Promise<T>;
  }}

  // -----------------------------------------------------------------------
  // Apps
  // -----------------------------------------------------------------------

  /** List all apps */
  listApps(): Promise<App[]> {{
    return this.request<App[]>("GET", "/apps");
  }}

  /** Get a single app by ID */
  getApp(id: string): Promise<App> {{
    return this.request<App>("GET", `/apps/${{id}}`);
  }}

  /** Create a new app */
  createApp(data: CreateAppRequest): Promise<App> {{
    return this.request<App>("POST", "/apps", data);
  }}

  /** Update an existing app */
  updateApp(id: string, data: UpdateAppRequest): Promise<App> {{
    return this.request<App>("PUT", `/apps/${{id}}`, data);
  }}

  /** Delete an app */
  deleteApp(id: string): Promise<void> {{
    return this.request<void>("DELETE", `/apps/${{id}}`);
  }}

  // -----------------------------------------------------------------------
  // Deployments
  // -----------------------------------------------------------------------

  /** Trigger a new deployment for an app */
  triggerDeploy(appId: string): Promise<Deployment> {{
    return this.request<Deployment>("POST", `/apps/${{appId}}/deploy`);
  }}

  /** List deployments for an app */
  getDeployments(appId: string): Promise<Deployment[]> {{
    return this.request<Deployment[]>("GET", `/apps/${{appId}}/deployments`);
  }}

  /** Get a single deployment by ID */
  getDeployment(id: string): Promise<Deployment> {{
    return this.request<Deployment>("GET", `/deployments/${{id}}`);
  }}

  /** Cancel an in-progress deployment */
  cancelDeployment(appId: string, deploymentId: string): Promise<void> {{
    return this.request<void>("POST", `/apps/${{appId}}/deployments/${{deploymentId}}/cancel`);
  }}

  /** Roll back to a previous deployment */
  rollbackDeployment(deploymentId: string): Promise<Deployment> {{
    return this.request<Deployment>("POST", `/deployments/${{deploymentId}}/rollback`);
  }}

  // -----------------------------------------------------------------------
  // App Lifecycle
  // -----------------------------------------------------------------------

  /** Start an app */
  startApp(id: string): Promise<void> {{
    return this.request<void>("POST", `/apps/${{id}}/start`);
  }}

  /** Stop an app */
  stopApp(id: string): Promise<void> {{
    return this.request<void>("POST", `/apps/${{id}}/stop`);
  }}

  /** Restart an app */
  restartApp(id: string): Promise<void> {{
    return this.request<void>("POST", `/apps/${{id}}/restart`);
  }}

  // -----------------------------------------------------------------------
  // Databases
  // -----------------------------------------------------------------------

  /** List all databases */
  listDatabases(): Promise<Database[]> {{
    return this.request<Database[]>("GET", "/databases");
  }}

  /** Get a single database by ID */
  getDatabase(id: string): Promise<Database> {{
    return this.request<Database>("GET", `/databases/${{id}}`);
  }}

  /** Create a new database */
  createDatabase(data: CreateDatabaseRequest): Promise<Database> {{
    return this.request<Database>("POST", "/databases", data);
  }}

  /** Start a database */
  startDatabase(id: string): Promise<void> {{
    return this.request<void>("POST", `/databases/${{id}}/start`);
  }}

  /** Stop a database */
  stopDatabase(id: string): Promise<void> {{
    return this.request<void>("POST", `/databases/${{id}}/stop`);
  }}

  // -----------------------------------------------------------------------
  // Services
  // -----------------------------------------------------------------------

  /** List all Docker Compose services */
  listServices(): Promise<Service[]> {{
    return this.request<Service[]>("GET", "/services");
  }}

  /** Get a single service by ID */
  getService(id: string): Promise<Service> {{
    return this.request<Service>("GET", `/services/${{id}}`);
  }}

  /** Start a service */
  startService(id: string): Promise<void> {{
    return this.request<void>("POST", `/services/${{id}}/start`);
  }}

  /** Stop a service */
  stopService(id: string): Promise<void> {{
    return this.request<void>("POST", `/services/${{id}}/stop`);
  }}

  /** Restart a service */
  restartService(id: string): Promise<void> {{
    return this.request<void>("POST", `/services/${{id}}/restart`);
  }}
}}

export default RivetrClient;
"#,
        base_url = base_url
    )
}
