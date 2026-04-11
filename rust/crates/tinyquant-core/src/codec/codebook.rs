//! Immutable quantization lookup table with Python-parity training.
//!
//! A [`Codebook`] owns `2^bit_width` sorted, strictly-ascending `f32`
//! entries — one representative value per quantized index. It mirrors
//! `tinyquant_cpu.codec.codebook.Codebook` field-for-field and produces
//! byte-identical output on the full training + quantize + dequantize
//! pipeline for the same inputs.
//!
//! The algorithmic contract is fixed by
//! `docs/design/rust/numerical-semantics.md` §Quantization and
//! §Codebook training. Changing anything here requires regenerating
//! the frozen fixtures under `tests/fixtures/codebook/` and
//! `tests/fixtures/quantize/` via `cargo xtask fixtures refresh-all`.
//!
//! Under `feature = "simd"` the quantize / dequantize methods route
//! through [`crate::codec::simd_api`], which is the single source of
//! truth for dispatch selection. No `unsafe` intrinsics are invoked
//! directly from this file, but the file-level `#![allow(unsafe_code)]`
//! is retained so that future SIMD specialisation (Phase 22) can add
//! direct intrinsic paths here without re-plumbing the lint surface.
#![allow(unsafe_code)]

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt;

use crate::codec::codec_config::CodecConfig;
#[cfg(not(feature = "simd"))]
use crate::codec::kernels::scalar as scalar_kernel;
use crate::errors::CodecError;

/// Immutable lookup table mapping quantized `u8` indices to `f32` values.
///
/// Construction always validates three invariants:
///
/// 1. `entries.len() == 1 << bit_width` (matches `CodecConfig::num_codebook_entries`).
/// 2. Entries are strictly ascending under `f32::total_cmp`.
/// 3. All entries are distinct (no adjacent equals).
///
/// The inner buffer is an `Arc<[f32]>` so `Clone` is `O(1)`. Equality
/// compares the numerical contents, not the allocation identity.
#[derive(Clone)]
pub struct Codebook {
    entries: Arc<[f32]>,
    bit_width: u8,
}

impl Codebook {
    /// Build a codebook from a caller-owned `Box<[f32]>`.
    ///
    /// # Errors
    ///
    /// * [`CodecError::CodebookEntryCount`] — `entries.len()` does not
    ///   equal `2^bit_width`.
    /// * [`CodecError::CodebookNotSorted`] — entries are not in
    ///   strictly ascending order (excluding duplicates).
    /// * [`CodecError::CodebookDuplicate`] — two adjacent entries
    ///   compare equal.
    pub fn new(entries: Box<[f32]>, bit_width: u8) -> Result<Self, CodecError> {
        let expected = 1u32
            .checked_shl(u32::from(bit_width))
            .ok_or(CodecError::UnsupportedBitWidth { got: bit_width })?;
        let got = u32::try_from(entries.len()).map_err(|_| CodecError::CodebookEntryCount {
            expected,
            got: u32::MAX,
            bit_width,
        })?;
        if got != expected {
            return Err(CodecError::CodebookEntryCount {
                expected,
                got,
                bit_width,
            });
        }

        // Validate monotonicity and distinctness in a single pass without
        // indexing, to satisfy `clippy::indexing_slicing`.
        let mut distinct: u32 = u32::from(!entries.is_empty());
        let mut prev: Option<f32> = None;
        for &value in &*entries {
            if let Some(p) = prev {
                match f32::total_cmp(&p, &value) {
                    Ordering::Less => distinct += 1,
                    Ordering::Equal => {
                        // Count distinct entries; `expected - distinct`
                        // is the minimum number of duplicates we must
                        // still resolve to make the codebook well-formed.
                        return Err(CodecError::CodebookDuplicate {
                            expected,
                            got: distinct,
                        });
                    }
                    Ordering::Greater => return Err(CodecError::CodebookNotSorted),
                }
            }
            prev = Some(value);
        }

        Ok(Self {
            entries: Arc::from(entries),
            bit_width,
        })
    }

    /// Train a codebook by uniform-quantile estimation over a flattened
    /// f32 sample buffer.
    ///
    /// Mirrors Python's
    /// `np.quantile(flat.astype(np.float64),
    /// np.linspace(0, 1, num_entries)).astype(np.float32)` exactly:
    ///
    /// 1. Promote every sample to `f64`.
    /// 2. Sort with `f64::total_cmp`.
    /// 3. For each `k` in `0..num_entries`, compute the linearly-
    ///    interpolated quantile value in `f64`.
    /// 4. Cast to `f32` (round-to-nearest-even) and enforce distinctness.
    ///
    /// `config.bit_width` determines the number of entries; `config.seed`
    /// and `config.dimension` are not consulted by this function.
    ///
    /// # Errors
    ///
    /// * [`CodecError::InsufficientTrainingData`] — `vectors` is empty
    ///   or produces fewer than `num_entries` distinct quantile
    ///   representatives.
    /// * Any error from [`Codebook::new`] on the freshly-built entries.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn train(vectors: &[f32], config: &CodecConfig) -> Result<Self, CodecError> {
        let num_entries_u32 = config.num_codebook_entries();
        let num_entries = num_entries_u32 as usize;
        if vectors.is_empty() {
            return Err(CodecError::InsufficientTrainingData {
                expected: num_entries_u32,
            });
        }

        // Step 1: promote to f64.
        let mut flat: Vec<f64> = vectors.iter().copied().map(f64::from).collect();

        // Step 2: sort with total ordering (matches NumPy's sort on non-
        // NaN data; also makes NaNs deterministic if they ever sneak in).
        flat.sort_by(f64::total_cmp);

