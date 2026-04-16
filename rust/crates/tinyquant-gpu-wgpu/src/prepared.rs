//! GPU-resident state for a `PreparedCodec` session.
//!
//! `GpuPreparedState` holds device buffers for the rotation matrix and
//! codebook, uploaded once via [`crate::backend::WgpuBackend::prepare_for_device`].

use std::sync::Arc;
use wgpu::Buffer;

/// Device-resident buffers uploaded from a [`tinyquant_core::codec::PreparedCodec`] session.
///
/// Stored as opaque state via [`tinyquant_core::codec::PreparedCodec::set_gpu_state`] so that
/// `tinyquant-core` stays free of GPU imports.
pub(crate) struct GpuPreparedState {
    /// Rotation matrix buffer (f32, row-major, dim×dim) — `R`.
    /// Bound by the inverse-rotate (decompress) pass.
    pub rotation_buf: Arc<Buffer>,
    /// Transposed rotation matrix buffer (f32, row-major, dim×dim) — `R^T`.
    /// Bound by the forward-rotate (compress) pass.
    pub rotation_t_buf: Arc<Buffer>,
    /// Codebook entries buffer (f32, n_entries).
    pub codebook_buf: Arc<Buffer>,
    /// Dimension of the rotation matrix (== codec dim).
    #[allow(dead_code)]
    pub dim: usize,
    /// Number of codebook entries.
    pub n_entries: usize,
}
