//! napi-rs bindings for the `tinyquant-core` codec layer.
//!
//! Surface mirrors `tinyquant_cpu.codec` (Python) and
//! `rust/crates/tinyquant-py/src/codec.rs` (PyO3) one-for-one, with
//! the camelCase / options-object conventions noted in
//! `docs/plans/rust/phase-25-typescript-npm-package.md` §API parity
//! specification — codec.
//!
//! Phase 25.2 scope: `CodecConfig`, `Codebook`, `CompressedVector`,
//! `RotationMatrix`, `Codec`, plus module-level `compress` /
//! `decompress`. Batch paths (`compressBatch`, `decompressBatch`),
//! residual / indices getters, and the `toBytes` / `fromBytes`
//! round-trip land in later Phase 25 slices.
//!
//! All algorithmic work — SHA-256 canonical hashing, QR rotation,
//! quantile codebook training, residual compression — is delegated
//! to `tinyquant-core`; this file is intentionally a thin FFI seam.

use napi::bindgen_prelude::{BigInt, Float32Array, Uint8Array};
use napi::{Error as NapiError, Status};
use napi_derive::napi;

use tinyquant_core::codec::{
    compress as core_compress, decompress as core_decompress, Codebook as CoreCodebook,
    Codec as CoreCodec, CodecConfig as CoreCodecConfig, CompressedVector as CoreCompressedVector,
    RotationMatrix as CoreRotationMatrix,
};

use crate::buffers::{float32_array_to_vec, vec_to_float32_array};
use crate::errors::map_codec_error;

// ---------------------------------------------------------------------------
// CodecConfigOpts — napi(object) options bag
// ---------------------------------------------------------------------------

/// Options-object mirror of `tinyquant_cpu.codec.CodecConfig(...)`.
///
/// Matches the TypeScript shape in
/// `javascript/@tinyquant/core/src/_loader.ts :: CodecConfigOpts`.
/// `seed` is `bigint` on the JS side because `u64` cannot round-trip
/// through JS `number` losslessly.
#[napi(object)]
pub struct CodecConfigOpts {
    pub bit_width: u32,
    pub seed: BigInt,
    pub dimension: u32,
    pub residual_enabled: Option<bool>,
}

/// Extract a `u64` seed from a `BigInt`, erroring if the JS value
/// was negative or exceeded `u64::MAX`.
fn bigint_to_u64(b: &BigInt) -> napi::Result<u64> {
    let (sign_bit, value, lossless) = b.get_u64();
    if sign_bit {
        return Err(NapiError::new(
            Status::InvalidArg,
            "seed must be a non-negative BigInt".to_string(),
        ));
    }
    if !lossless {
        return Err(NapiError::new(
            Status::InvalidArg,
            "seed BigInt exceeds u64::MAX".to_string(),
        ));
    }
    Ok(value)
}

/// Narrow a JS `number`-sourced `u32` into a `u8` bit width, raising
/// a clean FFI error if the caller passed something out of range.
fn u32_to_u8_bit_width(v: u32) -> napi::Result<u8> {
    u8::try_from(v).map_err(|_| {
        NapiError::new(
            Status::InvalidArg,
            format!("bitWidth {v} is out of range for u8"),
        )
    })
}

// ---------------------------------------------------------------------------
// CodecConfig
// ---------------------------------------------------------------------------

/// Immutable value object describing codec parameters.
#[napi]
pub struct CodecConfig {
    pub(crate) inner: CoreCodecConfig,
}

#[napi]
impl CodecConfig {
    #[napi(constructor)]
    pub fn new(opts: CodecConfigOpts) -> napi::Result<Self> {
        let seed = bigint_to_u64(&opts.seed)?;
        let bit_width = u32_to_u8_bit_width(opts.bit_width)?;
        let residual_enabled = opts.residual_enabled.unwrap_or(true);
        let inner = CoreCodecConfig::new(bit_width, seed, opts.dimension, residual_enabled)
            .map_err(map_codec_error)?;
        Ok(Self { inner })
    }

