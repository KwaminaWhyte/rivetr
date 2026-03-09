/**
 * Scheduled Jobs API module.
 * Handles CRUD operations for cron-based scheduled jobs
 * that run commands inside app containers.
 */

import { apiRequest } from "./core";
import type {
  ScheduledJob,
  ScheduledJobRun,
  CreateScheduledJobRequest,
  UpdateScheduledJobRequest,
} from "@/types/api";

export const jobsApi = {
  /** List all scheduled jobs for an app */
  getJobs: (appId: string, token?: string) =>
    apiRequest<ScheduledJob[]>(`/apps/${appId}/jobs`, {}, token),

  /** Get a single scheduled job */
  getJob: (appId: string, jobId: string, token?: string) =>
    apiRequest<ScheduledJob>(`/apps/${appId}/jobs/${jobId}`, {}, token),

  /** Create a new scheduled job */
  createJob: (appId: string, data: CreateScheduledJobRequest, token?: string) =>
    apiRequest<ScheduledJob>(`/apps/${appId}/jobs`, {
      method: "POST",
      body: JSON.stringify(data),
    }, token),

  /** Update an existing scheduled job */
  updateJob: (appId: string, jobId: string, data: UpdateScheduledJobRequest, token?: string) =>
    apiRequest<ScheduledJob>(`/apps/${appId}/jobs/${jobId}`, {
      method: "PUT",
      body: JSON.stringify(data),
    }, token),

  /** Delete a scheduled job */
  deleteJob: (appId: string, jobId: string, token?: string) =>
    apiRequest<void>(`/apps/${appId}/jobs/${jobId}`, {
      method: "DELETE",
    }, token),

  /** Manually trigger a job run */
  triggerJobRun: (appId: string, jobId: string, token?: string) =>
    apiRequest<ScheduledJobRun>(`/apps/${appId}/jobs/${jobId}/run`, {
      method: "POST",
    }, token),

  /** List job run history */
  getJobRuns: (appId: string, jobId: string, params?: { limit?: number; offset?: number }, token?: string) => {
    const query = new URLSearchParams();
    if (params?.limit) query.append("limit", params.limit.toString());
    if (params?.offset) query.append("offset", params.offset.toString());
    const qs = query.toString();
    return apiRequest<ScheduledJobRun[]>(
      `/apps/${appId}/jobs/${jobId}/runs${qs ? `?${qs}` : ""}`,
      {},
      token
    );
  },
};
