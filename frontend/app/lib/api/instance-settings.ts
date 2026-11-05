/**
 * Instance settings API: Advanced, Updates, Email, public IPs, registration.
 */

import { apiRequest } from "./core";
import type {
  AdvancedSettings,
  ConfirmationMode,
  EmailSettings,
  PublicIps,
  RegisterResponse,
  RegistrationStatus,
  UpdateAdvancedSettingsRequest,
  UpdateEmailSettingsRequest,
  UpdateSettings,
  UpdateUpdateSettingsRequest,
} from "@/types/instance-settings";

export const instanceSettingsApi = {
  getAdvancedSettings: () => apiRequest<AdvancedSettings>("/settings/advanced"),
  updateAdvancedSettings: (req: UpdateAdvancedSettingsRequest) =>
    apiRequest<AdvancedSettings>("/settings/advanced", {
      method: "PUT",
      body: JSON.stringify(req),
    }),
  getConfirmationMode: () =>
    apiRequest<{ mode: ConfirmationMode }>("/settings/destructive-confirmation"),

  getUpdateSettings: () => apiRequest<UpdateSettings>("/settings/updates"),
  updateUpdateSettings: (req: UpdateUpdateSettingsRequest) =>
    apiRequest<UpdateSettings>("/settings/updates", {
      method: "PUT",
      body: JSON.stringify(req),
    }),

  getEmailSettings: () => apiRequest<EmailSettings>("/settings/email"),
  updateEmailSettings: (req: UpdateEmailSettingsRequest) =>
    apiRequest<EmailSettings>("/settings/email", {
      method: "PUT",
      body: JSON.stringify(req),
    }),
  sendTestEmail: (to?: string) =>
    apiRequest<{ sent_to: string }>("/settings/email/test", {
      method: "POST",
      body: JSON.stringify({ to: to || null }),
    }),

  getPublicIps: () => apiRequest<PublicIps>("/settings/public-ip"),
  updatePublicIps: (req: { ipv4?: string; ipv6?: string }) =>
    apiRequest<PublicIps>("/settings/public-ip", {
      method: "PUT",
      body: JSON.stringify(req),
    }),
  detectPublicIps: () =>
    apiRequest<PublicIps>("/settings/public-ip/detect", { method: "POST" }),

  getRegistrationStatus: () => apiRequest<RegistrationStatus>("/auth/registration"),
  register: (req: { email: string; password: string; name: string }) =>
    apiRequest<RegisterResponse>("/auth/register", {
      method: "POST",
      body: JSON.stringify(req),
    }),
};
