//! In-process coordination for backup and restore work.
//!
//! - Resource locks stop two backup/restore jobs from touching the same database or
//!   volume at once.
//! - Paused containers are skipped by the container monitor while a restore (or a
//!   backup in `stop` snapshot mode) has stopped them on purpose, so the monitor does
//!   not restart them halfway through.

use std::collections::HashSet;
use std::sync::OnceLock;

use parking_lot::Mutex;

fn locks() -> &'static Mutex<HashSet<String>> {
    static LOCKS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    LOCKS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn paused() -> &'static Mutex<HashSet<String>> {
    static PAUSED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    PAUSED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Held while a job works on a resource; releases the lock on drop.
#[derive(Debug)]
pub struct ResourceLock {
    key: String,
}

impl Drop for ResourceLock {
    fn drop(&mut self) {
        locks().lock().remove(&self.key);
    }
}

/// Lock key for a backup resource, e.g. `volume:<id>`.
pub fn resource_key(kind: &str, id: &str) -> String {
    format!("{kind}:{id}")
}

/// Take the lock for `key`, or `None` when another job holds it.
pub fn try_lock(key: &str) -> Option<ResourceLock> {
    if locks().lock().insert(key.to_string()) {
        Some(ResourceLock {
            key: key.to_string(),
        })
    } else {
        None
    }
}

/// True when another job currently holds the lock for `key`.
pub fn is_locked(key: &str) -> bool {
    locks().lock().contains(key)
}

/// Keeps containers out of the container monitor's restart logic; resumes on drop.
#[derive(Debug)]
pub struct PausedContainers {
    ids: Vec<String>,
}

impl Drop for PausedContainers {
    fn drop(&mut self) {
        let mut set = paused().lock();
        for id in &self.ids {
            set.remove(id);
        }
    }
}

/// Mark containers as intentionally stopped until the returned guard is dropped.
pub fn pause_containers(ids: &[String]) -> PausedContainers {
    let mut set = paused().lock();
    for id in ids {
        set.insert(id.clone());
    }
    PausedContainers { ids: ids.to_vec() }
}

/// Called by the container monitor: skip containers a backup job stopped on purpose.
pub fn is_container_paused(container_id: &str) -> bool {
    paused().lock().contains(container_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_is_exclusive_and_released_on_drop() {
        let key = resource_key("volume", "lock-test-1");
        let guard = try_lock(&key).expect("first lock");
        assert!(is_locked(&key));
        assert!(try_lock(&key).is_none());
        drop(guard);
        assert!(!is_locked(&key));
        assert!(try_lock(&key).is_some());
    }

    #[test]
    fn paused_containers_resume_on_drop() {
        let ids = vec!["pause-test-a".to_string(), "pause-test-b".to_string()];
        let guard = pause_containers(&ids);
        assert!(is_container_paused("pause-test-a"));
        assert!(is_container_paused("pause-test-b"));
        drop(guard);
        assert!(!is_container_paused("pause-test-a"));
    }
}
