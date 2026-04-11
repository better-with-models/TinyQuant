//! Codec layer: deterministic `CodecConfig`, canonical rotation matrix,
//! rotation cache, and the `Codebook` quantization value object.
//!
//! Phase 13 introduced the rotation primitives; Phase 14 adds the
//! uniform-quantile [`Codebook`] plus scalar quantize / dequantize
//! kernels, which together form the byte-parity compress/decompress
//! reference for every later quantization stage.

pub mod codebook;
pub mod codec_config;
pub mod gaussian;
pub(crate) mod quantize;
pub mod rotation_cache;
pub mod rotation_matrix;

pub use codebook::Codebook;
pub use codec_config::{CodecConfig, SUPPORTED_BIT_WIDTHS};
pub use rotation_cache::{RotationCache, DEFAULT_CAPACITY as ROTATION_CACHE_DEFAULT_CAPACITY};
pub use rotation_matrix::RotationMatrix;
