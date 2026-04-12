//! AVX2+FMA kernel wrappers (`x86_64`, `feature = "simd"`).
//!
//! Phase 20 establishes the dispatch *framework*. To guarantee
//! byte-for-byte scalar parity on every target-feature matrix row, the
//! AVX2 entry points currently delegate to the scalar reference
//! kernels. Each wrapper carries `#[target_feature(enable =
//! "avx2,fma")]` so that, once the scalar bodies are replaced with
//! genuine intrinsic code, the compiler will emit AVX2+FMA instructions
//! without any call-site changes.
//!
//! This file is deliberately restricted to `unsafe` entry points —
//! `dispatch::current()` verifies the CPU supports AVX2+FMA at runtime
//! before calling any of them. The wrappers themselves perform no
//! unsafe operations internally beyond the delegation call, so no
//! `// SAFETY:` comments are needed inside the bodies.
#![allow(unsafe_code)]

use super::scalar;
use crate::errors::CodecError;

/// Quantize `values` into `indices` via the AVX2+FMA kernel path.
///
/// # Safety
///
/// Caller must ensure the target CPU supports both AVX2 and FMA
/// instructions. In practice this means routing through
/// [`crate::codec::dispatch::current()`] — that helper runs
/// `is_x86_feature_detected!` before selecting this path.
///
/// # Errors
///
/// Propagates errors from [`super::scalar::quantize_into`].
#[target_feature(enable = "avx2,fma")]
pub unsafe fn quantize_into(
    entries: &[f32],
    values: &[f32],
    indices: &mut [u8],
) -> Result<(), CodecError> {
    scalar::quantize_into(entries, values, indices)
}

/// Dequantize `indices` into `values` via the AVX2+FMA kernel path.
///
/// # Safety
///
/// See [`quantize_into`] for the AVX2+FMA availability contract.
///
/// # Errors
///
/// Propagates errors from [`super::scalar::dequantize_into`].
#[target_feature(enable = "avx2,fma")]
pub unsafe fn dequantize_into(
    entries: &[f32],
    indices: &[u8],
    values: &mut [f32],
) -> Result<(), CodecError> {
    scalar::dequantize_into(entries, indices, values)
}

/// Cosine similarity via the AVX2+FMA kernel path.
///
/// # Safety
///
/// See [`quantize_into`] for the AVX2+FMA availability contract.
#[target_feature(enable = "avx2,fma")]
pub unsafe fn cosine(a: &[f32], b: &[f32]) -> f32 {
    scalar::cosine(a, b)
}

/// Compute a fp16 residual buffer via the AVX2+FMA kernel path.
///
/// # Safety
///
/// See [`quantize_into`] for the AVX2+FMA availability contract.
#[target_feature(enable = "avx2,fma")]
pub unsafe fn compute_residual_into(original: &[f32], reconstructed: &[f32], out: &mut [u8]) {
    scalar::compute_residual_into(original, reconstructed, out);
}

/// Decode and apply an fp16 residual via the AVX2+FMA kernel path.
///
/// # Safety
///
/// See [`quantize_into`] for the AVX2+FMA availability contract.
///
/// # Errors
///
/// Propagates errors from [`super::scalar::apply_residual_into`].
#[target_feature(enable = "avx2,fma")]
pub unsafe fn apply_residual_into(values: &mut [f32], residual: &[u8]) -> Result<(), CodecError> {
    scalar::apply_residual_into(values, residual)
}
