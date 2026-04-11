//! Convenience re-exports for glob import (`use tinyquant_core::prelude::*`).
//!
//! Populated incrementally as each phase lands. After Phase 14: shared
//! type aliases, error enums, `CodecConfig`, `RotationMatrix`,
//! `RotationCache`, and `Codebook` are available. Corpus and backend
//! traits re-export from Phase 15 onward.

pub use crate::codec::{
    Codebook, CodecConfig, RotationCache, RotationMatrix, ROTATION_CACHE_DEFAULT_CAPACITY,
    SUPPORTED_BIT_WIDTHS,
};
pub use crate::errors::{BackendError, CodecError, CorpusError};
pub use crate::types::{ConfigHash, CorpusId, Timestamp, Vector, VectorId, VectorSlice};
