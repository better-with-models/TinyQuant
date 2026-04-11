//! Stateless `Codec` service (Phase 15).
//!
//! Pipeline mirrors `tinyquant_cpu.codec.Codec` exactly:
//!
//! - **compress**: rotate → quantize → (optional) residual on rotated vs reconstructed
//! - **decompress**: dequantize → (optional) add residual → inverse rotate

use crate::codec::{
    codebook::Codebook,
    codec_config::CodecConfig,
    compressed_vector::CompressedVector,
    parallelism::Parallelism,
    residual::{apply_residual_into, compute_residual},
    rotation_matrix::RotationMatrix,
};
use crate::errors::CodecError;
use alloc::{vec, vec::Vec};

/// Zero-sized stateless codec service. Mirrors Python `tinyquant_cpu.codec.Codec`.
#[derive(Default, Debug, Clone, Copy)]
pub struct Codec;

impl Codec {
    /// Create a new `Codec` instance (zero allocation).
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Compress a single vector. `vector.len()` must equal `config.dimension()`.
    ///
    /// Pipeline: rotate → quantize → (optional) residual.
    ///
    /// # Errors
    ///
    /// - [`CodecError::DimensionMismatch`] if `vector.len() != config.dimension()`
    /// - [`CodecError::CodebookIncompatible`] if `codebook.bit_width() != config.bit_width()`
    pub fn compress(
        &self,
        vector: &[f32],
        config: &CodecConfig,
        codebook: &Codebook,
    ) -> Result<CompressedVector, CodecError> {
        let dim = config.dimension() as usize;
        if vector.len() != dim {
            return Err(CodecError::DimensionMismatch {
                expected: config.dimension(),
                got: vector.len() as u32,
            });
        }
        if codebook.bit_width() != config.bit_width() {
            return Err(CodecError::CodebookIncompatible {
                expected: config.bit_width(),
                got: codebook.bit_width(),
            });
        }

        let rotation = RotationMatrix::from_config(config);
        let mut rotated = vec![0.0_f32; dim];
        rotation.apply_into(vector, &mut rotated)?;

        let mut indices = vec![0_u8; dim];
        codebook.quantize_into(&rotated, &mut indices)?;

        let residual = if config.residual_enabled() {
            let mut reconstructed = vec![0.0_f32; dim];
            codebook.dequantize_into(&indices, &mut reconstructed)?;
            Some(compute_residual(&rotated, &reconstructed).into_boxed_slice())
        } else {
            None
        };

        CompressedVector::new(
            indices.into_boxed_slice(),
            residual,
            config.config_hash().clone(),
            config.dimension(),
            config.bit_width(),
        )
    }

    /// Allocating decompress — returns a new `Vec<f32>`.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`Self::decompress_into`].
    pub fn decompress(
        &self,
        compressed: &CompressedVector,
        config: &CodecConfig,
        codebook: &Codebook,
    ) -> Result<Vec<f32>, CodecError> {
        let mut out = vec![0.0_f32; config.dimension() as usize];
        self.decompress_into(compressed, config, codebook, &mut out)?;
        Ok(out)
    }

    /// In-place decompress into caller-supplied buffer.
    ///
    /// Pipeline: dequantize → (optional) apply residual → inverse rotate.
    ///
    /// # Errors
    ///
    /// - [`CodecError::ConfigMismatch`] if `compressed.config_hash() != config.config_hash()`
    /// - [`CodecError::CodebookIncompatible`] on bit-width mismatch
    /// - [`CodecError::DimensionMismatch`] if `output.len() != config.dimension()`
    pub fn decompress_into(
        &self,
        compressed: &CompressedVector,
        config: &CodecConfig,
        codebook: &Codebook,
        output: &mut [f32],
    ) -> Result<(), CodecError> {
        if compressed.config_hash() != config.config_hash() {
            return Err(CodecError::ConfigMismatch {
                expected: config.config_hash().clone(),
                got: compressed.config_hash().clone(),
            });
        }
        if compressed.bit_width() != config.bit_width() {
            return Err(CodecError::CodebookIncompatible {
                expected: config.bit_width(),
                got: compressed.bit_width(),
            });
        }
        if codebook.bit_width() != config.bit_width() {
            return Err(CodecError::CodebookIncompatible {
                expected: config.bit_width(),
                got: codebook.bit_width(),
            });
        }
        if output.len() != config.dimension() as usize {
            return Err(CodecError::DimensionMismatch {
                expected: config.dimension(),
                got: output.len() as u32,
            });
        }

        let mut rotated = vec![0.0_f32; output.len()];
        codebook.dequantize_into(compressed.indices(), &mut rotated)?;

        if let Some(residual) = compressed.residual() {
            apply_residual_into(&mut rotated, residual)?;
        }

        let rotation = RotationMatrix::from_config(config);
        rotation.apply_inverse_into(&rotated, output)
    }

