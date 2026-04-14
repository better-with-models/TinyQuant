//! pyo3 Python bindings for `TinyQuant`.
//!
//! Exposes a `tinyquant_rs._core` Python extension module with API mirroring
//! `tinyquant_cpu`. The Python-side `tinyquant_rs/__init__.py` re-exports
//! every name from the sub-modules `tinyquant_rs.codec`,
//! `tinyquant_rs.corpus`, and `tinyquant_rs.backend`.
//!
//! See [[design/rust/ffi-and-bindings]] §Binding 1 for the full contract.
#![deny(clippy::all, clippy::unwrap_used, clippy::panic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::unnecessary_wraps,
    clippy::redundant_pub_crate,
    clippy::used_underscore_binding,
    clippy::needless_lifetimes,
    clippy::upper_case_acronyms
)]

use pyo3::prelude::*;

mod backend;
mod codec;
mod corpus;
mod errors;
mod numpy_bridge;

/// The PyPI / `tinyquant_cpu` parity version.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialise the `tinyquant_rs._core` Python extension module.
///
/// `maturin` ships this as `tinyquant_rs/_core.{abi3.so,abi3.pyd}`; the
/// pure-Python `tinyquant_rs/__init__.py` wrapper promotes the names to the
/// three user-facing sub-packages.
#[pymodule]
fn _core(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", VERSION)?;

    // Build sub-modules so `from tinyquant_rs._core.codec import ...` works
    // and the Python wrapper can re-export them directly.
    let codec_module = PyModule::new_bound(py, "codec")?;
    register_codec(py, &codec_module)?;

    let corpus_module = PyModule::new_bound(py, "corpus")?;
    register_corpus(py, &corpus_module)?;

    let backend_module = PyModule::new_bound(py, "backend")?;
    register_backend(py, &backend_module)?;

    errors::register(py, m, &codec_module, &corpus_module)?;

    m.add_submodule(&codec_module)?;
    m.add_submodule(&corpus_module)?;
    m.add_submodule(&backend_module)?;

    // Root-level re-exports matching the §Binding 1 class surface.
    m.add_class::<codec::PyCodecConfig>()?;
    m.add_class::<codec::PyCodebook>()?;
    m.add_class::<codec::PyRotationMatrix>()?;
    m.add_class::<codec::PyCompressedVector>()?;
    m.add_class::<codec::PyCodec>()?;
    m.add_function(wrap_pyfunction!(codec::compress, m)?)?;
    m.add_function(wrap_pyfunction!(codec::decompress, m)?)?;

    m.add_class::<corpus::PyCompressionPolicy>()?;
    m.add_class::<corpus::PyVectorEntry>()?;
    m.add_class::<corpus::PyCorpus>()?;
    m.add_class::<corpus::PyCorpusCreated>()?;
    m.add_class::<corpus::PyVectorsInserted>()?;
    m.add_class::<corpus::PyCorpusDecompressed>()?;
    m.add_class::<corpus::PyCompressionPolicyViolationDetected>()?;

    m.add_class::<backend::PySearchBackend>()?;
    m.add_class::<backend::PySearchResult>()?;
    m.add_class::<backend::PyBruteForceBackend>()?;

    Ok(())
}

fn register_codec(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<codec::PyCodecConfig>()?;
    m.add_class::<codec::PyCodebook>()?;
    m.add_class::<codec::PyRotationMatrix>()?;
    m.add_class::<codec::PyCompressedVector>()?;
    m.add_class::<codec::PyCodec>()?;
    m.add_function(wrap_pyfunction!(codec::compress, m)?)?;
    m.add_function(wrap_pyfunction!(codec::decompress, m)?)?;
    Ok(())
}

fn register_corpus(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<corpus::PyCompressionPolicy>()?;
    m.add_class::<corpus::PyVectorEntry>()?;
    m.add_class::<corpus::PyCorpus>()?;
    m.add_class::<corpus::PyCorpusCreated>()?;
    m.add_class::<corpus::PyVectorsInserted>()?;
    m.add_class::<corpus::PyCorpusDecompressed>()?;
    m.add_class::<corpus::PyCompressionPolicyViolationDetected>()?;
    Ok(())
}

fn register_backend(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<backend::PySearchBackend>()?;
    m.add_class::<backend::PySearchResult>()?;
    m.add_class::<backend::PyBruteForceBackend>()?;
    Ok(())
}

#[cfg(test)]
#[allow(missing_docs)]
mod tests {
    #[test]
    #[allow(clippy::missing_const_for_fn)]
    fn py_crate_builds() {}
}
