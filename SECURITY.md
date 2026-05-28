# Security Policy

## Supported Versions

Rivetr is pre-1.0 and ships from a single active release line. Security fixes are
applied to the latest released version. Please upgrade to the most recent release
before reporting an issue.

## Reporting a Vulnerability

**Do not report security vulnerabilities through public GitHub issues, discussions,
or pull requests.**

Report vulnerabilities privately via
[GitHub Security Advisories](https://github.com/KwaminaWhyte/rivetr/security/advisories/new).

Please include:

- A description of the vulnerability and its impact.
- Steps to reproduce, or a proof-of-concept.
- Affected version(s) and configuration (Docker/Podman, OS).
- Any suggested remediation, if known.

You can expect an initial acknowledgement within **5 business days**. We will keep you
informed of progress toward a fix and coordinate a disclosure timeline with you.

## Scope

In scope:

- The Rivetr backend (API, deployment engine, proxy, auth, webhooks).
- The embedded dashboard frontend.
- The default configuration shipped in `rivetr.example.toml`.

Out of scope:

- Vulnerabilities in third-party container images deployed *through* Rivetr.
- Issues requiring a compromised host or root access the attacker already controls.
- Misconfigurations that contradict the documented secure defaults.

## Security Practices for Contributors

When contributing code, follow these rules (also summarized in `CONTRIBUTING.md`):

- Never commit secrets, credentials, or API keys.
- Use parameterized queries for all database operations, no string-interpolated SQL.
- Validate all user input through the validation layer in `src/api/validation/`.
- Encrypt sensitive data at rest using the utilities in `src/crypto/` (AES-256-GCM).
- Follow OWASP guidelines for new authentication or session-related code.
