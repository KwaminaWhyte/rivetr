# AI Integration Opportunities for Rivetr

Rivetr is a single-binary PaaS written in Rust that deploys applications from Git webhooks with minimal resource usage (~30MB RAM idle). Because all deployment state — logs, metrics, uptime checks, cost snapshots, and resource usage — is stored locally in a single SQLite database, Rivetr is uniquely positioned to offer AI-powered features without shipping user data to third-party analytics services. The AI layer is an optional enhancement: every feature degrades gracefully when no API key is configured, and the platform operates identically without it. This document catalogues twelve concrete AI integration opportunities, organized by implementation complexity and expected user impact.

---

## Phased Rollout Plan

| Phase | Theme | Opportunities | Effort |
|---|---|---|---|
| Phase 1 | Quick Wins | Deployment Error Diagnosis, Deployment Insights & Patterns, Cost Optimization Advisor | Low |
| Phase 2 | High Impact | Uptime Anomaly Detection, Deployment Risk Assessment, Dockerfile Optimizer, Git Patch Suggestions | Medium |
| Phase 3 | Strategic Differentiators | Service Template Recommender, Alert Context & Troubleshooting Guide, Natural Language App Config, Security & Compliance Advisor, Database Query Optimization | High |

---

## Phase 1 — Quick Wins

### 1. Deployment Error Diagnosis ✅ Implemented

**Description**
When a deployment fails, the raw build log is often hundreds of lines long and difficult to parse at a glance. This feature sends the tail of the failure log to an AI model, asks it to identify the root cause, and streams a human-readable diagnosis along with a prioritized list of suggested fixes directly on the deployment detail page.

**Codebase Location**
- Backend: `src/api/ai_features.rs` → `diagnose_deployment()`
- Frontend: `frontend/app/routes/apps/$id/deployment-detail.tsx`

**Data Sources**
- `deployment_logs` table — raw log lines associated with the failed deployment

**UX Description**
A "Diagnose with AI" button appears on any deployment that has a `failed` status. Clicking it streams the diagnosis in real time into an expandable card beneath the log viewer. The card displays a short root-cause summary followed by a numbered list of suggested fixes, each with a copy-to-clipboard snippet where applicable. The button is disabled with a tooltip when no AI provider is configured.

**Complexity**
Low — the data is already fetched for the deployment detail page. The backend only needs to retrieve the relevant log rows, truncate to a token-safe length, and call the AI client.

**Implementation Notes**
- Truncate logs to the last 200–400 lines before sending to avoid exceeding context limits.
- Stream the response via SSE or a WebSocket to avoid a long blocking request.
- Cache the diagnosis result in a new `deployment_diagnosis` column or a small side table so repeated clicks do not re-query the model.
- Expose the endpoint at `POST /api/deployments/:id/diagnose`.

---

### 2. Deployment Insights & Patterns ✅ Implemented

**Description**
Aggregate a project's deployment history and surface meaningful trends: average build duration over time, failure rate by day of week, most common error categories, and which commit authors tend to introduce failures. The AI narrates these findings in a short paragraph rather than leaving the user to interpret raw charts.

**Codebase Location**
- Backend: `src/api/ai_features.rs` → `get_deployment_insights()`
- Frontend: app overview page (alongside the existing deployment list)

**Data Sources**
- `deployments` table — status, duration, triggered_by, created_at
- `deployment_logs` table — error patterns extracted via regex before being sent to the model

**UX Description**
An "Insights" card on the app overview page shows trend arrows (build time up/down vs. last 7 days, failure rate this week vs. last week) and a two-to-three sentence AI-narrated summary below them. The summary refreshes automatically after each new deployment completes, or on a configurable schedule. No raw log content is sent to the model — only aggregated statistics and anonymized error categories.

**Complexity**
Low — all required data is already present. The main work is writing the aggregation queries and composing a structured prompt from the results.

