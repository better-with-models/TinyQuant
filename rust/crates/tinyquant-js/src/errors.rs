//! Map `tinyquant_core::errors::{CodecError, CorpusError, BackendError}`
//! into napi-rs errors.
//!
//! Errors cross the FFI boundary as `napi::Error` whose `message`
//! starts with `<ClassName>: ` (e.g. `"DimensionMismatchError: expected
//! 768, got 128"`). The class name is NOT present in `err.code` on
//! the JS side — that field is napi-rs's internal `Status` enum, which
//! does not admit custom string codes in v2. The TS wrapper
//! (`@tinyquant/core/src/_errors.ts`) parses the message prefix and
//! re-exposes it via `err.code` for JS consumers.
//!
//! Phase 25.3 adds the corpus + backend mappings alongside the codec
//! mapping that landed in Phase 25.2.

use napi::{Error as NapiError, Status};
use tinyquant_bruteforce::BackendError as CoreBackendError;
use tinyquant_core::errors::{CodecError, CorpusError};

/// Convert a `CodecError` to a napi `Error` whose `message` is
/// prefixed with the Python-parity exception class name (e.g.
/// `"DimensionMismatchError: ..."`). The reason string is reused from
/// `CodecError::Display` so stack traces stay self-explanatory.
pub(crate) fn map_codec_error(err: CodecError) -> NapiError {
    let reason = err.to_string();
    // `CodecError` is `#[non_exhaustive]`; the wildcard arm surfaces
    // variant drift at runtime until the `non_exhaustive_omitted_patterns`
    // lint stabilises.
    let code = match err {
        CodecError::DimensionMismatch { .. } => "DimensionMismatchError",
        CodecError::ConfigMismatch { .. } => "ConfigMismatchError",
        CodecError::CodebookIncompatible { .. } => "CodebookIncompatibleError",
        CodecError::UnsupportedBitWidth { .. } => "UnsupportedBitWidthError",
        CodecError::InvalidDimension { .. } => "InvalidDimensionError",
        CodecError::CodebookEntryCount { .. }
        | CodecError::CodebookNotSorted
        | CodecError::CodebookDuplicate { .. }
        | CodecError::InsufficientTrainingData { .. }
        | CodecError::IndexOutOfRange { .. }
        | CodecError::LengthMismatch { .. }
        | CodecError::InvalidResidualFlag { .. } => "TinyQuantError",
        ref other => unreachable!(
            "new CodecError variant reached map_codec_error without an explicit mapping: {other:?}"
        ),
    };
    NapiError::new(Status::GenericFailure, format!("{code}: {reason}"))
}

/// Convert a `CorpusError` to a napi `Error` using the same
/// `"<ClassName>: reason"` message contract as `map_codec_error`.
pub(crate) fn map_corpus_error(err: CorpusError) -> NapiError {
    // Unwrap BatchAtomicityFailure / Codec into the nested cause so
    // the surfaced class name is the semantic leaf rather than a
    // wrapper — matches the PyO3 binding's behaviour.
    match err {
        CorpusError::Codec(inner) => map_codec_error(inner),
        CorpusError::BatchAtomicityFailure { source, .. } => map_corpus_error(*source),
        other => {
            let reason = other.to_string();
            let code = match other {
                CorpusError::DuplicateVectorId { .. } => "DuplicateVectorError",
                CorpusError::UnknownVectorId { .. } => "UnknownVectorError",
                CorpusError::DimensionMismatch { .. } => "DimensionMismatchError",
                CorpusError::PolicyImmutable => "PolicyImmutableError",
                // Codec + BatchAtomicityFailure are handled above; the
                // wildcard arm covers future additions to the
                // non-exhaustive enum.
                ref leftover => unreachable!(
                    "new CorpusError variant reached map_corpus_error without an explicit mapping: {leftover:?}"
                ),
            };
            NapiError::new(Status::GenericFailure, format!("{code}: {reason}"))
        }
    }
}

/// Convert a `BackendError` to a napi `Error` using the same
/// `"<ClassName>: reason"` message contract.
pub(crate) fn map_backend_error(err: CoreBackendError) -> NapiError {
    let reason = err.to_string();
    let code = match err {
        CoreBackendError::Empty => "BackendEmptyError",
        CoreBackendError::InvalidTopK => "InvalidTopKError",
        CoreBackendError::Adapter(_) => "BackendAdapterError",
        ref other => unreachable!(
            "new BackendError variant reached map_backend_error without an explicit mapping: {other:?}"
        ),
    };
    NapiError::new(Status::GenericFailure, format!("{code}: {reason}"))
}
