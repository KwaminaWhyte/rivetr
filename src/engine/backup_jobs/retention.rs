//! Retention pruning: which backups fall outside "keep the newest N".

/// A backup that retention can consider.
pub trait Retained {
    /// Sortable creation timestamp (RFC 3339 or SQLite datetime; both sort as text).
    fn created_at(&self) -> &str;
    /// Stable tie-breaker for backups created in the same second.
    fn id(&self) -> &str;
}

/// Backups to delete so only the newest `keep` remain. `keep == 0` keeps everything
/// (a retention of zero is treated as "unlimited", never as "delete all").
pub fn select_for_pruning<T: Retained>(items: &[T], keep: usize) -> Vec<&T> {
    if keep == 0 || items.len() <= keep {
        return Vec::new();
    }
    let mut sorted: Vec<&T> = items.iter().collect();
    sorted.sort_by(|a, b| {
        b.created_at()
            .cmp(a.created_at())
            .then_with(|| b.id().cmp(a.id()))
    });
    sorted.split_off(keep)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct B(&'static str, &'static str);
    impl Retained for B {
        fn created_at(&self) -> &str {
            self.1
        }
        fn id(&self) -> &str {
            self.0
        }
    }

    #[test]
    fn keeps_newest_n_regardless_of_input_order() {
        let items = vec![
            B("a", "2026-11-01T02:00:00Z"),
            B("c", "2026-11-03T02:00:00Z"),
            B("b", "2026-11-02T02:00:00Z"),
            B("d", "2026-11-04T02:00:00Z"),
        ];
        let pruned: Vec<&str> = select_for_pruning(&items, 2).iter().map(|b| b.0).collect();
        assert_eq!(pruned, vec!["b", "a"]);
    }

    #[test]
    fn nothing_pruned_when_within_limit_or_unlimited() {
        let items = vec![B("a", "2026-11-01"), B("b", "2026-11-02")];
        assert!(select_for_pruning(&items, 2).is_empty());
        assert!(select_for_pruning(&items, 5).is_empty());
        assert!(select_for_pruning(&items, 0).is_empty());
        let empty: Vec<B> = Vec::new();
        assert!(select_for_pruning(&empty, 1).is_empty());
    }

    #[test]
    fn same_timestamp_uses_id_as_tie_breaker() {
        let items = vec![
            B("x1", "2026-11-01"),
            B("x2", "2026-11-01"),
            B("x3", "2026-11-01"),
        ];
        let pruned: Vec<&str> = select_for_pruning(&items, 1).iter().map(|b| b.0).collect();
        assert_eq!(pruned, vec!["x2", "x1"]);
    }
}
