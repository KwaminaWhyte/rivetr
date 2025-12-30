/**
 * Notifications API module.
 * Handles notification channels and subscriptions.
 */

import { apiRequest } from "./core";
import type {
  NotificationChannel,
  NotificationSubscription,
  CreateNotificationChannelRequest,
  UpdateNotificationChannelRequest,
  CreateNotificationSubscriptionRequest,
  TestNotificationRequest,
} from "@/types/api";

export const notificationsApi = {
  // -------------------------------------------------------------------------
  // Notification Channels
  // -------------------------------------------------------------------------

  /** List all notification channels */
  getNotificationChannels: (token?: string) =>
    apiRequest<NotificationChannel[]>("/notification-channels", {}, token),

  /** Get a single notification channel */
  getNotificationChannel: (id: string, token?: string) =>
    apiRequest<NotificationChannel>(`/notification-channels/${id}`, {}, token),

  /** Create a new notification channel */
  createNotificationChannel: (
    data: CreateNotificationChannelRequest,
    token?: string
  ) =>
    apiRequest<NotificationChannel>(
      "/notification-channels",
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Update an existing notification channel */
  updateNotificationChannel: (
    id: string,
    data: UpdateNotificationChannelRequest,
    token?: string
  ) =>
    apiRequest<NotificationChannel>(
      `/notification-channels/${id}`,
      {
        method: "PUT",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Delete a notification channel */
  deleteNotificationChannel: (id: string, token?: string) =>
    apiRequest<void>(
      `/notification-channels/${id}`,
      {
        method: "DELETE",
      },
      token
    ),

  /** Test a notification channel by sending a test message */
  testNotificationChannel: (
    id: string,
    data?: TestNotificationRequest,
    token?: string
  ) =>
    apiRequest<void>(
      `/notification-channels/${id}/test`,
      {
        method: "POST",
        body: JSON.stringify(data || {}),
      },
      token
    ),

  // -------------------------------------------------------------------------
  // Notification Subscriptions
  // -------------------------------------------------------------------------

  /** Get all subscriptions for a channel */
  getNotificationSubscriptions: (channelId: string, token?: string) =>
    apiRequest<NotificationSubscription[]>(
      `/notification-channels/${channelId}/subscriptions`,
      {},
      token
    ),

  /** Create a new subscription */
  createNotificationSubscription: (
    channelId: string,
    data: CreateNotificationSubscriptionRequest,
    token?: string
  ) =>
    apiRequest<NotificationSubscription>(
      `/notification-channels/${channelId}/subscriptions`,
      {
        method: "POST",
        body: JSON.stringify(data),
      },
      token
    ),

  /** Delete a subscription */
  deleteNotificationSubscription: (id: string, token?: string) =>
    apiRequest<void>(
      `/notification-subscriptions/${id}`,
      {
        method: "DELETE",
      },
      token
    ),
};
