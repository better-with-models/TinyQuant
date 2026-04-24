//! Stateless `Codec` service (Phase 15).
//!
//! Pipeline mirrors `tinyquant_cpu.codec.Codec` exactly:
//!
//! - **compress**: rotate → quantize → (optional) residual on rotated vs reconstructed
//! - **decompress**: dequantize → (optional) add residual → inverse rotate
//!
//! Phase 26 adds `compress_prepared` / `decompress_prepared_into` which accept a
//! pre-built [`PreparedCodec`] so the `O(dim²)` rotation factorization is paid
//! only once per session rather than on every call.

use crate::codec::{
    codebook::Codebook,
    codec_config::CodecConfig,
    compressed_vector::CompressedVector,
    parallelism::Parallelism,
    prepared::PreparedCodec,
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
            // Dimension is bounded by u32 (from CodecConfig); cast is safe in practice.
            #[allow(clippy::cast_possible_truncation)]
            let got = vector.len() as u32;
            return Err(CodecError::DimensionMismatch {
                expected: config.dimension(),
                got,
            });
        }
        if codebook.bit_width() != config.bit_width() {
            return Err(CodecError::CodebookIncompatible {
                expected: config.bit_width(),
                got: codebook.bit_width(),
            });
        }
        let rotation = RotationMatrix::from_config(config);
        Self::compress_with_rotation(vector, config, codebook, &rotation)
    }

    /// Compress using a pre-built [`PreparedCodec`] (hot path — rotation not rebuilt).
    ///
    /// Identical output to [`Self::compress`] for the same inputs.
    ///
    /// # Errors
    ///
    /// [`CodecError::DimensionMismatch`] if `vector.len() != prepared.config().dimension()`.
    pub fn compress_prepared(
        &self,
        vector: &[f32],
        prepared: &PreparedCodec,
    ) -> Result<CompressedVector, CodecError> {
        let dim = prepared.config().dimension() as usize;
        if vector.len() != dim {
            #[allow(clippy::cast_possible_truncation)]
            let got = vector.len() as u32;
            return Err(CodecError::DimensionMismatch {
                expected: prepared.config().dimension(),
                got,
            });
        }
        Self::compress_with_rotation(
            vector,
            prepared.config(),
            prepared.codebook(),
            prepared.rotation(),
        )
    }

    /// Inner compute path — caller supplies an already-built rotation.
    ///
    /// Preconditions (asserted by callers):
    /// - `vector.len() == config.dimension()`
    /// - `codebook.bit_width() == config.bit_width()`
    fn compress_with_rotation(
        vector: &[f32],
        config: &CodecConfig,
        codebook: &Codebook,
        rotation: &RotationMatrix,
    ) -> Result<CompressedVector, CodecError> {
        let dim = config.dimension() as usize;
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
            #[allow(clippy::cast_possible_truncation)]
            let got = output.len() as u32;
            return Err(CodecError::DimensionMismatch {
                expected: config.dimension(),
                got,
            });
        }
        let rotation = RotationMatrix::from_config(config);
        Self::decompress_into_with_rotation(compressed, codebook, &rotation, output)
    }

    /// Decompress into a caller-allocated buffer using a pre-built [`PreparedCodec`]
    /// (hot path — rotation not rebuilt).
    ///
    /// Identical output to [`Self::decompress_into`] for the same inputs.
    ///
    /// # Errors
    ///
    /// - [`CodecError::ConfigMismatch`] if `cv.config_hash() != prepared.config().config_hash()`
    /// - [`CodecError::DimensionMismatch`] if `out.len() != prepared.config().dimension()`
    pub fn decompress_prepared_into(
        &self,
        cv: &CompressedVector,
        prepared: &PreparedCodec,
        out: &mut [f32],
    ) -> Result<(), CodecError> {
        if cv.config_hash() != prepared.config().config_hash() {
            return Err(CodecError::ConfigMismatch {
                expected: prepared.config().config_hash().clone(),
                got: cv.config_hash().clone(),
            });
        }
        // cv.bit_width() is not checked separately — the config_hash equality
        // above already covers all config fields including bit_width.  The
        // codebook bit_width is validated once at PreparedCodec::new time.
        if out.len() != prepared.config().dimension() as usize {
            #[allow(clippy::cast_possible_truncation)]
            let got = out.len() as u32;
            return Err(CodecError::DimensionMismatch {
                expected: prepared.config().dimension(),
                got,
            });
        }
        Self::decompress_into_with_rotation(cv, prepared.codebook(), prepared.rotation(), out)
    }

    /// Inner compute path — caller supplies an already-built rotation.
    ///
    /// Preconditions (asserted by callers):
    /// - `output.len() == config dimension`
    /// - `codebook` is compatible with the compressed vector
    fn decompress_into_with_rotation(
        compressed: &CompressedVector,
        codebook: &Codebook,
        rotation: &RotationMatrix,
        output: &mut [f32],
    ) -> Result<(), CodecError> {
        let mut rotated = vec![0.0_f32; output.len()];
        codebook.dequantize_into(compressed.indices(), &mut rotated)?;

        if let Some(residual) = compressed.residual() {
            apply_residual_into(&mut rotated, residual)?;
        }

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
    /// Phase 21: honours `parallelism`. `Serial` runs the existing single-threaded
    /// loop; `Custom(driver)` uses the `MaybeUninit + AtomicPtr<CompressedVector>` parallel path in
    /// `batch.rs` (requires the `std` feature).
    ///
    /// The determinism contract guarantees byte-identical output regardless of
    /// the driver or thread count (see `batch.rs` module doc).
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
        parallelism: Parallelism,
    ) -> Result<Vec<CompressedVector>, CodecError> {
        if cols != config.dimension() as usize {
            #[allow(clippy::cast_possible_truncation)]
            let got = cols as u32;
            return Err(CodecError::DimensionMismatch {
                expected: config.dimension(),
                got,
            });
        }
        let expected_len = rows.checked_mul(cols).ok_or(CodecError::LengthMismatch {
            left: vectors.len(),
            right: usize::MAX,
        })?;
        if vectors.len() != expected_len {
            return Err(CodecError::LengthMismatch {
                left: vectors.len(),
                right: expected_len,
            });
        }
        // Delegate to the parallel batch module when `std` is available.
        #[cfg(feature = "std")]
        {
            crate::codec::batch::compress_batch_parallel(
                vectors,
                rows,
                cols,
                config,
                codebook,
                parallelism,
            )
        }
        // no_std fallback: always serial regardless of `parallelism` argument.
        #[cfg(not(feature = "std"))]
        {
            let _ = parallelism;
            let mut out = Vec::with_capacity(rows);
            // Safety: vectors.len() == rows * cols (checked above); slices are in-bounds.
            #[allow(clippy::indexing_slicing)]
            for row in 0..rows {
                let start = row * cols;
                out.push(self.compress(&vectors[start..start + cols], config, codebook)?);
            }
            Ok(out)
        }
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
        // Safety: output.len() == compressed.len() * cols (checked above); slices are in-bounds.
        #[allow(clippy::indexing_slicing)]
        for (row, cv) in compressed.iter().enumerate() {
            let start = row * cols;
            self.decompress_into(cv, config, codebook, &mut output[start..start + cols])?;
        }
        Ok(())
    }
}

