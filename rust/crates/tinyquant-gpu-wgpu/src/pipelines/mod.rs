//! GPU compute pipelines for batch compress/decompress (Phase 27 Part B + C)
//! and corpus search (Phase 27.5).
//!
//! Sub-modules correspond to the WGSL shader stages:
//! - [`rotate`] — apply/invert the random rotation matrix
//! - [`quantize`] — scalar quantize rotated vectors to codebook indices
//! - [`residual`] — encode/decode FP16 residuals
//! - [`search`] — cosine-similarity scoring against a GPU-resident corpus

pub mod quantize;
pub mod residual;
pub mod rotate;
pub mod search;