**Implementation Notes**
- Pre-aggregate on the backend before calling the AI; pass only numbers and category labels, not raw text.
- Cache the insight response for at least 10 minutes to avoid repeated API calls during rapid deployments.
- Expose via `GET /api/apps/:id/insights`.

---

### 3. Cost Optimization Advisor ✅ Implemented

**Description**
Compare actual peak resource usage against the configured memory and CPU limits for each app. Identify apps that are significantly over-provisioned and calculate estimated monthly savings from right-sizing. Surface these recommendations as actionable cards on the costs page.

**Codebase Location**
- Backend: `src/api/ai_features.rs` → `get_cost_suggestions()`
- Frontend: costs page (existing, add an "AI Recommendations" section)

**Data Sources**
- `cost_snapshots` table — historical billing estimates per app
- `resource_metrics` table — actual CPU and memory usage samples
- App configuration — configured memory and CPU limits

**UX Description**
Each recommendation card reads like: "Save $15/mo by reducing the memory limit for `web-api` from 2GB to 512MB — peak usage over the last 30 days was 380MB." A "Apply Now" button opens the resource limits dialog pre-populated with the suggested values. Cards are sorted by estimated monthly saving, largest first.

**Complexity**
Low — the arithmetic is deterministic; the AI's role is limited to generating the natural-language summary and validating that headroom is sufficient given observed variance. The cost calculation itself stays in Rust.

**Implementation Notes**
- Only flag apps where peak usage is consistently below 50% of the configured limit for at least 14 days.
- Never recommend limits that are less than 1.5× the observed peak (safety headroom).
- Expose via `GET /api/cost/recommendations`.
- The AI call is optional: the recommendation cards can display without a narrated summary if no provider is configured.

---

## Phase 2 — Medium Effort, High Impact

### 4. Uptime Anomaly Detection

**Description**
Correlate response-time spikes and uptime check failures with recent deployments, configuration changes, and resource metric trends. Instead of showing raw uptime data, surface a timeline of events that explains what changed before an outage began.

**Codebase Location**
- Backend: `src/monitoring/anomaly_detector.rs`
- Frontend: uptime monitoring page — add a correlated event timeline below the status graph

**Data Sources**
- `uptime_checks` table — response time and status per check interval
- `deployments` table — deployment timestamps for correlation
- `resource_metrics` table — CPU/memory at the time of the anomaly

**UX Description**
When a degradation event is detected (response time exceeds a threshold or check status changes to failing), an event card appears on the monitoring page. The card shows: what changed (e.g., "Deployment #47 ran 3 minutes before the spike"), the likely contributing factors (e.g., "Memory usage reached 95% during the build"), and a suggested next step. A "View correlated deployment" link jumps directly to the deployment detail page.

**Complexity**
Medium — the correlation logic (joining uptime check timestamps with deployment timestamps) requires care around timezone handling and the choice of correlation window. The AI summarizes findings that have already been computed deterministically.

**Implementation Notes**
- Run the anomaly detector as a background Tokio task on a 1-minute polling interval.
- Store detected anomalies in a new `uptime_anomalies` table so the UI can display historical events.
- Only call the AI model when an anomaly transitions from `detected` to `confirmed` (persists for more than N minutes) to avoid noisy API calls on transient blips.

---

### 5. Deployment Risk Assessment

**Description**
Before a deployment runs, score the incoming commit as low, medium, or high risk. Use signals such as the commit message keywords (e.g., "refactor", "hotfix", "BREAKING"), the number and types of files changed, and the project's historical failure rate for similar changes.

**Codebase Location**
- Backend: `src/api/deployments/` — integrate into the deployment creation flow
- Frontend: deployment trigger confirmation modal and deployment list (risk badge per row)

**Data Sources**
- Commit message and file change list from the Git provider webhook payload
- `deployments` table — historical failure rates by author and file pattern
- `deployment_logs` table — past error categories

