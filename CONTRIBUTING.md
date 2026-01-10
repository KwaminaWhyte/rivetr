# Contributing to Rivetr

Thank you for your interest in contributing to Rivetr! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- **Rust 1.75+** - [Install Rust](https://rustup.rs/)
- **Git** - For version control
- **Docker or Podman** - Container runtime for testing deployments
- **Node.js 18+** - For frontend development

### Development Setup

```bash
# Clone the repository
git clone https://github.com/Kwaminawhyte/rivetr.git
cd rivetr

# Build the project
cargo build

# Run in development mode
cargo run -- --config rivetr.example.toml

# Run tests
cargo test

# Check code formatting and linting
cargo fmt --check
cargo clippy
```

### Frontend Development

```bash
cd frontend
npm install
npm run dev
```

## Development Workflow

### Branch Naming

Use descriptive branch names:
- `feat/feature-name` - New features
- `fix/bug-description` - Bug fixes
- `refactor/what-changed` - Code refactoring
- `docs/what-documented` - Documentation updates

### Commit Messages

Follow conventional commit format:
```
type: short description

Optional longer description explaining the change.

Co-Authored-By: Your Name <your.email@example.com>
```

Types:
- `feat` - New feature
- `fix` - Bug fix
- `refactor` - Code refactoring
- `docs` - Documentation changes
- `test` - Adding or updating tests
- `chore` - Maintenance tasks

### Pull Request Process

1. Create a feature branch from `main`
2. Make your changes with clear, focused commits
3. Ensure all tests pass: `cargo test`
4. Ensure code is formatted: `cargo fmt`
5. Ensure no clippy warnings: `cargo clippy`
6. Update documentation if needed
7. Submit a pull request with a clear description

## Code Style

### Rust

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Address all `cargo clippy` warnings
- Write doc comments for public APIs
- Prefer explicit error handling over `.unwrap()`
- Use async/await patterns consistent with the codebase

### TypeScript/React (Frontend)

- Use TypeScript for all new code
- Follow existing component patterns in `frontend/src/`
- Use shadcn/ui components when possible
- Keep components small and focused

## Architecture Overview

Understanding the codebase structure helps you contribute effectively:

```
src/
├── api/          # REST API routes (Axum)
├── config/       # Configuration handling
├── db/           # SQLite database + models
├── engine/       # Deployment pipeline
├── proxy/        # Reverse proxy
├── runtime/      # Container abstraction (Docker/Podman)
└── startup/      # Initialization
frontend/         # React + Vite dashboard
```

Key patterns:
- **Container Runtime Trait** - See `src/runtime/mod.rs` for the abstraction
- **Deployment Pipeline** - Jobs flow through `src/engine/`
- **State Management** - `AppState` in `src/lib.rs` holds shared state

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Frontend tests
cd frontend && npm test
```

### Writing Tests

- Add unit tests near the code they test
- Integration tests go in `tests/`
- Test both success and error cases
- Mock external dependencies (Docker/Podman) where appropriate

## Using Claude Code

This project includes Claude Code configuration for AI-assisted development.

### Sub-Agents

Available in `.claude/agents/`:
- `code-reviewer` - Rust code review
- `frontend-reviewer` - React/TypeScript review
- `debugger` - Error investigation
- `test-runner` - Test automation
- `security-reviewer` - Security audits

### Skills

Available in `.claude/skills/`:
- `rust-review` - Rust patterns and idioms
- `api-testing` - REST API testing commands
- `docker-testing` - Container testing
- `database-operations` - SQLite/SQLx patterns
- `deployment-pipeline` - Pipeline debugging

### Ralph (Autonomous Agent Loop)

For larger features, use the Ralph pattern in `scripts/ralph/`:
1. Create a PRD with the `prd` skill
2. Convert to `prd.json` with the `ralph` skill
3. Run `./scripts/ralph/ralph.sh` to autonomously implement

See `scripts/ralph/README.md` for details.

## Documentation

- Update `CLAUDE.md` when adding new patterns or architectural changes
- Update `README.md` for user-facing changes
- Update `ROADMAP.md` and `plan/TASKS.md` for feature planning
- Add inline code comments for non-obvious logic

## Security

- Never commit secrets or credentials
- Report security vulnerabilities privately
- Follow OWASP guidelines for web security
- Validate all user input
- Use parameterized queries for database operations

## Getting Help

- Open an issue for bugs or feature requests
- Check existing issues before creating new ones
- Read `plan/ARCHITECTURE.md` for design context
- See `plan/TASKS.md` for current development priorities

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