    #[napi(getter)]
    pub fn bit_width(&self) -> u32 {
        u32::from(self.inner.bit_width())
    }

    #[napi(getter)]
    pub fn seed(&self) -> BigInt {
        BigInt {
            sign_bit: false,
            words: std::vec![self.inner.seed()],
        }
    }

    #[napi(getter)]
    pub fn dimension(&self) -> u32 {
        self.inner.dimension()
    }

    #[napi(getter)]
    pub fn residual_enabled(&self) -> bool {
        self.inner.residual_enabled()
    }

    #[napi(getter)]
    pub fn num_codebook_entries(&self) -> u32 {
        self.inner.num_codebook_entries()
    }

    /// 64-hex SHA-256 digest of the canonical config string. Byte-identical
    /// to Python `tinyquant_cpu.codec.CodecConfig.config_hash` and to the
    /// Rust-core `CodecConfig::config_hash`.
    #[napi(getter)]
    pub fn config_hash(&self) -> String {
        self.inner.config_hash().to_string()
    }
}

// ---------------------------------------------------------------------------
// Codebook
// ---------------------------------------------------------------------------

/// Immutable lookup table mapping quantized indices to float32 values.
#[napi]
pub struct Codebook {
    pub(crate) inner: CoreCodebook,
}

#[napi]
impl Codebook {
    /// Train a codebook from a flat calibration buffer of size
    /// `n * dimension`. `n` is inferred from the length.
    #[napi(factory)]
    pub fn train(vectors: Float32Array, config: &CodecConfig) -> napi::Result<Self> {
        let dim = config.inner.dimension() as usize;
        if dim == 0 {
            return Err(NapiError::new(
                Status::InvalidArg,
                "config dimension must be > 0".to_string(),
            ));
        }
        let slice: &[f32] = vectors.as_ref();
        if slice.len() % dim != 0 {
            return Err(NapiError::new(
                Status::InvalidArg,
                format!(
                    "calibration buffer length {} is not a multiple of dimension {}",
                    slice.len(),
                    dim
                ),
            ));
        }
        let inner = CoreCodebook::train(slice, &config.inner).map_err(map_codec_error)?;
        Ok(Self { inner })
    }

    #[napi(getter)]
    pub fn bit_width(&self) -> u32 {
        u32::from(self.inner.bit_width())
    }

    #[napi(getter)]
    pub fn num_entries(&self) -> u32 {
        self.inner.num_entries()
    }

    /// Copy the codebook's FP32 entries into a fresh `Float32Array`.
    ///
    /// Exposed as a method rather than a getter because it allocates
    /// and memcpy's the full entries buffer on every call. Callers
    /// should hoist the result out of hot loops.
    #[napi]
    pub fn entries(&self) -> Float32Array {
        let v: Vec<f32> = self.inner.entries().to_vec();
        Float32Array::new(v)
    }
}

// ---------------------------------------------------------------------------
// RotationMatrix
// ---------------------------------------------------------------------------

/// Deterministic orthogonal rotation matrix for vector preconditioning.
///
/// Phase 25.2 only exposes the `fromConfig` factory + identity getters;
/// `apply` / `applyInverse` / `verifyOrthogonality` land in a later slice.
#[napi]
pub struct RotationMatrix {
    inner: CoreRotationMatrix,
}

#[napi]
impl RotationMatrix {
    #[napi(factory)]
    pub fn from_config(config: &CodecConfig) -> Self {
        Self {
            inner: CoreRotationMatrix::from_config(&config.inner),
        }
    }

    #[napi(getter)]
    pub fn seed(&self) -> BigInt {
        BigInt {
            sign_bit: false,
            words: std::vec![self.inner.seed()],
        }
    }

    #[napi(getter)]
    pub fn dimension(&self) -> u32 {
        self.inner.dimension()
    }
}

// ---------------------------------------------------------------------------
// CompressedVector
// ---------------------------------------------------------------------------

