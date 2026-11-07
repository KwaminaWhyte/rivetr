// Instance-level settings: Advanced, Updates, Email and public IPs.

export type ConfirmationMode = "simple" | "two_step";

export interface AdvancedSettings {
  registration_enabled: boolean;
  allow_password_registration: boolean;
  destructive_confirmation: ConfirmationMode;
  api_access_enabled: boolean;
  /** IPs / CIDRs allowed to use API tokens. Empty = every source. */
  api_allowed_ips: string[];
  mcp_enabled: boolean;
  dns_validation_enabled: boolean;
  /** Nameservers for domain checks. Empty = the server's resolver. */
  dns_servers: string[];
  outbound_allowed_targets: string[];
  outbound_allow_localhost: boolean;
  /** Users may register passkeys and sign in with them. */
  passkeys_enabled: boolean;
}

export type UpdateAdvancedSettingsRequest = Partial<AdvancedSettings>;

export type UpdateChannel = "stable" | "beta";

export interface UpdateSettings {
  channel: UpdateChannel;
  check_enabled: boolean;
  check_cron: string;
  auto_update_enabled: boolean;
  auto_update_cron: string;
}

export type UpdateUpdateSettingsRequest = Partial<UpdateSettings>;

export type EmailProvider = "smtp" | "resend" | "sendry";
export type SmtpEncryption = "none" | "starttls" | "tls";

export interface EmailSettings {
  enabled: boolean;
  provider: EmailProvider;
  from_name: string | null;
  from_address: string | null;
  smtp_host: string | null;
  smtp_port: number | null;
  smtp_encryption: SmtpEncryption;
  smtp_username: string | null;
  smtp_password_set: boolean;
  smtp_timeout_secs: number;
  smtp_ehlo_domain: string | null;
  resend_api_key_set: boolean;
  sendry_api_key_set: boolean;
  /** Which configuration is active: dashboard, rivetr.toml, or none. */
  active_source: "database" | "config" | null;
  config_file_configured: boolean;
}

export interface UpdateEmailSettingsRequest {
  enabled?: boolean;
  provider?: EmailProvider;
  from_name?: string;
  from_address?: string;
  smtp_host?: string;
  smtp_port?: number;
  smtp_encryption?: SmtpEncryption;
  smtp_username?: string;
  /** Omit to keep, "" to clear. */
  smtp_password?: string;
  smtp_timeout_secs?: number;
  smtp_ehlo_domain?: string;
  /** Omit to keep, "" to clear. */
  resend_api_key?: string;
  /** Omit to keep, "" to clear. */
  sendry_api_key?: string;
}

export interface PublicIps {
  ipv4: string | null;
  ipv6: string | null;
}

export interface RegistrationStatus {
  password_registration: boolean;
  external_registration: boolean;
}

export interface RegisterResponse {
  token: string;
  user: { id: string; email: string; name: string; role: string };
}
