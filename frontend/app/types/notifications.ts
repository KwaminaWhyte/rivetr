// -------------------------------------------------------------------------
// Notification channel types
// -------------------------------------------------------------------------

/** Notification channel types */
export type NotificationChannelType = "slack" | "discord" | "email" | "telegram" | "teams" | "pushover" | "ntfy";

/** Notification event types */
export type NotificationEventType =
  | "deployment_started"
  | "deployment_success"
  | "deployment_failed"
  | "app_stopped"
  | "app_started";

/** Slack webhook configuration */
export interface SlackConfig {
  webhook_url: string;
}

/** Discord webhook configuration */
export interface DiscordConfig {
  webhook_url: string;
}

/** Email (SMTP) configuration */
export interface EmailConfig {
  smtp_host: string;
  smtp_port: number;
  smtp_username?: string;
  smtp_password?: string;
  smtp_tls: boolean;
  from_address: string;
  to_addresses: string[];
}

/** Telegram Bot API configuration */
export interface TelegramConfig {
  bot_token: string;
  chat_id: string;
  topic_id?: number;
}

/** Microsoft Teams Incoming Webhook configuration */
export interface TeamsConfig {
  webhook_url: string;
}

/** Pushover API configuration */
export interface PushoverConfig {
  user_key: string;
  app_token: string;
  device?: string;
  /** Priority: -2 (silent) to 2 (emergency), default 0 */
  priority?: number;
}

/** Ntfy notification configuration */
export interface NtfyConfig {
  /** Server URL, defaults to "https://ntfy.sh" if not set */
  server_url?: string;
  topic: string;
  /** Priority: 1 (min) to 5 (max), default 3 */
  priority?: number;
  /** Comma-separated tags for the notification */
  tags?: string;
}

