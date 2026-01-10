# Ralph - Autonomous AI Agent Loop for Claude Code

Ralph is an autonomous AI agent loop that runs Claude Code repeatedly until all PRD items are complete. Each iteration is a fresh Claude Code instance with clean context. Memory persists via git history, `progress.txt`, and `prd.json`.

Based on [Geoffrey Huntley's Ralph pattern](https://ghuntley.com/ralph/) and adapted from [snarktank/ralph](https://github.com/snarktank/ralph).

## Prerequisites

- [Claude Code CLI](https://claude.ai/code) installed and authenticated
- `jq` installed (for bash script) - `brew install jq` on macOS, `choco install jq` on Windows
- Git repository initialized

## Quick Start

### 1. Create a PRD

Use the prd skill to generate a detailed requirements document:

```
Load the prd skill and create a PRD for [your feature description]
```

Answer the clarifying questions. The skill saves output to `tasks/prd-[feature-name].md`.

### 2. Convert PRD to Ralph Format

Use the ralph skill to convert the markdown PRD to JSON:

```
Load the ralph skill and convert tasks/prd-[feature-name].md to prd.json
```

This creates `scripts/ralph/prd.json` with user stories structured for autonomous execution.

### 3. Run Ralph

**Linux/macOS:**
```bash
./scripts/ralph/ralph.sh [max_iterations]
```

**Windows (PowerShell):**
```powershell
.\scripts\ralph\ralph.ps1 [max_iterations]
```

Default is 10 iterations.

## How It Works

Ralph will:
1. Create a feature branch (from PRD `branchName`)
2. Pick the highest priority story where `passes: false`
3. Implement that single story
4. Run quality checks (`cargo fmt`, `cargo clippy`, `cargo test`)
5. Commit if checks pass
6. Update `prd.json` to mark story as `passes: true`
7. Append learnings to `progress.txt`
8. Repeat until all stories pass or max iterations reached

## Key Files

| File | Purpose |
|------|---------|
| `ralph.sh` / `ralph.ps1` | The script that spawns fresh Claude Code instances |
| `prompt.md` | Instructions given to each Claude Code instance |
| `prd.json` | User stories with `passes` status (the task list) |
| `prd.json.example` | Example PRD format for reference |
| `progress.txt` | Append-only learnings for future iterations |

## Critical Concepts

### Each Iteration = Fresh Context

Each iteration spawns a **new Claude Code instance** with clean context. The only memory between iterations is:
- Git history (commits from previous iterations)
- `progress.txt` (learnings and context)
- `prd.json` (which stories are done)

### Small Tasks

Each PRD item should be small enough to complete in one context window. If a task is too big, the LLM runs out of context before finishing.

**Right-sized stories:**
- Add a database column and migration
- Add a UI component to an existing page
- Add an API endpoint with validation
- Update a handler with new logic

**Too big (split these):**
- "Build the entire dashboard"
- "Add authentication"
- "Refactor the deployment engine"

### CLAUDE.md Updates

After each iteration, Ralph updates CLAUDE.md with learnings. This helps future iterations (and human developers) understand patterns, gotchas, and conventions.

### Feedback Loops

Ralph only works with feedback loops:
- `cargo fmt --check` catches formatting issues
- `cargo clippy` catches common mistakes
- `cargo test` verifies behavior

### Stop Condition

When all stories have `passes: true`, Ralph outputs `<promise>COMPLETE</promise>` and the loop exits.

## Debugging

Check current state:

```bash
# See which stories are done
cat scripts/ralph/prd.json | jq '.userStories[] | {id, title, passes}'

# See learnings from previous iterations
cat scripts/ralph/progress.txt

# Check git history
git log --oneline -10
```

## Archiving

Ralph automatically archives previous runs when you start a new feature (different `branchName`). Archives are saved to `scripts/ralph/archive/YYYY-MM-DD-feature-name/`.

## References

- [Geoffrey Huntley's Ralph article](https://ghuntley.com/ralph/)
- [Original Ralph repository](https://github.com/snarktank/ralph)
- [Claude Code documentation](https://claude.ai/code)