/// Minimum batch size below which GPU offload is not attempted.
///
/// Below this threshold the host↔device transfer overhead exceeds the
/// compute savings.  Mirrors `tinyquant_gpu_wgpu::GPU_BATCH_THRESHOLD`.
pub const GPU_BATCH_THRESHOLD: usize = 512;

/// Trait that every `TinyQuant` GPU compute backend must satisfy.
///
/// Implementations of this trait live in external crates (e.g.
/// `tinyquant-gpu-wgpu`). The trait is defined here so
/// `tinyquant-core` can express the `compress_batch_gpu_with` method
/// without a dependency on any concrete GPU crate (which would create
/// a cyclic crate dependency, since GPU crates already depend on
/// `tinyquant-core`).
pub trait GpuComputeBackend {
    /// The error type returned by this backend's operations.
    ///
    /// Must be convertible to [`CodecError`] so
    /// [`Codec::compress_batch_gpu_with`] can map errors uniformly.
    type Error: core::fmt::Debug + Into<crate::errors::CodecError>;

    /// Upload `PreparedCodec` buffers to device memory. Idempotent.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` if device upload fails.
    fn prepare_for_device(&mut self, prepared: &mut PreparedCodec) -> Result<(), Self::Error>;

    /// Compress `rows` FP32 vectors of dimension `cols` on the GPU.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` if the GPU kernel dispatch or readback fails.
    fn compress_batch(
        &mut self,
        input: &[f32],
        rows: usize,
        cols: usize,
        prepared: &PreparedCodec,
    ) -> Result<alloc::vec::Vec<CompressedVector>, Self::Error>;
}

#[cfg(feature = "gpu-wgpu")]
impl Codec {
    /// Batch compress with automatic GPU routing.
    ///
    /// When `rows >= GPU_BATCH_THRESHOLD` (currently 512), the batch is
    /// dispatched to `backend` via the [`GpuComputeBackend`] trait.
    /// Otherwise the call falls through to the CPU path using `parallelism`.
    ///
    /// # Caller responsibility
    ///
    /// `backend` must have been created and initialized by the caller (which
    /// may require an async executor, depending on the backend). This method
    /// only calls synchronous trait methods on the backend.
    ///
    /// `prepared` is passed by mutable reference so `prepare_for_device` can
    /// attach GPU-resident state (idempotent — safe to call before every batch).
    ///
    /// # Async-context safety
    ///
    /// Do **not** call this function from an async context that owns an active
    /// tokio or other executor on the same thread. `wgpu` may internally call
    /// `device.poll(wgpu::Maintain::Wait)`, which blocks the thread.
    ///
    /// # Errors
    ///
    /// - [`CodecError::GpuUnavailable`] if `prepare_for_device` fails.
    /// - [`CodecError::GpuError`] if the GPU kernel dispatch or readback fails.
    /// - All errors from [`Codec::compress_batch_with`] when the CPU fallback
    ///   is taken.
    pub fn compress_batch_gpu_with<B>(
        &self,
        vectors: &[f32],
        rows: usize,
        cols: usize,
        prepared: &mut PreparedCodec,
        backend: &mut B,
        parallelism: Parallelism,
    ) -> Result<Vec<CompressedVector>, CodecError>
    where
        B: GpuComputeBackend,
    {
        if rows >= GPU_BATCH_THRESHOLD {
            backend.prepare_for_device(prepared).map_err(Into::into)?;
            backend
                .compress_batch(vectors, rows, cols, prepared)
                .map_err(Into::into)
        } else {
            self.compress_batch_with(
                vectors,
                rows,
                cols,
                prepared.config(),
                prepared.codebook(),
                parallelism,
            )
        }
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
