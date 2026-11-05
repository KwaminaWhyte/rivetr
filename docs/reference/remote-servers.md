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

Once registered, apps can be assigned to the server. A deploy builds on the
local host or a build server, pushes the image to the configured registry,
pulls it on the target server, starts the container and updates the proxy
route. **Servers > [server] > Terminal** opens a browser SSH session.

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
