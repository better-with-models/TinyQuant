//! Convenience re-exports for glob import (`use tinyquant_core::prelude::*`).
//!
//! Populated incrementally as each phase lands. After Phase 12:
//! shared type aliases and error enums are available. Codec types,
//! corpus, and backend are re-exported from Phase 13 onward.

pub use crate::errors::{BackendError, CodecError, CorpusError};
pub use crate::types::{ConfigHash, CorpusId, Timestamp, Vector, VectorId, VectorSlice};
