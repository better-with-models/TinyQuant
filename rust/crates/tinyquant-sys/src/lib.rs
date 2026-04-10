//! C ABI façade for `TinyQuant`.
//!
//! Exposes a stable `extern "C"` API. cbindgen header generation is wired
//! in Phase 22. This file is a Phase 11 stub.
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

/// Returns the `TinyQuant` ABI version. Phase 11 stub: always returns 0.
#[no_mangle]
pub const extern "C" fn tinyquant_version() -> u32 {
    0
}

#[cfg(test)]
#[allow(missing_docs)]
mod tests {
    use super::tinyquant_version;

    #[test]
    fn sys_crate_builds() {
        assert_eq!(tinyquant_version(), 0);
    }
}
