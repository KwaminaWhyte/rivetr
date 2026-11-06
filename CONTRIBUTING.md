# Contributing to Rivetr

Thank you for your interest in contributing to Rivetr. This document covers prerequisites, development setup, code organization, and the PR process.

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| [Rust](https://rustup.rs/) | 1.95+ | Install via rustup |
| [Node.js](https://nodejs.org/) | 22.22+ | For frontend development (React Router 8 requires it) |
| Docker or Podman | Docker 24+ / Podman 4+ | Required for testing deployments |
| Git | Any recent version | |

Optional but recommended:

```bash
cargo install cargo-watch   # auto-reload backend on file changes
```

## Development Setup

### 1. Clone and configure

```bash
git clone https://github.com/KwaminaWhyte/rivetr.git
cd rivetr
cp rivetr.example.toml rivetr.local.toml
# Edit rivetr.local.toml if needed (defaults work for local dev)
```

### 2. Run the backend

```bash
# With auto-reload (recommended)
cargo watch -x "run -- --config rivetr.local.toml"

# Without auto-reload
cargo run -- --config rivetr.local.toml
```

The API and dashboard are served on `http://localhost:8080`.

### 3. Run the frontend dev server

Open a second terminal:

```bash
cd frontend
npm install
npm run dev
```

The Vite dev server runs on `http://localhost:5173` and proxies all `/api/` requests to the backend on port 8080. During development, use the Vite URL to get hot-module reload; the backend URL works too but does not hot-reload React.

### 4. Quality checks

Before committing, run:

```bash
cargo fmt --check     # formatting
cargo clippy          # linting (no new warnings allowed)
cargo test            # tests
```

For frontend:

```bash
cd frontend
npm run lint
npm run build         # ensure production build compiles
```

## Code Organization

Any file that grows beyond ~1000 lines is split into a **subdirectory module**. `mod.rs` holds only module declarations, re-exports and route registration; logic lives in focused submodules:

```
src/api/apps.rs   →   src/api/apps/
                          ├── mod.rs     (route registration, re-exports)
                          ├── crud.rs    (list, get, create, update, delete)
                          ├── control.rs (start, stop, restart)
                          └── upload.rs  (ZIP upload deploy flow)
```

Rust resolves `mod apps;` transparently to either `apps.rs` or `apps/mod.rs`, so callers never need to change.

See [docs/architecture/architecture.md](docs/architecture/architecture.md#source-layout) for what each backend and frontend directory holds.

### Rules for new files

1. **Backend**: a new handler file expected to pass ~500 lines starts as a subdirectory module.
2. **Frontend**: a route file over ~600 lines extracts its tab or section panels into components.
3. **No deep relative imports**: submodules import from `crate::`, never `super::super::`.
4. **Re-export the public surface**: `mod.rs` re-exports every type and function used outside the module.

To split an existing Rust file: create the directory, move the code into submodules, add `mod.rs`, delete the old file, then run `cargo check` and `cargo test`.

## Adding New Features

### Backend

1. **Database changes first**: Add a migration in `migrations/`. Follow the existing numbered format.
2. **Add or update models**: Add or edit the relevant file under `src/db/models/` with new structs and SQLx queries, and re-export it from `src/db/models/mod.rs`.
3. **Add route handlers**: Create or extend the relevant module under `src/api/`. Register the route in the module's `mod.rs` and in `src/api/mod.rs`.
4. **Wire up state**: If the feature needs shared state, add it to `AppState` in `src/lib.rs`.
5. **Follow the `ContainerRuntime` trait**: If adding container operations, implement them for both `DockerRuntime` and `PodmanRuntime`.

### Frontend

1. **Add types**: Add TypeScript interfaces to the appropriate file under `frontend/app/types/` and re-export from `frontend/app/types/index.ts`.
2. **Add API calls**: Add fetch calls or React Query hooks in `frontend/app/lib/`.
3. **Add components**: Use shadcn/ui primitives where possible. Keep components small and focused.
4. **Add routes**: New pages go in `frontend/app/routes/`. React Router v8 uses file-based routing.

### Adding a service template

Service templates (the 74 one-click services in the gallery) are defined in `src/db/seeders/`. Each file covers a category:

```
src/db/seeders/
├── mod.rs           # Entry point, calls all sub-seeders
├── ai_ml.rs         # Ollama, Open WebUI, LiteLLM, etc.
├── analytics.rs     # Plausible, Umami, PostHog, etc.
├── cms.rs           # WordPress, Ghost, Strapi, etc.
└── ...
```

To add a new template:
1. Find the appropriate category file (or add a new one if the category doesn't exist).
2. Add a new `ServiceTemplate` entry following the pattern of existing entries in that file.
3. If you create a new category file, import and call it from `src/db/seeders/mod.rs`.
4. Run `cargo test` to verify compilation.

## Pull Request Checklist

Before submitting a PR, confirm:

- [ ] `cargo fmt --check` passes (no formatting changes needed)
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo test` passes
- [ ] For frontend changes: `npm run lint` and `npm run build` pass
- [ ] No secrets, credentials, or personal data committed
- [ ] New public Rust functions have doc comments
- [ ] If the change is user-facing, `README.md` is updated

## Branch Naming

```
feat/description       # New features
fix/description        # Bug fixes
refactor/description   # Code reorganization
docs/description       # Documentation only
```

## Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
feat: add Telegram notification channel
fix: correct watch path glob matching for nested directories
docs: update CONTRIBUTING with frontend patterns
refactor: split src/api/apps.rs into subdirectory module
```

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`

## Issue Labels

| Label | Meaning |
|---|---|
| `good first issue` | Self-contained, well-scoped: good starting point |
| `help wanted` | Maintainer could use community input |
| `bug` | Something is not working as documented |
| `enhancement` | New feature or improvement to an existing one |
| `documentation` | Docs-only change |
| `backend` | Rust/server-side change |
| `frontend` | React/TypeScript change |
| `security` | Security-related issue (see below for reporting) |

## Security

See [`SECURITY.md`](SECURITY.md) for the full vulnerability disclosure policy.

- Never commit secrets, credentials, or API keys.
- Report security vulnerabilities **privately** via [GitHub Security Advisories](https://github.com/KwaminaWhyte/rivetr/security/advisories/new), not as public issues.
- Use parameterized queries for all database operations.
- Validate all user input through the validation layer in `src/api/validation/`.
- Follow OWASP guidelines for new authentication or session-related code.

## Getting Help

- Search [existing issues](https://github.com/KwaminaWhyte/rivetr/issues) before opening a new one.
- For architecture and tech stack questions, see [docs/architecture/architecture.md](docs/architecture/architecture.md).

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
