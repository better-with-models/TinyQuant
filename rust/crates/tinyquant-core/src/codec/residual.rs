//! FP16 residual helpers (Phase 15).
//!
//! `compute_residual(original, reconstructed) = (original - reconstructed)` stored
//! as little-endian IEEE 754 binary16, matching Python's
//! `(original - reconstructed).astype(np.float16).tobytes()`.
//!
//! `apply_residual_into` adds the decoded residual back into a reconstruction buffer
//! in place.

use crate::errors::CodecError;
use alloc::vec::Vec;
use half::f16;

/// Compute the per-element residual `original - reconstructed` as little-endian f16 bytes.
///
/// The output has length `original.len() * 2`. Caller guarantees `original.len() ==
/// reconstructed.len()`.
#[must_use]
pub fn compute_residual(original: &[f32], reconstructed: &[f32]) -> Vec<u8> {
    debug_assert_eq!(original.len(), reconstructed.len());
    let mut out = Vec::with_capacity(original.len() * 2);
    for (o, r) in original.iter().zip(reconstructed.iter()) {
        let diff = f16::from_f32(*o - *r);
        out.extend_from_slice(&diff.to_le_bytes());
    }
    out
}

/// Decode a residual byte-slice as f16 LE values and add them in place into `values`.
///
/// Returns [`CodecError::LengthMismatch`] if `residual.len() != values.len() * 2`.
///
/// # Errors
///
/// Returns [`CodecError::LengthMismatch`] when the residual buffer has the wrong length.
pub fn apply_residual_into(values: &mut [f32], residual: &[u8]) -> Result<(), CodecError> {
    let expected = values.len() * 2;
    if residual.len() != expected {
        return Err(CodecError::LengthMismatch {
            left: residual.len(),
            right: expected,
        });
    }
    for (i, chunk) in residual.chunks_exact(2).enumerate() {
        let bits = u16::from_le_bytes([chunk[0], chunk[1]]);
        values[i] += f16::from_bits(bits).to_f32();
    }
    Ok(())
}
