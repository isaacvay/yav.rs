//! Real-Time Collaborative CRDTs, Multiplayer Presence & Fractional Indexing (`yav::collab`).
//!
//! Provides the core conflict-free data structures required for Notion/Linear/Figma-class apps:
//! - [`LwwRegister<T>`] & [`LwwMap<K, V>`]: Commutative, associative, idempotent Last-Write-Wins CRDTs with Lamport clocks.
//! - [`PresenceRoom`] & [`CollaboratorPresence`]: Multiplayer cursor, selection, and status tracking (Liveblocks equivalent).
//! - [`LexoRank`]: Fractional indexing (`O(1)` drag-and-drop reordering between any two items without reindexing the table).

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

// ─────────────────────────────────────────────────────────────────────────────
// 1. LwwRegister & LwwMap (Conflict-Free Replicated Data Types)
// ─────────────────────────────────────────────────────────────────────────────

/// Logical Lamport / Hybrid timestamp paired with a replica identifier for deterministic tie-breaking.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrdtClock {
    pub timestamp: u64,
    pub replica_id: String,
}

impl CrdtClock {
    pub fn new(timestamp: u64, replica_id: impl Into<String>) -> Self {
        Self {
            timestamp,
            replica_id: replica_id.into(),
        }
    }
}

/// Last-Write-Wins Register CRDT (`LwwRegister<T>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LwwRegister<T> {
    value: T,
    clock: CrdtClock,
}

impl<T: Clone> LwwRegister<T> {
    /// Create a new register with an initial value and clock.
    pub fn new(value: T, timestamp: u64, replica_id: impl Into<String>) -> Self {
        Self {
            value,
            clock: CrdtClock::new(timestamp, replica_id),
        }
    }

    /// Read the current value.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Return the current [`CrdtClock`].
    pub fn clock(&self) -> &CrdtClock {
        &self.clock
    }

    /// Update the value if the new clock is strictly greater than the current clock.
    pub fn set(&mut self, value: T, timestamp: u64, replica_id: impl Into<String>) -> bool {
        let candidate = CrdtClock::new(timestamp, replica_id);
        if candidate > self.clock {
            self.value = value;
            self.clock = candidate;
            true
        } else {
            false
        }
    }

    /// Merge a remote register state into `self` (commutative, associative, idempotent).
    pub fn merge(&mut self, remote: &Self) {
        if remote.clock > self.clock {
            self.value = remote.value.clone();
            self.clock = remote.clock.clone();
        }
    }
}

/// Conflict-Free Replicated Map (`LwwMap<K, V>`) supporting concurrent edits and tombstoned deletes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LwwMap<K: Ord, V> {
    entries: BTreeMap<K, LwwRegister<Option<V>>>,
}

impl<K: Ord + Clone, V: Clone> LwwMap<K, V> {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Set a key-value pair at `(timestamp, replica_id)`.
    pub fn insert(&mut self, key: K, value: V, timestamp: u64, replica_id: impl Into<String>) {
        let rep = replica_id.into();
        match self.entries.get_mut(&key) {
            Some(reg) => {
                reg.set(Some(value), timestamp, rep);
            }
            None => {
                self.entries
                    .insert(key, LwwRegister::new(Some(value), timestamp, rep));
            }
        }
    }

    /// Remove a key using a CRDT tombstone at `(timestamp, replica_id)`.
    pub fn remove(&mut self, key: K, timestamp: u64, replica_id: impl Into<String>) {
        let rep = replica_id.into();
        match self.entries.get_mut(&key) {
            Some(reg) => {
                reg.set(None, timestamp, rep);
            }
            None => {
                self.entries
                    .insert(key, LwwRegister::new(None, timestamp, rep));
            }
        }
    }

