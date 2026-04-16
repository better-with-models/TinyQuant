//! Codec layer: deterministic `CodecConfig`, canonical rotation matrix,
//! rotation cache, codebook quantization, residual helpers, the
//! stateless `Codec` service, and the `PreparedCodec` session object.
//!
//! Phase 13 introduced the rotation primitives; Phase 14 adds the
//! uniform-quantile [`Codebook`] plus scalar quantize / dequantize
//! kernels. Phase 15 adds the `Codec` service, FP16 residual helpers,
//! `CompressedVector` value object, and `Parallelism` enum.
//! Phase 26 adds [`PreparedCodec`] — a pre-built session that caches the
//! rotation matrix for reuse across batch calls, enabling the GPU backend
//! attachment point (Phase 27+).

#[cfg(feature = "std")]
pub(crate) mod batch;
#[cfg(feature = "std")]
pub(crate) mod batch_error;
pub mod codebook;
pub mod codec_config;
pub mod compressed_vector;
#[cfg(feature = "simd")]
pub mod dispatch;
pub mod gaussian;
pub(crate) mod kernels;
pub mod parallelism;
pub mod prepared;
pub(crate) mod quantize;
pub mod residual;
pub mod rotation_cache;
pub mod rotation_matrix;
pub mod service;
#[cfg(feature = "simd")]
pub mod simd_api;

pub use codebook::Codebook;
pub use codec_config::{CodecConfig, SUPPORTED_BIT_WIDTHS};
pub use compressed_vector::CompressedVector;
pub use parallelism::Parallelism;
pub use prepared::PreparedCodec;
pub use rotation_cache::{RotationCache, DEFAULT_CAPACITY as ROTATION_CACHE_DEFAULT_CAPACITY};
pub use rotation_matrix::RotationMatrix;
pub use service::{compress, decompress, Codec};
