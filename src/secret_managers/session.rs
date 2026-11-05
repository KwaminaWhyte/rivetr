//! Per-deployment, in-memory state: resolved values to mask in deployment logs and the
//! secrets fetched so far (so a deploy reads each path once). Nothing here is persisted;
//! a [`SessionGuard`] drops the state when the deployment task ends, and sessions left
//! behind (a rollback started outside a guarded task) expire after an hour.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use dashmap::DashMap;

use super::providers::SecretData;

/// Replacement for a masked value (same as service deployment logs).
pub const MASK: &str = "********";

/// Values shorter than this are not masked (masking "1" or "true" would garble logs).
const MIN_MASK_LEN: usize = 4;
/// Lines of a multi-line value are masked on their own when at least this long.
const MIN_LINE_MASK_LEN: usize = 8;
const SESSION_TTL: Duration = Duration::from_secs(3600);

#[derive(Default)]
struct Session {
    masks: Vec<String>,
    cache: HashMap<(String, String), SecretData>,
    touched: Option<Instant>,
}

static SESSIONS: LazyLock<DashMap<String, Session>> = LazyLock::new(DashMap::new);

fn prune() {
    let now = Instant::now();
    SESSIONS.retain(|_, s| {
        s.touched
            .is_none_or(|t| now.duration_since(t) < SESSION_TTL)
    });
}

/// Mask these values in every log line written for `deployment_id`.
pub fn register_values<'a>(deployment_id: &str, values: impl IntoIterator<Item = &'a str>) {
    prune();
    let mut session = SESSIONS.entry(deployment_id.to_string()).or_default();
    session.touched = Some(Instant::now());
    for v in values {
        let mut add = |s: &str| {
            if !s.is_empty() && !session.masks.iter().any(|m| m == s) {
                session.masks.push(s.to_string());
            }
        };
        if v.len() >= MIN_MASK_LEN {
            add(v);
        }
        if v.contains('\n') {
            for line in v.lines().map(str::trim) {
                if line.len() >= MIN_LINE_MASK_LEN {
                    add(line);
                }
            }
        }
    }
    // Longest first so a value containing another is masked whole.
    session.masks.sort_by_key(|m| std::cmp::Reverse(m.len()));
}

/// `message` with every registered value for `deployment_id` replaced by [`MASK`].
pub fn redact(deployment_id: &str, message: &str) -> String {
    match SESSIONS.get(deployment_id) {
        Some(s) if !s.masks.is_empty() => {
            let mut out = message.to_string();
            for m in &s.masks {
                if out.contains(m.as_str()) {
                    out = out.replace(m.as_str(), MASK);
                }
            }
            out
        }
        _ => message.to_string(),
    }
}

/// A secret already fetched during this deployment.
pub fn cached(deployment_id: &str, connection_id: &str, path: &str) -> Option<SecretData> {
    SESSIONS.get(deployment_id).and_then(|s| {
        s.cache
            .get(&(connection_id.to_string(), path.to_string()))
            .cloned()
    })
}

/// Remember a fetched secret for the rest of this deployment.
pub fn store(deployment_id: &str, connection_id: &str, path: &str, data: &SecretData) {
    prune();
    let mut session = SESSIONS.entry(deployment_id.to_string()).or_default();
    session.touched = Some(Instant::now());
    session
        .cache
        .insert((connection_id.to_string(), path.to_string()), data.clone());
}

/// Forget everything about a deployment.
pub fn clear(deployment_id: &str) {
    SESSIONS.remove(deployment_id);
}

/// Clears the deployment's session when dropped (end of the deployment task).
pub struct SessionGuard(String);

impl SessionGuard {
    pub fn new(deployment_id: &str) -> Self {
        Self(deployment_id.to_string())
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        clear(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_registered_values_longest_first() {
        let id = "dep-mask-1";
        register_values(id, ["s3cr3t-token", "s3cr3t", "no"]);
        assert_eq!(
            redact(id, "token=s3cr3t-token pw=s3cr3t x=no"),
            "token=******** pw=******** x=no"
        );
        assert_eq!(redact("other-dep", "s3cr3t"), "s3cr3t");
        clear(id);
        assert_eq!(redact(id, "s3cr3t"), "s3cr3t");
    }

    #[test]
    fn masks_lines_of_multiline_values() {
        let id = "dep-mask-2";
        register_values(
            id,
            ["-----BEGIN KEY-----\nMIIBOgIBAAJBAKj34GkxFhD9\n-----END KEY-----"],
        );
        assert_eq!(
            redact(id, "line: MIIBOgIBAAJBAKj34GkxFhD9"),
            "line: ********"
        );
        clear(id);
    }

    #[test]
    fn guard_clears_cache_and_masks() {
        let id = "dep-guard";
        {
            let _g = SessionGuard::new(id);
            store(id, "c1", "p", &SecretData::default());
            register_values(id, ["abcdef"]);
            assert!(cached(id, "c1", "p").is_some());
        }
        assert!(cached(id, "c1", "p").is_none());
        assert_eq!(redact(id, "abcdef"), "abcdef");
    }
}
