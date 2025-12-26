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

## Health Check

```bash
curl http://localhost:8080/health
# Expected: OK
```

## Apps API

### Create App
```bash
curl -X POST http://localhost:8080/api/apps \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-app",
    "git_url": "https://github.com/user/repo.git",
    "branch": "main",
    "port": 3000
  }'
```

### List Apps
```bash
curl -H "Authorization: Bearer YOUR_TOKEN" \
  http://localhost:8080/api/apps
```

### Get App
```bash
curl -H "Authorization: Bearer YOUR_TOKEN" \
  http://localhost:8080/api/apps/{app_id}
```

### Delete App
```bash
curl -X DELETE -H "Authorization: Bearer YOUR_TOKEN" \
  http://localhost:8080/api/apps/{app_id}
```

## Deployments API

### Trigger Deployment
```bash
curl -X POST -H "Authorization: Bearer YOUR_TOKEN" \
  http://localhost:8080/api/apps/{app_id}/deploy
```

### List Deployments
```bash
curl -H "Authorization: Bearer YOUR_TOKEN" \
  http://localhost:8080/api/apps/{app_id}/deployments
```

### Get Deployment Logs
```bash
curl -H "Authorization: Bearer YOUR_TOKEN" \
  http://localhost:8080/api/deployments/{deployment_id}/logs
```

## Webhooks (No Auth Required)

### GitHub Webhook
```bash
curl -X POST http://localhost:8080/webhooks/github \
  -H "Content-Type: application/json" \
  -d '{
    "ref": "refs/heads/main",
    "after": "abc123",
    "repository": {
      "clone_url": "https://github.com/user/repo.git",
      "ssh_url": "git@github.com:user/repo.git",
      "full_name": "user/repo"
    },
    "head_commit": {
      "id": "abc123",
      "message": "Test commit"
    }
  }'
```

## Expected Status Codes

| Endpoint | Success | Auth Error | Not Found |
|----------|---------|------------|-----------|
| GET /health | 200 | - | - |
| GET /api/apps | 200 | 401 | - |
| POST /api/apps | 201 | 401 | - |
| GET /api/apps/:id | 200 | 401 | 404 |
| DELETE /api/apps/:id | 204 | 401 | 404 |
| POST /webhooks/github | 200 | - | - |
