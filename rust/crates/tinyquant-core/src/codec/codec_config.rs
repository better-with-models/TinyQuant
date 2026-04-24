//! `CodecConfig`: immutable value object describing codec parameters.
//!
//! Mirrors `tinyquant_cpu.codec.codec_config.CodecConfig` field-for-field.
//! The canonical `config_hash` format is a SHA-256 digest of a fixed
//! canonical string and matches the Python reference byte-for-byte so
//! artifacts produced by either implementation are interchangeable.
//!
//! The canonical string format is:
//!
//! ```text
//! CodecConfig(bit_width={b},seed={s},dimension={d},residual_enabled={r})
//! ```
//!
//! where `{r}` stringifies the bool as Python's `str(bool)` spelling —
//! `"True"` or `"False"` (capitalized). Deviating from this in any way
//! (spaces, case, field order) breaks parity.

use alloc::format;
use alloc::sync::Arc;

use sha2::{Digest, Sha256};

use crate::errors::CodecError;
use crate::types::ConfigHash;

/// The complete set of quantization bit widths supported by `TinyQuant`.
///
/// Mirrors `tinyquant_cpu.codec.codec_config.SUPPORTED_BIT_WIDTHS`.
pub const SUPPORTED_BIT_WIDTHS: &[u8] = &[2, 4, 8];

/// Maximum supported `dimension` for any codec config or rotation matrix.
///
/// `RotationMatrix::build` allocates `O(dim²)` `f64` storage and runs
/// `O(dim³)` QR. Capping at 16384 bounds peak rotation memory at
/// ~2 GiB and keeps build time tractable on commodity hardware. The
/// largest production embedding sizes (e.g. 4096) sit well below this
/// cap. Phase 28.7 added the cap as defence-in-depth against DoS via
/// the new `RotationMatrix::from_seed_and_dim` PyO3 entry point.
pub const MAX_DIMENSION: u32 = 16_384;

/// Immutable configuration snapshot that fully determines codec behavior.
///
/// Two configs with identical primary fields are interchangeable. The
/// cached `config_hash` is computed eagerly in [`CodecConfig::new`] and
/// ignored by [`PartialEq`] / [`Hash`] so semantically equal configs
/// compare equal regardless of which instance owns the `Arc<str>`.
#[derive(Clone, Debug)]
pub struct CodecConfig {
    bit_width: u8,
    seed: u64,
    dimension: u32,
    residual_enabled: bool,
    config_hash: ConfigHash,
}

impl CodecConfig {
    /// Validate the field invariants and return a new `CodecConfig`.
    ///
    /// # Errors
    ///
    /// * [`CodecError::UnsupportedBitWidth`] — `bit_width` is not in
    ///   [`SUPPORTED_BIT_WIDTHS`].
    /// * [`CodecError::InvalidDimension`] — `dimension == 0`.
    /// * [`CodecError::DimensionTooLarge`] — `dimension > MAX_DIMENSION`.
    pub fn new(
        bit_width: u8,
        seed: u64,
        dimension: u32,
        residual_enabled: bool,
    ) -> Result<Self, CodecError> {
        if !SUPPORTED_BIT_WIDTHS.contains(&bit_width) {
            return Err(CodecError::UnsupportedBitWidth { got: bit_width });
        }
        if dimension == 0 {
            return Err(CodecError::InvalidDimension { got: 0 });
        }
        if dimension > MAX_DIMENSION {
            return Err(CodecError::DimensionTooLarge {
                got: dimension,
                max: MAX_DIMENSION,
            });
        }
        let config_hash = compute_config_hash(bit_width, seed, dimension, residual_enabled);
        Ok(Self {
            bit_width,
            seed,
            dimension,
            residual_enabled,
            config_hash,
        })
    }

    /// The bit width of the quantized indices.
    #[inline]
    pub const fn bit_width(&self) -> u8 {
        self.bit_width
    }

    /// The seed used for deterministic rotation and codebook generation.
    #[inline]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// The expected input vector dimensionality.
    #[inline]
    pub const fn dimension(&self) -> u32 {
        self.dimension
    }

    /// Whether stage-2 residual correction is enabled.
    #[inline]
    pub const fn residual_enabled(&self) -> bool {
        self.residual_enabled
    }

    /// `2^bit_width` — the number of quantization levels in the codebook.
    #[inline]
    pub const fn num_codebook_entries(&self) -> u32 {
        1u32 << self.bit_width
    }

    /// Cached SHA-256 hex digest of the canonical string representation.
    ///
    /// Returned as an `Arc<str>` borrow; clone with `.clone()` for owned use.
    #[inline]
    pub const fn config_hash(&self) -> &ConfigHash {
        &self.config_hash
    }
}

impl PartialEq for CodecConfig {
    fn eq(&self, other: &Self) -> bool {
        self.bit_width == other.bit_width
            && self.seed == other.seed
            && self.dimension == other.dimension
            && self.residual_enabled == other.residual_enabled
    }
}

impl Eq for CodecConfig {}

impl core::hash::Hash for CodecConfig {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.bit_width.hash(state);
        self.seed.hash(state);
        self.dimension.hash(state);
        self.residual_enabled.hash(state);
    }
}

/// Compute the canonical SHA-256 `config_hash` for the given field tuple.
///
/// Exposed at module scope (rather than as an associated function) so that
/// tests can verify hash computation without constructing a full config.
/// Kept `pub(crate)` because nothing outside the crate should need to
/// compute a hash without going through [`CodecConfig::new`].
pub(crate) fn compute_config_hash(
    bit_width: u8,
    seed: u64,
    dimension: u32,
    residual_enabled: bool,
) -> ConfigHash {
    // CRITICAL: Python bool stringifies as "True" / "False" (capitalized).
    // See scripts/generate_rust_fixtures.py and the Python reference in
    // src/tinyquant_cpu/codec/codec_config.py.
    let canonical = format!(
        "CodecConfig(bit_width={b},seed={s},dimension={d},residual_enabled={r})",
        b = bit_width,
        s = seed,
        d = dimension,
        r = if residual_enabled { "True" } else { "False" },
    );
    let digest = Sha256::digest(canonical.as_bytes());
    Arc::from(hex::encode(digest).as_str())
}
