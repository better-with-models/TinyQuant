//! FP16 residual encode/decode pipelines.
//!
//! - [`build_residual_encode_pipeline`] — quantisation residual packed as FP16 pairs.
//! - [`build_residual_decode_pipeline`] — unpacks FP16 pairs and adds them in-place to values.

use crate::context::WgpuContext;

/// Build the compute pipeline for the GPU residual encode pass.
///
/// Semantics: for each pair of elements, compute `original[i] - reconstructed[i]`
/// and pack as fp16 into `residual_u16`.  After this pass
/// `residual_u16[k]` encodes `f32_to_f16(original[2k] - reconstructed[2k])` in the
/// low 16 bits and the same for index `2k+1` in the high 16 bits.
///
/// Buffer bindings (group 0):
///   0 — `original`:      `storage, read`       — rotated FP32 values
///   1 — `reconstructed`: `storage, read`       — dequantised FP32 values
///   2 — `residual_u16`:  `storage, read_write` — packed fp16 output
pub(crate) fn build_residual_encode_pipeline(ctx: &WgpuContext) -> wgpu::ComputePipeline {
    ctx.build_compute_pipeline(
        "tinyquant/residual_encode",
        include_str!("../../shaders/residual_encode.wgsl"),
        "main",
    )
}

/// Build the compute pipeline for the GPU residual decode pass.
///
/// Semantics: for each packed fp16 pair in `residual_u16`, unpack both
/// fp16 values and add them in-place to `values`.  After this pass
/// `values[i]` equals the original pre-quantisation value (to fp16
/// precision), ready for the inverse-rotate pass.
///
/// Buffer bindings (group 0):
///   0 — `residual_u16`: `storage, read`       — packed fp16 pairs
///   1 — `values`:       `storage, read_write` — dequantised values
pub(crate) fn build_residual_decode_pipeline(ctx: &WgpuContext) -> wgpu::ComputePipeline {
    ctx.build_compute_pipeline(
        "tinyquant/residual_decode",
        include_str!("../../shaders/residual_decode.wgsl"),
        "main",
    )
}
