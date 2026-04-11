//! Full lifecycle integration tests for the [`Corpus`] aggregate root.
//!
//! Tests cover:
//! - Construction emits `Created` event.
//! - `insert` emits `VectorsInserted` and increments `vector_count`.
//! - `decompress` returns correctly sized output without emitting events.
//! - `decompress_all` emits `Decompressed`.
//! - `remove` is silent.
//! - `drain_events` leaves the buffer empty.
//! - Duplicate id raises `DuplicateVectorId`.
//! - Wrong dimension raises `DimensionMismatch`.
//! - `insert_batch` atomicity: corpus unchanged on failure.
//! - Three-policy round-trips: Compress, Passthrough, Fp16.

use std::{collections::BTreeMap, sync::Arc};

use tinyquant_core::{
    codec::{Codebook, CodecConfig},
    corpus::{BatchReport, CompressionPolicy, Corpus, CorpusEvent},
    errors::CorpusError,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

const DIM: u32 = 64;

fn make_config() -> CodecConfig {
    CodecConfig::new(4, 42, DIM, true).unwrap()
}

fn make_codebook(config: &CodecConfig) -> Codebook {
    let data: Vec<f32> = (0..10_000 * DIM as usize)
        .map(|i| i as f32 * 0.0001)
        .collect();
    Codebook::train(&data, config).unwrap()
}

fn make_vector(seed: u8) -> Vec<f32> {
    (0..DIM as usize)
        .map(|i| (i as f32 + f32::from(seed)) * 0.01)
        .collect()
}

fn make_corpus() -> Corpus {
    let config = make_config();
    let codebook = make_codebook(&config);
    Corpus::new(
        Arc::from("test-corpus"),
        config,
        codebook,
        CompressionPolicy::Compress,
        BTreeMap::new(),
    )
}

// ── Construction ──────────────────────────────────────────────────────────────

#[test]
fn corpus_construction_emits_created_event() {
    let mut corpus = make_corpus();
    assert_eq!(corpus.vector_count(), 0);
    assert!(corpus.is_empty());
    let events = corpus.drain_events();
    assert_eq!(events.len(), 1);
    assert!(
        matches!(events[0], CorpusEvent::Created { .. }),
        "expected Created, got {events:?}"
    );
}

#[test]
fn corpus_drain_events_leaves_empty_buffer() {
    let mut corpus = make_corpus();
    let _ = corpus.drain_events();
    let second = corpus.drain_events();
    assert!(second.is_empty());
}

// ── Insert ────────────────────────────────────────────────────────────────────

#[test]
fn corpus_create_insert_decompress_flow() {
    let mut corpus = make_corpus();

    // Consume the Created event.
    let events = corpus.drain_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], CorpusEvent::Created { .. }));

    let v = make_vector(0);
    corpus.insert(Arc::from("v1"), &v, None, 0).unwrap();
    assert_eq!(corpus.vector_count(), 1);
    assert!(!corpus.is_empty());
    assert!(corpus.contains(&Arc::from("v1")));

    let events = corpus.drain_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        CorpusEvent::VectorsInserted { count: 1, .. }
    ));

    let recovered = corpus.decompress(&Arc::from("v1")).unwrap();
    assert_eq!(recovered.len(), DIM as usize);
    // No event emitted by single decompress.
    assert!(corpus.drain_events().is_empty());
}

#[test]
fn insert_wrong_dim_raises_dimension_mismatch() {
    let mut corpus = make_corpus();
    let _ = corpus.drain_events();
    let short: Vec<f32> = vec![0.0; 32]; // wrong dim
    let err = corpus
        .insert(Arc::from("v-bad"), &short, None, 0)
        .unwrap_err();
    assert!(
        matches!(
            err,
            CorpusError::DimensionMismatch {
                expected: 64,
                got: 32
            }
        ),
        "got {err:?}"
    );
    // Corpus unchanged.
    assert_eq!(corpus.vector_count(), 0);
    assert!(corpus.drain_events().is_empty());
}

#[test]
fn insert_duplicate_id_raises_duplicate_vector_error() {
    let mut corpus = make_corpus();
    let _ = corpus.drain_events();
    let v = make_vector(0);
    corpus.insert(Arc::from("v1"), &v, None, 0).unwrap();
    let events1 = corpus.drain_events();
    assert_eq!(events1.len(), 1);

    let err = corpus.insert(Arc::from("v1"), &v, None, 0).unwrap_err();
    assert!(
        matches!(err, CorpusError::DuplicateVectorId { .. }),
        "got {err:?}"
    );
    // Still only one vector.
    assert_eq!(corpus.vector_count(), 1);
    // No new events.
    assert!(corpus.drain_events().is_empty());
}

// ── insert_batch atomicity ────────────────────────────────────────────────────