**UX Description**
A colored badge (green / amber / red) appears next to each deployment in the list and at the top of the deployment detail page. Hovering over the badge shows the factors that contributed to the score. On high-risk deployments, a confirmation step is added to manual triggers: "This deployment is rated High Risk — it touches database migration files and the author's last 3 deployments to this app failed. Continue?"

**Complexity**
Medium — the heuristic scoring can be entirely rule-based (no AI required for the badge). The AI adds value by generating the natural-language explanation of the risk factors and by learning failure patterns from historical data over time.

**Implementation Notes**
- Compute the risk score synchronously during webhook ingestion so it is available before the deployment starts.
- Store the score and factors in a `risk_score` JSON column on the `deployments` table.
- The AI call for generating the explanation can be async (fire-and-forget) to avoid delaying the deployment start.

---

### 6. Dockerfile Optimizer ✅ Implemented

**Description**
Analyze an app's Dockerfile and build log durations and suggest concrete improvements: layer ordering for better cache hit rates, removal of unnecessary packages, multi-stage build opportunities, and base image alternatives with smaller attack surfaces.

**Codebase Location**
- Backend: `src/api/ai_features.rs` → `suggest_dockerfile()`
- Frontend: app settings page → Build section

**Data Sources**
- `dockerfile` field on the `apps` table (or the Dockerfile content fetched from the repo)
- Build log durations from `deployment_logs` — to identify which steps are slowest

**UX Description**
An "AI Optimize" button sits next to the Dockerfile editor in app settings. Clicking it opens a side-by-side diff view: the current Dockerfile on the left, the suggested optimized version on the right, with changed lines highlighted. Each changed section has an inline annotation explaining the rationale. A "Apply Suggestion" button writes the optimized Dockerfile back via the existing patch system with a single click.

**Complexity**
Medium — Dockerfile analysis is a well-understood domain for large language models. The main complexity is rendering the diff cleanly in the frontend and wiring the one-click apply through the patch system.

**Implementation Notes**
- Expose the endpoint at `POST /api/apps/:id/suggest-dockerfile`.
- The response should include both the full optimized Dockerfile text and a structured list of changes with per-change rationale for the diff annotations.
- Store the suggestion temporarily (e.g., in memory or a short-lived DB row) so the user can review before applying; discard after 24 hours or on next manual edit.

---

### 7. Git Patch Suggestions

**Description**
When a commit diff indicates a significant stack change — for example, switching from `npm` to `pnpm`, adding a new environment variable, or upgrading a major dependency — automatically detect the change and suggest a corresponding Rivetr configuration patch to keep the app in sync.

**Codebase Location**
- Backend: `src/api/patches.rs` — extend the existing patch system with an AI-driven suggestion step
- Frontend: patch review flow (existing)

**Data Sources**
- Commit diff content from the Git provider webhook payload
- `app_patches` table — existing patches for context
- App configuration — current build command, env vars, watch paths

**UX Description**
After a deployment that included a significant stack change, a notification card appears on the app overview page: "Your commit changed the package manager from npm to pnpm. Rivetr suggests updating the build command to `pnpm install && pnpm build`." The user can accept, edit, or dismiss the suggestion. Accepted patches are applied immediately; dismissed patches are recorded so the same suggestion is not repeated.

**Complexity**
Medium — the patch suggestion depends on recognizing patterns in commit diffs, which is something large language models do well. The integration point (the existing `app_patches` table and patch application flow) is already built.

**Implementation Notes**
- Only analyze diffs for files that are relevant to the Rivetr configuration: `Dockerfile`, `package.json`, `Cargo.toml`, `requirements.txt`, `deploy.toml`, and similar.
- Keep the diff snippet sent to the AI small (changed lines only, not the full file) to minimize token usage.
- Record dismissed suggestions in a `dismissed_patch_suggestions` table keyed by a hash of the suggestion content so they are not re-surfaced.

