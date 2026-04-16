//! `PreparedCodec`: pre-built session object for the hot compress/decompress path.
//!
//! Owns a `RotationMatrix` built once from the supplied [`CodecConfig`], so
//! subsequent calls to [`Codec::compress_prepared`] and
//! [`Codec::decompress_prepared_into`] skip the `O(dim²)` QR factorization
//! that `RotationMatrix::build` performs.  This is the CPU prerequisite for
//! attaching a GPU backend (Phase 27+): a GPU backend simply calls
//! [`PreparedCodec::set_gpu_state`] to attach device-resident state without
//! requiring changes to `tinyquant-core`.
//!
//! # Construction cost
//!
//! `PreparedCodec::new` runs `RotationMatrix::build` once (O(dim² × dim) QR).
//! Amortise it by constructing once and reusing across all compress/decompress
//! calls over the same `(config, codebook)` pair.

use crate::{
    codec::{codec_config::CodecConfig, codebook::Codebook, rotation_matrix::RotationMatrix},
    errors::CodecError,
};

/// Pre-built compress/decompress session.
///
/// Construct once with [`PreparedCodec::new`], then pass to
/// [`Codec::compress_prepared`] / [`Codec::decompress_prepared_into`]
/// for each vector.  The [`RotationMatrix`] inside is computed exactly once.
pub struct PreparedCodec {
    config: CodecConfig,
    codebook: Codebook,
    rotation: RotationMatrix,
    /// Reserved for GPU-resident state attached by a GPU backend (Phase 27+).
    /// Stored as an erased type so `tinyquant-core` stays free of GPU imports.
    gpu_state: Option<alloc::boxed::Box<dyn core::any::Any + Send + Sync>>,
}

impl PreparedCodec {
    /// Build from a validated config and trained codebook.
    ///
    /// The rotation matrix is computed exactly once.
    ///
    /// # Errors
    ///
    /// [`CodecError::CodebookIncompatible`] if `codebook.bit_width() != config.bit_width()`.
    pub fn new(config: CodecConfig, codebook: Codebook) -> Result<Self, CodecError> {
        if codebook.bit_width() != config.bit_width() {
            return Err(CodecError::CodebookIncompatible {
                expected: config.bit_width(),
                got: codebook.bit_width(),
            });
        }
        let rotation = RotationMatrix::build(config.seed(), config.dimension());
        Ok(Self {
            config,
            codebook,
            rotation,
            gpu_state: None,
        })
    }

    /// The [`CodecConfig`] that defines this session.
    #[inline]
    pub const fn config(&self) -> &CodecConfig {
        &self.config
    }

    /// The trained [`Codebook`] for this session.
    #[inline]
    pub const fn codebook(&self) -> &Codebook {
        &self.codebook
    }

    /// The pre-built [`RotationMatrix`] — computed once at construction.
    #[inline]
    pub const fn rotation(&self) -> &RotationMatrix {
        &self.rotation
    }

    /// Attach opaque GPU state.
    ///
    /// Called by GPU backends — do not call from application code.
    pub fn set_gpu_state(
        &mut self,
        state: alloc::boxed::Box<dyn core::any::Any + Send + Sync>,
    ) {
        self.gpu_state = Some(state);
    }

    /// `true` if a GPU backend has attached device-resident state.
    #[inline]
    pub fn has_gpu_state(&self) -> bool {
        self.gpu_state.is_some()
    }

    /// Borrow the attached GPU state, if any.
    ///
    /// Called by GPU backends to retrieve device-resident buffers.
    /// Application code should not call this directly.
    #[inline]
    pub fn gpu_state(&self) -> Option<&(dyn core::any::Any + Send + Sync)> {
        self.gpu_state.as_deref()
    }
}
