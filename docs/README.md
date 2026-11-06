# Rivetr Documentation

Reference documentation for developers and operators.

## Documentation Map

| Area | File | Description |
|---|---|---|
| Architecture | [architecture/architecture.md](architecture/architecture.md) | Components, data flow, source layout and tech stack |
| Reference | [reference/configuration.md](reference/configuration.md) | Every `rivetr.toml` section and key |
| Reference | [reference/api.md](reference/api.md) | REST API reference |
| Reference | [reference/service-templates.md](reference/service-templates.md) | Service template registry, grouped by category |
| Reference | [reference/service-deployments.md](reference/service-deployments.md) | Deploying, building, hooks, rollback and env for Docker Compose services |
| Reference | [reference/backups.md](reference/backups.md) | Database, volume and instance backups: schedules, S3, restore, notifications |
| Reference | [reference/secret-managers.md](reference/secret-managers.md) | Vault/OpenBao, Infisical, Doppler and AWS Secrets Manager at deploy time |
| Reference | [reference/remote-servers.md](reference/remote-servers.md) | Remote servers: adding a server, non-root SSH users with passwordless sudo, Tailscale |
| Planning | [planning/coolify-parity.md](planning/coolify-parity.md) | Coolify v4.6 parity batches |
| Planning | [planning/ui-information-architecture.md](planning/ui-information-architecture.md) | Dashboard URL structure and naming |
| Planning | [planning/ai-opportunities.md](planning/ai-opportunities.md) | AI feature opportunities and implementation notes |
| Audit | [audit/improvement-plan-2026-10.html](audit/improvement-plan-2026-10.html) | Prioritized open findings (security, performance, UX, features) |
| Audit | [audit/security-audit.md](audit/security-audit.md), [performance-audit.md](audit/performance-audit.md), [ui-ux-audit.md](audit/ui-ux-audit.md) | Finding details |
| Research | [research/competitive-gap-analysis.md](research/competitive-gap-analysis.md) | Competitive analysis against Coolify and Dokploy |
| Research | [research/saml-decision.md](research/saml-decision.md) | Why SAML 2.0 is deferred |
| Branding | [branding/README.md](branding/README.md) | Logo and brand assets |

## Top-Level Docs

| File | Description |
|---|---|
| [README.md](../README.md) | Project overview, full feature list, quick start |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | Development setup, code organization, PR checklist |
| [DEPLOYMENT.md](../DEPLOYMENT.md) | Production install, upgrades, backup, troubleshooting |
| [CHANGELOG.md](../CHANGELOG.md) | Version history and release notes |
| [ROADMAP.md](../ROADMAP.md) | Open work and where it is tracked |
| [rivetr.example.toml](../rivetr.example.toml) | Copyable configuration starter |

## Testing

| File | Description |
|---|---|
| [live-testing/TESTING-GUIDE.md](../live-testing/TESTING-GUIDE.md) | Manual end-to-end testing procedures |
| [live-testing/TEST-STATUS.md](../live-testing/TEST-STATUS.md) | Which areas were tested, when, and open re-tests |
