//! Domain events emitted by the [`Corpus`](super::Corpus) aggregate.
//!
//! Exactly **four** event variants exist, matching `tinyquant_cpu.corpus.events`
//! one-for-one.  No new variants are added in Phase 18; see the plan for the
//! parity table and rationale.
//!
//! ## Events NOT raised (parity with Python)
//!
//! - **No** `VectorsRemoved` — `Corpus::remove` is a silent mutation.
//! - **No** `VectorDecompressed` (singular) — only `decompress_all` emits.
//! - **No** `CorpusCleared` — not a method on the Python aggregate.
//! - **No** `PolicyChanged` — `CompressionPolicy` is immutable after the
//!   first insert.

use alloc::sync::Arc;

use crate::codec::CodecConfig;
use crate::corpus::compression_policy::CompressionPolicy;
use crate::types::{CorpusId, Timestamp, VectorId};

/// A domain event emitted by the [`Corpus`](super::Corpus) aggregate.
///
/// Events are buffered in `pending_events` and drained atomically by
/// [`Corpus::drain_events`](super::Corpus::drain_events).
#[derive(Clone, Debug)]
pub enum CorpusEvent {
    /// Emitted once when a new `Corpus` is successfully constructed.
    ///
    /// Python parity: `CorpusCreated`.
    Created {
        /// The id of the newly created corpus.
        corpus_id: CorpusId,
        /// The codec configuration active at construction time.
        codec_config: CodecConfig,
        /// The compression policy set at construction time.
        compression_policy: CompressionPolicy,
        /// Nanosecond Unix timestamp of construction.
        timestamp: Timestamp,
    },

    /// Emitted after one or more vectors are successfully inserted.
    ///
    /// A single `VectorsInserted` event covers the entire batch on
    /// `insert_batch` success; `insert` emits exactly one event per call.
    ///
    /// Python parity: `VectorsInserted`.
    VectorsInserted {
        /// The id of the corpus that received the vectors.
        corpus_id: CorpusId,
        /// Ids of the vectors that were inserted, in insertion order.
        vector_ids: Arc<[VectorId]>,
        /// Number of vectors inserted (equals `vector_ids.len()`).
        count: u32,
        /// Nanosecond Unix timestamp of the insertion.
        timestamp: Timestamp,
    },

    /// Emitted after [`decompress_all`](super::Corpus::decompress_all) succeeds.
    ///
    /// Single-vector `decompress` does **not** emit an event (Python parity).
    ///
    /// Python parity: `CorpusDecompressed`.
    Decompressed {
        /// The id of the corpus whose vectors were decompressed.
        corpus_id: CorpusId,
        /// Total number of vectors decompressed.
        vector_count: u32,
        /// Nanosecond Unix timestamp of the operation.
        timestamp: Timestamp,
    },

    /// Emitted when a policy or dimension violation is detected but the
    /// operation is not immediately fatal (e.g., logging path).
    ///
    /// Python parity: `CompressionPolicyViolationDetected`.
    PolicyViolationDetected {
        /// The id of the corpus where the violation was detected.
        corpus_id: CorpusId,
        /// The category of the violation.
        kind: ViolationKind,
        /// Human-readable detail string, suitable for logging.
        detail: Arc<str>,
        /// Nanosecond Unix timestamp of detection.
        timestamp: Timestamp,
    },
}

/// Category of a policy or configuration violation detected by the corpus.
///
/// The `as_python_tag` strings must match Python's `violation_type` field
/// exactly for cross-language parity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ViolationKind {
    /// The supplied `CodecConfig` hash does not match the corpus's config.
    ConfigMismatch,
    /// An operation conflicts with the corpus's compression policy.
    PolicyConflict,
    /// A vector with the same id already exists in the corpus.
    DuplicateId,
    /// The supplied vector's dimension does not match the corpus's dimension.
    DimensionMismatch,
}

impl ViolationKind {
    /// Returns the canonical Python `violation_type` string for this kind.
    ///
    /// These strings are part of the public API contract; do **not** change
    /// them without updating the Python counterpart.
    #[must_use]
    pub const fn as_python_tag(self) -> &'static str {
        match self {
            Self::ConfigMismatch => "config_mismatch",
            Self::PolicyConflict => "policy_conflict",
            Self::DuplicateId => "duplicate_id",
            Self::DimensionMismatch => "dimension_mismatch",
        }
    }
}
