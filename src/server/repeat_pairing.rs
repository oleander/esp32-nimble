//! Repeat-pairing ordering, kept independent of ESP-IDF for host tests.

/// Ask the owner before deleting a bond. Retry only once deletion succeeded;
/// retrying with the old bond intact would re-enter the same GAP event forever.
pub(super) fn retry<T>(
    peer: &T,
    allow: Option<&(dyn Fn(&T) -> bool + Send + Sync)>,
    delete: impl FnOnce() -> bool,
) -> bool {
    if allow.is_some_and(|allow| !allow(peer)) {
        return false;
    }
    delete()
}

#[cfg(test)]
mod tests {
    use super::retry;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn veto_keeps_the_existing_bond_without_calling_delete() {
        let mut bond = Some(42);
        let allow = |peer: &u16| {
            assert_eq!(*peer, 7);
            false
        };
        assert!(!retry(&7, Some(&allow), || {
            bond = None;
            true
        }));
        assert_eq!(bond, Some(42));
    }

    #[test]
    fn permission_is_checked_before_deletion() {
        let checked = AtomicBool::new(false);
        let allow = |_: &u16| {
            checked.store(true, Ordering::Relaxed);
            true
        };
        let mut deleted = 0;
        assert!(retry(&7, Some(&allow), || {
            assert!(checked.load(Ordering::Relaxed));
            deleted += 1;
            true
        }));
        assert_eq!(deleted, 1);
    }

    #[test]
    fn no_callback_preserves_default_pairing_recovery() {
        let mut deleted = false;
        assert!(retry(&7, None, || {
            deleted = true;
            true
        }));
        assert!(deleted);
    }

    #[test]
    fn deletion_failure_does_not_retry_with_the_old_bond() {
        assert!(!retry(&7, None, || false));
    }
}
