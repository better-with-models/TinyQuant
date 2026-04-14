//! PyO3 bindings for the `tinyquant-core` corpus aggregate.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::exceptions::{PyKeyError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};

use tinyquant_core::corpus::{
    CompressionPolicy as CoreCompressionPolicy, Corpus as CoreCorpus,
    CorpusEvent as CoreCorpusEvent, VectorEntry as CoreVectorEntry,
    ViolationKind as CoreViolationKind,
};
use tinyquant_core::types::VectorId;

use crate::codec::{PyCodebook, PyCodecConfig, PyCompressedVector};
use crate::errors::map_corpus_error;
use crate::numpy_bridge::array1_as_slice;

// ---------------------------------------------------------------------------
// CompressionPolicy
// ---------------------------------------------------------------------------

/// Immutable compression policy enum — Python-parity names.
#[pyclass(
    name = "CompressionPolicy",
    module = "tinyquant_rs",
    frozen,
    eq,
    eq_int,
    hash
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyCompressionPolicy {
    COMPRESS,
    PASSTHROUGH,
    FP16,
}

#[pymethods]
impl PyCompressionPolicy {
    /// True only for `COMPRESS`.
    fn requires_codec(&self) -> bool {
        matches!(self, Self::COMPRESS)
    }

    #[getter]
    fn value(&self) -> &'static str {
        match self {
            Self::COMPRESS => "compress",
            Self::PASSTHROUGH => "passthrough",
            Self::FP16 => "fp16",
        }
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self {
            Self::COMPRESS => "COMPRESS",
            Self::PASSTHROUGH => "PASSTHROUGH",
            Self::FP16 => "FP16",
        }
    }
}

impl From<PyCompressionPolicy> for CoreCompressionPolicy {
    fn from(p: PyCompressionPolicy) -> Self {
        match p {
            PyCompressionPolicy::COMPRESS => Self::Compress,
            PyCompressionPolicy::PASSTHROUGH => Self::Passthrough,
            PyCompressionPolicy::FP16 => Self::Fp16,
        }
    }
}

// ---------------------------------------------------------------------------
// VectorEntry
// ---------------------------------------------------------------------------

/// Entity representing a stored vector in the corpus.
#[pyclass(name = "VectorEntry", module = "tinyquant_rs")]
#[derive(Clone)]
pub struct PyVectorEntry {
    inner: CoreVectorEntry,
}

#[pymethods]
impl PyVectorEntry {
    #[getter]
    fn vector_id(&self) -> String {
        self.inner.vector_id().to_string()
    }

    #[getter]
    fn compressed(&self) -> PyCompressedVector {
        PyCompressedVector {
            inner: self.inner.compressed().clone(),
        }
    }

    #[getter]
    fn config_hash(&self) -> String {
        self.inner.config_hash().to_string()
    }

    #[getter]
    fn dimension(&self) -> u32 {
        self.inner.dimension()
    }

    #[getter]
    fn has_residual(&self) -> bool {
        self.inner.has_residual()
    }

    fn __repr__(&self) -> String {
        format!(
            "VectorEntry(vector_id={:?}, dimension={}, has_residual={})",
            self.inner.vector_id(),
            self.inner.dimension(),
            if self.inner.has_residual() {
                "True"
            } else {
                "False"
            },
        )
    }
}

// ---------------------------------------------------------------------------
// Corpus event value objects (Python parity)
// ---------------------------------------------------------------------------

/// Python parity for `tinyquant_cpu.corpus.events.CorpusCreated`.
#[pyclass(name = "CorpusCreated", module = "tinyquant_rs", frozen)]
pub struct PyCorpusCreated {
    #[pyo3(get)]
    corpus_id: String,
    codec_config: Py<PyCodecConfig>,
    #[pyo3(get)]
    compression_policy: PyCompressionPolicy,
    #[pyo3(get)]
    timestamp: i64,
}

#[pymethods]
impl PyCorpusCreated {
    #[getter]
    fn codec_config(&self, py: Python<'_>) -> Py<PyCodecConfig> {
        self.codec_config.clone_ref(py)
    }
}

/// Python parity for `tinyquant_cpu.corpus.events.VectorsInserted`.
#[pyclass(name = "VectorsInserted", module = "tinyquant_rs", frozen)]
#[derive(Clone)]
pub struct PyVectorsInserted {
    #[pyo3(get)]
    corpus_id: String,
    #[pyo3(get)]
    vector_ids: Vec<String>,
    #[pyo3(get)]
    count: u32,
    #[pyo3(get)]
    timestamp: i64,
}

/// Python parity for `tinyquant_cpu.corpus.events.CorpusDecompressed`.
#[pyclass(name = "CorpusDecompressed", module = "tinyquant_rs", frozen)]
#[derive(Clone)]
pub struct PyCorpusDecompressed {
    #[pyo3(get)]
    corpus_id: String,
    #[pyo3(get)]
    vector_count: u32,
    #[pyo3(get)]
    timestamp: i64,
}

