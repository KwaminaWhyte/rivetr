# Remote servers

Rivetr manages remote servers over SSH: health checks, Docker install, package
patching, security checks, Docker cleanup, the server terminal and remote build
servers. This page covers connecting as a non-root user and reaching servers
over Tailscale.

## Adding a server

1. Open **Servers** in the dashboard and click **Add Server**.
2. Enter a name, host or IP, SSH port and username, and either an SSH private
   key or a password. Keys and passwords are stored encrypted (AES-256-GCM).
3. Rivetr tests the connection and checks that Docker is available.

Once registered, servers can be added to an app's placement (see
[One app on several servers](#one-app-on-several-servers)). A deploy builds
on the local host or a build server, sends the image to each server, starts
the containers and adds them to the proxy route. **Servers > [server] > Terminal** opens a browser SSH session.

## One app on several servers

An app can run on several servers at once, with the Rivetr proxy balancing
traffic across all of them. Open the app's **Resources** section and use the
**Servers** card: add the Rivetr host and/or registered servers, each with a
number of containers (1 to 10, up to 20 servers). Leave the list empty to keep
the single-host behaviour. `apps.server_id` stays the primary server (the
first placement entry) for older API clients.

### What a deploy does

1. **Build once.** The image is built on the Rivetr host or the app's build
   server (or pulled, for registry image apps).
2. **Distribute.** Each remote server gets the image in one of two ways
   (Servers card, **Image distribution**):

   | Mode | How | Tradeoff |
   |------|-----|----------|
   | Registry | Push once (Build > registry push), every server runs `docker pull` after a `docker login` fed on stdin | Fast for many servers, layer caching; needs a registry reachable from every server |
   | SSH streaming | `docker save \| gzip` on the Rivetr host piped into `gunzip \| docker load` over SSH | No registry; the whole image crosses the network once per server per deploy |
   | Automatic (default) | Registry when this build was pushed, SSH streaming otherwise | |

3. **Rolling start.** Containers are started in batches of **Batch size (max
   unavailable)**. Slots alternate between servers (A0, B0, A1, B1, ...) so a
   batch does not hit one server only. Each new container starts next to the
   old one (`rivetr-<app>-ms-<server>-<n>-<deployment>`), env vars are passed in
   a private temp env file on the server, and pre-deploy commands run once in
   the first container.
4. **Health check per container**, from the Rivetr host, against the address
   the proxy will use (`server address:published port`): the app's structured
   healthcheck (HTTP or command), or a TCP readiness wait when none is set.
5. **Shift traffic.** After each batch, healthy new containers join the
   proxy pool and the old containers on the same slots leave it.
6. **Roll back or finish.** If at least **Roll back when failed (%)** of the
   containers fail (0 means any failure), every new container is removed and the
   pool goes back to the previous containers, which never stopped. Otherwise the
   old containers are drained and removed; a slot whose new container failed
   keeps its previous container. Post-deploy commands run once.

The deployment page lists every target with its batch, status, upstream
address, error and log (`GET /api/deployments/{id}/targets`). Start, stop,
restart and delete act on every server.

### Load balancing

The proxy routes the app's domains to every healthy container across servers:

- **Round robin** (default) or **least connections** (fewest in-flight requests).
- **Passive health:** a container that refuses or drops a connection is
  skipped for 10 seconds. The periodic proxy health check still watches the
  first upstream.
- **Sticky sessions** (optional): a `rivetr_upstream` cookie (an opaque hash,
  not the address) pins a browser to one container; it is reassigned when that
  container is down.

### Network and security

The proxy runs on the Rivetr host, so each remote container publishes the app
port and the Rivetr host must reach it. That port is reachable by anything
that can reach the address it is bound to:

- With **Bind to private address** on (default) and a Tailscale address set
  on the server (see below), the port is published on the Tailscale interface
  only (`-p 100.x.y.z::<port>`; a MagicDNS name is resolved on the server with
  `tailscale ip -4`) and the proxy connects over the tailnet. This is the
  recommended setup.
- Without a private address, the port is published on every interface and
  the proxy uses the server's public host. The deployment log and the Servers
  card warn about this. Restrict the port in the server firewall to the Rivetr
  host, or move the servers onto Tailscale or a private network.
- On the Rivetr host, containers bind to `127.0.0.1`.

### Data and configuration

- **Env vars** (app, environment, project, team and linked databases) are
  applied on every server. Values with line breaks cannot go through an env
  file and are skipped with a warning.
- **Volumes are per server.** Each container mounts the app's volume paths on
  its own server; nothing is shared or copied. Use a managed database, object
  storage or another shared store for state.
- Per-domain internal ports are not published on remote servers; domains
  route to the app port.

### API

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/apps/{id}/placement` | Placement, settings, server count, live upstreams |
| PUT | `/api/apps/{id}/placement` | `{ "placements": [{ "server_id": "localhost", "replicas": 2 }, { "server_id": "<id>", "replicas": 1 }], "settings": { "max_unavailable": 1, "failure_threshold_pct": 50, "distribution": "auto", "lb_policy": "round_robin", "sticky_sessions": false, "bind_private": true } }`; an empty list turns it off |
| GET | `/api/apps/placement-counts` | `{ app_id: server_count }` |
| GET | `/api/deployments/{id}/targets` | Per-target status and logs |

Load balancing changes apply to the live pool when saved; placement changes
apply on the next deploy.

## Connecting as a non-root user (passwordless sudo)

Rivetr can connect as any SSH user. When that user is not `root`, every
privileged command is run through `sudo -n` by one shared helper
(`src/utils/privileged.rs`):

- `$SUDO` is `sudo -n` for a non-root user and empty for root. Package
  installs, `systemctl`, firewall and sshd checks use it.
- Docker commands go straight to `docker` when the user can reach the daemon
  (member of the `docker` group), and through `sudo -n docker` otherwise.

`-n` means sudo never waits for a password. If the user needs a password,
the privileged step fails with `sudo: a password is required` instead of hanging.

### Setup

On the server, as root (or with an existing sudo session), replace `deploy`
with your SSH user:

```bash
# 1. Create the user and install your Rivetr SSH public key
sudo adduser --disabled-password --gecos "" deploy
sudo mkdir -p /home/deploy/.ssh
echo "ssh-ed25519 AAAA... rivetr" | sudo tee /home/deploy/.ssh/authorized_keys
sudo chown -R deploy:deploy /home/deploy/.ssh
sudo chmod 700 /home/deploy/.ssh && sudo chmod 600 /home/deploy/.ssh/authorized_keys

# 2. Grant passwordless sudo (visudo checks the syntax before saving)
echo "deploy ALL=(ALL) NOPASSWD: ALL" | sudo EDITOR='tee' visudo -f /etc/sudoers.d/rivetr
sudo chmod 440 /etc/sudoers.d/rivetr

# 3. Optional: let the user run docker without sudo
sudo usermod -aG docker deploy
```

Check it as the SSH user:

```bash
sudo -n true && echo "passwordless sudo OK"
```

Then add the server in **Servers** with username `deploy` and run **Check**.

### Validation check

The server health check reports one of:

| Result | Meaning |
|--------|---------|
| Root user | Connected as root, no sudo needed |
| Passwordless sudo | Non-root user, `sudo -n true` works |
| Sudo missing | `sudo` asks for a password. The UI shows the exact line to add to `/etc/sudoers.d/rivetr` with a copy button |
| Sudo missing (not installed) | `sudo` is not installed. Install it, then add the same line |

The check is returned by `POST /api/servers/{id}/check` as the `privilege`
field (`ssh_user`, `status`, `ok`, `message`, `sudoers_line`).

### Narrower sudo rules

`NOPASSWD: ALL` is the simplest working setup. If you restrict it, Rivetr needs
at least: `docker` (unless the user is in the `docker` group), `systemctl`,
your package manager (`apt-get`, `dnf`, `yum`, `apk`, `pacman`, `zypper`),
`ufw` or `firewall-cmd`, `ss`, `grep` (sshd config), `tee`, `mkdir`, `sed` and
`usermod` (Docker install). Features whose command is not allowed fail with a
sudo error and leave the server unchanged.

## Reaching servers over Tailscale

A server can be reached over its public IP or over a Tailscale address. Set
**Connect via: Tailscale** when adding the server, or later on the server's
General page (Network section), and enter the server's Tailscale IP
(`100.x.y.z`) or MagicDNS name (`web-1.tail1234.ts.net`). All SSH traffic
from Rivetr to that server (health checks, operations, terminal, file
browser) then uses the Tailscale address. The public host is kept for display.

Rivetr does not manage your tailnet: it does not create auth keys, approve
devices or edit ACLs. Both machines must already be on the same tailnet.

### Setup

On the Rivetr host and on each server:

```bash
curl -fsSL https://tailscale.com/install.sh | sh
sudo tailscale up            # opens a login URL; or use --auth-key=tskey-... for headless hosts
tailscale ip -4              # the 100.x.y.z address to enter in Rivetr
```

Optional, on the server: allow SSH only over the tailnet, after checking the
Tailscale connection works.

```bash
sudo ufw allow in on tailscale0 to any port 22 proto tcp
sudo ufw delete allow 22/tcp   # removes public SSH access
```

If your tailnet uses ACLs, allow the Rivetr host to reach the servers on
port 22 (or your SSH port). Tailscale SSH is not required: Rivetr uses the
normal OpenSSH server and its own keys.

### Connectivity check

**Check connectivity** in the Network section (`POST /api/servers/{id}/network-check`) reports:

| Check | Passes when |
|-------|-------------|
| Tailscale on the Rivetr host | `tailscale status --json` shows `BackendState: Running` (skipped for public servers without Tailscale) |
| Tailscale ping | `tailscale ping -c 1` to the server's Tailscale address gets a pong (Tailscale mode only) |
| SSH port reachable | A TCP connection to the SSH port on the address Rivetr uses succeeds |

### Notes

- Rivetr in Docker: run Tailscale on the host and start the Rivetr container
  with `network_mode: host`, or run a `tailscale/tailscale` sidecar and share
  its network namespace. The `tailscale` CLI must be available to Rivetr for
  the first two checks; the SSH check works either way.
- Pinned SSH host keys are stored per address. Switching a server to Tailscale
  pins the key again on first connect; **Reset host key** forgets both addresses.
- Build servers still connect over their configured host.