/// Immutable output of a codec compression pass.
#[napi]
pub struct CompressedVector {
    pub(crate) inner: CoreCompressedVector,
}

#[napi]
impl CompressedVector {
    #[napi(getter)]
    pub fn config_hash(&self) -> String {
        self.inner.config_hash().to_string()
    }

    #[napi(getter)]
    pub fn dimension(&self) -> u32 {
        self.inner.dimension()
    }

    #[napi(getter)]
    pub fn bit_width(&self) -> u32 {
        u32::from(self.inner.bit_width())
    }

    #[napi(getter)]
    pub fn has_residual(&self) -> bool {
        self.inner.has_residual()
    }

    #[napi(getter)]
    pub fn size_bytes(&self) -> napi::Result<u32> {
        // `size_bytes` returns `usize`; surface a clean FFI error if
        // the buffer somehow exceeds `u32::MAX` rather than silently
        // truncating. JS `number` is a 53-bit safe integer, so u32 is
        // always safe once the conversion succeeds.
        let size_bytes = self.inner.size_bytes();
        u32::try_from(size_bytes).map_err(|_| {
            NapiError::new(
                Status::InvalidArg,
                format!("InvalidArg: sizeBytes {size_bytes} exceeds u32::MAX"),
            )
        })
    }

    /// Copy the packed index buffer into a fresh `Uint8Array`.
    ///
    /// Method, not a getter: each call allocates and memcpy's the
    /// full `sizeBytes` buffer. Callers that iterate should hoist
    /// the result out of tight loops.
    #[napi]
    pub fn indices(&self) -> Uint8Array {
        Uint8Array::new(self.inner.indices().to_vec())
    }
}

// ---------------------------------------------------------------------------
// Codec service
// ---------------------------------------------------------------------------

/// Stateless codec service — wraps `tinyquant_core::codec::Codec`.
#[napi]
pub struct Codec;

#[napi]
impl Codec {
    #[napi(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }

    #[napi]
    pub fn compress(
        &self,
        vector: Float32Array,
        config: &CodecConfig,
        codebook: &Codebook,
    ) -> napi::Result<CompressedVector> {
        let buf = float32_array_to_vec(&vector);
        // CoreCodec is stateless as of phase-20; per-call construction is intentional.
        let cv = CoreCodec::new()
            .compress(&buf, &config.inner, &codebook.inner)
            .map_err(map_codec_error)?;
        Ok(CompressedVector { inner: cv })
    }

    #[napi]
    pub fn decompress(
        &self,
        compressed: &CompressedVector,
        config: &CodecConfig,
        codebook: &Codebook,
    ) -> napi::Result<Float32Array> {
        // CoreCodec is stateless as of phase-20; per-call construction is intentional.
        let out = CoreCodec::new()
            .decompress(&compressed.inner, &config.inner, &codebook.inner)
            .map_err(map_codec_error)?;
        Ok(vec_to_float32_array(out))
    }
}

// ---------------------------------------------------------------------------
// Module-level convenience functions
// ---------------------------------------------------------------------------

/// Module-level convenience — equivalent to `new Codec().compress(...)`.
#[napi]
pub fn compress(
    vector: Float32Array,
    config: &CodecConfig,
    codebook: &Codebook,
) -> napi::Result<CompressedVector> {
    let buf = float32_array_to_vec(&vector);
    let cv = core_compress(&buf, &config.inner, &codebook.inner).map_err(map_codec_error)?;
    Ok(CompressedVector { inner: cv })
}

/// Module-level convenience — equivalent to `new Codec().decompress(...)`.
#[napi]
pub fn decompress(
    compressed: &CompressedVector,
    config: &CodecConfig,
    codebook: &Codebook,
) -> napi::Result<Float32Array> {
    let out = core_decompress(&compressed.inner, &config.inner, &codebook.inner)
        .map_err(map_codec_error)?;
    Ok(vec_to_float32_array(out))
}