---

## Phase 3 — Strategic Differentiators

### 8. Service Template Recommender

**Description**
After a user creates a new app, analyze the repository structure and recommend complementary services from the template library — databases, caches, queues, and observability tools that match the detected stack.

**Codebase Location**
- Backend: `src/api/service_templates.rs`
- Frontend: new app wizard (post-creation step) and app overview page (first-run empty state)

**Data Sources**
- Dockerfile content and package manifest files (package.json, Cargo.toml, requirements.txt, go.mod) — to detect the stack
- Existing services already deployed by the team — to avoid recommending duplicates
- The service template library in `src/api/service_templates.rs`

**UX Description**
After an app is created, a "Recommended Services" card appears on the overview page with up to three suggestions. Each card shows the service name, a one-line reason ("Your Node.js app imports `pg` — you may want a PostgreSQL database"), and a "Deploy" button that pre-fills the service template wizard. Users can dismiss individual recommendations or the whole card.

**Complexity**
High — detecting the stack from file contents is straightforward, but ranking recommendations in a way that feels genuinely useful (not generic) requires understanding the interplay between the detected stack and the team's existing infrastructure.

**Implementation Notes**
- A rule-based detection pass (regex on package names and Dockerfile instructions) should run first, without any AI call, for the common cases. The AI is invoked only for ambiguous or multi-stack repositories.
- Recommendations should reference actual template IDs from the template library so the "Deploy" button opens a pre-filled form, not a blank wizard.

---

### 9. Alert Context & Troubleshooting Guide

**Description**
When an alert fires (CPU spike, memory pressure, uptime failure, disk saturation), automatically generate a contextual troubleshooting guide that correlates the alert with recent deployments, current resource trends, and known patterns for that type of alert.

**Codebase Location**
- Backend: `src/api/alerts.rs` — hook into the `AlertEvent` emission path
- Frontend: alert detail page and notification payloads (Slack/email)

**Data Sources**
- `AlertEvent` data — type, severity, threshold, current value
- Correlated `resource_metrics` at the time of the alert
- Recent `deployments` (last 6 hours)
- Recent `deployment_logs` — to check for relevant warnings

**UX Description**
Every alert detail page gains an "AI Context" section that loads automatically when the alert is opened. It shows: a one-paragraph summary of what is happening, a timeline of correlated events, and a numbered troubleshooting checklist tailored to the alert type and the app's stack. The same context is appended to Slack and email notifications as a collapsed section.

**Complexity**
High — the value of this feature depends on the quality of the correlation logic. A low-quality AI summary that ignores the correlated events is worse than no summary at all.

**Implementation Notes**
- Generate the context asynchronously after the alert is stored; update the alert record with the AI-generated context once it is ready.
- Include a structured preamble in the prompt that describes the alert type, the app's stack, and the correlated events in a machine-readable format so the model can reason about them accurately.
- Troubleshooting steps should be actionable within Rivetr where possible (e.g., "Scale replicas from 1 to 2 — [click here]" rather than generic advice).

---

### 10. Natural Language App Config

**Description**
Allow users to describe their deployment requirements in plain English and have the AI generate a complete, valid Rivetr app configuration — including build commands, environment variables, health check paths, resource limits, and recommended service templates.

**Codebase Location**
- Backend: `src/api/apps/` — new endpoint for config generation
- Frontend: new app wizard — "AI Config Generator" toggle as an alternative to the manual form

**Data Sources**
- User's free-text description (provided in the UI)
- The repository's package manifests and Dockerfile (fetched after the user provides the repo URL)
- Rivetr's configuration schema (embedded in the prompt as a JSON Schema)

**UX Description**
In the new app wizard, a toggle switches between "Manual" and "AI Assisted" modes. In AI Assisted mode, a single text area prompts: "Describe your app and how you want it deployed." After the user types a description and clicks "Generate", the form fields populate automatically. The user reviews and edits before saving — AI-generated config is always editable, never applied without review.

