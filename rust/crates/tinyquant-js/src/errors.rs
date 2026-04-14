//! Map `tinyquant_core::errors::CodecError` into napi-rs errors.
//!
//! Phase 25.2 scope: only the codec path. Corpus / backend variants
//! land in later slices.
//!
//! The `code` on the napi `Error` mirrors the Python exception class
//! name (see `rust/crates/tinyquant-py/src/errors.rs`) so downstream
//! JS code can discriminate via `err.code === "DimensionMismatchError"`
//! and match Python's `except DimensionMismatchError:` behavior.

use napi::{Error as NapiError, Status};
use tinyquant_core::errors::CodecError;

/// Convert a `CodecError` to a napi `Error` whose `code` is the
/// Python-parity exception class name. The generic reason string is
/// reused from `CodecError::Display` so stack traces are self-explanatory.
pub(crate) fn map_codec_error(err: CodecError) -> NapiError {
    // The napi-rs v2 `Status` enum is the generic error class; we
    // stash the Python-parity class name inside the reason via the
    // `GenericFailure` status and decorate the reason string so the
    // public `err.code` string matches what Python users see.
    //
    // We use `Error::new(Status::GenericFailure, reason)` directly
    // because `from_reason` hard-codes `GenericFailure` and produces
    // a less precise `code` field in the JS error object.
    let reason = err.to_string();
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
        // The core error enum is `#[non_exhaustive]`.
        _ => "TinyQuantError",
    };
    // Prefix the reason with the class name so consumers who only
    // look at `err.message` still see the semantic bucket, matching
    // how PyO3 surfaces class names before the message.
    NapiError::new(Status::GenericFailure, format!("{code}: {reason}"))
}