    /// Get the active value for `key` (`None` if missing or tombstoned).
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key).and_then(|reg| reg.get().as_ref())
    }

    /// Merge another `LwwMap` into `self` without conflict.
    pub fn merge(&mut self, other: &Self) {
        for (k, remote_reg) in &other.entries {
            match self.entries.get_mut(k) {
                Some(local_reg) => local_reg.merge(remote_reg),
                None => {
                    self.entries.insert(k.clone(), remote_reg.clone());
                }
            }
        }
    }

    /// Return all currently active `(key, value)` pairs.
    pub fn active_entries(&self) -> Vec<(&K, &V)> {
        self.entries
            .iter()
            .filter_map(|(k, reg)| reg.get().as_ref().map(|v| (k, v)))
            .collect()
    }

    /// Purge all tombstones (`None` entries) to reclaim memory.
    pub fn purge_tombstones(&mut self) {
        self.entries.retain(|_, reg| reg.get().is_some());
    }

    /// Prune tombstones (`None` entries) whose deletion timestamp is strictly older than `before_timestamp`.
    /// Returns the number of reclaimed tombstone entries.
    pub fn prune_tombstones(&mut self, before_timestamp: u64) -> usize {
        let initial_len = self.entries.len();
        self.entries
            .retain(|_, reg| !(reg.get().is_none() && reg.clock().timestamp < before_timestamp));
        initial_len - self.entries.len()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Multiplayer PresenceRoom (Liveblocks Equivalent)
// ─────────────────────────────────────────────────────────────────────────────

/// Live cursor, selection, and status state for a connected collaborator.
#[derive(Debug, Clone, PartialEq)]
pub struct CollaboratorPresence {
    pub user_id: String,
    pub name: String,
    pub color: String,
    pub cursor_x: Option<f64>,
    pub cursor_y: Option<f64>,
    pub focused_element_id: Option<String>,
    pub typing: bool,
}

/// Thread-safe multiplayer presence room tracking active collaborators.
#[derive(Clone, Default)]
pub struct PresenceRoom {
    participants: Arc<RwLock<BTreeMap<String, CollaboratorPresence>>>,
}

impl PresenceRoom {
    pub fn new() -> Self {
        Self::default()
    }

    /// Join or update a collaborator's live presence state.
    pub fn upsert(&self, presence: CollaboratorPresence) {
        self.participants
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(presence.user_id.clone(), presence);
    }

    /// Remove a collaborator when they disconnect.
    pub fn leave(&self, user_id: &str) -> bool {
        self.participants
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(user_id)
            .is_some()
    }

    /// Return a snapshot of all currently connected collaborators in the room.
    pub fn list(&self) -> Vec<CollaboratorPresence> {
        self.participants
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. LexoRank Fractional Indexing (Linear / Jira O(1) Drag-and-Drop Ordering)
// ─────────────────────────────────────────────────────────────────────────────

const ALPHABET: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

fn char_to_rank(b: u8) -> usize {
    ALPHABET.iter().position(|&c| c == b).unwrap_or(0)
}

/// Lexicographical fractional indexing utility (`LexoRank`) for $O(1)$ drag-and-drop list/Kanban ordering.
pub struct LexoRank;

impl LexoRank {
    /// Generate an evenly spaced initial rank key for item `index` (e.g. `0 -> "4"`, `1 -> "8"`, ...).
    pub fn initial(index: usize) -> String {
        let step = (index + 1) * 4;
        if step < ALPHABET.len() - 2 {
            (ALPHABET[step] as char).to_string()
        } else {
            format!("z{:06}", index)
        }
    }

    /// Compute a lexicographical midpoint string strictly between `prev` and `next`:
    /// `prev < between(Some(prev), Some(next)) < next`.
    pub fn between(prev: Option<&str>, next: Option<&str>) -> String {
        match (prev, next) {
            (None, None) => "U".to_string(),
            (None, Some(n)) => {
                let bytes = n.as_bytes();
                let mut leading_zeros = 0;
                while leading_zeros < bytes.len() && bytes[leading_zeros] == b'0' {
                    leading_zeros += 1;
                }
                if leading_zeros < bytes.len() {
                    let first = bytes[leading_zeros];
                    let idx = char_to_rank(first);
                    if idx > 1 {
                        let mut res = "0".repeat(leading_zeros);
                        res.push(ALPHABET[idx / 2] as char);
                        res
                    } else {
                        let mut res = "0".repeat(leading_zeros);
                        res.push('0');
                        res.push('U');
                        res
                    }
                } else {
                    // `n` consists solely of '0' characters (the minimum alphabet char).
                    // No strictly smaller non-empty string exists within ALPHABET.
                    "0".to_string()
                }
            }
            (Some(p), None) => {
                let first = p.as_bytes().first().copied().unwrap_or(b'U');
                let idx = char_to_rank(first);
                if idx + 2 < ALPHABET.len() && p.len() <= 1 {
                    (ALPHABET[(idx + ALPHABET.len()) / 2] as char).to_string()
                } else {
                    format!("{p}U")
                }
            }
            (Some(p), Some(n)) => {
                if p >= n {
                    return format!("{p}U");
                }
                let p_bytes = p.as_bytes();
                let n_bytes = n.as_bytes();
                let mut i = 0;
                let mut prefix = String::new();
                while i < p_bytes.len() && i < n_bytes.len() && p_bytes[i] == n_bytes[i] {
                    prefix.push(p_bytes[i] as char);
                    i += 1;
                }

                // Case 1: `p` is a proper prefix of `n` (e.g. "a" vs "a04" or "a4")
                if i == p_bytes.len() {
                    let mut j = i;
                    while j < n_bytes.len() && n_bytes[j] == b'0' {
                        prefix.push('0');
                        j += 1;
                    }
                    if j < n_bytes.len() {
                        let ni = char_to_rank(n_bytes[j]);
                        if ni > 1 {
                            prefix.push(ALPHABET[ni / 2] as char);
                        } else {
                            prefix.push('0');
                            prefix.push('U');
                        }
                        return prefix;
                    }
                    // Adjacent strings ("a" and "a0...0"): append '0' beyond n to mark minimum boundary fallback
                    return format!("{n}0");
                }

                // Case 2: Diverge at index `i` with `p_bytes[i] < n_bytes[i]`
                let pi = char_to_rank(p_bytes[i]);
                let ni = char_to_rank(n_bytes[i]);
                if ni > pi + 1 {
                    prefix.push(ALPHABET[(pi + ni) / 2] as char);
                    return prefix;
                }

                // Adjacent digits at `i`: copy `p_bytes[i]` and advance `p`'s suffix upward
                prefix.push(p_bytes[i] as char);
                let mut found = false;
                for &rb in &p_bytes[i + 1..] {
                    let ri = char_to_rank(rb);
                    if ri + 1 < ALPHABET.len() {
                        let mid = (ri + ALPHABET.len()) / 2;
                        prefix.push(ALPHABET[mid] as char);
                        found = true;
                        break;
                    }
                    prefix.push(rb as char);
                }
                if !found {
                    prefix.push('U');
                }
                prefix
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crdt_lww_map_commutativity_and_idempotence() {
        let mut replica_a = LwwMap::new();
        let mut replica_b = LwwMap::new();

        replica_a.insert("title", "Draft 1", 10, "node_a");
        replica_b.insert("title", "Draft 2 (Newer)", 20, "node_b");
        replica_b.insert("author", "Alice", 15, "node_b");

        let mut merged_ab = replica_a.clone();
        merged_ab.merge(&replica_b);

        let mut merged_ba = replica_b.clone();
        merged_ba.merge(&replica_a);

        // Commutative & associative convergence
        assert_eq!(merged_ab, merged_ba);
        assert_eq!(merged_ab.get(&"title"), Some(&"Draft 2 (Newer)"));
        assert_eq!(merged_ab.get(&"author"), Some(&"Alice"));
    }

    #[test]
    fn test_lexorank_strict_ordering() {
        let r1 = LexoRank::initial(0);
        let r2 = LexoRank::initial(1);
        assert!(r1 < r2);

        let mid = LexoRank::between(Some(&r1), Some(&r2));
        assert!(r1 < mid && mid < r2);

        let mid2 = LexoRank::between(Some(&r1), Some(&mid));
        assert!(r1 < mid2 && mid2 < mid);
    }

    #[test]
    fn test_lexorank_large_indices_and_between() {
        for i in 0..50 {
            let cur = LexoRank::initial(i);
            let nxt = LexoRank::initial(i + 1);
            assert!(cur < nxt, "failed at index {i}: {cur} vs {nxt}");
        }

        let p = "BU";
        let n = "C";
        let mid = LexoRank::between(Some(p), Some(n));
        assert!(p < mid.as_str() && mid.as_str() < n, "{p} < {mid} < {n}");

        let p2 = "BZ";
        let mid2 = LexoRank::between(Some(p2), Some(n));
        assert!(
            p2 < mid2.as_str() && mid2.as_str() < n,
            "{p2} < {mid2} < {n}"
        );
    }

    #[test]
    fn test_lww_map_purge_tombstones() {
        let mut map = LwwMap::new();
        map.insert("k1", "v1", 1, "node");
        map.insert("k2", "v2", 1, "node");
        map.remove("k1", 2, "node");
        assert_eq!(map.get(&"k1"), None);
        assert_eq!(map.get(&"k2"), Some(&"v2"));
        map.purge_tombstones();
        assert_eq!(map.get(&"k1"), None);
        assert_eq!(map.get(&"k2"), Some(&"v2"));
    }

    #[test]
    fn test_lww_map_prune_tombstones() {
        let mut map = LwwMap::new();
        map.insert("k1", "v1", 10, "node");
        map.insert("k2", "v2", 20, "node");
        map.remove("k1", 15, "node"); // deleted at t=15
        map.remove("k2", 25, "node"); // deleted at t=25

        // Pruning before t=20 should only prune k1
        let reclaimed = map.prune_tombstones(20);
        assert_eq!(reclaimed, 1);
        assert_eq!(map.get(&"k1"), None);
        assert_eq!(map.get(&"k2"), None);

        // Pruning before t=30 should prune k2
        let reclaimed2 = map.prune_tombstones(30);
        assert_eq!(reclaimed2, 1);
    }
}
