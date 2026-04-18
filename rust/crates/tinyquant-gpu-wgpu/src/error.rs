//! `TinyQuantGpuError`: error type for the wgpu backend.

use thiserror::Error;

/// Errors emitted by the `tinyquant-gpu-wgpu` backend.
#[derive(Debug, Error)]
pub enum TinyQuantGpuError {
    /// No wgpu-compatible adapter was found.
    #[error("no wgpu adapter found")]
    NoAdapter,

    /// `wgpu` device request failed.
    #[error("wgpu device request failed: {0}")]
    DeviceRequest(#[from] wgpu::RequestDeviceError),

    /// Dimension or buffer size mismatch detected on the host.
    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    /// Buffer mapping failed.
    #[error("buffer map failed: {0}")]
    BufferMap(#[from] wgpu::BufferAsyncError),

    /// `prepare_for_device` was not called before `compress_batch`.
    #[error("PreparedCodec has no GPU state; call prepare_for_device first")]
    NotPrepared,

    /// A codec-layer error occurred while constructing a `CompressedVector`.
    #[error("codec error: {0}")]
    Codec(#[from] tinyquant_core::errors::CodecError),

    /// GPU backend does not support `residual_enabled = true`.
    ///
    /// This variant is retained for any future use but is no longer returned
    /// by `compress_batch` after Phase 28 wired the residual GPU pass.
    #[error("GPU backend does not support residual_enabled=true; use the CPU path")]
    ResidualNotSupported,

    /// No adapter matched the requested [`BackendPreference`].
    ///
    /// The requested backend (Vulkan, Metal, DX12, or Software) produced no
    /// adapters on this machine.  Use [`BackendPreference::Auto`] for
    /// transparent fallback.
    #[error("no adapter found for the requested backend preference")]
    NoPreferredAdapter,

    /// A vector in the batch is incompatible with the `PreparedCodec`.
    #[error("batch vector mismatch: {detail}")]
    BatchMismatch { detail: String },

    /// `prepare_corpus_for_device` was not called before `cosine_topk`.
    #[error("corpus not uploaded; call prepare_corpus_for_device before cosine_topk")]
    CorpusNotPrepared,

    /// `cosine_topk` called with `top_k == 0`.
    #[error("top_k must be at least 1")]
    InvalidTopK,
}