    /// Row-major batch compress using the serial strategy.
    ///
    /// # Errors
    ///
    /// - [`CodecError::DimensionMismatch`] if `cols != config.dimension()`
    /// - [`CodecError::LengthMismatch`] if `vectors.len() != rows * cols`
    pub fn compress_batch(
        &self,
        vectors: &[f32],
        rows: usize,
        cols: usize,
        config: &CodecConfig,
        codebook: &Codebook,
    ) -> Result<Vec<CompressedVector>, CodecError> {
        self.compress_batch_with(vectors, rows, cols, config, codebook, Parallelism::Serial)
    }

    /// Row-major batch compress with explicit parallelism strategy.
    ///
    /// Phase 15: serial only. Phase 21 will honour `parallelism`.
    ///
    /// # Errors
    ///
    /// Same as [`Self::compress_batch`].
    pub fn compress_batch_with(
        &self,
        vectors: &[f32],
        rows: usize,
        cols: usize,
        config: &CodecConfig,
        codebook: &Codebook,
        _parallelism: Parallelism,
    ) -> Result<Vec<CompressedVector>, CodecError> {
        if cols != config.dimension() as usize {
            return Err(CodecError::DimensionMismatch {
                expected: config.dimension(),
                got: cols as u32,
            });
        }
        if vectors.len() != rows * cols {
            return Err(CodecError::LengthMismatch {
                left: vectors.len(),
                right: rows * cols,
            });
        }
        let mut out = Vec::with_capacity(rows);
        for row in 0..rows {
            let start = row * cols;
            out.push(self.compress(&vectors[start..start + cols], config, codebook)?);
        }
        Ok(out)
    }

    /// Batch decompress into a contiguous row-major `output` buffer.
    ///
    /// # Errors
    ///
    /// - [`CodecError::LengthMismatch`] if `output.len() != compressed.len() * config.dimension()`
    /// - Propagates per-vector decompress errors.
    pub fn decompress_batch_into(
        &self,
        compressed: &[CompressedVector],
        config: &CodecConfig,
        codebook: &Codebook,
        output: &mut [f32],
    ) -> Result<(), CodecError> {
        let cols = config.dimension() as usize;
        let needed = compressed.len() * cols;
        if output.len() != needed {
            return Err(CodecError::LengthMismatch {
                left: output.len(),
                right: needed,
            });
        }
        for (row, cv) in compressed.iter().enumerate() {
            let start = row * cols;
            self.decompress_into(cv, config, codebook, &mut output[start..start + cols])?;
        }
        Ok(())
    }
}

/// Module-level `compress` free function — mirrors `tinyquant_cpu.codec.compress`.
///
/// # Errors
///
/// Propagates errors from [`Codec::compress`].
pub fn compress(
    vector: &[f32],
    config: &CodecConfig,
    codebook: &Codebook,
) -> Result<CompressedVector, CodecError> {
    Codec::new().compress(vector, config, codebook)
}

/// Module-level `decompress` free function — mirrors `tinyquant_cpu.codec.decompress`.
///
/// # Errors
///
/// Propagates errors from [`Codec::decompress`].
pub fn decompress(
    compressed: &CompressedVector,
    config: &CodecConfig,
    codebook: &Codebook,
) -> Result<Vec<f32>, CodecError> {
    Codec::new().decompress(compressed, config, codebook)
}
