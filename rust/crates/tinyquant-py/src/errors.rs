//! PyO3 exception classes mirroring `tinyquant_cpu.codec._errors` and the
//! corpus/backend error surfaces.
//!
//! Every exception inherits from Python's `ValueError` so existing `except
//! ValueError` blocks in downstream code continue to catch them — matching
//! `tinyquant_cpu` where each error subclasses `ValueError`.

use pyo3::create_exception;
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;

use tinyquant_core::errors::{BackendError, CodecError, CorpusError};

create_exception!(
    tinyquant_rs,
    DimensionMismatchError,
    PyValueError,
    "Raised when a vector's length does not match the expected dimension."
);
create_exception!(
    tinyquant_rs,
    ConfigMismatchError,
    PyValueError,
    "Raised when a compressed vector's config hash does not match the config."
);
create_exception!(
    tinyquant_rs,
    CodebookIncompatibleError,
    PyValueError,
    "Raised when a codebook's bit width does not match the codec config."
);
create_exception!(
    tinyquant_rs,
    DuplicateVectorError,
    PyValueError,
    "Raised when a vector ID already exists in the corpus."
);

/// Register every exception class on the top-level module and the three
/// sub-modules (`codec`, `corpus`, `backend`). Mirrors the Python package
/// layout where each error is importable from its native sub-package.
pub(crate) fn register(
    py: Python<'_>,
    root: &Bound<'_, PyModule>,
    codec: &Bound<'_, PyModule>,
    corpus: &Bound<'_, PyModule>,
) -> PyResult<()> {
    // Root
    root.add(
        "DimensionMismatchError",
        py.get_type_bound::<DimensionMismatchError>(),
    )?;
    root.add(
        "ConfigMismatchError",
        py.get_type_bound::<ConfigMismatchError>(),
    )?;
    root.add(
        "CodebookIncompatibleError",
        py.get_type_bound::<CodebookIncompatibleError>(),
    )?;
    root.add(
        "DuplicateVectorError",
        py.get_type_bound::<DuplicateVectorError>(),
    )?;

    // codec sub-module
    codec.add(
        "DimensionMismatchError",
        py.get_type_bound::<DimensionMismatchError>(),
    )?;
    codec.add(
        "ConfigMismatchError",
        py.get_type_bound::<ConfigMismatchError>(),
    )?;
    codec.add(
        "CodebookIncompatibleError",
        py.get_type_bound::<CodebookIncompatibleError>(),
    )?;
    codec.add(
        "DuplicateVectorError",
        py.get_type_bound::<DuplicateVectorError>(),
    )?;

    // corpus sub-module (same set, to match Python's
    // `tinyquant_cpu.corpus` re-exporting the codec errors through usage).
    corpus.add(
        "DimensionMismatchError",
        py.get_type_bound::<DimensionMismatchError>(),
    )?;
    corpus.add(
        "DuplicateVectorError",
        py.get_type_bound::<DuplicateVectorError>(),
    )?;

    Ok(())
}

/// Convert a `CodecError` into the matching Python exception.
pub(crate) fn map_codec_error(err: CodecError) -> PyErr {
    let message = err.to_string();
    match err {
        CodecError::DimensionMismatch { .. } => DimensionMismatchError::new_err(message),
        CodecError::ConfigMismatch { .. } => ConfigMismatchError::new_err(message),
        CodecError::CodebookIncompatible { .. } => CodebookIncompatibleError::new_err(message),
        _ => PyValueError::new_err(message),
    }
}

/// Convert a `CorpusError` into the matching Python exception.
pub(crate) fn map_corpus_error(err: CorpusError) -> PyErr {
    let message = err.to_string();
    match err {
        CorpusError::Codec(inner) => map_codec_error(inner),
        CorpusError::DuplicateVectorId { .. } => DuplicateVectorError::new_err(message),
        CorpusError::UnknownVectorId { .. } => PyKeyError::new_err(message),
        CorpusError::PolicyImmutable => PyRuntimeError::new_err(message),
        CorpusError::DimensionMismatch { .. } => DimensionMismatchError::new_err(message),
        CorpusError::BatchAtomicityFailure { source, .. } => {
            // Unwrap the wrapped cause for cleaner Python-side error type parity.
            map_corpus_error(*source)
        }
        _ => PyValueError::new_err(message),
    }
}

/// Convert a `BackendError` into the matching Python exception.
pub(crate) fn map_backend_error(err: BackendError) -> PyErr {
    let message = err.to_string();
    match err {
        BackendError::Empty => PyRuntimeError::new_err(message),
        BackendError::InvalidTopK => PyValueError::new_err(message),
        _ => PyRuntimeError::new_err(message),
    }
}
