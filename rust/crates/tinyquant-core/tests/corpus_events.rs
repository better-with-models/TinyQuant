//! Integration tests for [`CorpusEvent`] and [`ViolationKind`].
//!
//! Key contracts:
//! - Each variant carries the correct payload fields.
//! - `drain_events` uses `mem::take` semantics (leaves empty buffer).
//! - `ViolationKind::as_python_tag` returns the correct strings.

use std::{collections::BTreeMap, sync::Arc};

use tinyquant_core::{
    codec::CodecConfig,
    corpus::{CompressionPolicy, Corpus, CorpusEvent, ViolationKind},
};

// ── ViolationKind ──────────────────────────────────────────────────────────────

#[test]
fn violation_kind_python_tags_exact_match() {
    assert_eq!(
        ViolationKind::ConfigMismatch.as_python_tag(),
        "config_mismatch"
    );
    assert_eq!(
        ViolationKind::PolicyConflict.as_python_tag(),
        "policy_conflict"
    );
    assert_eq!(ViolationKind::DuplicateId.as_python_tag(), "duplicate_id");
    assert_eq!(
        ViolationKind::DimensionMismatch.as_python_tag(),
        "dimension_mismatch"
    );
}

#[test]
fn violation_kind_eq_and_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(ViolationKind::DuplicateId);
    set.insert(ViolationKind::DuplicateId); // dedup
    assert_eq!(set.len(), 1);
    assert_ne!(ViolationKind::DuplicateId, ViolationKind::DimensionMismatch);
}

// ── CorpusEvent::VectorsInserted ──────────────────────────────────────────────

#[test]
fn vectors_inserted_event_carries_ids_and_count() {
    let e = CorpusEvent::VectorsInserted {
        corpus_id: Arc::from("c1"),
        vector_ids: Arc::from([Arc::from("a") as Arc<str>, Arc::from("b")]),
        count: 2,
        timestamp: 0,
    };
    if let CorpusEvent::VectorsInserted {
        count,
        vector_ids,
        corpus_id,
        ..
    } = e
    {
        assert_eq!(count, 2);
        assert_eq!(vector_ids.len(), 2);
        assert_eq!(corpus_id.as_ref(), "c1");
    } else {
        panic!("wrong variant");
    }
}

// ── CorpusEvent::Created ──────────────────────────────────────────────────────

#[test]
fn created_event_carries_config_and_policy() {
    let config = CodecConfig::new(4, 42, 64, true).unwrap();
    let e = CorpusEvent::Created {
        corpus_id: Arc::from("corp"),
        codec_config: config.clone(),
        compression_policy: CompressionPolicy::Compress,
        timestamp: 42,
    };
    if let CorpusEvent::Created {
        compression_policy,
        timestamp,
        corpus_id,
        ..
    } = e
    {
        assert_eq!(compression_policy, CompressionPolicy::Compress);
        assert_eq!(timestamp, 42);
        assert_eq!(corpus_id.as_ref(), "corp");
    } else {
        panic!("wrong variant");
    }
}

// ── CorpusEvent::Decompressed ─────────────────────────────────────────────────

#[test]
fn decompressed_event_carries_vector_count() {
    let e = CorpusEvent::Decompressed {
        corpus_id: Arc::from("corp"),
        vector_count: 7,
        timestamp: 100,
    };
    if let CorpusEvent::Decompressed { vector_count, .. } = e {
        assert_eq!(vector_count, 7);
    } else {
        panic!("wrong variant");
    }
}

// ── CorpusEvent::PolicyViolationDetected ──────────────────────────────────────

#[test]
fn policy_violation_event_carries_kind_and_detail() {
    let e = CorpusEvent::PolicyViolationDetected {
        corpus_id: Arc::from("corp"),
        kind: ViolationKind::DimensionMismatch,
        detail: Arc::from("expected 64, got 32"),
        timestamp: 0,
    };
    if let CorpusEvent::PolicyViolationDetected { kind, detail, .. } = e {
        assert_eq!(kind, ViolationKind::DimensionMismatch);
        assert_eq!(detail.as_ref(), "expected 64, got 32");
    } else {
        panic!("wrong variant");
    }
}

// ── Exactly 4 variants ────────────────────────────────────────────────────────

#[test]
fn corpus_event_exhaustive_match_covers_exactly_four_variants() {
    // This test will fail to compile if new variants are added without updating.
    let events = [
        CorpusEvent::Created {
            corpus_id: Arc::from("c"),
            codec_config: CodecConfig::new(4, 0, 8, false).unwrap(),
            compression_policy: CompressionPolicy::Passthrough,
            timestamp: 0,
        },
        CorpusEvent::VectorsInserted {
            corpus_id: Arc::from("c"),
            vector_ids: Arc::from([] as [Arc<str>; 0]),
            count: 0,
            timestamp: 0,
        },
        CorpusEvent::Decompressed {
            corpus_id: Arc::from("c"),
            vector_count: 0,
            timestamp: 0,
        },
        CorpusEvent::PolicyViolationDetected {
            corpus_id: Arc::from("c"),
            kind: ViolationKind::PolicyConflict,
            detail: Arc::from(""),
            timestamp: 0,
        },
    ];
    let mut count = 0usize;
    for e in &events {
        count += match e {
            CorpusEvent::Created { .. } => 1,
            CorpusEvent::VectorsInserted { .. } => 1,
            CorpusEvent::Decompressed { .. } => 1,
            CorpusEvent::PolicyViolationDetected { .. } => 1,
        };
    }
    assert_eq!(count, 4);
}

// ── drain_events semantics via Corpus ─────────────────────────────────────────

#[test]
fn drain_events_empties_buffer_mem_take_semantics() {
    use tinyquant_core::codec::{Codebook, CodecConfig};

    let config = CodecConfig::new(4, 42, 8, false).unwrap();
    let data: Vec<f32> = (0..8000).map(|i| i as f32 * 0.001).collect();
    let codebook = Codebook::train(&data, &config).unwrap();
    let mut corpus = Corpus::new(
        Arc::from("c"),
        config,
        codebook,
        CompressionPolicy::Passthrough,
        BTreeMap::new(),
    );

    // Created event is buffered.
    assert_eq!(corpus.pending_events().len(), 1);

    // First drain returns [Created].
    let first = corpus.drain_events();
    assert_eq!(first.len(), 1);

    // Second drain returns [].
    let second = corpus.drain_events();
    assert!(second.is_empty());
}
