# PRD: Resource Alerts & Cost Estimation

## Introduction/Overview

Resource Alerts & Cost Estimation provides proactive monitoring and cost visibility for Rivetr deployments. This feature enables users to set threshold-based alerts for resource usage (CPU, memory, disk) and receive notifications through email or webhooks. Additionally, it provides cost estimation and projection capabilities to help teams understand and plan for infrastructure costs.

The feature addresses the need for:
- Early warning when applications approach resource limits
- Flexible notification delivery to existing team workflows
- Visibility into infrastructure costs across apps, projects, and teams
- Budget planning through monthly cost projections

---

## Goals

- Enable proactive resource monitoring with configurable threshold alerts
- Support flexible alert delivery via email and webhooks (Slack/Discord compatible)
- Provide accurate cost estimation with both default and custom pricing models
- Deliver cost visibility at app, project, and team levels
- Minimize false-positive alerts through sensible defaults and hysteresis
- Integrate seamlessly with existing Rivetr dashboard and API

---

## User Stories

### US-001: Add resource metrics collection infrastructure
**Description:** As a platform operator, I want Rivetr to collect CPU, memory, and disk metrics from containers so that I can monitor resource usage.

**Acceptance Criteria:**
- [ ] Create `resource_metrics` table with columns: id, app_id, timestamp, cpu_percent, memory_bytes, memory_limit_bytes, disk_bytes, disk_limit_bytes
- [ ] Add migration file for resource_metrics table
- [ ] Implement periodic metrics collection (every 60 seconds) from container runtime
- [ ] Store metrics with 24-hour retention by default
- [ ] cargo fmt --check passes
- [ ] cargo clippy passes
- [ ] cargo test passes

### US-002: Add alert configuration models and API
**Description:** As a developer, I want to configure resource alert thresholds for my applications so that I get notified before resources are exhausted.

**Acceptance Criteria:**
- [ ] Create `alert_configs` table with columns: id, app_id (nullable for global), metric_type (cpu/memory/disk), threshold_percent, enabled, created_at, updated_at
- [ ] Create `global_alert_defaults` table for system-wide default thresholds
- [ ] Add API endpoints: GET/POST/PUT/DELETE /api/apps/{id}/alerts
- [ ] Add API endpoint: GET/PUT /api/settings/alert-defaults
- [ ] Per-app configs override global defaults when present
- [ ] cargo fmt --check passes
- [ ] cargo clippy passes
- [ ] cargo test passes

### US-003: Implement alert evaluation engine
**Description:** As a platform operator, I want alerts to be automatically evaluated against thresholds so that notifications are triggered when limits are breached.

**Acceptance Criteria:**
- [ ] Create alert evaluation service that runs on metrics collection
- [ ] Compare current metrics against configured thresholds (per-app or global default)
- [ ] Implement hysteresis (alert only triggers after threshold exceeded for 2 consecutive checks)
- [ ] Create `alert_events` table to track triggered alerts with status (firing/resolved)
- [ ] Prevent duplicate alerts for same condition within 15-minute window
- [ ] cargo fmt --check passes
- [ ] cargo clippy passes
- [ ] cargo test passes

### US-004: Add notification channel configuration
**Description:** As a team admin, I want to configure where alerts are sent so that my team receives notifications in their preferred channels.

**Acceptance Criteria:**
- [ ] Create `notification_channels` table with columns: id, team_id, channel_type (email/webhook), config_json, enabled
- [ ] Add API endpoints: GET/POST/PUT/DELETE /api/teams/{id}/notification-channels
- [ ] Support email channel with recipient list configuration
- [ ] Support webhook channel with URL, headers, and payload template
- [ ] Validate webhook URLs are HTTPS
- [ ] cargo fmt --check passes
- [ ] cargo clippy passes
- [ ] cargo test passes

### US-005: Implement email notification sender
**Description:** As a developer, I want to receive email alerts when my app exceeds resource thresholds so that I can take action.

**Acceptance Criteria:**
- [ ] Create email notification service using existing SMTP configuration
- [ ] Design alert email template with: app name, metric type, current value, threshold, timestamp
- [ ] Include direct link to app dashboard in email
- [ ] Queue emails to prevent blocking alert evaluation
- [ ] Log email delivery status
- [ ] cargo fmt --check passes
- [ ] cargo clippy passes
- [ ] cargo test passes

