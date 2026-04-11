//! Codec layer: deterministic `CodecConfig`, canonical rotation matrix, and
//! a small LRU-style rotation cache.
//!
//! Phase 13 introduces the building blocks used by every later quantization
//! stage: a Python-parity config hash, a ChaCha-backed Gaussian stream, a
//! faer-QR + sign-corrected orthogonal matrix, and an 8-entry shared cache.

pub mod codec_config;
pub mod gaussian;
pub mod rotation_cache;
pub mod rotation_matrix;

pub use codec_config::{CodecConfig, SUPPORTED_BIT_WIDTHS};
pub use rotation_cache::{RotationCache, DEFAULT_CAPACITY as ROTATION_CACHE_DEFAULT_CAPACITY};
pub use rotation_matrix::RotationMatrix;
