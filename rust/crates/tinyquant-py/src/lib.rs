//! pyo3 Python bindings for `TinyQuant`.
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
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::unnecessary_wraps,
    clippy::redundant_pub_crate
)]

use pyo3::prelude::*;

/// Initialise the `tinyquant_rs` Python extension module.
#[pymodule]
fn tinyquant_rs(_py: Python<'_>, _m: &Bound<'_, PyModule>) -> PyResult<()> {
    Ok(())
}

#[cfg(test)]
#[allow(missing_docs)]
mod tests {
    #[test]
    #[allow(clippy::missing_const_for_fn)]
    fn py_crate_builds() {}
}
