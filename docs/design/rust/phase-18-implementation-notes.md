---
title: "Phase 18 Implementation Notes"
tags:
  - design
  - rust
  - phase-18
  - corpus
  - aggregate
date-created: 2026-04-11
category: design
---

# Phase 18 Implementation Notes

## What landed

Phase 18 added the `Corpus` aggregate root and surrounding domain layer to
`tinyquant-core`. The implementation covers vector insertion, batch atomicity,
three-policy decompression, domain events, and an insertion-ordered vector map.

### Files created

| File | Purpose |
|------|---------|
| `src/corpus/aggregate.rs` | `Corpus` aggregate root — insert, insert_batch, decompress, decompress_all, remove, drain_events, iter |
| `src/corpus/compression_policy.rs` | `CompressionPolicy` (Compress / Passthrough / Fp16) + `StorageTag` |
| `src/corpus/entry_meta_value.rs` | `EntryMetaValue` — no_std JSON substitute with bit-exact NaN equality |
| `src/corpus/vector_entry.rs` | `VectorEntry` entity (identity by `vector_id` only) |
| `src/corpus/vector_id_map.rs` | Insertion-ordered map with O(log n) lookup for no_std |
| `src/corpus/events.rs` | `CorpusEvent` (4 variants) + `ViolationKind` with Python tag strings |
| `src/corpus/errors.rs` | Re-export of `CorpusError`; adds `DimensionMismatch` + `BatchAtomicityFailure` |
| `src/corpus/mod.rs` | Module root; re-exports all public corpus types |
| `tests/corpus_aggregate.rs` | Full lifecycle integration tests for `Corpus` |
| `tests/corpus_events.rs` | `CorpusEvent` construction, matching, and drain semantics |
| `tests/corpus_insertion_order.rs` | JSON fixture round-trip verifying insertion-order preservation |
| `tests/entry_meta_value.rs` | `EntryMetaValue` variant tests |
| `tests/compression_policy.rs` | `CompressionPolicy` and `StorageTag` tests |
| `tests/vector_entry.rs` | `VectorEntry` entity tests |
| `tests/fixtures/corpus/insertion_order.json` | NumPy seed-42 permutation fixture for insertion-order test |

### Cargo.toml and module changes

- `src/lib.rs`: added `pub mod corpus`
- `src/prelude.rs`: added re-exports for `Corpus`, `CorpusEvent`, `CompressionPolicy`,
  `EntryMetaValue`, `VectorEntry`, `BatchReport`, `CorpusError`

## Deviations from plan

### D18.1 — `timestamp` parameter on `insert` and `insert_batch`

The Phase 18 spec API examples show:

```rust
corpus.insert(id, &vector, None)?;
corpus.insert_batch(&batch)?;
```

Both methods in the implementation take an additional `timestamp: Timestamp`
parameter:

```rust
corpus.insert(id, &vector, None, timestamp)?;
corpus.insert_batch(&batch, timestamp)?;
```

**Reason:** `tinyquant-core` is a `no_std` crate. Calling `std::time::SystemTime::now()`
is not available in a `no_std` context. Rather than internally hard-code a zero
timestamp (which would make event timestamps meaningless) or silently require a
`std` feature flag, the deviation pushes time-sourcing responsibility to the
caller. This is consistent with the `Corpus::new_at` constructor pattern and
with how the codec service layer (Phase 15) handles timestamps.

The parameter will remain until Phase 22 or later when a `std`-feature-gated
convenience wrapper can supply `SystemTime::now()` automatically.

### D18.2 — `VectorIdMap` only; `IndexMap` deferred to Phase 21

The plan considered using `IndexMap` (from the `indexmap` crate) for the
insertion-ordered vector map. The implementation uses a custom `VectorIdMap`
backed by a `BTreeMap<Arc<str>, usize>` index and a `Vec<(VectorId, V)>`
storage slab instead.

**Reason:** `indexmap` 2.x requires a `std` feature flag for its default hasher
(`RandomState`) and adds an external dependency. `VectorIdMap` is pure `no_std`,
uses only `alloc`, and satisfies the Phase 18 requirements (insertion-order
iteration, O(log n) lookup by string key, O(1) push). `IndexMap` integration
is deferred to Phase 21 per the plan's own recommendation.

### D18.3 — Module file named `aggregate.rs`, not `corpus.rs`

The corpus module root is `src/corpus/mod.rs`. The aggregate type lives in
`src/corpus/aggregate.rs` rather than `src/corpus/corpus.rs` to avoid the
`module_inception` Clippy lint, which fires when a module and its inner file
share the same name (e.g., `corpus::corpus`). The public API is unchanged.

### D18.4 — `pending_events()` accessor not exposed

The Phase 18 plan (Step 12a) explicitly defers a `peek_events` / `pending_events`
read-only accessor to Phase 22, where a pyo3-visible accessor will be added.
The `Corpus` struct has no `pub fn pending_events()` method; callers must use
`drain_events()` to inspect the event buffer. Test assertions that previously
used `corpus.pending_events().is_empty()` were rewritten to use
`corpus.drain_events().is_empty()` instead.

## Test counts

| Test file | Count |
|-----------|-------|
| `corpus_aggregate.rs` | 16 |
| `corpus_events.rs` | 8 |
| `vector_entry.rs` | 9 |
| `entry_meta_value.rs` | 13 |
| `compression_policy.rs` | 5 |
| `corpus_insertion_order.rs` | 1 |
| **Total (Phase 18 new)** | **52** |
