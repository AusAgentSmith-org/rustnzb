//! Per-article/file/job list of servers that have already been tried.
//!
//! Every [`Article`](super::article::Article),
//! [`NzbFile`](super::article::NzbFile), and
//! [`NzbObject`](super::article::NzbObject) has one. The rules:
//!
//! - An article is tried on each server **at most once**. Before dispatching,
//!   the caller checks [`TryList::contains`] for the candidate server; after
//!   a fetch failure, the server is recorded via [`TryList::add`].
//! - The file- and job-level try-lists remember which servers have been
//!   **exhausted across all articles in the file/job**. Once every article
//!   in a file has failed on server S, S is added to the file's try-list —
//!   future articles from that file skip S entirely.
//! - [`TryList::reset`] clears the list. Called when a cascade decides to
//!   give a partially-failed file or job another chance.
//!
//! Each object owns its own mutex (fine-grained locking), so high-parallelism
//! dispatch contends only on the specific article/file/job being updated.

use std::collections::HashSet;
use std::sync::Mutex;

/// A set of server IDs that have already been tried for this article/file/job.
///
/// Cloneable-by-inner-mutability: `Clone` produces a new `TryList` with a
/// **snapshot** of the current set (not a shared reference). If you want
/// shared mutable state, wrap the `TryList` in `Arc`.
#[derive(Debug, Default)]
pub struct TryList {
    servers: Mutex<HashSet<String>>,
}

impl TryList {
    /// Construct an empty try-list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Is this server already in the try-list?
    pub fn contains(&self, server_id: &str) -> bool {
        self.servers
            .lock()
            .expect("TryList mutex poisoned")
            .contains(server_id)
    }

    /// Record that `server_id` has been tried. Returns `true` if the server
    /// was newly added, `false` if it was already present.
    pub fn add(&self, server_id: &str) -> bool {
        self.servers
            .lock()
            .expect("TryList mutex poisoned")
            .insert(server_id.to_string())
    }

    /// Clear the list — every server is now a candidate again. Called when a
    /// higher-priority server comes back online and we want to give it
    /// another shot, or when a higher-level cascade resets this list.
    pub fn reset(&self) {
        self.servers.lock().expect("TryList mutex poisoned").clear();
    }

    /// Remove a single server from the try-list. Used when that server's
    /// circuit breaker has cleared and we want to re-include it without
    /// wiping the other entries.
    pub fn remove(&self, server_id: &str) -> bool {
        self.servers
            .lock()
            .expect("TryList mutex poisoned")
            .remove(server_id)
    }

    /// Snapshot of the current set. Copy: uses the lock briefly, then hands
    /// back an owned `HashSet`. For use in logging/telemetry.
    pub fn snapshot(&self) -> HashSet<String> {
        self.servers.lock().expect("TryList mutex poisoned").clone()
    }

    /// Number of servers currently in the list.
    pub fn len(&self) -> usize {
        self.servers.lock().expect("TryList mutex poisoned").len()
    }

    /// Is the list empty?
    pub fn is_empty(&self) -> bool {
        self.servers
            .lock()
            .expect("TryList mutex poisoned")
            .is_empty()
    }
}

impl Clone for TryList {
    /// Clone takes a snapshot of the inner set — the two `TryList`s are
    /// independent after cloning. If you want shared state, wrap in `Arc`.
    fn clone(&self) -> Self {
        let snapshot = self.snapshot();
        Self {
            servers: Mutex::new(snapshot),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_on_construction() {
        let tl = TryList::new();
        assert!(tl.is_empty());
        assert_eq!(tl.len(), 0);
        assert!(!tl.contains("any"));
    }

    #[test]
    fn add_returns_true_once() {
        let tl = TryList::new();
        assert!(tl.add("s1"));
        assert!(!tl.add("s1")); // duplicate → false
        assert!(tl.contains("s1"));
        assert_eq!(tl.len(), 1);
    }

    #[test]
    fn reset_clears() {
        let tl = TryList::new();
        tl.add("s1");
        tl.add("s2");
        assert_eq!(tl.len(), 2);
        tl.reset();
        assert!(tl.is_empty());
        assert!(!tl.contains("s1"));
    }

    #[test]
    fn remove_single() {
        let tl = TryList::new();
        tl.add("s1");
        tl.add("s2");
        assert!(tl.remove("s1"));
        assert!(!tl.contains("s1"));
        assert!(tl.contains("s2"));
        assert!(!tl.remove("s1"));
    }

    #[test]
    fn clone_is_snapshot_not_shared() {
        let tl = TryList::new();
        tl.add("s1");
        let clone = tl.clone();
        clone.add("s2");
        assert!(!tl.contains("s2"), "clone should be independent");
        assert!(clone.contains("s1"));
    }
}
