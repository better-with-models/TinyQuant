//! `SearchBackend` trait and `SearchResult` type.
//!
//! Every search backend — brute-force, pgvector, HNSW — implements
//! [`SearchBackend`]. [`SearchResult`] is the return element from
//! [`SearchBackend::search`]; it orders descending by score so that
//! a max-heap or sort gives the nearest-first ranking callers expect.
//!
//! See `docs/design/behavior-layer/backend-protocol.md` for the full
//! behavioural contract and Python-parity notes.

use alloc::vec::Vec;

use crate::errors::BackendError;
use crate::types::VectorId;

/// A single item returned by [`SearchBackend::search`].
///
/// `score` is the cosine similarity between the query and the stored
/// vector.  Values range from −1.0 to 1.0; higher is better.  NaN
/// inputs are mapped to `Equal` in the `Ord` impl so that a stable sort
/// degrades gracefully rather than panicking.
#[derive(Clone, Debug)]
pub struct SearchResult {
    /// The identifier of the matching stored vector.
    pub vector_id: VectorId,
    /// Cosine similarity score in [−1, 1].  Higher is better.
    pub score: f32,
}

impl PartialEq for SearchResult {
    fn eq(&self, other: &Self) -> bool {
        // NaN == NaN for our purposes (stable sort stability)
        self.score.total_cmp(&other.score).is_eq() && self.vector_id == other.vector_id
    }
}

impl Eq for SearchResult {}

impl PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SearchResult {
    /// Descending by score (highest score sorts first).
    ///
    /// NaN maps to `Equal` so a stable sort preserves insertion order for
    /// degenerate vectors rather than producing undefined behaviour.
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // `total_cmp` is stable and handles NaN: NaN < NaN is false, so
        // NaN.total_cmp(NaN) == Equal.  We then reverse for descending order.
        other.score.total_cmp(&self.score)
    }
}

/// The public contract every search backend must satisfy.
///
/// Implementors hold a mutable collection of `(VectorId, Vec<f32>)` pairs
/// and expose a uniform ingest / search / remove interface.  Backends may
/// be in-process (brute-force) or remote (pgvector); the trait hides the
/// difference from callers.
///
/// # Thread safety
///
/// The trait requires `Send` so that backends can be moved between threads,
/// e.g. wrapped in a `Mutex<dyn SearchBackend>`.  Individual implementations
/// may add `Sync` if they wish.
///
/// # Errors
///
/// All methods return `Result<_, BackendError>`.  The caller decides whether
/// to treat `BackendError::Empty` as a soft empty result or a hard failure.
pub trait SearchBackend: Send {
    /// Store the supplied `(VectorId, Vec<f32>)` pairs.
    ///
    /// - Dimension-locks on the first non-empty call; subsequent calls with a
    ///   different dimension return
    ///   `Err(BackendError::Adapter("dimension mismatch: expected {d}, got {got}"))`.
    /// - Duplicate `VectorId`s overwrite the stored vector silently (Python
    ///   parity).
    /// - An empty slice is a no-op returning `Ok(())`.
    ///
    /// # Errors
    ///
    /// Returns `Err(BackendError::Adapter(_))` if the vectors' dimension does
    /// not match the dimension locked by a previous `ingest` call.
    fn ingest(&mut self, vectors: &[(VectorId, Vec<f32>)]) -> Result<(), BackendError>;

    /// Return the `top_k` nearest vectors to `query` by cosine similarity,
    /// sorted descending.
    ///
    /// - `top_k == 0` → `Err(BackendError::InvalidTopK)`.
    /// - Empty backend → `Ok(vec![])`.
    /// - If the backend contains fewer than `top_k` vectors, returns all of
    ///   them sorted descending.
    ///
    /// # Errors
    ///
    /// Returns `Err(BackendError::InvalidTopK)` when `top_k == 0`.
    /// Returns `Err(BackendError::Adapter(_))` when the query dimension does
    /// not match the locked dimension, or on adapter-specific failures.
    fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<SearchResult>, BackendError>;

    /// Remove vectors by id.
    ///
    /// Unknown ids are silently ignored (Python parity).  An empty slice is a
    /// no-op returning `Ok(())`.
    ///
    /// # Errors
    ///
    /// Returns `Err(BackendError::Adapter(_))` on adapter-specific failures
    /// (e.g. a lost database connection).  In-process backends always return
    /// `Ok(())`.
    fn remove(&mut self, vector_ids: &[VectorId]) -> Result<(), BackendError>;

    /// Number of vectors currently stored.
    fn len(&self) -> usize;

    /// Returns `true` if no vectors are stored.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The locked dimension, or `None` if no vectors have been ingested yet.
    fn dim(&self) -> Option<usize>;
}
