# Service deployments

Docker Compose services have a deploy pipeline next to start, stop and restart.
Everything below is on the service page (Deployments, Build & Hooks, Containers,
Environment tabs) and in the API.

## What Deploy does

1. Records a deployment (who, trigger, start time) and reads the image ID and
   registry digest each compose service runs now.
2. If the service has a build source: fetches it, builds each configured image
   as `<repo>:<revision>` and `<repo>:latest`, and pushes both tags.
3. Writes the compose file (and the managed `.env`), keeping the previous files.
4. `docker compose pull --ignore-pull-failures` for every compose service it did
   not just build.
5. Runs the pre-deploy hooks. A failure stops here: the previous compose file
   and `.env` are restored, `<repo>:latest` points at the image the stack was
   running again, and no container is touched.
6. `docker compose up -d --remove-orphans`: only containers whose image or config
   changed are recreated. The stack is never taken down.
7. Records the new digests, then runs the post-deploy hooks (a failure is shown
   on the deployment, the new containers stay).

One deployment per service runs at a time (a second one gets `409`). A
deployment that was running when Rivetr restarted is marked failed at startup.

A service with no build source deploys by pulling, so existing services work
exactly as before. Start, stop and restart are unchanged.

## Build source

| Field | Notes |
|---|---|
| Source | `none` (pull only), `git` (repo + branch), or `path` (an absolute directory on the server) |
| Git credentials | The GitHub App installation or git provider token that apps use, or the global SSH key for SSH URLs |
| Revision | Git: the short commit sha. Path: the content of the revision file if set, else the commit of a git checkout, else a UTC timestamp |
| Images | Built in order. Target repository (e.g. `localhost:5000/app`), Dockerfile and context (relative to the source root or absolute), build args, target stage. Build args may use `${TAG}`, `${COMMIT_SHA}`, `${SHORT_SHA}` |
| Registry | Push is on by default. A local `registry:2` on `localhost:5000` needs no login. Username and password (stored encrypted) are used with a throwaway `DOCKER_CONFIG` |
| Auto deploy | Git sources only: a push to the branch (GitHub, GitLab, Gitea webhooks, same endpoints as apps) starts a deployment |

When Rivetr runs under systemd with `ProtectSystem=strict`, a path source and
its Dockerfiles only need to be readable by the `rivetr` user; git clones go to
`<data_dir>/service-builds/` and are removed after the deployment.

## Hooks

A hook runs `docker compose run --rm -T --no-deps --name <...> <service> <command>`,
so it gets that compose service's image, `env_file`, `environment`, volumes and
networks. The command is passed to the image entrypoint like `docker run <image>
<command>`; write `sh -c "a && b"` for shell features. Default timeout 600s.
Rollbacks skip hooks (migrations are not reversed).

## Rollback

Roll back to any deployment that recorded images. Each compose service whose
image differs from that deployment's is pinned to its registry digest (or local
image ID if it was never pushed), so `latest` moving does not matter; unchanged
services are not recreated. Pins survive start and restart. The next regular
Deploy clears them and the stack follows its tags again.

## Environment

Variables are stored encrypted (`rivetr db encrypt-secrets` covers them). On
every start, restart and deploy Rivetr writes them to `.env` (mode 600, with a
`# Managed by Rivetr` header) next to the service's `docker-compose.yml`:

- compose reads it for `${VAR}` interpolation;
- a compose service loads it with `env_file: .env`;
- or turn on "Add the .env file to every container" and Rivetr appends it to
  every compose service's `env_file` list.

An `env_file` that points at a file elsewhere on the server keeps working. A
`.env` that Rivetr did not write is never touched.

## Containers

The Containers tab lists every container of the compose project with state,
health, CPU and memory, and can restart one container, show its logs or open
`/bin/sh` in it.

## Moving a hand-built setup over

Example: a service whose images are built by a script that copies the source to
`/opt/myapp/src`, builds `localhost:5000/myapp-app` and `localhost:5000/myapp-worker`
(the worker `FROM` the app image via `--build-arg BASE=...:$REV`), pushes both,
runs a migration with `docker run --rm --network rivetr --env-file /etc/myapp/app.env
localhost:5000/myapp-app:latest bun run db:migrate`, then restarts the service.

1. Keep the compose file. It already uses `image: localhost:5000/myapp-app:latest`.
2. Build & Hooks > Source: `Path on the server`, `/opt/myapp/src`, revision file
   `/opt/myapp/REVISION` (or a git repo + branch instead).
3. Image 1: `localhost:5000/myapp-app`, Dockerfile `infra/Dockerfile`, context `.`.
4. Image 2: `localhost:5000/myapp-worker`, Dockerfile `/opt/myapp/Dockerfile.worker`,
   context `/opt/myapp`, build arg `BASE=localhost:5000/myapp-app:${TAG}`.
5. Pre-deploy hook: run in `api` (a compose service that loads
   `/etc/myapp/app.env`), command `bun run db:migrate`.
6. Save, Deploy, follow the log. Later, optionally import `/etc/myapp/app.env`
   in the Environment tab and switch the compose `env_file` to `.env`.

## API

| Method | Path | Description |
|---|---|---|
| POST | `/api/services/:id/deploy` | Start a deployment (`202`, `409` if one is running) |
| GET | `/api/services/:id/deployments?limit=` | Deployment history, newest first |
| GET | `/api/services/:id/deployments/:deployment_id` | One deployment with images before and after |
| GET | `/api/services/:id/deployments/:deployment_id/logs` | Stored log lines |
| WS | `/api/services/:id/deployments/:deployment_id/logs/stream?token=` | Live log lines, then `{"type":"end"}` |
| POST | `/api/services/:id/deployments/:deployment_id/rollback` | Redeploy that deployment's images by digest |
| GET, PUT | `/api/services/:id/deploy-config` | Build source, images, registry, hooks, auto deploy, env_file switch |
| GET | `/api/services/:id/containers` | Containers with state, health, CPU, memory |
| POST | `/api/services/:id/containers/:container/restart` | Restart one container |
| GET | `/api/services/:id/containers/:container/logs?lines=` | Recent log lines of one container |
| WS | `/api/services/:id/containers/:container/terminal?token=` | Shell in one container |
| GET, POST | `/api/services/:id/env-vars` | List (`?reveal=true`) or create variables |
| PUT, DELETE | `/api/services/:id/env-vars/:key` | Update or delete a variable |
| POST | `/api/services/:id/env-import` | Import a pasted `.env` file |
