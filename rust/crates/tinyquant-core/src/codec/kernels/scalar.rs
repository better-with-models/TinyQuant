//! Scalar reference kernels — the canonical "source of truth" that all
//! SIMD paths must match byte-for-byte (Phase 20).
//!
//! Each function in this module has a single responsibility:
//!
//! * [`quantize_into`] — nearest-entry quantization (delegates to [`crate::codec::quantize::scalar_quantize`]).
//! * [`dequantize_into`] — gather-by-index decode (delegates to [`crate::codec::quantize::scalar_dequantize`]).
//! * [`cosine`] — dot-product / norm cosine similarity. Mirrors `tinyquant-bruteforce` but omits debug NaN assertions to stay `no_std`-compatible.
//! * [`compute_residual_into`] — in-place fp16 residual encoding.
//! * [`apply_residual_into`] — in-place fp16 residual decoding (re-exports [`crate::codec::residual::apply_residual_into`]).

use crate::codec::quantize::{scalar_dequantize, scalar_quantize};
use crate::codec::residual::apply_residual_into as residual_apply_impl;
use crate::errors::CodecError;
use half::f16;

/// Quantize `values` against `entries`, writing nearest-entry indices
/// into `indices`.
///
/// # Errors
///
/// Propagates [`CodecError::LengthMismatch`] from
/// [`crate::codec::quantize::scalar_quantize`].
#[inline]
pub fn quantize_into(
    entries: &[f32],
    values: &[f32],
    indices: &mut [u8],
) -> Result<(), CodecError> {
    scalar_quantize(entries, values, indices)
}

/// Gather `entries[indices[i]]` into `values[i]`.
///
/// # Errors
///
/// Propagates errors from [`crate::codec::quantize::scalar_dequantize`].
#[inline]
pub fn dequantize_into(
    entries: &[f32],
    indices: &[u8],
    values: &mut [f32],
) -> Result<(), CodecError> {
    scalar_dequantize(entries, indices, values)
}

/// Compute cosine similarity between two equal-length `f32` slices.
///
/// Returns `0.0` when either vector is the zero vector (denominator
/// equals zero) **or** when either input contains a NaN. The NaN early
/// return makes the Phase 20 parity contract trivially satisfiable by
/// every dispatched path — SIMD kernels that sum NaN through FMA would
/// otherwise propagate NaN into the numerator and produce a divergent
/// bit pattern vs the scalar reference.
///
/// This function is `no_std`-compatible and deliberately omits the
/// debug-only panics that live in
/// `tinyquant-bruteforce::similarity::cosine_similarity`.
#[cfg_attr(not(feature = "simd"), allow(dead_code))]
#[inline]
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    // Phase 20 NaN semantics: "if either input vector contains any NaN,
    // return 0.0." Do the scan up front so the main accumulator loop
    // can remain branch-free and dispatch-parity-friendly.
    if a.iter().any(|x| x.is_nan()) || b.iter().any(|x| x.is_nan()) {
        return 0.0;
    }
    let mut dot: f32 = 0.0;
    let mut na: f32 = 0.0;
    let mut nb: f32 = 0.0;
    for (&x, &y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = libm::sqrtf(na * nb);
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

/// Compute `original - reconstructed` as little-endian fp16 bytes,
/// writing the 2 * N byte result into the pre-allocated `out` slice.
///
/// Mirrors [`crate::codec::residual::compute_residual`] exactly, but
/// writes into a caller-provided buffer instead of allocating a fresh
/// [`alloc::vec::Vec`].
///
/// # Panics
///
/// Debug-only: panics when `original.len() != reconstructed.len()` or
/// when `out.len() != original.len() * 2`. In release builds the short
/// iterator consumes whichever slice runs out first, which is an
/// implementation-defined but panic-free fallback.
#[cfg_attr(not(feature = "simd"), allow(dead_code))]
pub fn compute_residual_into(original: &[f32], reconstructed: &[f32], out: &mut [u8]) {
    debug_assert_eq!(original.len(), reconstructed.len());
    debug_assert_eq!(out.len(), original.len() * 2);
    for ((o, r), chunk) in original
        .iter()
        .zip(reconstructed.iter())
        .zip(out.chunks_exact_mut(2))
    {
        let diff = f16::from_f32(*o - *r);
        let bytes = diff.to_le_bytes();
        // chunk is guaranteed to have length 2 by chunks_exact_mut(2).
        if let Some(first) = chunk.first_mut() {
            *first = bytes[0];
        }
        if let Some(second) = chunk.get_mut(1) {
            *second = bytes[1];
        }
    }
}

/// Decode `residual` as fp16 LE and add in place into `values`.
///
/// # Errors
///
/// Propagates [`CodecError::LengthMismatch`] from
/// [`crate::codec::residual::apply_residual_into`].
#[cfg_attr(not(feature = "simd"), allow(dead_code))]
#[inline]
pub fn apply_residual_into(values: &mut [f32], residual: &[u8]) -> Result<(), CodecError> {
    residual_apply_impl(values, residual)
}
