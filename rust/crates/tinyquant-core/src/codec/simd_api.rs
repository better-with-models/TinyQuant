//! Public SIMD kernel façade for use by downstream crates and tests
//! (Phase 20).
//!
//! The real kernel implementations live in `crate::codec::kernels::*`
//! and are `pub(crate)` to keep the "scalar reference is the canonical
//! source of truth" invariant visually obvious inside the core crate.
//! This module exposes the handful of dispatch-routed entry points
//! that `tinyquant-bruteforce`, the parity tests, and the benchmarks
//! need, without widening the general-purpose public API of
//! `tinyquant-core`.
//!
//! Every function here routes through [`crate::codec::dispatch`] and
//! therefore benefits from the `OnceLock` kernel-selection cache. The
//! returned values are byte-for-byte identical to the scalar
//! reference — SIMD never changes output, only speed.

use crate::codec::dispatch::{current, DispatchKind};
use crate::codec::kernels;
use crate::errors::CodecError;

/// Re-export of the scalar reference kernels.
///
/// Downstream integration tests and benchmarks need a handle on the
/// canonical reference to compare against dispatch-routed output.
/// Re-exporting through `simd_api::scalar` keeps the internal
/// `crate::codec::kernels` module private while still exposing the
/// parity baseline.
pub mod scalar {
    pub use crate::codec::kernels::scalar::{
        apply_residual_into, compute_residual_into, cosine, dequantize_into, quantize_into,
    };
}

/// Dispatch-routed quantize: `values` -> `indices` against `entries`.
///
/// # Errors
///
/// See [`crate::codec::kernels::scalar::quantize_into`].
pub fn quantize_into(
    entries: &[f32],
    values: &[f32],
    indices: &mut [u8],
) -> Result<(), CodecError> {
    match current() {
        #[cfg(target_arch = "x86_64")]
        DispatchKind::Avx2 => {
            // SAFETY: dispatch::current() verified AVX2+FMA support.
            unsafe { kernels::avx2::quantize_into(entries, values, indices) }
        }
        #[cfg(target_arch = "aarch64")]
        DispatchKind::Neon => {
            // SAFETY: NEON is mandatory on ARMv8 aarch64.
            unsafe { kernels::neon::quantize_into(entries, values, indices) }
        }
        DispatchKind::Scalar => kernels::scalar::quantize_into(entries, values, indices),
    }
}

/// Dispatch-routed dequantize: `indices` -> `values` against `entries`.
///
/// # Errors
///
/// See [`crate::codec::kernels::scalar::dequantize_into`].
pub fn dequantize_into(
    entries: &[f32],
    indices: &[u8],
    values: &mut [f32],
) -> Result<(), CodecError> {
    match current() {
        #[cfg(target_arch = "x86_64")]
        DispatchKind::Avx2 => {
            // SAFETY: dispatch::current() verified AVX2+FMA support.
            unsafe { kernels::avx2::dequantize_into(entries, indices, values) }
        }
        #[cfg(target_arch = "aarch64")]
        DispatchKind::Neon => {
            // SAFETY: NEON is mandatory on ARMv8 aarch64.
            unsafe { kernels::neon::dequantize_into(entries, indices, values) }
        }
        DispatchKind::Scalar => kernels::scalar::dequantize_into(entries, indices, values),
    }
}

/// Dispatch-routed cosine similarity between equal-length slices.
#[must_use]
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    match current() {
        #[cfg(target_arch = "x86_64")]
        DispatchKind::Avx2 => {
            // SAFETY: dispatch::current() verified AVX2+FMA support.
            unsafe { kernels::avx2::cosine(a, b) }
        }
        #[cfg(target_arch = "aarch64")]
        DispatchKind::Neon => {
            // SAFETY: NEON is mandatory on ARMv8 aarch64.
            unsafe { kernels::neon::cosine(a, b) }
        }
        DispatchKind::Scalar => kernels::scalar::cosine(a, b),
    }
}

/// Dispatch-routed residual encoder: writes `original - reconstructed`
/// into `out` as little-endian fp16 bytes.
pub fn compute_residual_into(original: &[f32], reconstructed: &[f32], out: &mut [u8]) {
    match current() {
        #[cfg(target_arch = "x86_64")]
        DispatchKind::Avx2 => {
            // SAFETY: dispatch::current() verified AVX2+FMA support.
            unsafe { kernels::avx2::compute_residual_into(original, reconstructed, out) }
        }
        #[cfg(target_arch = "aarch64")]
        DispatchKind::Neon => {
            // SAFETY: NEON is mandatory on ARMv8 aarch64.
            unsafe { kernels::neon::compute_residual_into(original, reconstructed, out) }
        }
        DispatchKind::Scalar => {
            kernels::scalar::compute_residual_into(original, reconstructed, out);
        }
    }
}

/// Dispatch-routed residual decoder: decodes `residual` as fp16 LE and
/// adds in place into `values`.
///
/// # Errors
///
/// See [`crate::codec::kernels::scalar::apply_residual_into`].
pub fn apply_residual_into(values: &mut [f32], residual: &[u8]) -> Result<(), CodecError> {
    match current() {
        #[cfg(target_arch = "x86_64")]
        DispatchKind::Avx2 => {
            // SAFETY: dispatch::current() verified AVX2+FMA support.
            unsafe { kernels::avx2::apply_residual_into(values, residual) }
        }
        #[cfg(target_arch = "aarch64")]
        DispatchKind::Neon => {
            // SAFETY: NEON is mandatory on ARMv8 aarch64.
            unsafe { kernels::neon::apply_residual_into(values, residual) }
        }
        DispatchKind::Scalar => kernels::scalar::apply_residual_into(values, residual),
    }
}
