//! NEON kernel wrappers (aarch64, `feature = "simd"`).
//!
//! Mirrors [`super::avx2`] — Phase 20 establishes the dispatch
//! framework with byte-for-byte scalar parity. NEON is mandatory on
//! ARMv8 aarch64, so `dispatch::current()` unconditionally selects this
//! path on aarch64 targets when `feature = "simd"` is enabled.
//!
//! The wrappers currently forward to [`super::scalar`]; when real NEON
//! intrinsic implementations land, the `#[target_feature(enable =
//! "neon")]` annotation will ensure the generated code uses the NEON
//! instruction set.
#![allow(unsafe_code)]

use super::scalar;
use crate::errors::CodecError;

/// Quantize `values` into `indices` via the NEON kernel path.
///
/// # Safety
///
/// Caller must ensure the target CPU supports NEON. On aarch64 this is
/// guaranteed by the base ARMv8 instruction set, so
/// [`crate::codec::dispatch::current()`] may pick this path
/// unconditionally.
///
/// # Errors
///
/// Propagates errors from [`super::scalar::quantize_into`].
#[target_feature(enable = "neon")]
pub unsafe fn quantize_into(
    entries: &[f32],
    values: &[f32],
    indices: &mut [u8],
) -> Result<(), CodecError> {
    scalar::quantize_into(entries, values, indices)
}

/// Dequantize `indices` into `values` via the NEON kernel path.
///
/// # Safety
///
/// See [`quantize_into`] for the NEON availability contract.
///
/// # Errors
///
/// Propagates errors from [`super::scalar::dequantize_into`].
#[target_feature(enable = "neon")]
pub unsafe fn dequantize_into(
    entries: &[f32],
    indices: &[u8],
    values: &mut [f32],
) -> Result<(), CodecError> {
    scalar::dequantize_into(entries, indices, values)
}

/// Cosine similarity via the NEON kernel path.
///
/// # Safety
///
/// See [`quantize_into`] for the NEON availability contract.
#[target_feature(enable = "neon")]
pub unsafe fn cosine(a: &[f32], b: &[f32]) -> f32 {
    scalar::cosine(a, b)
}

/// Compute a fp16 residual buffer via the NEON kernel path.
///
/// # Safety
///
/// See [`quantize_into`] for the NEON availability contract.
#[target_feature(enable = "neon")]
pub unsafe fn compute_residual_into(original: &[f32], reconstructed: &[f32], out: &mut [u8]) {
    scalar::compute_residual_into(original, reconstructed, out);
}

/// Decode and apply an fp16 residual via the NEON kernel path.
///
/// # Safety
///
/// See [`quantize_into`] for the NEON availability contract.
///
/// # Errors
///
/// Propagates errors from [`super::scalar::apply_residual_into`].
#[target_feature(enable = "neon")]
pub unsafe fn apply_residual_into(values: &mut [f32], residual: &[u8]) -> Result<(), CodecError> {
    scalar::apply_residual_into(values, residual)
}
