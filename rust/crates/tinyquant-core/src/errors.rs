//! Error enums for the `TinyQuant` core.
//!
//! Three enums cover the three architectural layers:
//!
//! - [`CodecError`] — codec layer (config, codebook, quantize, compress)
//! - [`CorpusError`] — corpus aggregate layer
//! - [`BackendError`] — backend/search layer
//!
//! All variants are `Clone + PartialEq` so they can be threaded
//! through `Arc` and matched in tests.
//!
//! See `docs/design/rust/error-model.md` for the full taxonomy,
//! Python-exception mapping, and FFI conversion table.

use alloc::sync::Arc;
use thiserror::Error;

use crate::types::VectorId;

/// Errors produced by the codec layer.
///
/// Maps 1:1 to `tinyquant_cpu.codec._errors` Python exceptions; see the
/// mapping table in `docs/design/rust/error-model.md`.
#[non_exhaustive]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CodecError {
    /// `bit_width` is outside the supported set `{2, 4, 8}`.
    #[error("bit_width must be one of [2, 4, 8], got {got}")]
    UnsupportedBitWidth {
        /// The unsupported bit width value that was provided.
        got: u8,
    },

    /// `dimension` is zero — every codec operation requires a non-zero dim.
    #[error("dimension must be > 0, got {got}")]
    InvalidDimension {
        /// The invalid dimension value that was provided.
        got: u32,
    },

    /// Input vector length does not match the config's declared dimension.
    #[error("vector length {got} does not match config dimension {expected}")]
    DimensionMismatch {
        /// The dimension declared in the codec config.
        expected: u32,
        /// The actual length of the input vector.
        got: u32,
    },

    /// Codebook was trained under a different `bit_width` than the config.
    #[error("codebook bit_width {got} does not match config bit_width {expected}")]
    CodebookIncompatible {
        /// The bit width from the codec config.
        expected: u8,
        /// The bit width embedded in the codebook.
        got: u8,
    },

    /// A `CompressedVector` carries a `config_hash` that differs from the
    /// supplied `CodecConfig`.
    #[error("compressed config_hash {got:?} does not match config hash {expected:?}")]
    ConfigMismatch {
        /// Expected hash from the codec config.
        expected: Arc<str>,
        /// Actual hash found in the compressed vector.
        got: Arc<str>,
    },

    /// Codebook entry count is inconsistent with `bit_width`.
    #[error("codebook must have {expected} entries for bit_width={bit_width}, got {got}")]
    CodebookEntryCount {
        /// Expected number of entries.
        expected: u32,
        /// Actual number of entries found.
        got: u32,
        /// The bit width that determines the expected count.
        bit_width: u8,
    },

    /// Codebook entries are not sorted in non-decreasing order.
    #[error("codebook entries must be sorted ascending")]
    CodebookNotSorted,

    /// Codebook contains non-unique entries (violates `np.unique` invariant).
    #[error("codebook must contain {expected} distinct values, got {got}")]
    CodebookDuplicate {
        /// Expected number of distinct values.
        expected: u32,
        /// Actual number of distinct values found.
        got: u32,
    },

    /// Training data contains fewer distinct values than needed to fill
    /// the codebook.
    #[error("insufficient training data for {expected} distinct entries")]
    InsufficientTrainingData {
        /// Minimum distinct entries required.
        expected: u32,
    },

    /// A codebook index exceeds the codebook's valid range `[0, 2^bit_width)`.
    #[error("index {index} is out of range [0, {bound})")]
    IndexOutOfRange {
        /// The out-of-range index value.
        index: u8,
        /// The exclusive upper bound.
        bound: u32,
    },

    /// An internal length invariant was violated (caller or internal bug).
    #[error("input and output lengths disagree: {left} vs {right}")]
    LengthMismatch {
        /// Length of the left operand.
        left: usize,
        /// Length of the right operand.
        right: usize,
    },
}

/// Errors produced by the corpus aggregate layer.
#[non_exhaustive]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CorpusError {
    /// An upstream codec operation failed.
    ///
    /// `#[from]` generates `impl From<CodecError> for CorpusError`.
    /// `#[source]` makes thiserror implement `source()` as `Some(self.0)`,
    /// so the inner `CodecError` is accessible via the standard error chain —
    /// matching the manual `impl` shown in `docs/design/rust/error-model.md`.
    ///
    /// Do NOT use `#[error(transparent)]` — that delegates `source()` to
    /// `self.0.source()` (None for leaf variants), losing the chain link.
    #[error("codec error: {0}")]
    Codec(
        #[from]
        #[source]
        CodecError,
    ),

    /// A vector with this id already exists in the corpus.
    #[error("duplicate vector_id {id:?}")]
    DuplicateVectorId {
        /// The duplicate vector identifier.
        id: VectorId,
    },

    /// A lookup requested a `VectorId` that does not exist.
    #[error("unknown vector_id {id:?}")]
    UnknownVectorId {
        /// The unknown vector identifier.
        id: VectorId,
    },

    /// An attempt was made to change the compression policy after it was
    /// frozen.
    #[error("compression policy is immutable once set")]
    PolicyImmutable,
}

/// Errors produced by the search-backend layer.
#[non_exhaustive]
#[derive(Debug, Error, Clone)]
pub enum BackendError {
    /// The backend contains no vectors — search cannot proceed.
    ///
    /// This is informational; callers may treat it as an empty result
    /// rather than a hard failure.
    #[error("backend is empty")]
    Empty,

    /// `top_k` was zero or would overflow internal heap sizing.
    #[error("top_k must be > 0")]
    InvalidTopK,

    /// An adapter-specific (e.g. pgvector) failure, with a human-readable
    /// message. `Arc<str>` avoids allocation on the hot path when the
    /// error is not inspected.
    #[error("adapter error: {0}")]
    Adapter(Arc<str>),
}
