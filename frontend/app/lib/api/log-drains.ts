/**
 * Log Drains API module.
 * Handles log drain CRUD and testing for app log forwarding.
 */

import { apiRequest } from "./core";
import type {
  LogDrain,
  CreateLogDrainRequest,
  UpdateLogDrainRequest,
  TestLogDrainResponse,
} from "@/types/api";

export const logDrainsApi = {
  /** List all log drains for an app */
  getLogDrains: (appId: string, token?: string) =>
    apiRequest<LogDrain[]>(`/apps/${appId}/log-drains`, {}, token),

  /** Create a new log drain for an app */
  createLogDrain: (
    appId: string,
    data: CreateLogDrainRequest,
    token?: string
  ) =>
    apiRequest<LogDrain>(`/apps/${appId}/log-drains`, {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** Update an existing log drain */
  updateLogDrain: (
    appId: string,
    drainId: string,
    data: UpdateLogDrainRequest,
    token?: string
  ) =>
    apiRequest<LogDrain>(`/apps/${appId}/log-drains/${drainId}`, {
      method: "PUT",
      body: JSON.stringify(data),
    }, token),

  /** Delete a log drain */
  deleteLogDrain: (appId: string, drainId: string, token?: string) =>
    apiRequest<void>(`/apps/${appId}/log-drains/${drainId}`, {
      method: "DELETE",
    }, token),

  /** Test a log drain by sending a test log entry */
  testLogDrain: (appId: string, drainId: string, token?: string) =>
    apiRequest<TestLogDrainResponse>(
      `/apps/${appId}/log-drains/${drainId}/test`,
      {
        method: "POST",
      },
      token
    ),
};