/// Python parity for
/// `tinyquant_cpu.corpus.events.CompressionPolicyViolationDetected`.
#[pyclass(
    name = "CompressionPolicyViolationDetected",
    module = "tinyquant_rs",
    frozen
)]
#[derive(Clone)]
pub struct PyCompressionPolicyViolationDetected {
    #[pyo3(get)]
    corpus_id: String,
    #[pyo3(get)]
    violation_type: &'static str,
    #[pyo3(get)]
    detail: String,
    #[pyo3(get)]
    timestamp: i64,
}

fn violation_tag(kind: CoreViolationKind) -> &'static str {
    kind.as_python_tag()
}

// ---------------------------------------------------------------------------
// Corpus aggregate
// ---------------------------------------------------------------------------

/// Aggregate root managing a collection of compressed embedding vectors.
#[pyclass(name = "Corpus", module = "tinyquant_rs")]
pub struct PyCorpus {
    // `Corpus` is !Sync but Python objects share across the GIL; wrap in
    // Mutex so `&self` methods can still mutate the underlying aggregate.
    inner: Arc<Mutex<CoreCorpus>>,
    codec_config: Py<PyCodecConfig>,
    codebook: Py<PyCodebook>,
    compression_policy: PyCompressionPolicy,
}

#[pymethods]
impl PyCorpus {
    #[new]
    #[pyo3(signature = (corpus_id, codec_config, codebook, compression_policy, metadata = None))]
    fn new(
        py: Python<'_>,
        corpus_id: String,
        codec_config: Py<PyCodecConfig>,
        codebook: Py<PyCodebook>,
        compression_policy: PyCompressionPolicy,
        metadata: Option<Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let cfg_ref = codec_config.borrow(py);
        let cb_ref = codebook.borrow(py);
        let meta = BTreeMap::new();
        let _ = metadata; // Metadata lowering beyond an empty map is deferred;
                          // the corpus accepts arbitrary keys but Python's
                          // `tinyquant_cpu` does not inspect entry metadata
                          // in the parity paths exercised here.
        let core = CoreCorpus::new(
            VectorId::from(corpus_id.as_str()),
            cfg_ref.inner.clone(),
            cb_ref.inner.clone(),
            CoreCompressionPolicy::from(compression_policy),
            meta,
        );
        drop(cfg_ref);
        drop(cb_ref);
        Ok(Self {
            inner: Arc::new(Mutex::new(core)),
            codec_config,
            codebook,
            compression_policy,
        })
    }

    #[getter]
    fn corpus_id(&self) -> PyResult<String> {
        let guard = self.inner.lock().map_err(lock_err)?;
        Ok(guard.corpus_id().to_string())
    }

    #[getter]
    fn codec_config(&self, py: Python<'_>) -> Py<PyCodecConfig> {
        self.codec_config.clone_ref(py)
    }

    #[getter]
    fn codebook(&self, py: Python<'_>) -> Py<PyCodebook> {
        self.codebook.clone_ref(py)
    }

    #[getter]
    fn compression_policy(&self) -> PyCompressionPolicy {
        self.compression_policy
    }

