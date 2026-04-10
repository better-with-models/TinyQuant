//! Integration tests for `tinyquant-core` error enums.
//!
//! Verifies:
//!   - `Display` output matches Python's error message strings exactly
//!   - `From` conversions (`CodecError` → `CorpusError`) work
//!   - `core::error::Error::source` chains are correct
//!   - Variants carry the right field types

extern crate alloc;
use alloc::string::ToString;
use alloc::sync::Arc;

use tinyquant_core::errors::{BackendError, CodecError, CorpusError};

// --- CodecError display ---

#[test]
fn unsupported_bit_width_message() {
    let e = CodecError::UnsupportedBitWidth { got: 3 };
    assert_eq!(e.to_string(), "bit_width must be one of [2, 4, 8], got 3");
}

#[test]
fn invalid_dimension_message() {
    let e = CodecError::InvalidDimension { got: 0 };
    assert_eq!(e.to_string(), "dimension must be > 0, got 0");
}

#[test]
fn dimension_mismatch_message() {
    let e = CodecError::DimensionMismatch { expected: 768, got: 1024 };
    assert_eq!(
        e.to_string(),
        "vector length 1024 does not match config dimension 768",
    );
}

#[test]
fn codebook_incompatible_message() {
    let e = CodecError::CodebookIncompatible { expected: 4, got: 8 };
    assert_eq!(
        e.to_string(),
        "codebook bit_width 8 does not match config bit_width 4",
    );
}

#[test]
fn config_mismatch_message() {
    let e = CodecError::ConfigMismatch {
        expected: Arc::from("aaa"),
        got: Arc::from("bbb"),
    };
    assert_eq!(
        e.to_string(),
        r#"compressed config_hash "bbb" does not match config hash "aaa""#,
    );
}

#[test]
fn codebook_entry_count_message() {
    let e = CodecError::CodebookEntryCount { expected: 16, got: 8, bit_width: 4 };
    assert_eq!(
        e.to_string(),
        "codebook must have 16 entries for bit_width=4, got 8",
    );
}

#[test]
fn codebook_not_sorted_message() {
    let e = CodecError::CodebookNotSorted;
    assert_eq!(e.to_string(), "codebook entries must be sorted ascending");
}

#[test]
fn codebook_duplicate_message() {
    let e = CodecError::CodebookDuplicate { expected: 16, got: 14 };
    assert_eq!(
        e.to_string(),
        "codebook must contain 16 distinct values, got 14",
    );
}

#[test]
fn insufficient_training_data_message() {
    let e = CodecError::InsufficientTrainingData { expected: 16 };
    assert_eq!(
        e.to_string(),
        "insufficient training data for 16 distinct entries",
    );
}

#[test]
fn index_out_of_range_message() {
    let e = CodecError::IndexOutOfRange { index: 20, bound: 16 };
    assert_eq!(e.to_string(), "index 20 is out of range [0, 16)");
}

#[test]
fn length_mismatch_message() {
    let e = CodecError::LengthMismatch { left: 8, right: 4 };
    assert_eq!(e.to_string(), "input and output lengths disagree: 8 vs 4");
}

// --- CorpusError ---

#[test]
fn corpus_codec_wraps_via_from() {
    let ce = CodecError::InvalidDimension { got: 0 };
    let corpus: CorpusError = ce.clone().into();
    assert!(matches!(corpus, CorpusError::Codec(CodecError::InvalidDimension { got: 0 })));
}

#[test]
fn corpus_error_source_is_codec_error() {
    // `#[source]` on the Codec variant makes thiserror v2 implement
    // `source()` as `Some(self.0)` — the inner CodecError itself, NOT
    // the codec error's own source. This matches the manual impl shown
    // in docs/design/rust/error-model.md.
    use core::error::Error;
    let ce = CodecError::DimensionMismatch { expected: 768, got: 100 };
    let corpus: CorpusError = ce.into();
    let source = corpus.source();
    assert!(source.is_some(), "CorpusError::Codec must expose the inner CodecError via source()");
    assert_eq!(
        source.unwrap().to_string(),
        "vector length 100 does not match config dimension 768",
    );
}

#[test]
fn duplicate_vector_id_message() {
    use tinyquant_core::types::VectorId;
    let id: VectorId = Arc::from("v-001");
    let e = CorpusError::DuplicateVectorId { id };
    assert_eq!(e.to_string(), r#"duplicate vector_id "v-001""#);
}

#[test]
fn unknown_vector_id_message() {
    use tinyquant_core::types::VectorId;
    let id: VectorId = Arc::from("v-999");
    let e = CorpusError::UnknownVectorId { id };
    assert_eq!(e.to_string(), r#"unknown vector_id "v-999""#);
}

#[test]
fn policy_immutable_message() {
    let e = CorpusError::PolicyImmutable;
    assert_eq!(e.to_string(), "compression policy is immutable once set");
}

// --- BackendError ---

#[test]
fn backend_empty_message() {
    let e = BackendError::Empty;
    assert_eq!(e.to_string(), "backend is empty");
}

#[test]
fn backend_invalid_top_k_message() {
    let e = BackendError::InvalidTopK;
    assert_eq!(e.to_string(), "top_k must be > 0");
}

#[test]
fn backend_adapter_message() {
    let e = BackendError::Adapter(Arc::from("connection refused"));
    assert_eq!(e.to_string(), "adapter error: connection refused");
}

// --- Clone and PartialEq ---

#[test]
fn codec_error_clone_and_eq() {
    let a = CodecError::UnsupportedBitWidth { got: 3 };
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn corpus_error_clone_and_eq() {
    let a = CorpusError::PolicyImmutable;
    let b = a.clone();
    assert_eq!(a, b);
}
