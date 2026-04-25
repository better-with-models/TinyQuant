//! PyO3 bindings for the search-backend layer.
//!
//! Mirrors `tinyquant_cpu.backend` with the two public types `SearchResult`
//! and `BruteForceBackend`, plus a lightweight `SearchBackend` protocol
//! placeholder (the Python side uses `typing.Protocol`, which has no runtime
//! effect, so we do not expose a pyclass for it).

use std::sync::{Arc, Mutex};

use numpy::PyReadonlyArray1;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use tinyquant_bruteforce::BruteForceBackend as CoreBruteForceBackend;
use tinyquant_core::backend::{
    SearchBackend as CoreSearchBackend, SearchResult as CoreSearchResult,
};
use tinyquant_core::types::VectorId;

use crate::errors::map_backend_error;
use crate::numpy_bridge::array1_as_slice;

// ---------------------------------------------------------------------------
// SearchResult
// ---------------------------------------------------------------------------

/// Immutable (vector_id, score) pair returned by `BruteForceBackend.search`.
#[pyclass(name = "SearchResult", module = "tinyquant_rs", frozen)]
#[derive(Clone)]
pub struct PySearchResult {
    #[pyo3(get)]
    vector_id: String,
    #[pyo3(get)]
    score: f32,
}

#[pymethods]
impl PySearchResult {
    #[new]
    fn new(vector_id: String, score: f32) -> Self {
        Self { vector_id, score }
    }

    fn __repr__(&self) -> String {
        format!(
            "SearchResult(vector_id={:?}, score={})",
            self.vector_id, self.score
        )
    }

    fn __lt__(&self, other: &Self) -> bool {
        // Python parity: order by score descending (higher scores sort first).
        self.score > other.score
    }
}

impl From<CoreSearchResult> for PySearchResult {
    fn from(r: CoreSearchResult) -> Self {
        Self {
            vector_id: r.vector_id.to_string(),
            score: r.score,
        }
    }
}

// ---------------------------------------------------------------------------
// SearchBackend placeholder
// ---------------------------------------------------------------------------

/// Runtime placeholder for the `SearchBackend` protocol.
///
/// Python's `typing.Protocol` is a structural type that carries no runtime
/// behavior; instantiation raises to preserve API parity with the reference
/// implementation where `SearchBackend()` is not meaningful either.
#[pyclass(name = "SearchBackend", module = "tinyquant_rs", frozen)]
pub struct PySearchBackend;

#[pymethods]
impl PySearchBackend {
    #[new]
    fn new() -> PyResult<Self> {
        Err(PyValueError::new_err(
            "SearchBackend is a structural protocol; instantiate a concrete backend like BruteForceBackend instead",
        ))
    }
}

// ---------------------------------------------------------------------------
// BruteForceBackend
// ---------------------------------------------------------------------------

/// Reference search backend: exhaustive cosine similarity.
#[pyclass(name = "BruteForceBackend", module = "tinyquant_rs")]
pub struct PyBruteForceBackend {
    inner: Arc<Mutex<CoreBruteForceBackend>>,
}

#[pymethods]
impl PyBruteForceBackend {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(CoreBruteForceBackend::new())),
        }
    }

    #[getter]
    fn count(&self) -> PyResult<usize> {
        let guard = self.inner.lock().map_err(|_| lock_err())?;
        Ok(guard.len())
    }

    /// Ingest a mapping of `vector_id -> 1-D f32 array`.
    fn ingest<'py>(&self, py: Python<'py>, vectors: &Bound<'py, PyDict>) -> PyResult<()> {
        let mut staged: Vec<(VectorId, Vec<f32>)> = Vec::with_capacity(vectors.len());
        for (key, value) in vectors.iter() {
            let id: String = key.extract()?;
            let arr: PyReadonlyArray1<'_, f32> = value.extract()?;
            let slice = array1_as_slice(&arr)?;
            staged.push((VectorId::from(id.as_str()), slice.to_vec()));
        }
        let inner = Arc::clone(&self.inner);
        py.allow_threads(move || {
            let mut guard = inner.lock().expect("backend mutex poisoned");
            guard.ingest(&staged)
        })
        .map_err(map_backend_error)
    }

    fn search<'py>(
        &self,
        py: Python<'py>,
        query: PyReadonlyArray1<'py, f32>,
        top_k: usize,
    ) -> PyResult<Vec<PySearchResult>> {
        let slice = array1_as_slice(&query)?;
        let query_vec: Vec<f32> = slice.to_vec();
        let inner = Arc::clone(&self.inner);
        let results = py
            .allow_threads(move || {
                let guard = inner.lock().expect("backend mutex poisoned");
                guard.search(&query_vec, top_k)
            })
            .map_err(map_backend_error)?;
        Ok(results.into_iter().map(PySearchResult::from).collect())
    }

    fn remove(&self, vector_ids: Vec<String>) -> PyResult<()> {
        let ids: Vec<VectorId> = vector_ids.into_iter().map(VectorId::from).collect();
        let mut guard = self.inner.lock().map_err(|_| lock_err())?;
        guard.remove(&ids).map_err(map_backend_error)
    }

    fn clear(&self) -> PyResult<()> {
        // The bruteforce backend has no explicit `clear`; remove all ids.
        let mut guard = self.inner.lock().map_err(|_| lock_err())?;
        // Collect first to release the borrow before mutating the store.
        // `len()` + indexed iteration keeps us within SearchBackend's surface.
        // We rebuild a fresh backend instead of enumerating ids through a
        // public API — this is the cheapest safe equivalent.
        *guard = CoreBruteForceBackend::new();
        Ok(())
    }
}

fn lock_err() -> PyErr {
    PyValueError::new_err("backend mutex poisoned")
}
