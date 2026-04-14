//! napi-rs bindings for the search-backend layer.
//!
//! Phase 25.3 scope: `SearchResult` value object and
//! `BruteForceBackend` — the reference exhaustive cosine-similarity
//! backend from `tinyquant_bruteforce`. The napi surface mirrors
//! `tinyquant_cpu.backend` one-for-one (camelCase); all logic is
//! delegated to `tinyquant_core`'s `SearchBackend` trait implementation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::Float32Array;
use napi::{Error as NapiError, Status};
use napi_derive::napi;

use tinyquant_bruteforce::BruteForceBackend as CoreBruteForceBackend;
use tinyquant_core::backend::{
    SearchBackend as CoreSearchBackend, SearchResult as CoreSearchResult,
};
use tinyquant_core::types::VectorId;

use crate::errors::map_backend_error;

// ---------------------------------------------------------------------------
// SearchResult
// ---------------------------------------------------------------------------

/// Immutable `(vectorId, score)` pair returned by `BruteForceBackend.search`.
#[napi]
pub struct SearchResult {
    vector_id: String,
    score: f64,
}

#[napi]
impl SearchResult {
    #[napi(getter)]
    pub fn vector_id(&self) -> String {
        self.vector_id.clone()
    }

    #[napi(getter)]
    pub fn score(&self) -> f64 {
        self.score
    }
}

impl From<CoreSearchResult> for SearchResult {
    fn from(r: CoreSearchResult) -> Self {
        Self {
            vector_id: r.vector_id.to_string(),
            // Widen f32 to f64 at the FFI boundary — JS numbers are
            // 64-bit floats so this is the natural representation.
            score: f64::from(r.score),
        }
    }
}

// ---------------------------------------------------------------------------
// BruteForceBackend
// ---------------------------------------------------------------------------

/// Reference search backend — exhaustive cosine similarity scan.
#[napi]
pub struct BruteForceBackend {
    inner: Arc<Mutex<CoreBruteForceBackend>>,
}

#[napi]
impl BruteForceBackend {
    #[napi(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(CoreBruteForceBackend::new())),
        }
    }

    #[napi(getter)]
    pub fn count(&self) -> napi::Result<u32> {
        let guard = self.inner.lock().map_err(lock_err)?;
        #[allow(clippy::cast_possible_truncation)]
        Ok(guard.len() as u32)
    }

    /// Ingest a mapping of `vectorId → Float32Array`.
    #[napi]
    pub fn ingest(&self, vectors: HashMap<String, Float32Array>) -> napi::Result<()> {
        let mut staged: Vec<(VectorId, Vec<f32>)> = Vec::with_capacity(vectors.len());
        for (k, v) in vectors {
            let s: &[f32] = v.as_ref();
            staged.push((VectorId::from(k.as_str()), s.to_vec()));
        }
        let mut guard = self.inner.lock().map_err(lock_err)?;
        guard.ingest(&staged).map_err(map_backend_error)
    }

    /// Return the top `top_k` matches ordered descending by cosine
    /// similarity.  `top_k = 0` raises `InvalidTopKError`.
    #[napi]
    pub fn search(&self, query: Float32Array, top_k: u32) -> napi::Result<Vec<SearchResult>> {
        let slice: &[f32] = query.as_ref();
        let query_vec = slice.to_vec();
        let guard = self.inner.lock().map_err(lock_err)?;
        let results = guard
            .search(&query_vec, top_k as usize)
            .map_err(map_backend_error)?;
        Ok(results.into_iter().map(SearchResult::from).collect())
    }

    /// Remove by id — unknown ids are silently ignored.
    #[napi]
    pub fn remove(&self, vector_ids: Vec<String>) -> napi::Result<()> {
        let ids: Vec<VectorId> = vector_ids
            .into_iter()
            .map(|s| VectorId::from(s.as_str()))
            .collect();
        let mut guard = self.inner.lock().map_err(lock_err)?;
        guard.remove(&ids).map_err(map_backend_error)
    }

    /// Remove every stored vector.
    #[napi]
    pub fn clear(&self) -> napi::Result<()> {
        // The bruteforce backend has no explicit `clear`; rebuild from
        // scratch — cheapest safe equivalent that keeps the
        // SearchBackend trait's surface honest.
        let mut guard = self.inner.lock().map_err(lock_err)?;
        *guard = CoreBruteForceBackend::new();
        Ok(())
    }
}

fn lock_err<T>(_err: std::sync::PoisonError<T>) -> NapiError {
    NapiError::new(
        Status::GenericFailure,
        "TinyQuantError: backend mutex poisoned".to_string(),
    )
}
