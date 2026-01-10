---
name: ralph
description: "Convert PRDs to prd.json format for the Ralph autonomous agent system. Use when you have an existing PRD and need to convert it to Ralph's JSON format. Triggers on: convert this prd, turn this into ralph format, create prd.json from this, ralph json."
---

# Ralph PRD Converter

Converts existing PRDs to the prd.json format that Ralph uses for autonomous execution.

---

## The Job

Take a PRD (markdown file or text) and convert it to `scripts/ralph/prd.json`.

---

## Output Format

```json
{
  "project": "Rivetr",
  "branchName": "ralph/[feature-name-kebab-case]",
  "description": "[Feature description from PRD title/intro]",
  "userStories": [
    {
      "id": "US-001",
      "title": "[Story title]",
      "description": "As a [user], I want [feature] so that [benefit]",
      "acceptanceCriteria": [
        "Criterion 1",
        "Criterion 2",
        "cargo fmt --check passes",
        "cargo clippy passes",
        "cargo test passes"
      ],
      "priority": 1,
      "passes": false,
      "notes": ""
    }
  ]
}
```

---

## Story Size: The Number One Rule

**Each story must be completable in ONE Ralph iteration (one context window).**

Ralph spawns a fresh Claude Code instance per iteration with no memory of previous work. If a story is too big, the LLM runs out of context before finishing.

### Right-sized stories:
- Add a database column and migration
- Add an API endpoint
- Add a UI component to an existing page
- Update a handler with new logic

### Too big (split these):
- "Build the entire dashboard" - Split into: schema, queries, UI components, filters
- "Add authentication" - Split into: schema, middleware, login UI, session handling
- "Refactor the deployment engine" - Split into one story per component

**Rule of thumb:** If you cannot describe the change in 2-3 sentences, it is too big.

---

## Story Ordering: Dependencies First

Stories execute in priority order. Earlier stories must not depend on later ones.

**Correct order:**
1. Schema/database changes (migrations)
2. Backend logic / API endpoints
3. UI components that use the backend
4. Dashboard/summary views that aggregate data

**Wrong order:**
1. UI component (depends on API that does not exist yet)
2. API endpoint

---

## Acceptance Criteria: Must Be Verifiable

Each criterion must be something Ralph can CHECK, not something vague.

### Good criteria (verifiable):
- "Add `status` column to apps table with default 'pending'"
- "GET /api/apps returns JSON array"
- "Clicking delete shows confirmation dialog"
- "cargo fmt --check passes"
- "cargo clippy passes"
- "cargo test passes"

### Bad criteria (vague):
- "Works correctly"
- "User can do X easily"
- "Good UX"

### Always include as final criteria:
```
"cargo fmt --check passes",
"cargo clippy passes",
"cargo test passes"
```

For frontend stories, also include:
```
"npm run lint passes",
"npm run build passes"
```

---

## Conversion Rules

1. **Each user story becomes one JSON entry**
2. **IDs**: Sequential (US-001, US-002, etc.)
3. **Priority**: Based on dependency order, then document order
4. **All stories**: `passes: false` and empty `notes`
5. **branchName**: Derive from feature name, kebab-case, prefixed with `ralph/`
6. **Always add**: Quality check criteria to every story

---

## Splitting Large PRDs

If a PRD has big features, split them:

**Original:**
> "Add deployment notifications"

**Split into:**
1. US-001: Add notifications table to database
2. US-002: Create notification service
3. US-003: Add notification API endpoints
4. US-004: Add notification bell to dashboard header
5. US-005: Create notification list component
6. US-006: Add notification preferences

Each is one focused change that can be completed and verified independently.

---

## Archiving Previous Runs

**Before writing a new prd.json, check if there is an existing one from a different feature:**

1. Read the current `scripts/ralph/prd.json` if it exists
2. Check if `branchName` differs from the new feature's branch name
3. If different AND `progress.txt` has content beyond the header:
   - Create archive folder: `scripts/ralph/archive/YYYY-MM-DD-feature-name/`
   - Copy current `prd.json` and `progress.txt` to archive
   - Reset `progress.txt` with fresh header

---

## Checklist Before Saving

Before writing prd.json, verify:

- [ ] **Previous run archived** (if prd.json exists with different branchName)
- [ ] Each story is completable in one iteration (small enough)
- [ ] Stories are ordered by dependency (schema to backend to UI)
- [ ] Every story has quality check criteria (cargo fmt, clippy, test)
- [ ] Acceptance criteria are verifiable (not vague)
- [ ] No story depends on a later story
- [ ] Saved to `scripts/ralph/prd.json`
