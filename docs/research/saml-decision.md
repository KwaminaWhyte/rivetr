# Decision: SAML 2.0 SSO (deferred)

Date: 2026-11-05. Status: deferred. Revisit: when the criteria below are met.

## Question

Can Rivetr add SAML 2.0 SP-initiated login (signed assertions, IdP metadata,
attribute mapping) without system `libxml2` / `xmlsec1` / OpenSSL, so it
stays a single static binary cross-compiled with cargo-zigbuild?

## Findings

| Option | Pure Rust | Verifies signatures | Notes |
|--------|-----------|---------------------|-------|
| `samael` 0.0.22 | No | Only with the `xmlsec` feature | `xmlsec` (libxml2 + xmlsec1 via bindgen) is the only signing backend and is on by default. `openssl` is a hard dependency even with it off. With `xmlsec` off, `verify_signed_xml` returns `Ok(())` without checking anything, so it cannot be used for login. |
| `saml-rs` 0.6.1 on `bergshamra` 0.11.1 | Yes | Yes (XML-DSig, exclusive C14N, RSA-SHA256) | First release 2026-06, pre-1.0 with breaking minor releases, a handful of users, no external security audit. `bergshamra` and its XML parser (`uppsala`) and crypto layer (`kryptering`) were all created in 2026. |
| `xml_c14n`, `xmlsec` crates | No | n/a | Thin wrappers over libxml2 / xmlsec1. |
| Hand-rolled XML-DSig on `quick-xml` + `rsa` | Yes | Would have to be written | Exclusive C14N with InclusiveNamespaces, ID reference resolution and binding the verified element to the one actually read are where signature-wrapping, comment-injection and namespace-confusion bugs keep appearing (ruby-saml, samlify, xmlseclibs CVEs). Weeks of work plus an attack-test corpus. |

## Decision

Do not ship SAML now.

- `samael` needs native libraries (and always OpenSSL), which breaks the
  single-binary build.
- The only pure-Rust stack (`saml-rs` + `bergshamra`) is technically capable
  but months old and unaudited. SAML signature verification is the
  authentication boundary for every SSO login; a bug there is an account
  takeover, so it should not rest on brand-new, unaudited XML crypto.
- Hand-rolling XML-DSig is out of scope for the same reason.

Rivetr already supports generic OIDC, which works with almost every IdP that
also offers SAML (Okta, Entra ID, Keycloak, Auth0, Google Workspace,
Authentik, Zitadel and others).
Teams on a SAML-only IdP can put an OIDC bridge in front of it (Keycloak or
Authentik brokering the SAML IdP).

## Revisit when

- `saml-rs` / `bergshamra` reach 1.0 or get an external security audit, or a
  maintained pure-Rust SAML SP with a track record appears; and
- it builds with cargo-zigbuild for `x86_64-unknown-linux-gnu` and
  `aarch64-unknown-linux-gnu`.

Then: SP-initiated login, metadata URL or XML upload, attribute mapping
(email, name, groups), auto-provision gated by the registration setting,
mounted with the SSO/OIDC settings. Require signed assertions, RSA-SHA2 only,
a replay cache, and our own XSW regression tests.

## Sources

- samael: https://github.com/njaremko/samael (`src/crypto/crypto_disabled.rs`, `Cargo.toml`)
- saml-rs: https://github.com/salasebas/saml-rs
- bergshamra: https://github.com/kushaldas/bergshamra
