# Ralph Guide - Autonomous AI Development for Rivetr

Ralph is an autonomous AI agent loop that runs Claude Code repeatedly until all tasks in a Product Requirements Document (PRD) are complete. This guide explains how to use Ralph for developing features in Rivetr.

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Quick Start](#quick-start)
4. [Step-by-Step Workflow](#step-by-step-workflow)
5. [Writing Good PRDs](#writing-good-prds)
6. [Running Ralph](#running-ralph)
7. [Monitoring Progress](#monitoring-progress)
8. [Troubleshooting](#troubleshooting)
9. [Best Practices](#best-practices)

---

## Overview

Ralph automates feature development by:
1. Reading a task list (`prd.json`)
2. Picking the next incomplete task
3. Implementing it using Claude Code
4. Running quality checks
5. Committing changes
6. Repeating until all tasks are done

Each iteration is a **fresh Claude Code instance** with clean context. Memory persists through:
- Git commit history
- `progress.txt` (learnings log)
- `prd.json` (task status)

---

## Prerequisites

### Required Tools

1. **Claude Code CLI** - Must be installed and authenticated
   ```bash
   # Verify installation
   claude --version
   ```

2. **jq** (for parsing JSON in bash script)
   - macOS: `brew install jq`
   - Windows: `choco install jq` or `winget install jqlang.jq`
   - Linux: `apt install jq` or `yum install jq`

3. **Git** - Repository must be initialized

4. **Rust toolchain** - For running quality checks
   ```bash
   cargo --version
   rustc --version
   ```

---

## Quick Start

```bash
# 1. Create a PRD for your feature
# In Claude Code, run:
Load the prd skill and create a PRD for adding user notifications

# 2. Answer the clarifying questions
# PRD will be saved to tasks/prd-user-notifications.md

# 3. Convert PRD to Ralph format
Load the ralph skill and convert tasks/prd-user-notifications.md to prd.json

# 4. Run Ralph (Windows)
.\scripts\ralph\ralph.ps1

# 4. Run Ralph (Linux/macOS)
./scripts/ralph/ralph.sh
```

---

## Step-by-Step Workflow

### Step 1: Plan Your Feature

Before creating a PRD, think about:
- What problem are you solving?
- What are the key components needed?
- What's the order of dependencies?

### Step 2: Create a PRD

Start Claude Code and use the PRD skill:

```
Load the prd skill and create a PRD for [your feature description]
```

**Example:**
```
Load the prd skill and create a PRD for adding deployment rollback functionality
```

The skill will:
1. Ask 3-5 clarifying questions with multiple choice options
2. Generate a structured PRD based on your answers
3. Save it to `tasks/prd-[feature-name].md`

**Answering Questions:**

Questions will look like:
```
1. What should trigger a rollback?
   A. Manual user action only
   B. Automatic on health check failure
   C. Both manual and automatic
   D. Other: [please specify]

2. What data should be preserved during rollback?
   A. Keep all environment variables
   B. Reset to previous deployment's config
   C. Let user choose
```

You can answer quickly: `1C, 2B`

### Step 3: Review the PRD

Open `tasks/prd-[feature-name].md` and verify:
- [ ] User stories are small enough (one task per iteration)
- [ ] Dependencies are in correct order
- [ ] Acceptance criteria are specific and verifiable
- [ ] Non-goals are clearly defined

### Step 4: Convert to Ralph Format

Use the ralph skill:

```
Load the ralph skill and convert tasks/prd-[feature-name].md to prd.json
```

This creates `scripts/ralph/prd.json` with:
- User stories as JSON objects
- Priority ordering
- Pass/fail status tracking

### Step 5: Run Ralph

**Windows (PowerShell):**
```powershell
cd C:\Users\pc\Desktop\rivetr
.\scripts\ralph\ralph.ps1

# With custom iteration limit
.\scripts\ralph\ralph.ps1 -MaxIterations 20
```

**Linux/macOS:**
```bash
cd /path/to/rivetr
./scripts/ralph/ralph.sh

# With custom iteration limit
./scripts/ralph/ralph.sh 20
```

### Step 6: Monitor and Review

Watch the terminal output. After completion:
1. Review the git log for commits
2. Check `scripts/ralph/progress.txt` for learnings
3. Test the implemented feature
4. Create a PR for review

---

## Writing Good PRDs

### Story Size

**The #1 rule: Each story must fit in one context window.**

#### Good (Small) Stories:
```markdown
### US-001: Add rollback column to deployments table
- Add `rollback_target_id` column (nullable UUID)
- Create migration file
- Update Deployment model
```

```markdown
### US-002: Add rollback API endpoint
- POST /api/deployments/{id}/rollback
- Returns new deployment ID
- Validates target deployment exists
```

#### Bad (Too Large) Stories:
```markdown
### US-001: Implement complete rollback system
- Database changes
- API endpoints
- Frontend UI
- Email notifications
```

Split large stories into 3-5 smaller ones.

### Dependency Order

Stories execute in priority order. Put foundations first:

```
Priority 1: Database schema changes
Priority 2: Backend models/services
Priority 3: API endpoints
Priority 4: Frontend components
Priority 5: Integration/polish
```

### Acceptance Criteria

Must be **verifiable**, not vague.

#### Good Criteria:
- "Migration creates `rollback_target_id` column as nullable UUID"
- "POST /api/deployments/123/rollback returns 201 with deployment ID"
- "Rollback button appears only for successful deployments"
- "cargo test passes"

#### Bad Criteria:
- "Rollback works correctly"
- "Good user experience"
- "Handles edge cases"

### Always Include Quality Checks

Every story should end with:
```markdown
- [ ] cargo fmt --check passes
- [ ] cargo clippy passes
- [ ] cargo test passes
```

For frontend stories, add:
```markdown
- [ ] npm run lint passes
- [ ] npm run build passes
```

---

## Running Ralph

### Command Options

**Windows:**
```powershell
# Default (10 iterations)
.\scripts\ralph\ralph.ps1

# Custom iteration limit
.\scripts\ralph\ralph.ps1 -MaxIterations 25
```

**Linux/macOS:**
```bash
# Default (10 iterations)
./scripts/ralph/ralph.sh

# Custom iteration limit
./scripts/ralph/ralph.sh 25
```

### What Happens During Execution

Each iteration:
1. Reads `prd.json` to find next incomplete story
2. Checks out the correct branch
3. Implements the story
4. Runs `cargo fmt --check && cargo clippy && cargo test`
5. Commits if checks pass
6. Updates `prd.json` to mark story complete
7. Appends learnings to `progress.txt`

### Completion

When all stories pass, Ralph outputs:
```
<promise>COMPLETE</promise>
```

If max iterations reached without completion:
```
Ralph reached max iterations (10) without completing all tasks.
Check scripts/ralph/progress.txt for status.
```

---

## Monitoring Progress

### Check Story Status

```bash
# See which stories are done
cat scripts/ralph/prd.json | jq '.userStories[] | {id, title, passes}'
```

Output:
```json
{"id": "US-001", "title": "Add rollback column", "passes": true}
{"id": "US-002", "title": "Add rollback API", "passes": true}
{"id": "US-003", "title": "Add rollback button", "passes": false}
```

### Check Learnings

```bash
cat scripts/ralph/progress.txt
```

### Check Git History

```bash
git log --oneline -10
```

---

## Troubleshooting

### "No stories with passes: false found"

All stories are complete. Check if there's a new PRD to process.

### Story Keeps Failing

1. Check `progress.txt` for error details
2. The story might be too large - split it
3. Acceptance criteria might be unclear - make them specific
4. Dependencies might be wrong - check priority order

### Quality Checks Failing

```bash
# Run checks manually to see errors
cargo fmt --check
cargo clippy
cargo test
```

Fix issues in the codebase, then re-run Ralph.

### Context Running Out

Story is too large. Split into smaller stories:

**Before:**
```json
{
  "id": "US-001",
  "title": "Add complete notification system"
}
```

**After:**
```json
{
  "id": "US-001",
  "title": "Add notifications table"
},
{
  "id": "US-002",
  "title": "Add notification service"
},
{
  "id": "US-003",
  "title": "Add notification API endpoints"
}
```

### Branch Conflicts

Ralph creates branches like `ralph/feature-name`. If conflicts occur:

```bash
git checkout main
git pull
git branch -D ralph/feature-name
# Re-run Ralph
```

---

## Best Practices

### 1. Start Small

For your first Ralph run, try a simple feature:
- Add a single API endpoint
- Add a database column
- Create a UI component

### 2. Review PRDs Before Running

Always verify:
- Stories are small enough
- Dependencies are ordered correctly
- Criteria are verifiable

### 3. Monitor First Few Iterations

Watch Ralph work on the first 2-3 stories to catch issues early.

### 4. Keep CI Green

Ralph depends on quality checks. If tests are flaky or broken, fix them first.

### 5. Use Progress Learnings

Check `progress.txt` for patterns and gotchas discovered during execution. Add important ones to `CLAUDE.md`.

### 6. Archive Completed Runs

Ralph auto-archives when switching features. Manual archive:
```bash
mkdir -p scripts/ralph/archive/$(date +%Y-%m-%d)-feature-name
cp scripts/ralph/prd.json scripts/ralph/progress.txt scripts/ralph/archive/$(date +%Y-%m-%d)-feature-name/
```

### 7. Review Before Merging

After Ralph completes:
1. Review all commits
2. Run full test suite
3. Manual QA testing
4. Create PR for team review

---

## Example: Adding Deployment Metrics

### 1. Create PRD

```
Load the prd skill and create a PRD for adding deployment duration metrics
```

Answer questions:
```
1B (track build and deploy time separately)
2A (store in database)
3C (show in dashboard and API)
```

### 2. Review Generated PRD

`tasks/prd-deployment-metrics.md`:
```markdown
## User Stories

### US-001: Add metrics columns to deployments table
### US-002: Track build duration in engine
### US-003: Track deploy duration in engine
### US-004: Add metrics to deployment API response
### US-005: Display metrics in dashboard
```

### 3. Convert to JSON

```
Load the ralph skill and convert tasks/prd-deployment-metrics.md to prd.json
```

### 4. Run Ralph

```powershell
.\scripts\ralph\ralph.ps1
```

### 5. Monitor

```bash
# Watch progress
cat scripts/ralph/prd.json | jq '.userStories[] | {id, passes}'

# Check commits
git log --oneline -5
```

### 6. Complete

When finished:
```
Ralph completed all tasks!
Completed at iteration 5 of 10
```

Review and create PR.

---

## File Reference

| File | Location | Purpose |
|------|----------|---------|
| `ralph.sh` | `scripts/ralph/` | Bash runner script |
| `ralph.ps1` | `scripts/ralph/` | PowerShell runner script |
| `prompt.md` | `scripts/ralph/` | Instructions for each iteration |
| `prd.json` | `scripts/ralph/` | Current task list |
| `prd.json.example` | `scripts/ralph/` | Example format |
| `progress.txt` | `scripts/ralph/` | Learnings log |
| `archive/` | `scripts/ralph/` | Previous runs |
| `prd/SKILL.md` | `.claude/skills/` | PRD generation skill |
| `ralph/SKILL.md` | `.claude/skills/` | PRD to JSON converter |

---

## Further Reading

- [Original Ralph Pattern](https://ghuntley.com/ralph/) by Geoffrey Huntley
- [Ralph Repository](https://github.com/snarktank/ralph)
- [Claude Code Documentation](https://claude.ai/code)
- [Rivetr CLAUDE.md](../CLAUDE.md) - Project-specific patterns