#[test]
fn insert_batch_rolls_back_on_dimension_mismatch() {
    let mut corpus = make_corpus();
    let _ = corpus.drain_events();

    let v0 = make_vector(0);
    let v1_bad: Vec<f32> = vec![0.0; 32]; // wrong dim — index 1
    let v2 = make_vector(2);
    let id0: Arc<str> = Arc::from("v0");
    let id1: Arc<str> = Arc::from("v1");
    let id2: Arc<str> = Arc::from("v2");

    let batch: Vec<(
        Arc<str>,
        &[f32],
        Option<tinyquant_core::corpus::EntryMetaValue>,
    )> = vec![
        (Arc::clone(&id0), v0.as_slice(), None),
        (Arc::clone(&id1), v1_bad.as_slice(), None),
        (Arc::clone(&id2), v2.as_slice(), None),
    ];

    let err = corpus.insert_batch(&batch, 0).unwrap_err();

    // Must be BatchAtomicityFailure at index 1 wrapping DimensionMismatch.
    match &err {
        CorpusError::BatchAtomicityFailure { index, source } => {
            assert_eq!(*index, 1);
            assert!(matches!(
                source.as_ref(),
                CorpusError::DimensionMismatch {
                    expected: 64,
                    got: 32
                }
            ));
        }
        other => panic!("expected BatchAtomicityFailure, got {other:?}"),
    }

    // Corpus is unchanged — atomicity guarantee.
    assert_eq!(corpus.vector_count(), 0);
    // No events emitted.
    assert!(corpus.drain_events().is_empty());
}

#[test]
fn insert_batch_success_emits_single_event() {
    let mut corpus = make_corpus();
    let _ = corpus.drain_events();

    let v0 = make_vector(0);
    let v1 = make_vector(1);
    let batch: Vec<(
        Arc<str>,
        &[f32],
        Option<tinyquant_core::corpus::EntryMetaValue>,
    )> = vec![
        (Arc::from("b0"), v0.as_slice(), None),
        (Arc::from("b1"), v1.as_slice(), None),
    ];

    let report = corpus.insert_batch(&batch, 0).unwrap();
    assert_eq!(
        report,
        BatchReport {
            inserted: 2,
            skipped: 0,
            first_error: None
        }
    );
    assert_eq!(corpus.vector_count(), 2);

    let events = corpus.drain_events();
    assert_eq!(
        events.len(),
        1,
        "expected exactly one VectorsInserted event"
    );
    if let CorpusEvent::VectorsInserted {
        count, vector_ids, ..
    } = &events[0]
    {
        assert_eq!(*count, 2);
        assert_eq!(vector_ids.len(), 2);
    } else {
        panic!("expected VectorsInserted, got {events:?}");
    }
}

#[test]
fn insert_batch_duplicate_within_batch_is_rejected() {
    let mut corpus = make_corpus();
    let _ = corpus.drain_events();

    let v = make_vector(0);
    let batch: Vec<(
        Arc<str>,
        &[f32],
        Option<tinyquant_core::corpus::EntryMetaValue>,
    )> = vec![
        (Arc::from("dup"), v.as_slice(), None),
        (Arc::from("dup"), v.as_slice(), None), // duplicate at index 1
    ];

    let err = corpus.insert_batch(&batch, 0).unwrap_err();
    assert!(
        matches!(err, CorpusError::BatchAtomicityFailure { index: 1, .. }),
        "got {err:?}"
    );
    assert_eq!(corpus.vector_count(), 0);
}

// ── decompress_all ────────────────────────────────────────────────────────────

#[test]
fn decompress_all_emits_decompressed_event() {
    let mut corpus = make_corpus();
    let _ = corpus.drain_events();
    for i in 0..3_u8 {
        let id = format!("v{i}");
        corpus
            .insert(Arc::from(id.as_str()), &make_vector(i), None, 0)
            .unwrap();
    }
    let _ = corpus.drain_events();

    let all = corpus.decompress_all_at(0).unwrap();
    assert_eq!(all.len(), 3);

    let events = corpus.drain_events();
    assert_eq!(events.len(), 1);
    assert!(
        matches!(
            events[0],
            CorpusEvent::Decompressed {
                vector_count: 3,
                ..
            }
        ),
        "got {events:?}"
    );
}

// ── remove ────────────────────────────────────────────────────────────────────

#[test]
fn remove_is_silent_no_event() {
    let mut corpus = make_corpus();
    let _ = corpus.drain_events();
    corpus
        .insert(Arc::from("v1"), &make_vector(0), None, 0)
        .unwrap();
    let _ = corpus.drain_events();

    let entry = corpus.remove(&Arc::from("v1"));
    assert!(entry.is_some());
    assert_eq!(corpus.vector_count(), 0);
    // No events emitted.
    assert!(corpus.drain_events().is_empty());
}

#[test]
fn remove_nonexistent_returns_none() {
    let mut corpus = make_corpus();
    let _ = corpus.drain_events();
    let entry = corpus.remove(&Arc::from("missing"));
    assert!(entry.is_none());
}

// ── Unknown id ────────────────────────────────────────────────────────────────

#[test]
fn decompress_unknown_id_raises_unknown_vector_id() {
    let corpus = make_corpus();
    let err = corpus.decompress(&Arc::from("nope")).unwrap_err();
    assert!(
        matches!(err, CorpusError::UnknownVectorId { .. }),
        "got {err:?}"
    );
}

