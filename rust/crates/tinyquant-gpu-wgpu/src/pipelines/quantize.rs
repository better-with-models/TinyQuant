//! Scalar quantization compute pipeline.
//!
//! Maps rotated FP32 vectors to codebook indices using a device-resident
//! codebook buffer. Corresponds to `shaders/quantize.wgsl`.

use crate::context::WgpuContext;

/// Build the quantization pipeline from the embedded WGSL shader.
pub(crate) fn build_quantize_pipeline(ctx: &WgpuContext) -> wgpu::ComputePipeline {
    ctx.build_compute_pipeline(
        "tinyquant/quantize",
        include_str!("../../shaders/quantize.wgsl"),
        "main",
    )
}

/// Build the dequantization pipeline from the embedded WGSL shader.
pub(crate) fn build_dequantize_pipeline(ctx: &WgpuContext) -> wgpu::ComputePipeline {
    ctx.build_compute_pipeline(
        "tinyquant/dequantize",
        include_str!("../../shaders/dequantize.wgsl"),
        "main",
    )
}
