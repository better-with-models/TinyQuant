//! Insertion-ordered map used in `no_std + alloc` builds.
//!
//! `VectorIdMap<V>` is a purpose-built ordered-map that preserves insertion
//! order on iteration while providing O(log n) lookups via a secondary sorted
//! index.  It is used by [`Corpus`](super::Corpus) for all builds in Phase 18;
//! `IndexMap` integration is deferred to Phase 21+.
//!
//! # Complexity
//!
//! | Operation | Time |
//! |---|---|
//! | `insert` | O(n) — sorted-index insertion |
//! | `get` | O(log n) — binary search |
//! | `contains_key` | O(log n) |
//! | `remove` | O(n) — linear scan + vec compaction |
//! | `iter` | O(n) — walks `entries` in insertion order |
//! | `len` / `is_empty` | O(1) |
//!
//! For the target footprint (embedded, < 10 k entries) this is acceptable.

use crate::types::VectorId;
use alloc::vec::Vec;

/// Insertion-ordered map from [`VectorId`] to `V`.
///
/// Provides [`iter`](Self::iter) in insertion order and O(log n) lookup.
///
/// This type is `pub` but lives in a `pub(crate)` module, so it is
/// effectively crate-private.
pub struct VectorIdMap<V> {
    /// Parallel vector storing `(key, value)` pairs in insertion order.
    entries: Vec<(VectorId, V)>,
    /// Sorted indices into `entries` (sorted by the `VectorId` at that position).
    /// Invariant: `sorted_index.len() == entries.len()`.
    sorted_index: Vec<usize>,
}

impl<V> VectorIdMap<V> {
    /// Create an empty map.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            sorted_index: Vec::new(),
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the map contains no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Binary-search the sorted index for `key`.
    ///
    /// Returns `Ok(pos)` when found (position in `sorted_index`),
    /// `Err(pos)` with the insertion point when not found.
    fn sorted_search(&self, key: &str) -> Result<usize, usize> {
        binary_search_by(&self.sorted_index, |&idx| {
            // `idx` is always a valid index: `sorted_index` is only populated
            // via `insert`, which pushes to `entries` first.  The entry at `idx`
            // is therefore guaranteed to exist.
            let entry_key: &str = self.entries.get(idx).map_or("", |(k, _)| k.as_ref());
            entry_key.cmp(key)
        })
    }

    /// Returns `true` if `key` is present.
    pub fn contains_key(&self, key: &str) -> bool {
        self.sorted_search(key).is_ok()
    }

    /// Look up a key by reference; returns `None` if absent.
    pub fn get(&self, key: &str) -> Option<&V> {
        self.sorted_search(key).ok().and_then(|sorted_pos| {
            self.sorted_index
                .get(sorted_pos)
                .and_then(|&entry_idx| self.entries.get(entry_idx).map(|(_, v)| v))
        })
    }

    /// Insert a key-value pair at the end (insertion order).
    ///
    /// Returns `false` if the key already exists (no-op in that case).
    pub fn insert(&mut self, key: VectorId, value: V) -> bool {
        match self.sorted_search(&key) {
            Ok(_) => false, // key exists
            Err(sorted_pos) => {
                let entry_idx = self.entries.len();
                self.entries.push((key, value));
                self.sorted_index.insert(sorted_pos, entry_idx);
                true
            }
        }
    }

    /// Remove a key from the map.
    ///
    /// Returns the removed value, or `None` if the key was absent.
    /// O(n): rebuilds `sorted_index` after compaction.
    pub fn remove(&mut self, key: &str) -> Option<V> {
        match self.sorted_search(key) {
            Err(_) => None,
            Ok(sorted_pos) => {
                let entry_idx = self.sorted_index.remove(sorted_pos);
                let (_k, v) = self.entries.remove(entry_idx);
                // Rebuild sorted_index: all indices > entry_idx are shifted by -1.
                for si in &mut self.sorted_index {
                    if *si > entry_idx {
                        *si -= 1;
                    }
                }
                Some(v)
            }
        }
    }

    /// Iterate in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&VectorId, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
}

/// `std`-independent binary search that returns `Ok(pos)` on match or `Err(pos)` on miss.
///
/// Equivalent to `slice::binary_search_by` but works on any `Vec<T>`.
fn binary_search_by<T, F>(slice: &[T], mut pred: F) -> Result<usize, usize>
where
    F: FnMut(&T) -> core::cmp::Ordering,
{
    use core::cmp::Ordering;
    let mut lo = 0usize;
    let mut hi = slice.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match slice.get(mid).map_or(Ordering::Greater, &mut pred) {
            Ordering::Less => lo = mid + 1,
            Ordering::Greater => hi = mid,
            Ordering::Equal => return Ok(mid),
        }
    }
    Err(lo)
}
