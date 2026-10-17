# Configuration Reference (`rivetr.toml`)

Rivetr reads its server configuration from a TOML file (default: `rivetr.toml`, override with `--config <path>`). If the file is missing, Rivetr starts with all defaults. Every table and field is optional, omit anything and the documented default applies.

Source of truth: `src/config/mod.rs`. A copyable starter is in `rivetr.example.toml`.

## Table of Contents

- [`[server]`](#server)
- [`[auth]`](#auth)
- [`[runtime]`](#runtime)
- [`[proxy]`](#proxy)
- [`[logging]`](#logging)
- [`[webhooks]`](#webhooks)
- [`[oauth]`](#oauth)
- [`[rate_limit]`](#rate_limit)
- [`[cleanup]`](#cleanup)
- [`[disk_monitor]`](#disk_monitor)
- [`[container_monitor]`](#container_monitor)
- [`[database_backup]`](#database_backup)
- [`[stats_retention]`](#stats_retention)
- [`[data_retention]`](#data_retention)
- [`[email]`](#email)
- [`[auto_update]`](#auto_update)
- [`[ai]`](#ai)

---

## `[server]`

Network bind addresses and data location.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | string | `"0.0.0.0"` | Address the API and proxy bind to. `0.0.0.0` binds all interfaces. |
| `api_port` | u16 | `8080` | Port for the REST API / dashboard. |
| `proxy_port` | u16 | `80` | Port for the embedded HTTP reverse proxy. |
| `proxy_https_port` | u16 | `443` | Port for the embedded HTTPS reverse proxy. |
| `data_dir` | path | `"./data"` | Directory for the SQLite DB, ACME cache, backups, etc. |
| `external_url` | string? | _none_ | Externally reachable base URL (e.g. an ngrok URL in dev). Used for GitHub App callbacks and webhook URLs. |

## `[auth]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `admin_token` | string | random 256-bit hex | Bearer token for programmatic API access. **Auto-generated per start if unset**: set explicitly in production so it is stable. |
| `encryption_key` | string? | _none_ | Secret used to encrypt env vars, SSH keys, OAuth and TOTP secrets at rest in the DB. Use a strong, random 32+ char string (`openssl rand -hex 32`). If unset, see [Encryption key resolution](#encryption-key-resolution). |

### Encryption key resolution

At startup the key is resolved in this order:

1. `[auth] encryption_key`, if set (always wins).
2. `<data_dir>/.encryption_key`, if the file exists.
3. Fresh install (no users, env vars, servers, build servers or git providers in the DB): a random 256-bit key is generated and written to `<data_dir>/.encryption_key` (mode `0600`, created atomically, never overwritten). A one-time notice is logged. **Back this file up together with the database**: without it, encrypted values cannot be recovered.
4. Existing install with neither: behavior is unchanged (plaintext) and a startup warning shows the exact line to add. Existing plaintext values stay readable after a key is enabled; new and updated values are encrypted.

The key is stretched with PBKDF2-HMAC-SHA256 (100k iterations, fixed salt). Upgrading the KDF (per-install salt, Argon2id) would break existing ciphertext and needs a versioned ciphertext format first; it is a tracked follow-up.

## `[runtime]`

Container runtime selection and host-protection defaults applied to every container.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `runtime_type` | enum | `"auto"` | One of `auto`, `docker`, `podman`. `auto` detects an available runtime. |
| `docker_socket` | string | `/var/run/docker.sock` (unix) / `npipe:////./pipe/docker_engine` (windows) | Path/URL to the Docker daemon socket. |
| `build_cpu_limit` | string | `"2"` | CPU limit applied during builds (e.g. `"2"`, `"0.5"`). |
| `build_memory_limit` | string | `"2g"` | Memory limit applied during builds (e.g. `"2g"`, `"512m"`). |
| `default_memory_limit` | string | `"512m"` | Fallback memory cap for any app/service/database container without its own limit. Container is OOM-killed at this cap. Empty string disables the fallback (unbounded, not recommended). Per-resource limits override. |
| `default_pids_limit` | i64 | `512` | Fallback PID limit per container (fork-bomb protection). `0` disables. |
| `default_oom_score_adj` | i64 | `500` | OOM score adjustment so the kernel kills a runaway container before host daemons. Range `-1000..1000`; higher = killed sooner. |

## `[proxy]`

Embedded reverse proxy: TLS/ACME, health checks, and automatic domain generation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `acme_enabled` | bool | `false` | Enable automatic HTTPS via Let's Encrypt. |
| `acme_email` | string? | _none_ | Email for the Let's Encrypt account (required if `acme_enabled`). |
| `acme_staging` | bool | `false` | Use the Let's Encrypt staging environment (avoids rate limits while testing). |
| `acme_cache_dir` | path | `"./data/acme"` | Directory for ACME account data and certificates. |
| `health_check_interval` | u64 | `30` | Seconds between backend health checks. |
| `health_check_timeout` | u64 | `5` | Seconds before a health check request times out. |
| `health_check_threshold` | u32 | `3` | Consecutive failures before a backend is marked unhealthy. |
| `drain_timeout_secs` | u64 | `30` | After a deploy or rollback swaps routes, wait up to this many seconds for in-flight requests on the old container to finish before stopping it. `0` disables draining. WebSocket tunnels are not waited for. |
| `base_domain` | string? | _none_ | Base domain for auto-generated subdomains (e.g. `rivetr.example.com` → `my-app.rivetr.example.com`). |
| `auto_subdomain_enabled` | bool | `false` | Enable automatic subdomain generation for new apps (requires `base_domain`). |
| `server_ip` | string? | _none_ | Public IP used for `sslip.io` / `traefik.me` style domains. Auto-detected if unset. |
| `sslip_enabled` | bool | `false` | Enable `sslip.io` automatic domains (e.g. `abc123.192.168.1.1.sslip.io`). |
| `preview_domain` | string? | _none_ | Base domain for PR preview deployments (e.g. `pr-123.my-app.preview.example.com`). |
| `instance_domain` | string? | _none_ | Domain for the Rivetr dashboard/API itself; proxy forwards this domain to the API server. |

## `[logging]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | string | `"info"` | Log level: `trace`, `debug`, `info`, `warn`, or `error`. |

## `[webhooks]`

Secrets for verifying inbound Git webhook signatures. All optional but recommended.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `github_secret` | string? | _none_ | Verifies the GitHub `X-Hub-Signature-256` header (HMAC-SHA256). |
| `gitlab_token` | string? | _none_ | Matched against the GitLab `X-Gitlab-Token` header. |
| `gitea_secret` | string? | _none_ | Verifies the Gitea signature header (HMAC-SHA256). |
| `bitbucket_secret` | string? | _none_ | Verifies the Bitbucket webhook signature (HMAC-SHA256). |

## `[oauth]`

Social-login / Git provider OAuth apps. Each provider is an optional subtable: `[oauth.github]`, `[oauth.gitlab]`, `[oauth.bitbucket]`.

Each provider subtable accepts:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `client_id` | string | _required_ | OAuth application client ID. |
| `client_secret` | string | _required_ | OAuth application client secret. |
| `redirect_uri` | string? | _none_ | OAuth callback URL. |

## `[rate_limit]`

Per-tier request throttling.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Master switch for rate limiting. |
| `api_requests_per_window` | u32 | `100` | Limit for general API endpoints per window. |
| `webhook_requests_per_window` | u32 | `500` | Limit for webhook endpoints per window. |
| `auth_requests_per_window` | u32 | `20` | Limit for auth endpoints per window (stricter, anti-brute-force). |
| `window_seconds` | u64 | `60` | Window duration in seconds. |
| `cleanup_interval` | u64 | `300` | Seconds between cleanups of expired rate-limit entries. |
| `trusted_proxies` | string[] | `[]` | IPs or CIDRs of reverse proxies whose `X-Forwarded-For` / `X-Real-IP` headers are trusted. |

**Client IP resolution.** Rate limiting and audit logs key on the TCP peer address. `X-Forwarded-For` / `X-Real-IP` are honored only when the peer is a trusted proxy, and `X-Forwarded-For` is read right to left, skipping trusted hops, so values a client prepends are ignored. Loopback peers (`127.0.0.0/8`, `::1`) are always trusted, which covers the embedded proxy and nginx / Caddy / Traefik on the same host. Add other proxies explicitly, for example the Docker bridge range (`172.17.0.0/16`) when Rivetr runs in a container behind a proxy; without it every client shares the bridge IP's bucket. Do not list ranges that untrusted clients can connect from.

**Account lockout.** Independently of IP limits, 5 failed password attempts for the same email (or 5 failed 2FA codes for the same user) within 15 minutes lock that account with an exponential backoff (30 s doubling, capped at 15 min). `/api/auth/login` and `/api/auth/2fa/validate` return `429` while locked; a successful sign-in clears the counter. State is in memory and resets on restart.

## `[cleanup]`

Automatic pruning of old deployments and images.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable automatic deployment cleanup. |
| `max_deployments_per_app` | u32 | `3` | Deployments kept per app; older ones (and their containers/images) are removed. |
| `cleanup_interval_seconds` | u64 | `3600` | Seconds between cleanup runs. |
| `prune_images` | bool | `true` | Prune dangling Docker/Podman images after cleanup. |

> Note: `rivetr.example.toml` shows `max_deployments_per_app = 10` as a sample value; the built-in default is `3`.

## `[disk_monitor]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Monitor the `data_dir` filesystem; expose metrics via Prometheus. |
| `check_interval_seconds` | u64 | `300` | Seconds between disk checks. |
| `warning_threshold` | u8 | `80` | Disk usage % that triggers a warning log. |
| `critical_threshold` | u8 | `90` | Disk usage % that triggers a critical log. |

## `[container_monitor]`

Crash detection and auto-restart with exponential backoff.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable crash monitoring and auto-restart. |
| `check_interval_secs` | u64 | `30` | Seconds between container health checks. |
| `max_restart_attempts` | u32 | `5` | Restart attempts before the deployment is marked failed. |
| `initial_backoff_secs` | u64 | `5` | Initial backoff after a crash; doubles each retry. |
| `max_backoff_secs` | u64 | `300` | Maximum backoff delay. |
| `stable_duration_secs` | u64 | `120` | Seconds a container must run before being considered stable (resets the restart counter). |

## `[database_backup]`

Scheduling for managed-database backups.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable automatic backup scheduling. |
| `check_interval_seconds` | u64 | `60` | Seconds between schedule checks. |
| `backup_dir` | string | `"backups"` | Backup directory, relative to `data_dir`. |
| `timeout_seconds` | u64 | `3600` | Timeout for an individual backup command. |

## `[stats_retention]`

Retention and aggregation of resource-usage stats.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable stats retention/aggregation cleanup. |
| `raw_retention_days` | i64 | `7` | Days to keep raw stats (recorded ~every 5 min). |
| `hourly_retention_days` | i64 | `30` | Days to keep hourly-aggregated stats. |
| `daily_retention_days` | i64 | `365` | Days to keep daily-aggregated stats. |
| `cleanup_interval_seconds` | u64 | `3600` | Seconds between cleanup/aggregation runs. |

## `[data_retention]`

Daily pruning of high-volume operational tables. Rows are deleted in bounded batches (10,000 per statement) so a run never holds the SQLite write lock for long. A value of `0` disables pruning for that item. Expired rows in `sessions` (including OAuth state rows) are always removed after a one-day grace period.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `proxy_logs_days` | i64 | `14` | Days to keep proxy access logs (`proxy_logs`). |
| `webhook_events_days` | i64 | `30` | Days to keep webhook delivery events (`webhook_events`). |
| `audit_logs_days` | i64 | `90` | Days to keep `audit_logs` and `team_audit_logs`. |
| `scheduled_job_runs_keep` | i64 | `100` | Most recent runs kept per scheduled job (`scheduled_job_runs`). |
| `deployment_snapshots_keep` | i64 | `3` | Most recent deployments per app whose pre-deploy database snapshots (`deployment_db_snapshots` and their dump files) are kept. Only used by apps with `snapshot_before_deploy` enabled. |

## `[email]`

SMTP for invitations and notifications. Considered configured only when `enabled` is true and both `smtp_host` and `from_address` are set.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Enable email sending. |
| `smtp_host` | string? | _none_ | SMTP server host (e.g. `smtp.gmail.com`). |
| `smtp_port` | u16 | `587` | SMTP server port. |
| `smtp_username` | string? | _none_ | SMTP auth username. |
| `smtp_password` | string? | _none_ | SMTP auth password. |
| `smtp_tls` | bool | `true` | Use TLS for the SMTP connection. |
| `from_address` | string? | _none_ | From address for outgoing email. |
| `from_name` | string | `"Rivetr"` | From display name. |

## `[auto_update]`

Self-update behavior.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable update checking. |
| `auto_apply` | bool | `false` | Automatically download and apply updates. When false, updates are only detected and reported via the API. |
| `check_interval_hours` | u64 | `6` | Hours between update checks. |
| `github_repo` | string | `"KwaminaWhyte/rivetr"` | GitHub repository to check for releases. |
| `include_prereleases` | bool | `false` | Include pre-release versions. |

## `[ai]`

AI-powered features (deployment diagnosis, insights, etc.). All fields optional.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | string? | `"claude"` (when unset) | Provider: `claude`, `openai`, `gemini`, or `moonshot`. |
| `api_key` | string? | _none_ | API key for the selected provider. |
| `model` | string? | provider default | Model override. |
| `max_tokens` | u32? | `2048` | Max output tokens. |
