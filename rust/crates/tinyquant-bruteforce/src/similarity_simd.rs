//! SIMD-dispatched cosine similarity kernel for `BruteForceBackend` (Phase 20).
//!
//! Routes through `tinyquant_core::codec::simd_api::cosine`, which consults the
//! runtime ISA dispatch cache. Byte-identical to the scalar reference on all inputs.

/// Cosine similarity via the runtime-dispatched SIMD kernel.
///
/// Requires `feature = "simd"` on both this crate and `tinyquant-core`.
pub fn cosine_dispatched(a: &[f32], b: &[f32]) -> f32 {
    tinyquant_core::codec::simd_api::cosine(a, b)
}
