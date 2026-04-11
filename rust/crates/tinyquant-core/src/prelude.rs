//! Convenience re-exports for glob import (`use tinyquant_core::prelude::*`).
//!
//! Populated incrementally as each phase lands. After Phase 15: shared
//! type aliases, error enums, `CodecConfig`, `RotationMatrix`,
//! `RotationCache`, `Codebook`, `Codec`, `CompressedVector`, `Parallelism`,
//! and the `compress`/`decompress` free functions are available.

pub use crate::codec::{
    compress, decompress, Codebook, Codec, CodecConfig, CompressedVector, Parallelism,
    RotationCache, RotationMatrix, ROTATION_CACHE_DEFAULT_CAPACITY, SUPPORTED_BIT_WIDTHS,
};
pub use crate::errors::{BackendError, CodecError, CorpusError};
pub use crate::types::{ConfigHash, CorpusId, Timestamp, Vector, VectorId, VectorSlice};
