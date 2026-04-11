//! Convenience re-exports for glob import (`use tinyquant_core::prelude::*`).
//!
//! Populated incrementally as each phase lands. After Phase 13: shared
//! type aliases, error enums, `CodecConfig`, `RotationMatrix`, and the
//! shared `RotationCache` are available. Corpus and backend traits
//! re-export from Phase 15 onward.

pub use crate::codec::{
    CodecConfig, RotationCache, RotationMatrix, ROTATION_CACHE_DEFAULT_CAPACITY,
    SUPPORTED_BIT_WIDTHS,
};
pub use crate::errors::{BackendError, CodecError, CorpusError};
pub use crate::types::{ConfigHash, CorpusId, Timestamp, Vector, VectorId, VectorSlice};
