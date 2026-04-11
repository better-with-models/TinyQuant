//! Scalar reference kernels for `Codebook::quantize_into` and
//! `Codebook::dequantize_into`.
//!
//! These are the panic-free, lint-clean scalar baselines the future
//! SIMD path (Phase 20) will benchmark against. Both functions take
//! borrowed slices so callers can drive them without a [`Codebook`]
//! when needed (e.g. from cross-check tests).
//!
//! Algorithmic parity:
//!
//! * `scalar_quantize` mirrors
//!   `np.searchsorted(entries, values, side="left")` followed by the
//!   clip-and-compare tie-break from
//!   `tinyquant_cpu.codec.codebook.Codebook.quantize`. Ties favor the
//!   right neighbor because the Python code uses strict `<`
//!   (`left_dist < right_dist`), which is replicated here element by
//!   element.
//! * `scalar_dequantize` is a pure gather with an upfront bounds
//!   check. It matches `entries[indices]` from `NumPy`.

use core::cmp::Ordering;

use crate::errors::CodecError;

/// Quantize every `f32` in `values` against a sorted, strictly-ascending
/// `entries` slice, writing each nearest entry's index into `indices`.
///
/// # Errors
///
/// * [`CodecError::LengthMismatch`] — `values.len() != indices.len()`.
///
/// # Contract
///
/// * `entries` must be strictly ascending and contain between 1 and
///   256 elements inclusive. Callers should obtain the slice from a
///   validated [`crate::codec::Codebook`]; feeding in a raw
///   unverified slice produces implementation-defined output on
///   violation but never panics.
/// * `NaN` inputs fall through to the right-most comparison branch,
///   matching the Python reference's implementation-defined behavior.
pub fn scalar_quantize(
    entries: &[f32],
    values: &[f32],
    indices: &mut [u8],
) -> Result<(), CodecError> {
    if values.len() != indices.len() {
        return Err(CodecError::LengthMismatch {
            left: values.len(),
            right: indices.len(),
        });
    }
    let n = entries.len();
    if n == 0 {
        // No entries to snap to; leave indices untouched.
        return Ok(());
    }
    let last = n - 1;

    for (value, slot) in values.iter().zip(indices.iter_mut()) {
        // Python: `idx = np.searchsorted(entries, v, side="left")`
        // Find the first position where `entries[i] >= v`, or `n`.
        let insertion = entries
            .iter()
            .position(|e| !matches!(f32_cmp(*e, *value), Ordering::Less))
            .unwrap_or(n);

        // `np.clip(idx, 0, n - 1)` then the nearest-neighbor tie-break.
        let right = insertion.min(last);
        let left = right.saturating_sub(1);

        let right_entry = entries.get(right).copied().unwrap_or(f32::NAN);
        let left_entry = entries.get(left).copied().unwrap_or(f32::NAN);

        let ld = libm::fabsf(*value - left_entry);
        let rd = libm::fabsf(*value - right_entry);

        // Strict `<` so ties go to the right neighbor, matching Python's
        // `np.where(left_dist < right_dist, left_idx, idx)`.
        let chosen = if ld < rd { left } else { right };

        // `chosen` is bounded by `last < 256` because `num_entries <=
        // 2^8`. Saturating cast keeps lint happy.
        *slot = u8::try_from(chosen).unwrap_or(u8::MAX);
    }
    Ok(())
}

/// Gather the entries at every index in `indices`, writing them into
/// `values`.
///
/// # Errors
///
/// * [`CodecError::LengthMismatch`] — `indices.len() != values.len()`.
/// * [`CodecError::IndexOutOfRange`] — any index is `>= entries.len()`.
pub fn scalar_dequantize(
    entries: &[f32],
    indices: &[u8],
    values: &mut [f32],
) -> Result<(), CodecError> {
    if indices.len() != values.len() {
        return Err(CodecError::LengthMismatch {
            left: indices.len(),
            right: values.len(),
        });
    }
    let bound = u32::try_from(entries.len()).unwrap_or(u32::MAX);
    for (&idx, slot) in indices.iter().zip(values.iter_mut()) {
        let idx_u32 = u32::from(idx);
        if idx_u32 >= bound {
            return Err(CodecError::IndexOutOfRange { index: idx, bound });
        }
        *slot = entries.get(idx as usize).copied().unwrap_or(f32::NAN);
    }
    Ok(())
}

/// Total-order-friendly comparison that treats NaN as "greater" so the
/// left-hand scan in [`scalar_quantize`] degrades gracefully instead of
/// panicking on `partial_cmp` returning `None`.
fn f32_cmp(a: f32, b: f32) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Greater)
}
