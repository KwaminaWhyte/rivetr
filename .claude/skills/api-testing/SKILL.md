---
name: api-testing
description: Test Rivetr REST API endpoints. Use when testing API functionality, debugging webhook issues, or verifying deployments.
allowed-tools: Bash, Read
---

# API Testing for Rivetr

## Prerequisites

Server running on default port:
```bash
cargo run -- --config rivetr.example.toml
```

Get admin token from server output or config file.

## Authentication

All `/api/*` endpoints require authentication:
```bash
# Using Authorization header
curl -H "Authorization: Bearer YOUR_TOKEN" http://localhost:8080/api/apps

# Using X-API-Key header
curl -H "X-API-Key: YOUR_TOKEN" http://localhost:8080/api/apps
```

## Health & System

```bash
# Basic health check
curl http://localhost:8080/health
# Expected: OK

# Detailed system health
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/system/health

# Disk usage
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/system/disk

# Prometheus metrics
curl http://localhost:8080/metrics
```

## Apps API

### Create App
```bash
curl -X POST http://localhost:8080/api/apps \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-app",
    "git_url": "https://github.com/user/repo.git",
    "branch": "main",
    "port": 3000,
    "environment": "development",
    "project_id": "PROJECT_ID"
  }'
```

### List Apps
```bash
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/apps
```

### Get App
```bash
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/apps/{app_id}
```

### Update App
```bash
curl -X PUT http://localhost:8080/api/apps/{app_id} \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "updated-name", "port": 8000}'
```

### Delete App
```bash
curl -X DELETE -H "Authorization: Bearer TOKEN" http://localhost:8080/api/apps/{app_id}
```

### App Status & Controls
```bash
# Get app status
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/apps/{app_id}/status

# Start app
curl -X POST -H "Authorization: Bearer TOKEN" http://localhost:8080/api/apps/{app_id}/start

# Stop app
curl -X POST -H "Authorization: Bearer TOKEN" http://localhost:8080/api/apps/{app_id}/stop

# Restart app
curl -X POST -H "Authorization: Bearer TOKEN" http://localhost:8080/api/apps/{app_id}/restart

# Get app stats
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/apps/{app_id}/stats
```

## Deployments API

### Trigger Deployment
```bash
curl -X POST -H "Authorization: Bearer TOKEN" http://localhost:8080/api/apps/{app_id}/deploy
```

### Upload & Deploy (ZIP)
```bash
curl -X POST http://localhost:8080/api/apps/{app_id}/deploy/upload \
  -H "Authorization: Bearer TOKEN" \
  -F "file=@myapp.zip"
```

### List Deployments (Paginated)
```bash
curl -H "Authorization: Bearer TOKEN" \
  "http://localhost:8080/api/apps/{app_id}/deployments?page=1&per_page=20"
```

### Get Deployment
```bash
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/deployments/{deployment_id}
```

### Get Deployment Logs
```bash
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/deployments/{deployment_id}/logs
```

### Rollback Deployment
```bash
curl -X POST -H "Authorization: Bearer TOKEN" http://localhost:8080/api/deployments/{deployment_id}/rollback
```

## Environment Variables

```bash
# List env vars
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/apps/{app_id}/env

# Create env var
curl -X POST http://localhost:8080/api/apps/{app_id}/env \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"key": "DATABASE_URL", "value": "postgres://...", "is_secret": true}'

# Update env var
curl -X PUT http://localhost:8080/api/apps/{app_id}/env/{key} \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"value": "new-value"}'

# Delete env var
curl -X DELETE -H "Authorization: Bearer TOKEN" http://localhost:8080/api/apps/{app_id}/env/{key}
```

## Projects API

```bash
# List projects
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/projects

# Create project
curl -X POST http://localhost:8080/api/projects \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "my-project", "description": "Project description"}'

# Get project
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/projects/{project_id}

# Delete project
curl -X DELETE -H "Authorization: Bearer TOKEN" http://localhost:8080/api/projects/{project_id}
```

