# Remote servers

Rivetr manages remote servers over SSH: health checks, Docker install, package
patching, security checks, Docker cleanup, the server terminal and remote build
servers. This page covers connecting as a non-root user.

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
