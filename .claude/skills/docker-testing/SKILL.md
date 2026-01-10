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

# Check Rivetr's auto-detection
# In logs, look for: "Using Docker runtime" or "Using Podman runtime"
```

## Docker Socket

Verify Docker socket access:
```bash
# Linux
ls -la /var/run/docker.sock

# Windows (named pipe)
# npipe:////./pipe/docker_engine

# macOS (Docker Desktop)
ls -la ~/.docker/run/docker.sock
```

## Build Types

Rivetr supports multiple build methods:

### 1. Dockerfile (default)
```bash
# Standard Docker build
docker build -t rivetr-app:latest .

# With build args
docker build --build-arg NODE_ENV=production -t rivetr-app:latest .

# With custom Dockerfile path
docker build -f docker/Dockerfile.prod -t rivetr-app:latest .
```

### 2. Nixpacks
```bash
# Install nixpacks
curl -sSL https://nixpacks.com/install.sh | bash

# Build with nixpacks
nixpacks build . --name my-app

# Build with specific provider
nixpacks build . --name my-app --pkgs nodejs-18_x

# Generate Dockerfile only
nixpacks plan . --format dockerfile > Dockerfile
```

### 3. Railpack (Nixpacks successor)
```bash
# Install railpack
cargo install railpack
# OR
curl -sSL https://railpack.io/install.sh | bash

# Build with railpack
railpack build . --name my-app

# With custom commands
railpack build . --name my-app --install "npm ci" --build "npm run build"
```

### 4. Cloud Native Buildpacks (pack)
```bash
# Install pack CLI
# macOS
brew install buildpacks/tap/pack

# Linux
curl -sSL https://github.com/buildpacks/pack/releases/download/v0.33.0/pack-v0.33.0-linux.tgz | tar -xz

# Build with Heroku builder
pack build my-app --builder heroku/builder:24

# Build with Paketo builder
pack build my-app --builder paketobuildpacks/builder-jammy-base

# Available Paketo builders:
# - paketobuildpacks/builder-jammy-base (general purpose)
# - paketobuildpacks/builder-jammy-full (full dependencies)
# - paketobuildpacks/builder-jammy-tiny (minimal)
```

### 5. Static Site Builder
```bash
# Rivetr generates a multi-stage Dockerfile:
# Stage 1: Build (npm run build)
# Stage 2: NGINX to serve static files

# Test manually:
docker build -f- . <<EOF
FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
EXPOSE 80
EOF
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

# Run with resource limits (like Rivetr does)
docker run -d --name rivetr-test \
  --cpus=1 \
  --memory=512m \
  --restart=unless-stopped \
  -p 3000:3000 \
  rivetr-test:latest

# Check status
docker ps -a | grep rivetr-test

# View logs
docker logs rivetr-test-container

# Stream logs (like Rivetr log streaming)
docker logs -f rivetr-test-container

# Get container stats
docker stats rivetr-test-container --no-stream

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

# Check with timeout (like Rivetr health checks)
curl --max-time 5 http://localhost:8080/health

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

#[tokio::test]
async fn test_container_stats() {
    let docker = Docker::connect_with_local_defaults().unwrap();
    let options = bollard::container::StatsOptions {
        stream: false,
        one_shot: true,
    };
    let stats = docker.stats("container_id", Some(options)).next().await;
    println!("{:?}", stats);
}
```

## Common Issues

### Permission Denied
```bash
# Add user to docker group
sudo usermod -aG docker $USER
# Then logout/login

# Verify
groups | grep docker
```

### Docker Not Running
```bash
# Start Docker daemon
sudo systemctl start docker

# Check status
sudo systemctl status docker

# Enable on boot
sudo systemctl enable docker
```

### Podman Rootless
```bash
# Run as user (no sudo)
podman run -d --name test alpine sleep 300
podman ps

# Enable podman socket (for Bollard compatibility)
systemctl --user enable podman.socket
systemctl --user start podman.socket
export DOCKER_HOST=unix://$XDG_RUNTIME_DIR/podman/podman.sock
```

### Build Fails with OOM
```bash
# Check available memory
free -h

# Increase Docker's memory limit (Docker Desktop)
# Settings > Resources > Memory

# Or limit build resources in rivetr.toml:
# [runtime]
# build_memory_limit = "4g"
# build_cpu_limit = "2"
```

### Image Not Found After Build
```bash
# List images
docker images | grep rivetr

# Check build output for image ID
docker build . 2>&1 | tail -5

# Verify image exists
docker image inspect rivetr-app:latest
```

## Cleanup Commands

```bash
# Remove all Rivetr containers
docker ps -a | grep rivetr | awk '{print $1}' | xargs -r docker rm -f

# Remove all Rivetr images
docker images | grep rivetr | awk '{print $3}' | xargs -r docker rmi -f

# Remove dangling images (from failed builds)
docker image prune -f

# Prune all unused resources
docker system prune -f

# Nuclear option: remove everything
docker system prune -a --volumes -f
```

## Container Logs for Debugging

```bash
# View recent logs
docker logs --tail 100 CONTAINER_ID

# Follow logs (streaming)
docker logs -f CONTAINER_ID

# Logs with timestamps
docker logs -t CONTAINER_ID

# Logs since a specific time
docker logs --since 5m CONTAINER_ID
```
