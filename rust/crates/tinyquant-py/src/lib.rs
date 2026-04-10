//! pyo3 Python bindings for TinyQuant.
//!
//! Exposes a `tinyquant_rs` Python extension module with API mirroring
//! `tinyquant_cpu`. Populated in Phase 22.
#![deny(
    warnings,
    missing_docs,
    unsafe_op_in_unsafe_fn,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cognitive_complexity
)]
#![allow(clippy::module_name_repetitions, clippy::must_use_candidate)]

use pyo3::prelude::*;

/// Initialise the `tinyquant_rs` Python extension module.
#[pymodule]
fn tinyquant_rs(_py: Python<'_>, _m: &PyModule) -> PyResult<()> {
    Ok(())
}

#[cfg(test)]
#[allow(missing_docs)]
mod tests {
    #[test]
    fn py_crate_builds() {
        assert!(true);
    }
}