        // Step 3: quantile interpolation.
        let len = flat.len();
        let last_idx = len.saturating_sub(1);
        let num_entries_minus_one =
            num_entries
                .checked_sub(1)
                .ok_or(CodecError::InsufficientTrainingData {
                    expected: num_entries_u32,
                })?;
        let divisor = num_entries_minus_one as f64;
        let span = last_idx as f64;

        let mut entries_f32: Vec<f32> = Vec::with_capacity(num_entries);
        for k in 0..num_entries {
            let q = (k as f64) / divisor;
            let h = q * span;
            let floor_h = libm::floor(h);
            let frac = h - floor_h;
            let i = floor_h as usize;
            let i_plus_one = i.saturating_add(1).min(last_idx);
            let lo = *flat.get(i).ok_or(CodecError::InsufficientTrainingData {
                expected: num_entries_u32,
            })?;
            let hi = *flat
                .get(i_plus_one)
                .ok_or(CodecError::InsufficientTrainingData {
                    expected: num_entries_u32,
                })?;
            let value_f64 = lo + frac * (hi - lo);
            entries_f32.push(value_f64 as f32);
        }

        // Step 4: defensive sort (NumPy's linspace-driven quantiles are
        // already monotone, but explicit sorting guards against any
        // future change to the interpolation recipe).
        entries_f32.sort_by(f32::total_cmp);

        // Distinctness check — surface a dedicated error before handing
        // off to `Codebook::new`, because the insufficient-training-data
        // story is more informative than a raw duplicate-entries error
        // in this context.
        let mut distinct: u32 = 1;
        let mut iter = entries_f32.iter();
        if let Some(first) = iter.next() {
            let mut prev = *first;
            for &value in iter {
                if f32::total_cmp(&prev, &value) == Ordering::Less {
                    distinct += 1;
                }
                prev = value;
            }
        }
        if distinct < num_entries_u32 {
            return Err(CodecError::InsufficientTrainingData {
                expected: num_entries_u32,
            });
        }

        // Delegating to `Codebook::new` re-runs the invariant checks so
        // the constructor stays the single source of truth.
        Self::new(entries_f32.into_boxed_slice(), config.bit_width())
    }

    /// Number of entries (`2^bit_width`).
    #[inline]
    pub fn num_entries(&self) -> u32 {
        // Constructor guarantees the length fits in `u32`.
        u32::try_from(self.entries.len()).unwrap_or(u32::MAX)
    }

    /// The bit width this codebook was built for.
    #[inline]
    pub const fn bit_width(&self) -> u8 {
        self.bit_width
    }

    /// Borrow the underlying sorted entries.
    #[inline]
    pub fn entries(&self) -> &[f32] {
        &self.entries
    }

    /// Quantize `values` into `indices` by finding the nearest entry for
    /// each value. Ties favor the right (higher-valued) neighbor, matching
    /// Python's strict `<` tie-break.
    ///
    /// Under `feature = "simd"` this delegates to
    /// [`crate::codec::simd_api::quantize_into`], which is the single
    /// source of truth for dispatch selection. Without the feature,
    /// it calls the scalar reference kernel directly.
    ///
    /// # Errors
    ///
    /// * [`CodecError::LengthMismatch`] — `values.len() != indices.len()`.
    pub fn quantize_into(&self, values: &[f32], indices: &mut [u8]) -> Result<(), CodecError> {
        let entries = &self.entries;
        #[cfg(feature = "simd")]
        {
            crate::codec::simd_api::quantize_into(entries, values, indices)
        }
        #[cfg(not(feature = "simd"))]
        {
            scalar_kernel::quantize_into(entries, values, indices)
        }
    }

    /// Dequantize `indices` into `values` by gathering the corresponding
    /// codebook entries.
    ///
    /// Under `feature = "simd"` this delegates to
    /// [`crate::codec::simd_api::dequantize_into`].
    ///
    /// # Errors
    ///
    /// * [`CodecError::LengthMismatch`] — `indices.len() != values.len()`.
    /// * [`CodecError::IndexOutOfRange`] — any index is
    ///   `>= num_entries()`.
    pub fn dequantize_into(&self, indices: &[u8], values: &mut [f32]) -> Result<(), CodecError> {
        let entries = &self.entries;
        #[cfg(feature = "simd")]
        {
            crate::codec::simd_api::dequantize_into(entries, indices, values)
        }
        #[cfg(not(feature = "simd"))]
        {
            scalar_kernel::dequantize_into(entries, indices, values)
        }
    }

    /// Convenience: allocate and return the quantized indices.
    ///
    /// # Errors
    ///
    /// See [`Codebook::quantize_into`].
    pub fn quantize(&self, values: &[f32]) -> Result<Vec<u8>, CodecError> {
        let mut out = vec![0u8; values.len()];
        self.quantize_into(values, &mut out)?;
        Ok(out)
    }

    /// Convenience: allocate and return the dequantized values.
    ///
    /// # Errors
    ///
    /// See [`Codebook::dequantize_into`].
    pub fn dequantize(&self, indices: &[u8]) -> Result<Vec<f32>, CodecError> {
        let mut out = vec![0.0f32; indices.len()];
        self.dequantize_into(indices, &mut out)?;
        Ok(out)
    }
}

impl fmt::Debug for Codebook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Codebook")
            .field("bit_width", &self.bit_width)
            .field("num_entries", &self.num_entries())
            .field("entries", &self.entries)
            .finish()
    }
}

impl PartialEq for Codebook {
    fn eq(&self, other: &Self) -> bool {
        if self.bit_width != other.bit_width {
            return false;
        }
        if self.entries.len() != other.entries.len() {
            return false;
        }
        self.entries
            .iter()
            .zip(other.entries.iter())
            .all(|(a, b)| a.to_bits() == b.to_bits())
    }
}
