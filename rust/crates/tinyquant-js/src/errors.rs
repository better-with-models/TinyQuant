//! Map `tinyquant_core::errors::CodecError` into napi-rs errors.
//!
//! Phase 25.2 scope: only the codec path. Corpus / backend variants
//! land in later slices.
//!
//! Errors cross the FFI boundary as `napi::Error` whose `message`
//! starts with `<ClassName>: ` (e.g. `"DimensionMismatchError: expected
//! 768, got 128"`). The class name is NOT present in `err.code` on
//! the JS side — that field is napi-rs's internal `Status` enum, which
//! does not admit custom string codes in v2. Phase 25.4 will add TS
//! wrapper classes (`TinyQuantError`, `DimensionMismatchError`, …)
//! that parse the message prefix and re-expose a populated `err.code`.
//! Until then, consumers that need to switch on the error class
//! should parse `err.message.split(':', 1)[0]`.

use napi::{Error as NapiError, Status};
use tinyquant_core::errors::CodecError;

/// Convert a `CodecError` to a napi `Error` whose `message` is
/// prefixed with the Python-parity exception class name (e.g.
/// `"DimensionMismatchError: ..."`). The reason string is reused from
/// `CodecError::Display` so stack traces stay self-explanatory. See
/// the module-level doc comment for the full contract — in particular,
/// the class name lives in `message`, not in `err.code`.
pub(crate) fn map_codec_error(err: CodecError) -> NapiError {
    // napi-rs v2's `Status` enum does not admit custom string codes,
    // so we encode the Python-parity class name as a prefix on the
    // message. Phase 25.4 will add a TS wrapper that parses this
    // prefix and re-exposes it via `err.code` for JS consumers.
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
