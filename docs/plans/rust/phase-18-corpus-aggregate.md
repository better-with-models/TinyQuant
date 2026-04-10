---
title: "Phase 18: Corpus Aggregate and Domain Events"
tags:
  - plans
  - rust
  - phase-18
  - corpus
  - domain-events
date-created: 2026-04-10
status: draft
category: planning
---

# Phase 18: Corpus Aggregate and Domain Events

> [!info] Goal
> Implement the `Corpus` aggregate root, `VectorEntry` entity,
> `CompressionPolicy` enum, and the `CorpusEvent` tagged union
> matching Python's event semantics.

> [!note] Reference docs
> - [[design/rust/type-mapping|Type Mapping]] §Corpus layer
> - [[design/domain-layer/aggregates-and-entities|Aggregates and Entities]]
> - [[design/behavior-layer/corpus-management|Corpus Management]]

## Prerequisites

- Phase 17 complete (serialization + mmap).

## Deliverables

### Files to create

| File | Purpose |
|------|---------|
| `rust/crates/tinyquant-core/src/corpus/mod.rs` | Corpus submodule root |
| `rust/crates/tinyquant-core/src/corpus/compression_policy.rs` | Enum |
| `rust/crates/tinyquant-core/src/corpus/vector_entry.rs` | Entity |
| `rust/crates/tinyquant-core/src/corpus/events.rs` | `CorpusEvent` tagged union |
| `rust/crates/tinyquant-core/src/corpus/corpus.rs` | Aggregate root |
| `rust/crates/tinyquant-core/tests/compression_policy.rs` | Enum tests |
| `rust/crates/tinyquant-core/tests/vector_entry.rs` | Entity tests |
| `rust/crates/tinyquant-core/tests/corpus_events.rs` | Event frozen-ness |
| `rust/crates/tinyquant-core/tests/corpus_aggregate.rs` | Full lifecycle |

## Steps (TDD order)

- [ ] **Step 1: Failing `CompressionPolicy` tests**

```rust
#[test]
fn compress_policy_requires_codec() {
    assert!(CompressionPolicy::Compress.requires_codec());
    assert!(!CompressionPolicy::Passthrough.requires_codec());
    assert!(!CompressionPolicy::Fp16.requires_codec());
}

#[test]
fn storage_tag_mapping() {
    assert_eq!(CompressionPolicy::Compress.storage_tag(), StorageTag::U8);
    assert_eq!(CompressionPolicy::Passthrough.storage_tag(), StorageTag::F32);
    assert_eq!(CompressionPolicy::Fp16.storage_tag(), StorageTag::F16);
}
```

- [ ] **Step 2: Implement `compression_policy.rs`**

Per [[design/rust/type-mapping|Type Mapping]] §CompressionPolicy.

- [ ] **Step 3: Failing `VectorEntry` tests**

- Equality by `vector_id` only.
- Hash by `vector_id` only.
- `config_hash`, `dimension`, `has_residual` delegate to
  `compressed`.
- Metadata mutability (`metadata_mut() -> &mut BTreeMap`).

- [ ] **Step 4: Implement `vector_entry.rs`**

```rust
#[derive(Clone, Debug)]
pub struct VectorEntry {
    vector_id: VectorId,
    compressed: CompressedVector,
    inserted_at: Timestamp,
    metadata: alloc::collections::BTreeMap<alloc::string::String, EntryMetaValue>,
}

impl PartialEq for VectorEntry {
    fn eq(&self, other: &Self) -> bool {
        self.vector_id == other.vector_id
    }
}

impl Eq for VectorEntry {}

impl core::hash::Hash for VectorEntry {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.vector_id.hash(state);
    }
}
```

`EntryMetaValue` is a simple `enum` with `Null`, `Bool(bool)`,
`Int(i64)`, `Float(f64)`, `String(Arc<str>)`, `Array(Vec<Self>)`,
`Object(BTreeMap<String, Self>)` — a `no_std`-compatible substitute
for `serde_json::Value`.

- [ ] **Step 5: Failing events test**

```rust
#[test]
fn vectors_inserted_event_carries_ids_and_count() {
    let e = CorpusEvent::VectorsInserted {
        corpus_id: Arc::from("c1"),
        vector_ids: Arc::from([Arc::from("a"), Arc::from("b")]),
        count: 2,
        timestamp: 0,
    };
    if let CorpusEvent::VectorsInserted { count, vector_ids, .. } = e {
        assert_eq!(count, 2);
        assert_eq!(vector_ids.len(), 2);
    } else {
        panic!("wrong variant");
    }
}
```