**Complexity**
High — generating valid structured configuration from free text is prone to hallucination. The implementation must validate the generated config against the schema before displaying it, and must clearly indicate to the user that the output needs review.

**Implementation Notes**
- Always validate the AI output against the Rivetr config schema in Rust before passing it to the frontend.
- If validation fails, retry once with the validation errors appended to the prompt.
- If the second attempt also fails, surface the raw text output with a warning rather than silently discarding it.
- Store the user's original description alongside the generated config for auditing and future model fine-tuning.

---

### 11. Security & Compliance Advisor ✅ Implemented

**Description**
Scan for common security misconfigurations: exposed secrets in environment variable names or build logs, outdated base images with known CVEs, missing HTTPS enforcement, containers running as root, and open ports that are not expected by the app type.

**Codebase Location**
- Backend: `src/api/ai_features.rs` → `scan_app_security()` (per-app), `scan_all_security()` (platform-wide)
- Frontend: new "Security" tab on the app detail page and a platform-wide security overview page

**Data Sources**
- `deployment_logs` — scanned with regex for secret-like patterns (tokens, private keys, connection strings) before any AI call
- Environment variable names — checked against a known-sensitive list (e.g., `PASSWORD`, `SECRET`, `TOKEN` in plaintext, not encrypted)
- Image names — for checking against a known-outdated-image list
- App configuration — port mappings, health check settings, replica count

**UX Description**
A findings card on the app detail page lists security findings grouped by severity (Critical, High, Medium, Low). Each finding has a short description, a severity badge, and a per-finding recommendation with a link to the relevant setting. A "Rescan" button triggers a fresh scan. On the platform-wide security page, findings are aggregated across all apps with a total risk score.

**Complexity**
High — secret detection in logs must be extremely precise to avoid false positives that erode user trust. The AI's role is to contextualize findings and generate recommendations; the detection itself should be rule-based.

**Implementation Notes**
- Regex-based secret detection runs entirely in Rust with no AI call; the AI is used only to generate the recommendation text for each finding.
- Never include the actual secret value in the prompt — pass only the type of finding (e.g., "AWS access key pattern detected in build log on line 47") and the surrounding context.
- Expose findings via `GET /api/apps/:id/security-scan` (per-app) and `GET /api/security/scan` (platform-wide).
- Findings should be stored in a `security_findings` table so they persist between page loads and can be tracked over time.

---

### 12. Database Query Optimization

**Description**
Parse slow query logs collected via the log drain feature, identify the most expensive queries, and suggest indexes or query rewrites that would reduce their execution time.

**Codebase Location**
- Backend: `src/api/databases/:id/` — new query analysis endpoint
- Frontend: database detail page — new "Query Analysis" tab

**Data Sources**
- Slow query logs ingested via `log_drains` — filtered for the specific database service
- Database engine type (PostgreSQL, MySQL, etc.) — to tailor index syntax in suggestions
- Current schema (if accessible via the runtime API)

**UX Description**
The "Query Analysis" tab on a database's detail page shows a ranked list of slow queries (anonymized — parameter values replaced with placeholders). Each entry includes: average execution time, frequency, and an AI-generated suggestion (e.g., "Adding an index on `(user_id, created_at)` would make this query ~10× faster — here is the `CREATE INDEX` statement"). A copy button lets the user paste the statement directly into a database client.

**Complexity**
High — parsing slow query logs varies significantly between database engines, and the AI must understand the query structure to suggest valid indexes. Schema-awareness (knowing which columns exist) is required to avoid suggesting indexes on non-existent columns.

**Implementation Notes**
- Parse slow query logs in Rust first; extract the query template (with parameters replaced by `$1`, `$2`, etc.) before sending to the AI to avoid leaking user data.
- If the schema is not available, instruct the model to caveat its suggestions with "assuming a column named X exists".
- Cache suggestions per unique query template so repeated occurrences of the same slow query do not generate duplicate AI calls.
- Expose via `GET /api/databases/:id/query-analysis`.

