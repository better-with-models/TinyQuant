//! Convenience re-exports for glob import (`use tinyquant_core::prelude::*`).
//!
//! Populated incrementally as each phase lands. After Phase 18: corpus
//! aggregate types (`Corpus`, `VectorEntry`, `CompressionPolicy`,
//! `EntryMetaValue`, `CorpusEvent`, `BatchReport`, `StorageTag`,
//! `ViolationKind`) are included in addition to Phase 15 items.

pub use crate::codec::{
    compress, decompress, Codebook, Codec, CodecConfig, CompressedVector, Parallelism,
    RotationCache, RotationMatrix, ROTATION_CACHE_DEFAULT_CAPACITY, SUPPORTED_BIT_WIDTHS,
};
pub use crate::corpus::{
    BatchReport, CompressionPolicy, Corpus, CorpusEvent, EntryMetaValue, StorageTag, VectorEntry,
    ViolationKind,
};
pub use crate::errors::{BackendError, CodecError, CorpusError};
pub use crate::types::{ConfigHash, CorpusId, Timestamp, Vector, VectorId, VectorSlice};