## Databases API

```bash
# List databases
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/databases

# Create database
curl -X POST http://localhost:8080/api/databases \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-postgres",
    "db_type": "postgres",
    "version": "16",
    "project_id": "PROJECT_ID"
  }'

# Get database
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/databases/{db_id}

# Update database (toggle public access)
curl -X PUT http://localhost:8080/api/databases/{db_id} \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"public_access": true, "external_port": 5432}'

# Start/stop database
curl -X POST -H "Authorization: Bearer TOKEN" http://localhost:8080/api/databases/{db_id}/start
curl -X POST -H "Authorization: Bearer TOKEN" http://localhost:8080/api/databases/{db_id}/stop

# Delete database
curl -X DELETE -H "Authorization: Bearer TOKEN" http://localhost:8080/api/databases/{db_id}
```

## Services API (Docker Compose)

```bash
# List services
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/services

# Create service from compose
curl -X POST http://localhost:8080/api/services \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-stack",
    "compose_content": "version: \"3\"\nservices:\n  web:\n    image: nginx",
    "project_id": "PROJECT_ID"
  }'

# Start/stop service
curl -X POST -H "Authorization: Bearer TOKEN" http://localhost:8080/api/services/{service_id}/start
curl -X POST -H "Authorization: Bearer TOKEN" http://localhost:8080/api/services/{service_id}/stop

# Delete service
curl -X DELETE -H "Authorization: Bearer TOKEN" http://localhost:8080/api/services/{service_id}
```

## Templates API

```bash
# List templates
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/templates

# Get template
curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/templates/{template_id}

# Deploy template
curl -X POST http://localhost:8080/api/templates/{template_id}/deploy \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "my-grafana", "project_id": "PROJECT_ID"}'
```

## Webhooks (No Auth Required)

### GitHub Webhook
```bash
curl -X POST http://localhost:8080/webhooks/github \
  -H "Content-Type: application/json" \
  -H "X-Hub-Signature-256: sha256=SIGNATURE" \
  -d '{
    "ref": "refs/heads/main",
    "after": "abc123",
    "repository": {
      "clone_url": "https://github.com/user/repo.git",
      "full_name": "user/repo"
    },
    "head_commit": {"id": "abc123", "message": "Test commit"}
  }'
```

### GitLab Webhook
```bash
curl -X POST http://localhost:8080/webhooks/gitlab \
  -H "Content-Type: application/json" \
  -H "X-Gitlab-Token: SECRET" \
  -d '{
    "ref": "refs/heads/main",
    "checkout_sha": "abc123",
    "project": {"git_http_url": "https://gitlab.com/user/repo.git"}
  }'
```

### Gitea Webhook
```bash
curl -X POST http://localhost:8080/webhooks/gitea \
  -H "Content-Type: application/json" \
  -d '{
    "ref": "refs/heads/main",
    "after": "abc123",
    "repository": {"clone_url": "https://gitea.example.com/user/repo.git"}
  }'
```

## Expected Status Codes

| Endpoint | Success | Auth Error | Not Found | Validation |
|----------|---------|------------|-----------|------------|
| GET /health | 200 | - | - | - |
| GET /api/apps | 200 | 401 | - | - |
| POST /api/apps | 201 | 401 | - | 400 |
| GET /api/apps/:id | 200 | 401 | 404 | - |
| PUT /api/apps/:id | 200 | 401 | 404 | 400 |
| DELETE /api/apps/:id | 204 | 401 | 404 | - |
| POST /api/apps/:id/deploy | 201 | 401 | 404 | - |
| POST /webhooks/github | 200/202 | - | - | 400 |

## Rate Limits

Default rate limits (per IP):
- Standard tier: 100 requests/minute
- Auth tier: 1000 requests/minute
- Webhook tier: 50 requests/minute

Rate limit headers:
- `X-RateLimit-Limit`: Max requests
- `X-RateLimit-Remaining`: Requests remaining
- `X-RateLimit-Reset`: Reset timestamp
