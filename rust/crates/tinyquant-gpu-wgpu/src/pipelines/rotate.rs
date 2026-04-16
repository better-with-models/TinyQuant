//! Rotate / inverse-rotate compute pipeline.
//!
//! Both the forward and inverse passes use the same `rotate.wgsl` shader.
//! The difference is which rotation buffer is bound:
//!
//! - **Forward (compress):** binds `R^T` (`rotation_t_buf`). The shader computes
//!   `output = input @ R^T`, which matches the CPU column-vector convention `R @ v`.
//! - **Inverse (decompress):** binds `R` (`rotation_buf`). The shader computes
//!   `output = values @ R`, which matches `R^T @ v` (valid because `R` is
//!   orthogonal: `R^{-1} = R^T`).

use crate::context::WgpuContext;

/// Build the forward (and inverse) rotation pipeline from the embedded WGSL shader.
///
/// Both compress and decompress reuse this pipeline — differentiation comes
/// from which buffer is bound, not from a different pipeline object.
pub(crate) fn build_rotate_pipeline(ctx: &WgpuContext) -> wgpu::ComputePipeline {
    ctx.build_compute_pipeline(
        "tinyquant/rotate",
        include_str!("../../shaders/rotate.wgsl"),
        "main",
    )
}
