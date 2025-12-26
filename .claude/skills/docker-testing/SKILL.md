---
name: docker-testing
description: Test Docker and Podman container operations for Rivetr. Use when debugging container builds, testing runtime detection, or troubleshooting deployment issues.
allowed-tools: Bash, Read
---

# Container Runtime Testing for Rivetr

## Runtime Detection

Check which runtime is available:

```bash
# Check Docker
docker --version
docker info

# Check Podman
podman --version
podman info
```

## Docker Socket

Verify Docker socket access:
```bash
# Linux
ls -la /var/run/docker.sock

# Windows (named pipe)
# npipe:////./pipe/docker_engine
```

## Test Container Build

Create a test Dockerfile:
```dockerfile
FROM alpine:latest
RUN echo "Hello from Rivetr test"
CMD ["echo", "Container started successfully"]
```

Build manually:
```bash
docker build -t rivetr-test:latest .
```

## Test Container Run

```bash
# Run with port mapping
docker run -d --name rivetr-test-container -p 3000:3000 rivetr-test:latest

# Check status
docker ps -a | grep rivetr-test

# View logs
docker logs rivetr-test-container

# Stop and remove
docker stop rivetr-test-container
docker rm rivetr-test-container
```

## Test Health Check

```bash
# Start container with health endpoint
docker run -d --name health-test -p 8080:8080 nginx

# Check health
curl http://localhost:8080/

# Cleanup
docker stop health-test && docker rm health-test
```

## Bollard API Test

The Rivetr DockerRuntime uses Bollard. Test the connection:

```rust
// In Rust code or tests
use bollard::Docker;

#[tokio::test]
async fn test_docker_connection() {
    let docker = Docker::connect_with_local_defaults().unwrap();
    let info = docker.ping().await;
    assert!(info.is_ok());
}
```

## Common Issues

### Permission Denied
```bash
# Add user to docker group
sudo usermod -aG docker $USER
# Then logout/login
```

### Docker Not Running
```bash
# Start Docker daemon
sudo systemctl start docker

# Check status
sudo systemctl status docker
```

### Podman Rootless
```bash
# Run as user (no sudo)
podman run -d --name test alpine sleep 300
podman ps
```

## Cleanup Commands

```bash
# Remove all Rivetr containers
docker ps -a | grep rivetr | awk '{print $1}' | xargs docker rm -f

# Remove all Rivetr images
docker images | grep rivetr | awk '{print $3}' | xargs docker rmi -f

# Prune unused resources
docker system prune -f
```