### US-006: Implement webhook notification sender
**Description:** As a team admin, I want alerts sent to webhooks so that I can integrate with Slack, Discord, or custom systems.

**Acceptance Criteria:**
- [ ] Create webhook notification service with configurable payload templates
- [ ] Support JSON payload with variables: app_name, metric_type, value, threshold, timestamp, severity
- [ ] Include default templates for Slack and Discord webhook formats
- [ ] Implement retry logic (3 attempts with exponential backoff)
- [ ] Log webhook delivery status and response codes
- [ ] cargo fmt --check passes
- [ ] cargo clippy passes
- [ ] cargo test passes

### US-007: Add cost configuration and default rates
**Description:** As a platform operator, I want to configure cost rates so that users can see estimated infrastructure costs.

**Acceptance Criteria:**
- [ ] Create `cost_rates` table with columns: id, resource_type (cpu/memory/disk), rate_per_unit, unit_description, is_default, created_at
- [ ] Add API endpoint: GET/PUT /api/settings/cost-rates
- [ ] Seed default rates (e.g., $0.05/GB RAM/month, $0.02/CPU core/month, $0.10/GB disk/month)
- [ ] Allow admin to customize rates
- [ ] cargo fmt --check passes
- [ ] cargo clippy passes
- [ ] cargo test passes

### US-008: Implement cost calculation service
**Description:** As a developer, I want to see estimated costs for my applications based on resource usage so that I can optimize spending.

**Acceptance Criteria:**
- [ ] Create cost calculation service that computes costs from resource metrics
- [ ] Calculate daily average resource usage from metrics table
- [ ] Apply configured rates (custom if set, otherwise defaults)
- [ ] Store daily cost snapshots in `cost_snapshots` table
- [ ] Support cost calculation for: per-app, per-project, per-team aggregation
- [ ] cargo fmt --check passes
- [ ] cargo clippy passes
- [ ] cargo test passes

### US-009: Add cost estimation API endpoints
**Description:** As a developer, I want API access to cost data so that I can integrate costs into my workflows.

**Acceptance Criteria:**
- [ ] Add GET /api/apps/{id}/costs?period=7d|30d|90d endpoint
- [ ] Add GET /api/projects/{id}/costs?period=7d|30d|90d endpoint
- [ ] Add GET /api/teams/{id}/costs?period=7d|30d|90d endpoint
- [ ] Return breakdown by resource type (cpu, memory, disk)
- [ ] Include projected monthly cost based on current usage
- [ ] cargo fmt --check passes
- [ ] cargo clippy passes
- [ ] cargo test passes

### US-010: Add alerts management UI
**Description:** As a developer, I want to configure alerts in the dashboard so that I can set thresholds without using the API.

**Acceptance Criteria:**
- [ ] Add "Alerts" tab to app settings page
- [ ] Show current alert configurations with enable/disable toggles
- [ ] Form to add/edit threshold for each metric type (CPU, memory, disk)
- [ ] Show alert history with status (firing/resolved) and timestamps
- [ ] npm run lint passes
- [ ] npm run build passes

### US-011: Add notification channels UI
**Description:** As a team admin, I want to manage notification channels in the dashboard so that I can configure where alerts are sent.

**Acceptance Criteria:**
- [ ] Add "Notifications" section to team settings page
- [ ] List configured channels with type, status, and test button
- [ ] Form to add email channel with recipient list
- [ ] Form to add webhook with URL and template selection (Slack/Discord/Custom)
- [ ] "Test" button sends sample notification to verify configuration
- [ ] npm run lint passes
- [ ] npm run build passes

### US-012: Add cost dashboard widget
**Description:** As a developer, I want to see cost information on the dashboard so that I have visibility into spending.

**Acceptance Criteria:**
- [ ] Add cost summary card to main dashboard showing total monthly cost
- [ ] Show cost breakdown by top 5 apps
- [ ] Add cost trend sparkline (last 30 days)
- [ ] Link to detailed cost page
- [ ] npm run lint passes
- [ ] npm run build passes

### US-013: Add detailed cost page
**Description:** As a team admin, I want a detailed cost page so that I can analyze spending across apps, projects, and teams.

**Acceptance Criteria:**
- [ ] Create /costs page with period selector (7d, 30d, 90d)
- [ ] Show costs at team level with drill-down to projects and apps
- [ ] Display cost breakdown chart by resource type
- [ ] Show projected monthly cost with trend indicator
- [ ] Export costs as CSV
- [ ] npm run lint passes
- [ ] npm run build passes

