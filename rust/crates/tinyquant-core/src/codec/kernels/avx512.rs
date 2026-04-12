//! AVX-512 kernel stubs (Phase 22).
//!
//! Phase 20 ships the dispatch framework; full AVX-512 intrinsic bodies
//! land in Phase 22 when the MSRV accommodates `avx512f` target-feature
//! gating. Until then every function delegates to the portable (scalar) path.
//!
//! Feature-gated: `simd` + `avx512`.

#[allow(unused_imports)]
pub use super::portable::{
    apply_residual_into, compute_residual_into, cosine, dequantize_into, quantize_into,
};
