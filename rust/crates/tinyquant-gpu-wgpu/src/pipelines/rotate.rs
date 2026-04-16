//! Rotate / inverse-rotate compute pipeline.
//!
//! Both the forward and inverse (transpose) passes use the same `rotate.wgsl`
//! shader. The difference is which rotation buffer is bound: the forward pass
//! binds the original rotation matrix `R`; the inverse pass binds `R^T` (valid
//! because `R` is orthogonal, so `R^{-1} = R^T`).

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