/** Notification channel */
export interface NotificationChannel {
  id: string;
  name: string;
  channel_type: NotificationChannelType;
  config: SlackConfig | DiscordConfig | EmailConfig | TelegramConfig | TeamsConfig | PushoverConfig | NtfyConfig | Record<string, unknown>;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

/** Notification subscription */
export interface NotificationSubscription {
  id: string;
  channel_id: string;
  event_type: NotificationEventType;
  app_id: string | null;
  app_name: string | null;
  created_at: string;
}

/** Request to create a notification channel */
export interface CreateNotificationChannelRequest {
  name: string;
  channel_type: NotificationChannelType;
  config: SlackConfig | DiscordConfig | EmailConfig | TelegramConfig | TeamsConfig | PushoverConfig | NtfyConfig;
  enabled?: boolean;
}

/** Request to update a notification channel */
export interface UpdateNotificationChannelRequest {
  name?: string;
  config?: SlackConfig | DiscordConfig | EmailConfig | TelegramConfig | TeamsConfig | PushoverConfig | NtfyConfig;
  enabled?: boolean;
}

/** Request to create a notification subscription */
export interface CreateNotificationSubscriptionRequest {
  event_type: NotificationEventType;
  app_id?: string;
}

/** Request to test a notification channel */
export interface TestNotificationRequest {
  message?: string;
}

// -------------------------------------------------------------------------
// Team Notification Channel types
// -------------------------------------------------------------------------

/** Webhook configuration for team notification channels */
export interface WebhookConfig {
  url: string;
  headers?: Record<string, string>;
  payload_template?: "json" | "slack" | "discord" | "custom";
  custom_template?: string;
}

/** Team notification channel types including webhook, telegram, teams, pushover, and ntfy */
export type TeamNotificationChannelType =
  | "slack"
  | "discord"
  | "email"
  | "webhook"
  | "telegram"
  | "teams"
  | "pushover"
  | "ntfy";

/** Team notification channel */
export interface TeamNotificationChannel {
  id: string;
  team_id: string;
  name: string;
  channel_type: TeamNotificationChannelType;
  config:
    | SlackConfig
    | DiscordConfig
    | EmailConfig
    | WebhookConfig
    | TelegramConfig
    | TeamsConfig
    | PushoverConfig
    | NtfyConfig
    | Record<string, unknown>;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

/** Request to create a team notification channel */
export interface CreateTeamNotificationChannelRequest {
  name: string;
  channel_type: TeamNotificationChannelType;
  config: SlackConfig | DiscordConfig | EmailConfig | WebhookConfig | TelegramConfig | TeamsConfig | PushoverConfig | NtfyConfig;
  enabled?: boolean;
}

/** Request to update a team notification channel */
export interface UpdateTeamNotificationChannelRequest {
  name?: string;
  config?: SlackConfig | DiscordConfig | EmailConfig | WebhookConfig | TelegramConfig | TeamsConfig | PushoverConfig | NtfyConfig;
  enabled?: boolean;
}

// -------------------------------------------------------------------------
// Log Drain Types
// -------------------------------------------------------------------------

/** Log drain provider type */
export type LogDrainProvider = "axiom" | "newrelic" | "http" | "datadog" | "logtail";

/** Log drain configuration */
export interface LogDrain {
  id: string;
  app_id: string;
  name: string;
  provider: LogDrainProvider;
  config: Record<string, unknown>;
  enabled: boolean;
  last_sent_at: string | null;
  error_count: number;
  last_error: string | null;
  team_id: string | null;
  created_at: string;
  updated_at: string;
}

/** Request to create a log drain */
export interface CreateLogDrainRequest {
  name: string;
  provider: LogDrainProvider;
  config: Record<string, unknown>;
  enabled?: boolean;
}

/** Request to update a log drain */
export interface UpdateLogDrainRequest {
  name?: string;
  config?: Record<string, unknown>;
  enabled?: boolean;
}

/** Response from test log drain endpoint */
export interface TestLogDrainResponse {
  success: boolean;
  message: string;
}

// -------------------------------------------------------------------------
// Alert Configuration types
// -------------------------------------------------------------------------

/** Metric types for alerts */
export type AlertMetricType = "cpu" | "memory" | "disk";

/** Alert configuration response */
export interface AlertConfigResponse {
  id: string;
  app_id: string | null;
  metric_type: string;
  threshold_percent: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

/** Request to create an alert configuration */
export interface CreateAlertConfigRequest {
  metric_type: AlertMetricType;
  threshold_percent: number;
  enabled?: boolean;
}

/** Request to update an alert configuration */
export interface UpdateAlertConfigRequest {
  threshold_percent?: number;
  enabled?: boolean;
}

/** Alert event response (triggered alerts) */
export interface AlertEventResponse {
  id: string;
  app_id: string;
  metric_type: string;
  threshold_percent: number;
  current_value: number;
  status: "firing" | "resolved";
  consecutive_breaches: number;
  fired_at: string;
  resolved_at: string | null;
  last_notified_at: string | null;
}

/** Global alert default response */
export interface GlobalAlertDefaultResponse {
  id: string;
  metric_type: string;
  threshold_percent: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

/** Global alert defaults (all metric types) */
export interface GlobalAlertDefaultsResponse {
  cpu: GlobalAlertDefaultResponse | null;
  memory: GlobalAlertDefaultResponse | null;
  disk: GlobalAlertDefaultResponse | null;
}

/** Request to update global alert defaults */
export interface UpdateGlobalAlertDefaultsRequest {
  cpu?: GlobalAlertDefaultUpdate;
  memory?: GlobalAlertDefaultUpdate;
  disk?: GlobalAlertDefaultUpdate;
}

/** Update for a single metric type's global default */
export interface GlobalAlertDefaultUpdate {
  threshold_percent?: number;
  enabled?: boolean;
}

/** Alert configuration statistics */
export interface AlertStatsResponse {
  total_apps: number;
  apps_with_custom_configs: number;
  apps_using_defaults: number;
}