### US-014: Add global alert defaults UI
**Description:** As a platform admin, I want to configure global alert defaults so that all apps have baseline monitoring.

**Acceptance Criteria:**
- [ ] Add "Alert Defaults" section to admin settings
- [ ] Configure default thresholds for CPU, memory, disk (e.g., 80%, 85%, 90%)
- [ ] Toggle to enable/disable global alerts
- [ ] Show count of apps using defaults vs custom configs
- [ ] npm run lint passes
- [ ] npm run build passes

---

## Functional Requirements

- **FR-1:** The system must collect CPU, memory, and disk metrics from all running containers at configurable intervals (default 60 seconds)
- **FR-2:** The system must evaluate alert thresholds on each metrics collection cycle
- **FR-3:** The system must support both global default thresholds and per-app override configurations
- **FR-4:** The system must send notifications via email when alerts trigger or resolve
- **FR-5:** The system must send notifications via webhooks with customizable payload templates
- **FR-6:** The system must prevent duplicate notifications for the same alert condition within a cooldown period
- **FR-7:** The system must calculate costs based on average resource usage and configured rates
- **FR-8:** The system must support both default pricing rates and user-configurable custom rates
- **FR-9:** The system must aggregate costs at app, project, and team levels
- **FR-10:** The system must project monthly costs based on current usage patterns
- **FR-11:** The system must retain metrics data for at least 24 hours for alert evaluation
- **FR-12:** The system must retain cost snapshots for at least 90 days for trend analysis

---

## Non-Goals (Out of Scope)

- **SMS notifications** - Only email and webhooks in initial release
- **Native Slack/Discord apps** - Use incoming webhooks instead of bot integrations
- **Auto-scaling based on alerts** - Alerts are informational only, no automatic remediation
- **Cloud provider API integration** - No fetching real costs from AWS/GCP/Azure
- **Per-container metrics** - Metrics are per-app (may have multiple containers)
- **Custom metric types** - Only CPU, memory, and disk; no custom/application metrics
- **Budget limits/spending caps** - Cost estimation is informational, no enforcement
- **Historical metrics beyond 24 hours** - Longer retention requires separate metrics storage solution

---

## Technical Considerations

### Database Schema

New tables required:
- `resource_metrics` - Time-series metrics storage
- `alert_configs` - Per-app alert threshold configuration
- `global_alert_defaults` - System-wide default thresholds
- `alert_events` - Alert history and status tracking
- `notification_channels` - Email/webhook channel configuration
- `cost_rates` - Pricing configuration
- `cost_snapshots` - Daily cost calculations

### Integration Points

- **Container Runtime:** Extend `ContainerRuntime` trait with `stats()` method for metrics
- **Background Tasks:** Add metrics collection and alert evaluation to existing task scheduler
- **SMTP:** Leverage existing email configuration for alert emails
- **WebSocket:** Consider pushing alert events to dashboard in real-time

### Performance Requirements

- Metrics collection should complete within 5 seconds for 100 containers
- Alert evaluation should not block metrics collection
- Cost calculations can run asynchronously (daily batch acceptable)
- Dashboard cost widget should load within 500ms

### Dependencies

- Existing SMTP configuration for email notifications
- HTTP client (reqwest) for webhook delivery
- Container runtime stats API (Docker stats / Podman stats)

---

## Success Metrics

- **Alert Coverage:** >80% of production apps have alerts configured within 30 days of release
- **Alert Accuracy:** <5% false positive rate on alerts
- **Notification Delivery:** >99% successful delivery rate for email and webhooks
- **Cost Visibility:** >50% of teams view cost dashboard weekly
- **User Satisfaction:** Positive feedback on usefulness of cost projections

---

## Open Questions

1. **Metrics retention:** Should we support configurable retention beyond 24 hours? (Requires more storage)
2. **Alert aggregation:** Should we aggregate alerts across apps (e.g., "5 apps above 80% CPU") or always per-app?
3. **Webhook security:** Should we support webhook signing (HMAC) for verification?
4. **Cost currency:** Should costs support multiple currencies or USD only?
5. **Rate limiting:** Should we rate-limit notifications per channel to prevent spam during widespread issues?
