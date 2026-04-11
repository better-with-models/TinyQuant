//! In-memory `CompressedVector` value object (Phase 15).
//!
//! Serialization (`to_bytes` / `from_bytes`) arrives in Phase 16 inside
//! `tinyquant-io`. Field layout is intentionally stable for that transition.
//!
//! One byte per dimension in `indices` (upper bits zero for `bit_width < 8`);
//! when present, `residual` is exactly `2 * dimension` bytes of f16 LE.

use crate::codec::codec_config::SUPPORTED_BIT_WIDTHS;
use crate::errors::CodecError;
use crate::types::ConfigHash;
use alloc::boxed::Box;
use alloc::sync::Arc;

/// Immutable per-vector compressed payload. Mirrors Python `CompressedVector`.
///
/// # Invariants
///
/// - `indices.len() == dimension as usize`
/// - `bit_width ∈ {2, 4, 8}`
/// - If `residual.is_some()`, `residual.len() == dimension as usize * 2`
#[derive(Clone, Debug)]
pub struct CompressedVector {
    indices: Arc<[u8]>,
    residual: Option<Arc<[u8]>>,
    config_hash: ConfigHash,
    dimension: u32,
    bit_width: u8,
}

impl CompressedVector {
    /// Validated constructor. Mirrors Python `CompressedVector.__post_init__`.
    ///
    /// # Errors
    ///
    /// - [`CodecError::UnsupportedBitWidth`] if `bit_width ∉ {2, 4, 8}`
    /// - [`CodecError::DimensionMismatch`] if `indices.len() != dimension`
    /// - [`CodecError::LengthMismatch`] if `residual.len() != dimension * 2`
    pub fn new(
        indices: Box<[u8]>,
        residual: Option<Box<[u8]>>,
        config_hash: ConfigHash,
        dimension: u32,
        bit_width: u8,
    ) -> Result<Self, CodecError> {
        if !SUPPORTED_BIT_WIDTHS.contains(&bit_width) {
            return Err(CodecError::UnsupportedBitWidth { got: bit_width });
        }
        // Dimensions are bounded by u32 in CodecConfig; casting is safe in practice
        // and checked by the mismatch error path below.
        #[allow(clippy::cast_possible_truncation)]
        let got_dim = indices.len() as u32;
        if got_dim != dimension {
            return Err(CodecError::DimensionMismatch {
                expected: dimension,
                got: got_dim,
            });
        }
        if let Some(r) = residual.as_ref() {
            let expected = indices.len() * 2;
            if r.len() != expected {
                return Err(CodecError::LengthMismatch {
                    left: r.len(),
                    right: expected,
                });
            }
        }
        Ok(Self {
            indices: Arc::from(indices),
            residual: residual.map(Arc::from),
            config_hash,
            dimension,
            bit_width,
        })
    }

    /// Raw index bytes — one byte per dimension (upper bits zero for `bit_width < 8`).
    #[inline]
    #[must_use]
    pub fn indices(&self) -> &[u8] {
        &self.indices
    }

    /// Optional f16 LE residual bytes — `2 * dimension` bytes when present.
    #[inline]
    #[must_use]
    pub fn residual(&self) -> Option<&[u8]> {
        self.residual.as_deref()
    }

    /// The `config_hash` of the [`CodecConfig`](crate::codec::CodecConfig) used to compress this vector.
    #[inline]
    #[must_use]
    pub const fn config_hash(&self) -> &ConfigHash {
        &self.config_hash
    }

    /// Number of dimensions (= `indices.len()`).
    #[inline]
    #[must_use]
    pub const fn dimension(&self) -> u32 {
        self.dimension
    }

    /// Bit width used during compression.
    #[inline]
    #[must_use]
    pub const fn bit_width(&self) -> u8 {
        self.bit_width
    }

    /// `true` when a residual buffer is present.
    #[inline]
    #[must_use]
    // Option::is_some() is not const-stable in MSRV 1.81.
    #[allow(clippy::missing_const_for_fn)]
    pub fn has_residual(&self) -> bool {
        self.residual.is_some()
    }

    /// Python-parity size estimate: `ceil(dim * bit_width / 8) + residual.len()`.
    ///
    /// This is the *packed* on-disk footprint, not the in-memory footprint
    /// (indices are stored unpacked, 1 byte/dim, until Phase 16).
    #[must_use]
    pub fn size_bytes(&self) -> usize {
        let dim = self.dimension as usize;
        let bw = self.bit_width as usize;
        let packed = (dim * bw + 7) / 8;
        packed + self.residual.as_ref().map_or(0, |r| r.len())
    }
}
