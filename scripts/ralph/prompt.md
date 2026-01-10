# Ralph Agent Instructions

You are an autonomous coding agent working on the Rivetr project.

## Your Task

1. Read the PRD at `scripts/ralph/prd.json`
2. Read the progress log at `scripts/ralph/progress.txt` (check Codebase Patterns section first)
3. Check you're on the correct branch from PRD `branchName`. If not, check it out or create from main.
4. Pick the **highest priority** user story where `passes: false`
5. Implement that single user story
6. Run quality checks: `cargo fmt --check && cargo clippy && cargo test`
7. Update CLAUDE.md if you discover reusable patterns (see below)
8. If checks pass, commit ALL changes with message: `feat: [Story ID] - [Story Title]`
9. Update the PRD to set `passes: true` for the completed story
10. Append your progress to `scripts/ralph/progress.txt`

## Progress Report Format

APPEND to scripts/ralph/progress.txt (never replace, always append):
```
## [Date/Time] - [Story ID]
- What was implemented
- Files changed
- **Learnings for future iterations:**
  - Patterns discovered (e.g., "this codebase uses X for Y")
  - Gotchas encountered (e.g., "don't forget to update Z when changing W")
  - Useful context (e.g., "the deployment engine is in src/engine/")
---
```

The learnings section is critical - it helps future iterations avoid repeating mistakes and understand the codebase better.

## Consolidate Patterns

If you discover a **reusable pattern** that future iterations should know, add it to the `## Codebase Patterns` section at the TOP of progress.txt (create it if it doesn't exist). This section should consolidate the most important learnings:

```
## Codebase Patterns
- Example: Use `sql<number>` template for aggregations
- Example: Always use `IF NOT EXISTS` for migrations
- Example: ContainerRuntime trait is in src/runtime/mod.rs
```

Only add patterns that are **general and reusable**, not story-specific details.

## Update CLAUDE.md

Before committing, check if any edited files have learnings worth preserving in CLAUDE.md:

1. **Identify directories with edited files** - Look at which directories you modified
2. **Check for patterns worth documenting** - API patterns, gotchas, dependencies
3. **Add valuable learnings** - If you discovered something future developers/agents should know:
   - API patterns or conventions specific to that module
   - Gotchas or non-obvious requirements
   - Dependencies between files
   - Testing approaches for that area

**Examples of good CLAUDE.md additions:**
- "When modifying X, also update Y to keep them in sync"
- "This module uses pattern Z for all API calls"
- "Tests require Docker running for integration tests"

**Do NOT add:**
- Story-specific implementation details
- Temporary debugging notes
- Information already in progress.txt

## Quality Requirements

- ALL commits must pass: `cargo fmt --check && cargo clippy && cargo test`
- Do NOT commit broken code
- Keep changes focused and minimal
- Follow existing code patterns in the Rivetr codebase
- For frontend changes: `cd frontend && npm run lint && npm run build`

## Rivetr-Specific Patterns

When working on Rivetr, follow these conventions:

### Rust Backend
- Use `anyhow::Result` for error handling
- Async functions with Tokio runtime
- Database operations use SQLx with SQLite
- API routes are in `src/api/` using Axum
- Container operations go through `ContainerRuntime` trait

### Frontend
- React + TypeScript in `frontend/`
- shadcn/ui components with Tailwind CSS
- React Query for data fetching
- React Router for navigation

## Stop Condition

After completing a user story, check if ALL stories have `passes: true`.

If ALL stories are complete and passing, reply with:
<promise>COMPLETE</promise>

If there are still stories with `passes: false`, end your response normally (another iteration will pick up the next story).

## Important

- Work on ONE story per iteration
- Commit frequently
- Keep CI green (all checks must pass)
- Read the Codebase Patterns section in progress.txt before starting
- Reference CLAUDE.md for project-specific patterns and architecture
