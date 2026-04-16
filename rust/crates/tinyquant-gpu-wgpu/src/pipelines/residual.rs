//! FP16 residual encode pipeline.
//!
//! Computes the quantization residual and packs it as FP16 pairs into `u32`
//! words. Corresponds to `shaders/residual_encode.wgsl`.
//!
//! Residual decode is handled on the CPU in Phase 27 (the parity test gates
//! only on `residual_enabled = false`; full GPU residual decode is Phase 28+).

use crate::context::WgpuContext;

/// Build the residual-encode pipeline from the embedded WGSL shader.
#[allow(dead_code)]
pub(crate) fn build_residual_encode_pipeline(ctx: &WgpuContext) -> wgpu::ComputePipeline {
    ctx.build_compute_pipeline(
        "tinyquant/residual_encode",
        include_str!("../../shaders/residual_encode.wgsl"),
        "main",
    )
}
