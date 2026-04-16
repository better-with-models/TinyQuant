//! GPU compute pipelines for batch compress/decompress (Phase 27 Part B + C).
//!
//! Sub-modules correspond to the three WGSL shader stages:
//! - [`rotate`] — apply/invert the random rotation matrix
//! - [`quantize`] — scalar quantize rotated vectors to codebook indices
//! - [`residual`] — encode/decode FP16 residuals

pub mod quantize;
pub mod residual;
pub mod rotate;
