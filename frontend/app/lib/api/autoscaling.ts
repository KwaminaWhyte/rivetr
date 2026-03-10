/**
 * Autoscaling Rules API module.
 */

import { apiRequest } from "./core";
import type { AutoscalingRule, CreateAutoscalingRuleRequest } from "../../types/apps";

export const autoscalingApi = {
  /**
   * List all autoscaling rules for an app
   */
  list: (appId: string): Promise<AutoscalingRule[]> =>
    apiRequest<AutoscalingRule[]>(`/apps/${appId}/autoscaling`),

  /**
   * Create a new autoscaling rule
   */
  create: (appId: string, req: CreateAutoscalingRuleRequest): Promise<AutoscalingRule> =>
    apiRequest<AutoscalingRule>(`/apps/${appId}/autoscaling`, {
      method: "POST",
      body: JSON.stringify(req),
    }),

  /**
   * Update an existing autoscaling rule
   */
  update: (
    appId: string,
    ruleId: string,
    req: CreateAutoscalingRuleRequest
  ): Promise<AutoscalingRule> =>
    apiRequest<AutoscalingRule>(`/apps/${appId}/autoscaling/${ruleId}`, {
      method: "PUT",
      body: JSON.stringify(req),
    }),

  /**
   * Delete an autoscaling rule
   */
  delete: (appId: string, ruleId: string): Promise<void> =>
    apiRequest<void>(`/apps/${appId}/autoscaling/${ruleId}`, {
      method: "DELETE",
    }),
};
