/**
 * Per-domain TLS certificates: status, mode (automatic, DNS provider, custom)
 * and custom certificate uploads for an app's or a service's domains.
 */

import { apiRequest } from "./core";

export type CertificateMode = "auto" | "dns" | "custom";

export type CertificateStatus =
  | "issued"
  | "pending"
  | "failed"
  | "custom"
  | "expiring"
  | "expired"
  | "no_https";

export interface DomainCertificate {
  domain: string;
  mode: CertificateMode;
  dns_provider_id: string | null;
  status: CertificateStatus;
  /** Certificate served right now. */
  serving: "custom" | "acme" | "legacy" | null;
  issuer: string | null;
  names: string[];
  not_after: string | null;
  last_error: string | null;
  failure_count: number;
  next_attempt_at: string | null;
  https_enabled: boolean;
  acme_enabled: boolean;
}

export type CertificateResource = "apps" | "services";

const base = (resource: CertificateResource, id: string) =>
  `/${resource}/${encodeURIComponent(id)}/certificates`;

export const certificatesApi = {
  list: (resource: CertificateResource, id: string) =>
    apiRequest<DomainCertificate[]>(base(resource, id)),

  setMode: (
    resource: CertificateResource,
    id: string,
    domain: string,
    mode: Exclude<CertificateMode, "custom">,
    dnsProviderId?: string,
  ) =>
    apiRequest<DomainCertificate>(`${base(resource, id)}/${encodeURIComponent(domain)}`, {
      method: "PUT",
      body: JSON.stringify({ mode, dns_provider_id: dnsProviderId ?? null }),
    }),

  upload: (
    resource: CertificateResource,
    id: string,
    domain: string,
    certificate: string,
    privateKey: string,
  ) =>
    apiRequest<DomainCertificate>(
      `${base(resource, id)}/${encodeURIComponent(domain)}/custom`,
      {
        method: "POST",
        body: JSON.stringify({ certificate, private_key: privateKey }),
      },
    ),

  removeCustom: (resource: CertificateResource, id: string, domain: string) =>
    apiRequest<DomainCertificate>(
      `${base(resource, id)}/${encodeURIComponent(domain)}/custom`,
      { method: "DELETE" },
    ),
};
