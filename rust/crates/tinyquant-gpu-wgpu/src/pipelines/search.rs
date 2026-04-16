//! Cosine top-k search compute pipeline (Phase 27.5).
//!
//! Wraps the `cosine_topk.wgsl` shader which scores a query vector against
//! every row of a GPU-resident corpus in a single dispatch pass.  Top-k
//! selection is performed on the host after the GPU writes all scores.

use crate::context::WgpuContext;

/// Build the cosine-similarity scoring pipeline from the embedded WGSL shader.
///
/// The pipeline is used by
/// [`WgpuBackend::cosine_topk`](crate::backend::WgpuBackend::cosine_topk)
/// to score a query vector against all corpus rows in a single GPU pass.
pub(crate) fn build_cosine_topk_pipeline(ctx: &WgpuContext) -> wgpu::ComputePipeline {
    ctx.build_compute_pipeline(
        "tinyquant/cosine_topk",
        include_str!("../../shaders/cosine_topk.wgsl"),
        "main",
    )
}