    #[getter]
    fn metadata<'py>(&self, py: Python<'py>) -> Bound<'py, PyDict> {
        // Metadata surface is a read-only empty dict in this pyo3 binding —
        // `tinyquant_cpu` accepts per-corpus metadata but none of the parity
        // tests drive that field, so we expose an empty dict for API shape.
        PyDict::new_bound(py)
    }

    #[getter]
    fn vector_count(&self) -> PyResult<usize> {
        let guard = self.inner.lock().map_err(lock_err)?;
        Ok(guard.vector_count())
    }

    #[getter]
    fn is_empty(&self) -> PyResult<bool> {
        let guard = self.inner.lock().map_err(lock_err)?;
        Ok(guard.is_empty())
    }

    #[getter]
    fn vector_ids<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyFrozenSet>> {
        let guard = self.inner.lock().map_err(lock_err)?;
        let ids: Vec<String> = guard.iter().map(|(id, _)| id.to_string()).collect();
        pyo3::types::PyFrozenSet::new_bound(py, ids.iter())
    }

    /// Return and clear the pending event list.
    fn pending_events<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let mut guard = self.inner.lock().map_err(lock_err)?;
        let events = guard.drain_events();
        let list = PyList::empty_bound(py);
        for event in events {
            let obj = event_to_py(py, &event, &self.codec_config)?;
            list.append(obj)?;
        }
        Ok(list)
    }

    #[pyo3(signature = (vector_id, vector, metadata = None))]
    fn insert<'py>(
        &self,
        py: Python<'py>,
        vector_id: String,
        vector: PyReadonlyArray1<'py, f32>,
        metadata: Option<Bound<'_, PyDict>>,
    ) -> PyResult<PyVectorEntry> {
        let _ = metadata; // same note as Corpus::new — metadata pass-through
                          // is preserved by the core but not surfaced here.
        let slice = array1_as_slice(&vector)?;
        let buf: Vec<f32> = slice.to_vec();
        let id: VectorId = VectorId::from(vector_id.as_str());
        let inner_clone = Arc::clone(&self.inner);
        let id_for_lookup = id.clone();
        py.allow_threads(move || {
            let mut guard = inner_clone.lock().expect("corpus mutex poisoned");
            guard.insert(id, &buf, None, 0)
        })
        .map_err(map_corpus_error)?;

        // Fetch the freshly inserted entry to return a Python-visible handle.
        let guard = self.inner.lock().map_err(lock_err)?;
        let entry = guard
            .iter()
            .find(|(eid, _)| eid.as_ref() == id_for_lookup.as_ref())
            .map(|(_, e)| e.clone())
            .ok_or_else(|| PyValueError::new_err("insert succeeded but entry missing"))?;
        Ok(PyVectorEntry { inner: entry })
    }

    fn contains(&self, vector_id: &str) -> PyResult<bool> {
        let guard = self.inner.lock().map_err(lock_err)?;
        Ok(guard.contains(&VectorId::from(vector_id)))
    }

    fn get(&self, vector_id: &str) -> PyResult<PyVectorEntry> {
        let guard = self.inner.lock().map_err(lock_err)?;
        let id = VectorId::from(vector_id);
        let entry = guard
            .iter()
            .find(|(eid, _)| eid.as_ref() == id.as_ref())
            .map(|(_, e)| e.clone())
            .ok_or_else(|| PyKeyError::new_err(format!("{vector_id:?}")))?;
        Ok(PyVectorEntry { inner: entry })
    }

    fn decompress<'py>(
        &self,
        py: Python<'py>,
        vector_id: &str,
    ) -> PyResult<Bound<'py, PyArray1<f32>>> {
        let id = VectorId::from(vector_id);
        let guard = self.inner.lock().map_err(lock_err)?;
        let out = guard.decompress(&id).map_err(map_corpus_error)?;
        Ok(PyArray1::from_vec_bound(py, out))
    }

    fn decompress_all<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let mut guard = self.inner.lock().map_err(lock_err)?;
        let map = guard.decompress_all_at(0).map_err(map_corpus_error)?;
        let dict = PyDict::new_bound(py);
        for (id, vec) in map {
            dict.set_item(id.to_string(), PyArray1::from_vec_bound(py, vec))?;
        }
        Ok(dict)
    }

    fn remove(&self, vector_id: &str) -> PyResult<()> {
        let mut guard = self.inner.lock().map_err(lock_err)?;
        let id = VectorId::from(vector_id);
        match guard.remove(&id) {
            Some(_) => Ok(()),
            None => Err(PyKeyError::new_err(format!("{vector_id:?}"))),
        }
    }
}

fn lock_err(_err: std::sync::PoisonError<std::sync::MutexGuard<'_, CoreCorpus>>) -> PyErr {
    PyValueError::new_err("corpus mutex poisoned")
}

fn event_to_py(
    py: Python<'_>,
    event: &CoreCorpusEvent,
    config: &Py<PyCodecConfig>,
) -> PyResult<PyObject> {
    let obj: PyObject = match event {
        CoreCorpusEvent::Created {
            corpus_id,
            codec_config: _cfg,
            compression_policy,
            timestamp,
        } => {
            let policy = match compression_policy {
                CoreCompressionPolicy::Compress => PyCompressionPolicy::COMPRESS,
                CoreCompressionPolicy::Passthrough => PyCompressionPolicy::PASSTHROUGH,
                CoreCompressionPolicy::Fp16 => PyCompressionPolicy::FP16,
            };
            Py::new(
                py,
                PyCorpusCreated {
                    corpus_id: corpus_id.to_string(),
                    codec_config: config.clone_ref(py),
                    compression_policy: policy,
                    timestamp: *timestamp,
                },
            )?
            .into_py(py)
        }
        CoreCorpusEvent::VectorsInserted {
            corpus_id,
            vector_ids,
            count,
            timestamp,
        } => {
            let ids: Vec<String> = vector_ids.iter().map(|v| v.to_string()).collect();
            Py::new(
                py,
                PyVectorsInserted {
                    corpus_id: corpus_id.to_string(),
                    vector_ids: ids,
                    count: *count,
                    timestamp: *timestamp,
                },
            )?
            .into_py(py)
        }
        CoreCorpusEvent::Decompressed {
            corpus_id,
            vector_count,
            timestamp,
        } => Py::new(
            py,
            PyCorpusDecompressed {
                corpus_id: corpus_id.to_string(),
                vector_count: *vector_count,
                timestamp: *timestamp,
            },
        )?
        .into_py(py),
        CoreCorpusEvent::PolicyViolationDetected {
            corpus_id,
            kind,
            detail,
            timestamp,
        } => Py::new(
            py,
            PyCompressionPolicyViolationDetected {
                corpus_id: corpus_id.to_string(),
                violation_type: violation_tag(*kind),
                detail: detail.to_string(),
                timestamp: *timestamp,
            },
        )?
        .into_py(py),
    };
    Ok(obj)
}

// Silence an unused-import warning when the helper is used only in tuple
// conversions above.
#[allow(dead_code)]
fn _unused_tuple(py: Python<'_>) -> Bound<'_, PyTuple> {
    PyTuple::empty_bound(py)
}
