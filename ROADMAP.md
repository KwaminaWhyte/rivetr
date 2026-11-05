# Rivetr Roadmap

> A fast, lightweight deployment engine built in Rust

Rivetr 1.0.0 shipped on 2026-11-05. Everything that has shipped is listed in [CHANGELOG.md](./CHANGELOG.md), and the full feature list is in the [README](./README.md#features). This file only tracks what is still open.

## Where open work is tracked

| Topic | File |
|---|---|
| Coolify v4.6 parity batches | [docs/planning/coolify-parity.md](./docs/planning/coolify-parity.md) |
| Dashboard URL structure and naming | [docs/planning/ui-information-architecture.md](./docs/planning/ui-information-architecture.md) |
| Open security, performance and UX findings, prioritized | [docs/audit/improvement-plan-2026-10.html](./docs/audit/improvement-plan-2026-10.html) (details in `docs/audit/*-audit.md`) |
| Feature gaps against Coolify and Dokploy | [docs/research/competitive-gap-analysis.md](./docs/research/competitive-gap-analysis.md) |
| AI features not built yet | [docs/planning/ai-opportunities.md](./docs/planning/ai-opportunities.md) |

## Open items

- **SAML 2.0**: deferred until a pure-Rust SP stack matures. OIDC covers SSO today. See [docs/research/saml-decision.md](./docs/research/saml-decision.md).
- **Docker Swarm overlay networking**: Swarm init, nodes, scaling and the per-app service spec (placement, update and rollback config) are done; overlay networks are not.
- **HTTP/3** in the embedded proxy.
- **Plugin system**: extension points for custom builders and runtimes.
- **Kubernetes** as an alternative orchestrator.
- **Terraform provider** for infrastructure as code.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md). Service templates and documentation are good first contributions.
