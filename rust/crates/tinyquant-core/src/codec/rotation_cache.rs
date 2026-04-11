//! Process-wide LRU cache for rotation matrices, keyed by `(seed, dim)`.
//!
//! Mirrors Python's `functools.lru_cache(maxsize=8)` on
//! `RotationMatrix._cached_build`. The cache stores up to `capacity`
//! entries and evicts the least-recently-used on insertion. Hits also
//! promote the touched entry to the front of the queue.
//!
//! The implementation is intentionally minimal — it uses a
//! `spin::Mutex<Vec<RotationMatrix>>` rather than a proper hash map
//! because the expected capacity is tiny (default 8) and the linear
//! scan fits in a single cache line at that size.

use alloc::vec::Vec;

use spin::Mutex;

use crate::codec::rotation_matrix::RotationMatrix;

/// Default cache capacity, matching Python's `functools.lru_cache(maxsize=8)`.
pub const DEFAULT_CAPACITY: usize = 8;

/// A small `(seed, dim)`-keyed LRU cache over [`RotationMatrix`] values.
pub struct RotationCache {
    entries: Mutex<Vec<RotationMatrix>>,
    capacity: usize,
}

impl RotationCache {
    /// Construct a new cache with the given capacity.
    ///
    /// A capacity of `0` is legal but makes every lookup a miss.
    #[must_use]
    pub const fn new(capacity: usize) -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            capacity,
        }
    }

    /// Construct the default-sized cache (capacity [`DEFAULT_CAPACITY`]).
    #[must_use]
    pub const fn default_sized() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }

    /// The maximum number of entries this cache will retain.
    #[inline]
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// The number of entries currently held by the cache.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.lock().len()
    }

    /// `true` iff the cache currently holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.lock().is_empty()
    }

    /// Return the rotation matrix for `(seed, dimension)`, building and
    /// caching one on miss. Returned values are `Arc`-backed clones, so
    /// the caller never blocks subsequent users of the cache.
    pub fn get_or_build(&self, seed: u64, dimension: u32) -> RotationMatrix {
        if self.capacity == 0 {
            return RotationMatrix::build(seed, dimension);
        }
        let mut guard = self.entries.lock();
        if let Some(pos) = guard
            .iter()
            .position(|m| m.seed() == seed && m.dimension() == dimension)
        {
            let touched = guard.remove(pos);
            let clone = touched.clone();
            guard.insert(0, touched);
            return clone;
        }
        // Miss: build, evict if necessary, insert at MRU position.
        let built = RotationMatrix::build(seed, dimension);
        if guard.len() >= self.capacity {
            guard.pop();
        }
        guard.insert(0, built.clone());
        built
    }

    /// Drop all cached entries.
    pub fn clear(&self) {
        self.entries.lock().clear();
    }
}

impl Default for RotationCache {
    fn default() -> Self {
        Self::default_sized()
    }
}

impl core::fmt::Debug for RotationCache {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // `entries` is intentionally reduced to its length — printing the
        // full `RotationMatrix` payload would dump megabytes of f64 data.
        f.debug_struct("RotationCache")
            .field("capacity", &self.capacity)
            .field("entries", &self.len())
            .finish()
    }
}
