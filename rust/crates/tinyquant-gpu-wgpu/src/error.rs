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
}
