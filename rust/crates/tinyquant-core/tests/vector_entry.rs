//! Integration tests for [`VectorEntry`].
//!
//! Core contracts:
//! - Equality and hashing are by `vector_id` only.
//! - Metadata accessors work correctly.
//! - Delegation helpers (`dimension`, `has_residual`, `config_hash`) forward correctly.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use tinyquant_core::{
    codec::{Codebook, CodecConfig},
    corpus::{EntryMetaValue, VectorEntry},
};

fn make_config() -> CodecConfig {
    CodecConfig::new(4, 42, 8, false).unwrap()
}

fn make_codebook(config: &CodecConfig) -> Codebook {
    // 8-dim training data — enough for 16 codebook entries (2^4).
    let data: Vec<f32> = (0..8000).map(|i| i as f32 * 0.001).collect();
    Codebook::train(&data, config).unwrap()
}

fn make_entry(id: &str, dim: u8) -> VectorEntry {
    let config = CodecConfig::new(4, 42, dim as u32, false).unwrap();
    let data: Vec<f32> = (0..usize::from(dim) * 1000)
        .map(|i| i as f32 * 0.001)
        .collect();
    let codebook = Codebook::train(&data, &config).unwrap();
    let vector: Vec<f32> = (0..usize::from(dim)).map(|i| i as f32 * 0.01).collect();

    use tinyquant_core::codec::Codec;
    let compressed = Codec::new().compress(&vector, &config, &codebook).unwrap();
    VectorEntry::new(Arc::from(id), compressed, dim as u32, 0, BTreeMap::new())
}

// ── Identity ──────────────────────────────────────────────────────────────────

#[test]
fn vector_entry_equality_by_id_only() {
    let a = make_entry("v1", 8);
    let b = make_entry("v1", 8); // same id, potentially different payload
    assert_eq!(a, b);
}

#[test]
fn vector_entry_inequality_by_id() {
    let a = make_entry("v1", 8);
    let b = make_entry("v2", 8);
    assert_ne!(a, b);
}

#[test]
fn vector_entry_hash_by_id_only() {
    let mut map: HashMap<VectorEntry, &str> = HashMap::new();
    let a = make_entry("v1", 8);
    let b = make_entry("v1", 8);
    map.insert(a, "first");
    map.insert(b, "second"); // should overwrite because same id
    assert_eq!(map.len(), 1);
    assert_eq!(*map.values().next().unwrap(), "second");
}

// ── Accessors ─────────────────────────────────────────────────────────────────

#[test]
fn vector_entry_vector_id_accessor() {
    let entry = make_entry("my-vec", 8);
    assert_eq!(entry.vector_id().as_ref(), "my-vec");
}

#[test]
fn vector_entry_dimension_delegates_to_compressed() {
    let entry = make_entry("v1", 8);
    assert_eq!(entry.dimension(), 8);
}

#[test]
fn vector_entry_has_residual_false_when_disabled() {
    let entry = make_entry("v1", 8); // residual_enabled = false
    assert!(!entry.has_residual());
}

#[test]
fn vector_entry_config_hash_is_stable() {
    let config = make_config();
    let codebook = make_codebook(&config);
    use tinyquant_core::codec::Codec;
    let vector: Vec<f32> = (0..8).map(|i| i as f32 * 0.01).collect();
    let compressed = Codec::new().compress(&vector, &config, &codebook).unwrap();
    let entry = VectorEntry::new(Arc::from("v"), compressed, 8, 0, BTreeMap::new());
    assert_eq!(entry.config_hash().as_ref(), config.config_hash().as_ref());
}

// ── Metadata ──────────────────────────────────────────────────────────────────

#[test]
fn vector_entry_metadata_mut_allows_update() {
    let config = make_config();
    let codebook = make_codebook(&config);
    use tinyquant_core::codec::Codec;
    let vector: Vec<f32> = (0..8).map(|i| i as f32 * 0.01).collect();
    let compressed = Codec::new().compress(&vector, &config, &codebook).unwrap();
    let mut entry = VectorEntry::new(Arc::from("v"), compressed, 8, 0, BTreeMap::new());

    entry
        .metadata_mut()
        .insert("key".to_string(), EntryMetaValue::Int(99));
    assert_eq!(entry.metadata().get("key"), Some(&EntryMetaValue::Int(99)));
}

#[test]
fn vector_entry_inserted_at_is_stored() {
    let config = make_config();
    let codebook = make_codebook(&config);
    use tinyquant_core::codec::Codec;
    let vector: Vec<f32> = (0..8).map(|i| i as f32 * 0.01).collect();
    let compressed = Codec::new().compress(&vector, &config, &codebook).unwrap();
    let ts = 1_700_000_000_000_000_000_i64;
    let entry = VectorEntry::new(Arc::from("v"), compressed, 8, ts, BTreeMap::new());
    assert_eq!(entry.inserted_at(), ts);
}
