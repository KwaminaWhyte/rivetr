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

export type DnsProviderKind = "cloudflare" | "route53" | "hetzner" | "digitalocean";

export const DNS_PROVIDER_LABELS: Record<DnsProviderKind, string> = {
  cloudflare: "Cloudflare",
  route53: "AWS Route 53",
  hetzner: "Hetzner DNS",
  digitalocean: "DigitalOcean",
};

export interface DnsCredentials {
  api_token?: string;
  access_key_id?: string;
  secret_access_key?: string;
  region?: string;
  hosted_zone_id?: string;
}

export interface DnsProvider {
  id: string;
  name: string;
  provider: DnsProviderKind;
  summary: string;
  used_by: string[];
  created_at: string;
  updated_at: string;
}

export interface WildcardCertificate {
  domain: string | null;
  dns_provider_id: string | null;
  suggested_domain: string | null;
  status: string | null;
  not_after: string | null;
  last_error: string | null;
}

export const dnsProvidersApi = {
  list: () => apiRequest<DnsProvider[]>("/dns-providers"),

  create: (data: { name: string; provider: DnsProviderKind; credentials: DnsCredentials }) =>
    apiRequest<DnsProvider>("/dns-providers", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  update: (id: string, data: { name?: string; credentials?: DnsCredentials }) =>
    apiRequest<DnsProvider>(`/dns-providers/${encodeURIComponent(id)}`, {
      method: "PUT",
      body: JSON.stringify(data),
    }),

  delete: (id: string) =>
    apiRequest<void>(`/dns-providers/${encodeURIComponent(id)}`, { method: "DELETE" }),

  verify: (id: string) =>
    apiRequest<{ ok: boolean; message: string }>(
      `/dns-providers/${encodeURIComponent(id)}/verify`,
      { method: "POST" },
    ),

  getWildcard: () => apiRequest<WildcardCertificate>("/certificates/wildcard"),

  setWildcard: (domain: string, dnsProviderId: string | null) =>
    apiRequest<WildcardCertificate>("/certificates/wildcard", {
      method: "PUT",
      body: JSON.stringify({ domain, dns_provider_id: dnsProviderId }),
    }),
};