// ── iter ──────────────────────────────────────────────────────────────────────

#[test]
fn iter_yields_all_entries_in_insertion_order() {
    let mut corpus = make_corpus();
    let _ = corpus.drain_events();
    let ids = ["alpha", "beta", "gamma"];
    for id in &ids {
        corpus
            .insert(Arc::from(*id), &make_vector(0), None, 0)
            .unwrap();
    }
    let collected: Vec<&str> = corpus.iter().map(|(id, _)| id.as_ref()).collect();
    assert_eq!(collected, ids.as_slice());
}

// ── Three-policy round-trips ──────────────────────────────────────────────────

#[test]
fn passthrough_policy_round_trip_bit_exact() {
    let config = make_config();
    let codebook = make_codebook(&config);
    let mut corpus = Corpus::new(
        Arc::from("pt-corpus"),
        config,
        codebook,
        CompressionPolicy::Passthrough,
        BTreeMap::new(),
    );
    let _ = corpus.drain_events();

    let original = make_vector(7);
    corpus.insert(Arc::from("v1"), &original, None, 0).unwrap();
    let recovered = corpus.decompress(&Arc::from("v1")).unwrap();

    assert_eq!(recovered.len(), original.len());
    for (orig, rec) in original.iter().zip(recovered.iter()) {
        assert_eq!(
            orig.to_bits(),
            rec.to_bits(),
            "Passthrough must be bit-exact"
        );
    }
}

#[test]
fn fp16_policy_round_trip_within_tolerance() {
    let config = make_config();
    let codebook = make_codebook(&config);
    let mut corpus = Corpus::new(
        Arc::from("fp16-corpus"),
        config,
        codebook,
        CompressionPolicy::Fp16,
        BTreeMap::new(),
    );
    let _ = corpus.drain_events();

    let original = make_vector(3);
    corpus.insert(Arc::from("v1"), &original, None, 0).unwrap();
    let recovered = corpus.decompress(&Arc::from("v1")).unwrap();

    assert_eq!(recovered.len(), original.len());
    let max_err = original
        .iter()
        .zip(recovered.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, f32::max);
    assert!(max_err < 1e-2, "Fp16 max error {max_err} >= 1e-2");
}

#[test]
fn compress_policy_round_trip_within_mse_bound() {
    let mut corpus = make_corpus();
    let _ = corpus.drain_events();

    let original = make_vector(0);
    corpus.insert(Arc::from("v1"), &original, None, 0).unwrap();
    let recovered = corpus.decompress(&Arc::from("v1")).unwrap();

    assert_eq!(recovered.len(), original.len());
    let mse: f32 = original
        .iter()
        .zip(recovered.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f32>()
        / original.len() as f32;
    assert!(mse < 1e-2, "Compress MSE {mse} >= 1e-2");
}

// ── VectorEntry::dimension() returns f32 vector dim, not byte count ───────────

#[test]
fn vector_entry_dimension_returns_f32_dim_for_passthrough() {
    let config = make_config();
    let codebook = make_codebook(&config);
    let mut corpus = Corpus::new(
        Arc::from("pt-dim-corpus"),
        config,
        codebook,
        CompressionPolicy::Passthrough,
        BTreeMap::new(),
    );
    let _ = corpus.drain_events();

    let original = make_vector(0);
    corpus.insert(Arc::from("v1"), &original, None, 0).unwrap();

    // dimension() must return the f32 vector length (DIM), not DIM*4 (byte count).
    let (_, entry) = corpus.iter().next().unwrap();
    assert_eq!(
        entry.dimension(),
        DIM,
        "Passthrough dimension() should return {} (f32 count), not {}",
        DIM,
        DIM * 4,
    );
}

#[test]
fn vector_entry_dimension_returns_f32_dim_for_fp16() {
    let config = make_config();
    let codebook = make_codebook(&config);
    let mut corpus = Corpus::new(
        Arc::from("fp16-dim-corpus"),
        config,
        codebook,
        CompressionPolicy::Fp16,
        BTreeMap::new(),
    );
    let _ = corpus.drain_events();

    let original = make_vector(0);
    corpus.insert(Arc::from("v1"), &original, None, 0).unwrap();

    // dimension() must return the f32 vector length (DIM), not DIM*2 (byte count).
    let (_, entry) = corpus.iter().next().unwrap();
    assert_eq!(
        entry.dimension(),
        DIM,
        "Fp16 dimension() should return {} (f32 count), not {}",
        DIM,
        DIM * 2,
    );
}

// ── decompress_all on empty corpus emits no event ─────────────────────────────

#[test]
fn decompress_all_on_empty_corpus_emits_no_event() {
    let mut corpus = make_corpus();
    // Drain the initial Created event.
    let _ = corpus.drain_events();

    let result = corpus.decompress_all_at(0).unwrap();
    assert!(result.is_empty());

    // No Decompressed event should be emitted for an empty corpus.
    let events = corpus.drain_events();
    assert!(
        events.is_empty(),
        "expected no events for empty corpus decompress_all_at, got {events:?}"
    );
}
