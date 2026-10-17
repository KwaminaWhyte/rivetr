//! SSH host key pinning for connections to managed servers and build servers.
//!
//! Host keys are trusted on first use and stored in `<data_dir>/ssh/known_hosts`
//! (`StrictHostKeyChecking=accept-new`). A later connection that presents a
//! different key is refused by OpenSSH, which blocks man-in-the-middle attacks.
//! If a server is legitimately rebuilt, forget its pinned key with
//! [`forget_host`] (exposed as `DELETE /api/servers/:id/host-key`).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static KNOWN_HOSTS: OnceLock<PathBuf> = OnceLock::new();

/// Create `<data_dir>/ssh` (mode 0700) and remember the known_hosts path.
/// Safe to call more than once; the first call wins.
pub fn init(data_dir: &Path) -> std::io::Result<()> {
    let dir = data_dir.join("ssh");
    std::fs::create_dir_all(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }
    let _ = KNOWN_HOSTS.set(dir.join("known_hosts"));
    Ok(())
}

/// Path of the pinned host keys file.
pub fn known_hosts_path() -> PathBuf {
    KNOWN_HOSTS
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("data/ssh/known_hosts"))
}

/// Value for `-o UserKnownHostsFile=...`.
pub fn known_hosts_option() -> String {
    format!("UserKnownHostsFile={}", known_hosts_path().display())
}

/// The `-o` options, as a string for embedding in `rsync -e "ssh ..."`.
pub fn host_key_opts_string() -> String {
    format!(
        "-o StrictHostKeyChecking=accept-new -o '{}'",
        known_hosts_option()
    )
}

/// Remove the pinned key for `host:port`. Returns true if an entry existed.
pub async fn forget_host(host: &str, port: u16) -> std::io::Result<bool> {
    let path = known_hosts_path();
    if !path.exists() {
        return Ok(false);
    }
    // ssh-keygen -R stores non-default ports as "[host]:port".
    let pattern = if port == 22 {
        host.to_string()
    } else {
        format!("[{}]:{}", host, port)
    };
    let output = tokio::process::Command::new("ssh-keygen")
        .arg("-R")
        .arg(&pattern)
        .arg("-f")
        .arg(&path)
        .output()
        .await?;
    // ssh-keygen keeps a "<file>.old" copy of the previous contents; drop it.
    let _ = tokio::fs::remove_file(path.with_extension("old")).await;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(output.status.success() && stdout.contains("found"))
}

/// True when OpenSSH output says a pinned host key no longer matches.
pub fn is_host_key_changed(stderr: &str) -> bool {
    stderr.contains("REMOTE HOST IDENTIFICATION HAS CHANGED")
        || stderr.contains("Host key verification failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_strings_use_known_hosts_path() {
        let opt = known_hosts_option();
        assert!(opt.starts_with("UserKnownHostsFile="));
        assert!(opt.ends_with("known_hosts"));
        let s = host_key_opts_string();
        assert!(s.contains("StrictHostKeyChecking=accept-new"));
        assert!(!s.contains("StrictHostKeyChecking=no"));
    }

    #[tokio::test]
    async fn forget_host_removes_only_the_matching_entry() {
        let dir = std::env::temp_dir().join(format!("rivetr-ssh-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(&dir).unwrap();
        let path = known_hosts_path();
        // Only meaningful if this test won the OnceLock race; skip otherwise.
        if !path.starts_with(&dir) {
            return;
        }
        let key =
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
        std::fs::write(
            &path,
            format!("example.com {key}\n[example.com]:2222 {key}\nother.org {key}\n"),
        )
        .unwrap();

        assert!(forget_host("example.com", 2222).await.unwrap());
        let left = std::fs::read_to_string(&path).unwrap();
        assert!(!left.contains("[example.com]:2222"));
        assert!(left.contains("example.com ssh-ed25519"));
        assert!(left.contains("other.org"));

        // Nothing pinned for this host: reports false instead of erroring.
        assert!(!forget_host("missing.example", 22).await.unwrap());
        // Default port entries use the bare hostname.
        assert!(forget_host("example.com", 22).await.unwrap());
        assert!(!std::fs::read_to_string(&path)
            .unwrap()
            .contains("example.com"));
        assert!(!path.with_extension("old").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detects_changed_host_key() {
        assert!(is_host_key_changed(
            "@@@ WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED! @@@"
        ));
        assert!(is_host_key_changed("Host key verification failed."));
        assert!(!is_host_key_changed("Permission denied (publickey)."));
    }
}
