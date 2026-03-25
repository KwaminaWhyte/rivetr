/**
 * AI features API module.
 * Provides AI-powered diagnostics, insights, cost suggestions, dockerfile optimization, and security scanning.
 */
import { apiRequest } from "./core";

export const aiApi = {
  diagnoseDeployment: (appId: string, deploymentId: string) =>
    apiRequest<{ diagnosis: string; suggestions: string[] }>(
      `/apps/${appId}/deployments/${deploymentId}/diagnose`,
      { method: "POST" }
    ),

  getInsights: (appId: string) =>
    apiRequest<{
      summary: string;
      avg_build_minutes: number;
      success_rate_percent: number;
      total_deployments: number;
      trend: "improving" | "degrading" | "stable";
    }>(`/apps/${appId}/insights`),

  getCostSuggestions: (appId: string) =>
    apiRequest<{
      suggestions: Array<{ title: string; description: string; action: string }>;
    }>(`/apps/${appId}/cost-suggestions`),

  suggestDockerfile: (appId: string) =>
    apiRequest<{
      original: string;
      suggested: string;
      improvements: string[];
    }>(`/apps/${appId}/suggest-dockerfile`, { method: "POST" }),

  scanAppSecurity: (appId: string) =>
    apiRequest<{
      app_id: string;
      app_name: string;
      findings: Array<{
        severity: "critical" | "high" | "medium" | "low";
        category: string;
        title: string;
        description: string;
        recommendation: string;
      }>;
      critical: number;
      high: number;
      medium: number;
      low: number;
      ai_summary: string | null;
    }>(`/apps/${appId}/security-scan`),

  scanAllSecurity: () =>
    apiRequest<any[]>(`/security/scan`),
};
