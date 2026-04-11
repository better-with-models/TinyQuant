//! `OwnedStore` — insertion-order-preserving map from `VectorId` to `Arc<[f32]>`.
//!
//! Wraps `IndexMap` so that `BruteForceBackend` can remove an id and still
//! iterate the survivors in deterministic insertion order.

use std::sync::Arc;

use indexmap::IndexMap;
use tinyquant_core::types::VectorId;

/// Insertion-order-preserving store of vectors.
///
/// Each entry is keyed by its `VectorId` and the value is an atomically
/// reference-counted slice so that `clone` is O(1).
pub struct OwnedStore(IndexMap<VectorId, Arc<[f32]>>);

impl OwnedStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    /// Insert or overwrite a vector.
    pub fn insert(&mut self, id: VectorId, vec: Arc<[f32]>) {
        self.0.insert(id, vec);
    }

    /// Return a reference to the stored slice, or `None`.
    #[allow(dead_code)]
    pub fn get(&self, id: &str) -> Option<&Arc<[f32]>> {
        self.0.get(id)
    }

    /// Remove a vector by id.  Silent no-op if the id is unknown.
    pub fn remove(&mut self, id: &str) {
        self.0.swap_remove(id);
    }

    /// Iterate over `(VectorId, Arc<[f32]>)` pairs in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&VectorId, &Arc<[f32]>)> {
        self.0.iter()
    }

    /// Number of stored vectors.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// `true` if the store contains no vectors.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// `true` if the store contains a vector with this id.
    #[allow(dead_code)]
    pub fn contains_key(&self, id: &str) -> bool {
        self.0.contains_key(id)
    }
}