---

## Technical Implementation

### AI Client Architecture (`src/ai.rs`)

The AI integration is implemented as a single multi-provider client module at `src/ai.rs`. The module exposes a unified `AiClient` struct that abstracts over all supported providers. Callers use the same `complete()` and `stream()` methods regardless of which provider is configured; the client handles routing, request serialization, and response parsing internally.

Supported providers:

| Provider | API Endpoint | Protocol |
|---|---|---|
| Claude (Anthropic) | `https://api.anthropic.com/v1/messages` | Anthropic Messages API |
| OpenAI | `https://api.openai.com/v1/chat/completions` | OpenAI Chat Completions |
| Gemini (Google) | `https://generativelanguage.googleapis.com/v1beta` | Gemini REST API |
| Moonshot | `https://api.moonshot.cn/v1/chat/completions` | OpenAI-compatible |

Because Moonshot uses the OpenAI-compatible chat completions format, it shares the same request/response serialization path as the OpenAI provider — only the base URL and API key header differ.

### Configuration (Dashboard — no restart required)

AI features are configured from the **Settings → AI Provider** panel in the Rivetr dashboard. The provider, API key, model override, and max-token cap are stored in the `instance_settings` table (keys: `ai_provider`, `ai_api_key`, `ai_model`, `ai_max_tokens`) and applied immediately without a server restart.

The `PUT /api/settings/instance` endpoint accepts the new values and hot-reloads the in-memory `AiClient` stored in `AppState.ai_client` (a `parking_lot::RwLock<Option<Arc<AiClient>>>`). The API response includes `ai_configured: bool` but never returns the raw API key.

For backwards compatibility, the `[ai]` section in `rivetr.toml` is still read on startup as a fallback when no key is set in the database:

```toml
[ai]
provider   = "claude"          # "claude" | "openai" | "gemini" | "moonshot"
api_key    = "sk-..."          # provider API key (fallback; prefer dashboard config)
model      = "claude-opus-4-6" # model ID (provider-specific)
max_tokens = 2048              # maximum tokens per response
```

If neither the database nor `rivetr.toml` supplies an API key, the `AiClient` is initialized in a disabled state and all AI feature endpoints return `503 AI not configured`.

### Graceful Degradation

Every AI-powered feature is designed so that the application functions identically without an API key:

- AI-powered buttons and tabs are hidden (not just disabled) when no provider is configured, to avoid cluttering the UI with unavailable features.
- API endpoints that require an AI call return `503` with a JSON body indicating that AI is not configured, rather than `500`, so the frontend can display an appropriate message.
- Background tasks (anomaly detection, risk scoring) skip their AI summarization step and store only the deterministic computed values.
- No AI feature is on the critical path for deployments, monitoring, or any core platform operation.

### Privacy Principles

Rivetr is designed for self-hosted use cases where users may be deploying sensitive internal applications. The following principles govern all AI feature implementations:

- **Minimize data sent to external APIs.** Aggregate and summarize data in Rust before constructing prompts. Raw log lines, environment variable values, and file contents are never sent verbatim.
- **Never log prompt content.** The AI client logs only metadata (provider, model, token counts, latency) — never the prompt text or the response.
- **Anonymize before sending.** Query templates have parameter values replaced. Log lines have secret-like patterns redacted before inclusion in any prompt.
- **Respect self-hosted deployments.** Users running Rivetr on an air-gapped network or with strict data residency requirements can configure a locally-hosted model endpoint by pointing `api_key` and the provider URL at a compatible local server (e.g., Ollama with an OpenAI-compatible API).
- **User control.** AI can be disabled at any time by clearing the API key in **Settings → AI Provider**. No restart is required. Future versions may add per-feature opt-in toggles.
