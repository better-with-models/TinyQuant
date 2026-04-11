//! `BruteForceBackend` — linear-scan `SearchBackend` implementation.
//!
//! Scans all stored vectors at query time. `O(n·d)` per query, no index
//! structure. Suitable for corpora up to ~100 k vectors.

use std::sync::Arc;

use tinyquant_core::backend::{SearchBackend, SearchResult};
use tinyquant_core::types::VectorId;

use crate::errors::BackendError;
use crate::similarity::cosine_similarity;
use crate::store::OwnedStore;

/// A brute-force search backend that computes cosine similarity against every
/// stored vector on each query.
///
/// # Dimension locking
///
/// The first non-empty call to [`ingest`](BruteForceBackend::ingest) locks the
/// dimension.  Subsequent calls with a different dimension return
/// `Err(BackendError::Adapter("dimension mismatch: expected {d}, got {got}"))`.
///
/// # Python parity
///
/// - Overwriting an existing id is silent (matches Python `dict[id] = vec`).
/// - `remove` with unknown ids is a no-op (matches Python `dict.pop(id, None)`).
pub struct BruteForceBackend {
    dim: Option<usize>,
    store: OwnedStore,
}

impl BruteForceBackend {
    /// Create a new, empty backend.
    pub fn new() -> Self {
        Self {
            dim: None,
            store: OwnedStore::new(),
        }
    }
}

impl Default for BruteForceBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchBackend for BruteForceBackend {
    fn ingest(&mut self, vectors: &[(VectorId, Vec<f32>)]) -> Result<(), BackendError> {
        if vectors.is_empty() {
            return Ok(());
        }

        for (id, vec) in vectors {
            // Dimension lock: set on first vector, reject mismatches.
            match self.dim {
                None => {
                    self.dim = Some(vec.len());
                }
                Some(expected) if vec.len() != expected => {
                    return Err(BackendError::Adapter(Arc::from(format!(
                        "dimension mismatch: expected {expected}, got {}",
                        vec.len()
                    ))));
                }
                Some(_) => {}
            }
            let arc_slice: Arc<[f32]> = Arc::from(vec.as_slice());
            self.store.insert(Arc::clone(id), arc_slice);
        }
        Ok(())
    }

    fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<SearchResult>, BackendError> {
        if top_k == 0 {
            return Err(BackendError::InvalidTopK);
        }
        if self.store.is_empty() {
            return Ok(Vec::new());
        }
        // Dimension check: if locked, query must match.
        if let Some(expected) = self.dim {
            if query.len() != expected {
                return Err(BackendError::Adapter(Arc::from(format!(
                    "dimension mismatch: expected {expected}, got {}",
                    query.len()
                ))));
            }
        }

        let mut results: Vec<SearchResult> = self
            .store
            .iter()
            .map(|(id, vec)| {
                let score = cosine_similarity(query, vec);
                SearchResult {
                    vector_id: Arc::clone(id),
                    score,
                }
            })
            .collect();

        // Sort descending (SearchResult::Ord is descending by score).
        results.sort();

        results.truncate(top_k);
        Ok(results)
    }

    fn remove(&mut self, vector_ids: &[VectorId]) -> Result<(), BackendError> {
        for id in vector_ids {
            self.store.remove(id);
        }
        Ok(())
    }

    fn len(&self) -> usize {
        self.store.len()
    }

    fn dim(&self) -> Option<usize> {
        self.dim
    }
}
