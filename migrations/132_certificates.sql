-- Migration 132: per-domain TLS certificates, custom uploads and DNS providers
-- for DNS-01 (wildcard) issuance. Idempotent.

-- DNS provider credentials for ACME DNS-01 challenges. `credentials` is a JSON
-- object encrypted with the instance encryption key when one is configured.
CREATE TABLE IF NOT EXISTS dns_providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    credentials TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- One row per hostname with certificate settings and issuance state.
-- mode: 'auto' (HTTP-01), 'dns' (DNS-01 via dns_provider_id, issues the name
-- and its wildcard) or 'custom' (uploaded PEM; key encrypted at rest).
CREATE TABLE IF NOT EXISTS domain_certificates (
    domain TEXT PRIMARY KEY,
    mode TEXT NOT NULL DEFAULT 'auto',
    dns_provider_id TEXT REFERENCES dns_providers(id) ON DELETE SET NULL,
    custom_cert_pem TEXT,
    custom_key TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    issuer TEXT,
    names TEXT,
    not_before TEXT,
    not_after TEXT,
    last_error TEXT,
    failure_count INTEGER NOT NULL DEFAULT 0,
    last_attempt_at TEXT,
    next_attempt_at TEXT,
    expiry_notified_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_domain_certificates_provider
    ON domain_certificates(dns_provider_id);