- [ ] **Step 6: Implement `events.rs`**

Tagged union as in
[[design/rust/type-mapping|Type Mapping]] §Events.

- [ ] **Step 7: Failing aggregate lifecycle test**

```rust
#[test]
fn corpus_create_insert_decompress_flow() {
    let config = /* CodecConfig(4, 42, 64, true) */;
    let codebook = /* trained */;
    let mut corpus = Corpus::new(
        Arc::from("corpus-1"),
        config,
        codebook,
        CompressionPolicy::Compress,
        BTreeMap::new(),
    );
    assert_eq!(corpus.vector_count(), 0);
    assert!(corpus.is_empty());

    let events = corpus.drain_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], CorpusEvent::Created { .. }));

    let v: Vec<f32> = (0..64).map(|i| (i as f32) * 0.01).collect();
    corpus.insert(Arc::from("v1"), &v, None).unwrap();
    assert_eq!(corpus.vector_count(), 1);

    let events = corpus.drain_events();
    assert!(matches!(events[0], CorpusEvent::VectorsInserted { .. }));

    let recovered = corpus.decompress(&Arc::from("v1")).unwrap();
    assert_eq!(recovered.len(), 64);
}
```

- [ ] **Step 8: Implement `corpus.rs`**

Key points:

- `Corpus` owns an insertion-ordered map
  (`indexmap::IndexMap<VectorId, VectorEntry, ahash::RandomState>`
  if `std` is enabled; falls back to a hand-rolled
  `VectorIdMap` on `no_std`).
- `insert` validates dimension, checks for duplicate vector id,
  compresses per policy, and appends a `VectorsInserted` event to
  `pending_events`.
- `drain_events(&mut self) -> Vec<CorpusEvent>` replaces
  `pending_events`.
- `decompress(&self, id) -> Result<Vec<f32>, CorpusError>` delegates
  to the codec service for `Compress` policy and reinterprets bytes
  for `Fp16` / `Passthrough`.
- `insert_batch` is atomic: validates all vectors before inserting
  any.

- [ ] **Step 9: Atomicity test**

```rust
#[test]
fn insert_batch_rolls_back_on_dimension_mismatch() {
    // 3 vectors where vector[1] has wrong length.
    // Expect DimensionMismatchError, corpus unchanged.
}
```

- [ ] **Step 10: Duplicate id test**

```rust
#[test]
fn insert_duplicate_id_raises_duplicate_vector_error() { /* … */ }
```

- [ ] **Step 11: Policy round-trip tests**

- `Compress`: recovered vector has MSE < 1e-2.
- `Passthrough`: `recovered == original` exactly.
- `Fp16`: `max(abs(recovered - original)) < 1e-2`.

- [ ] **Step 12: `decompress_all` test**

Asserts insertion-order iteration matches Python's dict order and
the right event is raised.

- [ ] **Step 13: Run workspace tests, clippy, fmt.**

- [ ] **Step 14: Update prelude**

```rust
pub use crate::corpus::{
    CompressionPolicy, Corpus, CorpusEvent, VectorEntry, ViolationKind,
    StorageTag,
};
```

- [ ] **Step 15: Commit**

```bash
git add rust/crates/tinyquant-core
git commit -m "feat(tinyquant-core): add Corpus aggregate, VectorEntry, and domain events"
```

## Acceptance criteria

- Aggregate lifecycle (create → insert → decompress → drain events)
  produces the expected events in Python order.
- Duplicate id and dimension mismatch rejected with exact error
  variants.
- All three compression policies round-trip correctly.
- `insert_batch` is atomic.
- Insertion order preserved for iteration.
- `no_std` build still green.
- Clippy + fmt clean.

## Out of scope

- pgvector adapter (phase 19).
- Backend search (phase 19).
- Corpus file persistence API (already landed as primitives in
  phase 17; no higher-level wrapper yet).

## See also

- [[plans/rust/phase-17-zero-copy-mmap|Phase 17]]
- [[plans/rust/phase-19-brute-force-pgvector|Phase 19]]
- [[design/domain-layer/aggregates-and-entities|Aggregates and Entities]]
- [[design/behavior-layer/corpus-management|Corpus Management]]
