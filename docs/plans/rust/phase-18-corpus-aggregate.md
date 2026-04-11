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
> - [[design/rust/error-model|Error Model]] §CorpusError
> - [[design/rust/memory-layout|Memory Layout]] §Corpus layout
> - [[design/rust/feature-flags|Feature Flags]] §`std` vs `no_std`
> - [[design/rust/testing-strategy|Testing Strategy]] §Aggregate tests
> - [[design/rust/crate-topology|Crate Topology]] — `tinyquant-core` owns `corpus/`
> - [[design/domain-layer/aggregates-and-entities|Aggregates and Entities]]
> - [[design/domain-layer/domain-events|Domain Events]] §Corpus Context
> - [[design/behavior-layer/corpus-management|Corpus Management]]

## Prerequisites

- Phase 17 complete (serialization + mmap).
- [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]]
  read — the CI health and LFS hydration lessons (L1/L2) and the
  clippy profile gotchas (L7) apply verbatim to this phase.

## Deliverables

### Domain event schema

Matches `tinyquant_cpu.corpus.events` one-for-one. The Python module
declares exactly four event classes; Rust mirrors those four and adds
**no new variants**. Removal and clear operations in Python are silent
mutations (no event emitted); Rust preserves that parity.

| Variant | Trigger | Payload fields | Python parity type |
|---|---|---|---|
| `Created` | `Corpus::new` returns Ok | `corpus_id: CorpusId`, `codec_config: CodecConfig`, `compression_policy: CompressionPolicy`, `timestamp: Timestamp` | `CorpusCreated` |
| `VectorsInserted` | `insert` / `insert_batch` success | `corpus_id: CorpusId`, `vector_ids: Arc<[VectorId]>`, `count: u32`, `timestamp: Timestamp` | `VectorsInserted` |
| `Decompressed` | `decompress_all` success | `corpus_id: CorpusId`, `vector_count: u32`, `timestamp: Timestamp` | `CorpusDecompressed` |
| `PolicyViolationDetected` | Dimension mismatch, duplicate id, config mismatch, or `Passthrough`-only op on `Compress` corpus | `corpus_id: CorpusId`, `kind: ViolationKind`, `detail: Arc<str>`, `timestamp: Timestamp` | `CompressionPolicyViolationDetected` |

> [!warning] Events NOT raised (parity)
> - **No** `VectorsRemoved` — `Corpus::remove` is a silent `del` in
>   Python. Do not add one.
> - **No** `VectorDecompressed` (singular) — Python's `decompress(id)`
>   does not emit; only the batch `decompress_all` path does.
> - **No** `CorpusCleared` — not a method on the Python aggregate.
> - **No** `PolicyChanged` — Python makes `CompressionPolicy`
>   immutable after the first insert; Rust follows suit and returns
>   `CorpusError::PolicyImmutable`.

`ViolationKind` values must string-match Python's `violation_type`
exactly: `config_mismatch`, `policy_conflict`, `duplicate_id`,
`dimension_mismatch`.

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ViolationKind {
    ConfigMismatch,
    PolicyConflict,
    DuplicateId,
    DimensionMismatch,
}

impl ViolationKind {
    pub const fn as_python_tag(self) -> &'static str {
        match self {
            Self::ConfigMismatch => "config_mismatch",
            Self::PolicyConflict => "policy_conflict",
            Self::DuplicateId => "duplicate_id",
            Self::DimensionMismatch => "dimension_mismatch",
        }
    }
}
```

### Files to create

| File | Purpose | Module visibility | Feature-gated |
|---|---|---|---|
| `rust/crates/tinyquant-core/src/corpus/mod.rs` | Corpus submodule root + re-exports | `pub` | no |
| `rust/crates/tinyquant-core/src/corpus/compression_policy.rs` | `CompressionPolicy`, `StorageTag` | `pub(super)` + `pub` items | no |
| `rust/crates/tinyquant-core/src/corpus/vector_entry.rs` | `VectorEntry` entity | `pub(super)` + `pub` struct | no |
| `rust/crates/tinyquant-core/src/corpus/entry_meta_value.rs` | `EntryMetaValue` — no_std JSON substitute | `pub(super)` + `pub` enum | no |
| `rust/crates/tinyquant-core/src/corpus/vector_id_map.rs` | no_std fallback insertion-ordered map | `pub(crate)` | `cfg(not(feature = "std"))` |
| `rust/crates/tinyquant-core/src/corpus/events.rs` | `CorpusEvent`, `ViolationKind` tagged unions | `pub(super)` + `pub` items | no |
| `rust/crates/tinyquant-core/src/corpus/errors.rs` | `CorpusError` (or fold into `crate::errors`) | `pub(crate)` | no |
| `rust/crates/tinyquant-core/src/corpus/corpus.rs` | `Corpus` aggregate root | `pub(super)` + `pub` struct | no |
| `rust/crates/tinyquant-core/tests/compression_policy.rs` | Enum tests | n/a (integration) | no |
| `rust/crates/tinyquant-core/tests/vector_entry.rs` | Entity tests | n/a | no |
| `rust/crates/tinyquant-core/tests/entry_meta_value.rs` | Meta value equality + clone-cost tests | n/a | no |
| `rust/crates/tinyquant-core/tests/corpus_events.rs` | Event frozen-ness + payload parity | n/a | no |
| `rust/crates/tinyquant-core/tests/corpus_aggregate.rs` | Full lifecycle | n/a | no |
| `rust/crates/tinyquant-core/tests/corpus_insertion_order.rs` | Python-fixture parity | n/a | no |
| `rust/crates/tinyquant-core/tests/fixtures/corpus/insertion_order.json` | Committed fixture — 100 vector ids in Python's insertion order | n/a | no (plain, non-LFS; see CI section) |

> [!note] `errors.rs` placement
> The `CorpusError` type already appears in
> [[design/rust/type-mapping|Type Mapping]] under the crate-level
> `errors` module. This phase ships a thin
> `corpus/errors.rs` that `pub use`s from `crate::errors::CorpusError`
> so that downstream imports can read
> `tinyquant_core::corpus::CorpusError`. The canonical definition
> stays in `crate::errors` to match Phase 14's layout.

### EntryMetaValue schema

`EntryMetaValue` is TinyQuant's `no_std` substitute for
`serde_json::Value`. It is the payload type of
`VectorEntry::metadata`.

```rust
// rust/crates/tinyquant-core/src/corpus/entry_meta_value.rs
use alloc::collections::BTreeMap;
use alloc::sync::Arc;

#[derive(Clone, Debug)]
pub enum EntryMetaValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Arc<str>),
    Bytes(Arc<[u8]>),
    Array(Arc<[Self]>),
    Object(Arc<BTreeMap<Arc<str>, Self>>),
}
```

**Why this shape:**

- **`Arc` everywhere.** Clone is O(1) refcount bump. A `VectorEntry`
  is cloned for every `VectorsInserted` event dispatch and every
  iterator yield; a deep-copying enum would blow the allocator budget.
  See [[design/rust/memory-layout|Memory Layout]] §Batch
  compression allocation discipline.
- **No `serde_json::Value`.** `serde_json` pulls `std` unconditionally
  and allocates owned `String`/`Vec` for every node. That violates the
  [[design/rust/feature-flags|`tinyquant-core`]] `no_std + alloc`
  posture.
- **`BTreeMap`, not `HashMap`.** `HashMap` needs `std` for its
  default hasher; `BTreeMap` is in `alloc`. Key ordering also becomes
  deterministic, which helps with test fixtures.
- **`Bytes` variant.** Python's metadata accepts `bytes` values
  directly; without this variant we'd have to base64-encode through
  `String`, which loses Python parity.

**Equality semantics:**

- `PartialEq` is derived on every variant **except** `Float`, which
  is hand-implemented to match Python's `dict` key-stability
  contract: `Float(a) == Float(b)` iff
  `a.to_bits() == b.to_bits()`. That makes `NaN == NaN` return
  `true`, which is *not* IEEE 754 semantics but **is** what Python's
  dict uses for hashing; metadata round-tripping must not drop
  `NaN` keys or values. Document the divergence in the rustdoc and
  a test named
  `entry_meta_value_nan_equal_for_dict_stability`.
- `Eq` is implemented by fiat via the bit-exact float comparison.
- **No `Ord` impl.** JSON values have no natural total order; adding
  one would invite bugs. If a caller needs ordered metadata they use
  the `Object` variant's `BTreeMap` directly.

**Size budget:** on 64-bit, the enum is one `u64` discriminant + the
largest variant (`Arc<BTreeMap>` is 8 bytes). Total 16 bytes.
`VectorEntry::metadata: BTreeMap<String, EntryMetaValue>` therefore
costs ~40 bytes per entry plus the Arc payloads.

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

- [ ] **Step 1a: Define `CorpusError`**

Canonical definition lives in `crate::errors`, re-exported through
`corpus::errors`. Variants:

| Variant | When raised | Python parity | Test |
|---|---|---|---|
| `DimensionMismatch { expected: u32, got: u32 }` | `insert` / `insert_batch` with wrong-length vector | `DimensionMismatchError` | `insert_wrong_dim_raises_dimension_mismatch` |
| `DuplicateVectorId { id: VectorId }` | `insert` with an id already in the map | `DuplicateVectorError` | `insert_duplicate_id_raises_duplicate_vector_error` |
| `UnknownVectorId { id: VectorId }` | `decompress(&id)` / `remove(&id)` with missing id | `KeyError` (Python dict semantics) | `decompress_unknown_id_raises_unknown_vector_id` |
| `Codec(CodecError)` | `#[from]` wraps any lower-layer codec failure | whichever codec error the cause raised | `compress_failure_wraps_codec_error` |
| `PolicyImmutable` | attempting to change `compression_policy` after any insert | `ValueError` ("policy immutability error") | `policy_change_after_insert_rejected` |
| `BatchAtomicityFailure { index: usize, source: Box<CorpusError> }` | `insert_batch` rolled back because vector at `index` failed validation; `source` is the first failing error | `ValueError` with index in message | `insert_batch_rolls_back_on_dimension_mismatch` |

`BatchAtomicityFailure` is new to Phase 18. It wraps the first failing
sub-error plus the zero-based index at which the failure occurred, so
the caller can attribute the fault without re-running validation. The
`source` field is `Box<CorpusError>` to keep the enum `Sized`.

- [ ] **Step 2: Implement `compression_policy.rs`**

Per [[design/rust/type-mapping|Type Mapping]] §CompressionPolicy.

- [ ] **Step 3: Failing `VectorEntry` tests**

- Equality by `vector_id` only.
- Hash by `vector_id` only.
- `config_hash`, `dimension`, `has_residual` delegate to
  `compressed`.
- Metadata mutability (`metadata_mut() -> &mut BTreeMap`).
- `EntryMetaValue::Float(nan)` equals itself (dict-stability test).

- [ ] **Step 4: Implement `vector_entry.rs` and `entry_meta_value.rs`**

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

`EntryMetaValue` ships with its schema as defined in the
[[#EntryMetaValue schema|§EntryMetaValue schema]] subsection above.

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

Tagged union as in [[design/rust/type-mapping|Type Mapping]] §Events.
Include the `ViolationKind` enum and its `as_python_tag` helper.

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

- `Corpus` owns an insertion-ordered map. Under `feature = "std"` it
  is `indexmap::IndexMap<VectorId, VectorEntry, ahash::RandomState>`.
  Under `no_std + alloc` it falls back to the hand-rolled
  `VectorIdMap` (see [[#Insertion-order parity|§Insertion-order parity]]).
- `insert` validates dimension, checks for duplicate vector id,
  compresses per policy, and appends a `VectorsInserted` event to
  `pending_events` **only after** the mutation succeeds.
- `drain_events(&mut self) -> Vec<CorpusEvent>` swaps
  `pending_events` with an empty `Vec` and returns the old contents
  (see [[#Event drain semantics|§Event drain semantics]]).
- `decompress(&self, id) -> Result<Vec<f32>, CorpusError>` delegates
  to the codec service for `Compress` policy and reinterprets bytes
  for `Fp16` / `Passthrough`. Does **not** emit an event (parity).
- `decompress_all(&mut self) -> Result<BTreeMap<VectorId, Vec<f32>>, CorpusError>`
  iterates in insertion order and emits exactly one
  `CorpusEvent::Decompressed` on success.
- `insert_batch` is atomic — see [[#Atomicity proof strategy|§Atomicity proof strategy]]
  and [[#insert_batch API contract|§insert_batch API contract]].

### Insertion-order parity

Python's `dict` has preserved insertion order since 3.7; tests and
consumers rely on it. Rust replicates this via
`indexmap::IndexMap<VectorId, VectorEntry, ahash::RandomState>` under
`feature = "std"`. Important subtleties:

- **`ahash` randomizes per-process, not per-corpus.** Two `Corpus`
  instances in the same process share hash state; two in different
  processes do not. Aggregate equality tests that compare serialized
  corpora across process boundaries must therefore iterate in
  **insertion order**, not hash order — do not hash-compare or
  rely on `IndexMap::Debug` format.
- **Committed fixture.** `tests/fixtures/corpus/insertion_order.json`
  is a plain JSON array of 100 string ids captured from a seeded
  Python scenario:

  ```python
  # scripts/generate_rust_fixtures.py corpus-insertion-order
  rng = np.random.default_rng(42)
  ids = [f"v{i:04d}" for i in rng.permutation(100)]
  json.dump(ids, open("insertion_order.json", "w"))
  ```

  The parity test reads the file at runtime (matching Phase 14's
  runtime-fixture discipline), inserts each id into a `Corpus`
  instance, then asserts `corpus.iter().map(|(id, _)| id).collect::<Vec<_>>()`
  equals the fixture sequence.
- **Fixture size.** <4 KiB — stays in Git plain, **not** LFS. Keeps
  `actions/checkout` happy without `lfs: true`.

**`no_std` fallback — `VectorIdMap`.** Under `no_std`, `ahash` and
`indexmap` are available but pull `std` by default. Gating them with
`default-features = false` works for `indexmap` but not cleanly for
`ahash`. Phase 18 ships a small purpose-built map instead:

```rust
// corpus/vector_id_map.rs (only compiled when std is OFF)
pub(crate) struct VectorIdMap<V> {
    // Parallel vectors — insertion-ordered. Lookup is O(log n) via
    // a secondary sorted index.
    entries: alloc::vec::Vec<(VectorId, V)>,
    sorted_index: alloc::vec::Vec<usize>, // sorted by VectorId
}
```

- `insert`: O(log n) lookup + O(n) sorted-index insert. Amortized
  acceptable for the target footprint (<10k entries on embedded).
- `get`: O(log n) binary search.
- `iter`: straight walk of `entries` — preserves insertion order.
- `remove`: O(n) — acceptable, removal is rare.

**Why not `Vec<(VectorId, V)>` alone?** At 10k entries linear scan is
~2 µs on a Cortex-M4F; the sorted-index pays for itself above ~500
entries.

### Atomicity proof strategy

`insert_batch` is all-or-nothing. Proof protocol:

1. **Pre-state checksum.** Compute a SHA-256 over
   `corpus.iter().flat_map(|(id, entry)| (id.as_bytes(), entry.compressed.to_bytes()))`
   and stash it before the call.
2. **Run the failing batch.** Build a batch of 3 vectors where
   `vectors[1]` has wrong `dim`. Call `insert_batch`.
3. **Assert error.** Expect
   `Err(CorpusError::BatchAtomicityFailure { index: 1, source: Box::new(CorpusError::DimensionMismatch { .. }) })`.
4. **Post-state checksum.** Recompute the checksum and assert bit
   equality with the pre-state. No rollback is allowed to leave stray
   partial state.
5. **Event channel unchanged.** Assert
   `corpus.pending_events().is_empty()` — no half-fired
   `VectorsInserted` event.

**Implementation note — staging buffer vs rollback log.** Two options:

- **(A) Staging buffer.** Collect every `CompressedVector` into a
  `Vec<(VectorId, VectorEntry)>` **first**, validate every entry,
  then flip into the map in a single pass at the end.
- **(B) Rollback log.** Record an `undo` entry per insertion; on
  failure replay the undo entries in reverse to restore state.

**Pick (A) staging buffer.** Reasons: (1) the invariant is simpler
— "map is never touched until we commit"; (2) peak memory is the
same (the rollback log also carries one entry per insert); (3) no
mid-batch event visibility; (4) the `pub(crate) fn commit_staged`
internal helper naturally slots in as a single atomic flip.

The staging buffer is a local variable in `insert_batch`; it is
`drop`-freed on both success and failure. No persistent state.

### Three-policy round-trip matrix

Step 11 lands one round-trip test per policy, matching the fidelity
bars Python asserts on the same inputs.

| Policy | `CompressionPolicy` variant | `StorageTag` | Pre-compression | Post-compression size | Fidelity gate |
|---|---|---|---|---|---|
| Compress (bw=4 + residual) | `CompressionPolicy::Compress` | `StorageTag::U8` | 1024 × f32 = 4096 B | 1024 × 4 bits + 1024 × 2 B residual = 2560 B (≈ 8× with bw=4 no residual, ≈ 1.6× with residual) | `MSE(reconstructed, original) < 1e-2` |
| Passthrough | `CompressionPolicy::Passthrough` | `StorageTag::F32` | 1024 × f32 = 4096 B | 4096 B (1× — no transform) | `recovered == original` (bit-exact) |
| Fp16 | `CompressionPolicy::Fp16` | `StorageTag::F16` | 1024 × f32 = 4096 B | 2048 B (2×) | `max(abs(recovered - original)) < 1e-2` |

Fidelity bars derive from the Python reference in
`tests/corpus/test_corpus_policy_roundtrip.py`. Each Rust test
re-uses the same seeded input vector (`np.random.default_rng(42)
.standard_normal(1024).astype(np.float32)`) captured as a fixture
file to prevent drift.

### insert_batch API contract

**Default (strict, all-or-nothing) signature:**

```rust
pub fn insert_batch(
    &mut self,
    vectors: &[(VectorId, &[f32], Option<EntryMetaValue>)],
) -> Result<BatchReport, CorpusError>;
```

`BatchReport` carries accounting so the caller does not need to
re-count:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchReport {
    /// Number of vectors successfully inserted. In strict mode this
    /// is either 0 (on `Err`) or `vectors.len()` (on `Ok`).
    pub inserted: usize,
    /// Number of vectors skipped because of soft failure. Always 0
    /// in strict mode.
    pub skipped: usize,
    /// First error encountered, with its zero-based index. `None`
    /// in strict mode on success; `Some` in lenient mode if any
    /// vector was rejected.
    pub first_error: Option<(usize, CorpusError)>,
}
```

**Event emission.** On success, a **single** `VectorsInserted` event
is emitted with all ids in insertion order. Never one event per
vector. Batch atomicity means the event corresponds to a single
state transition.

**Alternative (follow-up, not Phase 18):** `insert_batch_lenient`
with signature identical to `insert_batch` but semantics
`skip_duplicates: true, continue_on_dimension_mismatch: true`. Each
skipped vector increments `skipped`; the returned `BatchReport`
records `first_error` but `inserted` reflects what landed. This
variant fires **one** `VectorsInserted` event per successful insert
group (still not per vector). Python does not expose it yet, so the
default stays strict. Tracked in Phase 22 follow-ups.

- [ ] **Step 9: Atomicity test**

Follows the [[#Atomicity proof strategy|§Atomicity proof strategy]]
above.

```rust
#[test]
fn insert_batch_rolls_back_on_dimension_mismatch() {
    // Pre-checksum the empty corpus.
    // Build 3 vectors; vectors[1] has length 32 instead of 64.
    // Call insert_batch, expect BatchAtomicityFailure { index: 1, .. }.
    // Post-checksum and assert equality with pre-checksum.
    // Assert corpus.pending_events() is empty.
}
```

- [ ] **Step 10: Duplicate id test**

```rust
#[test]
fn insert_duplicate_id_raises_duplicate_vector_error() {
    // Insert v1 successfully; insert v1 again; expect
    // CorpusError::DuplicateVectorId { id }. Post-state: one entry,
    // one VectorsInserted event (from the successful first call).
}
```

- [ ] **Step 11: Policy round-trip tests**

See the [[#Three-policy round-trip matrix|§Three-policy round-trip matrix]]
above. One test per policy; all three share the seeded input fixture.

- [ ] **Step 12: `decompress_all` test**

Asserts insertion-order iteration matches the Python dict order fed
by `insertion_order.json` and that exactly one
`CorpusEvent::Decompressed` is emitted with
`vector_count == corpus.len()`.

- [ ] **Step 12a: Event drain semantics**

The only public API that empties the event buffer is `drain_events`.
Contract (enforced by tests in `corpus_events.rs`):

```rust
pub fn drain_events(&mut self) -> Vec<CorpusEvent> {
    core::mem::take(&mut self.pending_events)
}
```

- **Swap, don't clone.** Use `core::mem::take` (which internally
  `mem::replace`s with `Vec::new()`). Never `clone().into_iter().collect()`
  — that would double-allocate and `unwrap_used`/`expect_used` bans
  make the clone path noisier than necessary.
- **Post-drain chain.** After `drain_events`, the next mutation
  produces a fresh event chain starting from the current state.
  There is no residual partial state; the buffer is observably
  empty.
- **Events emit after state mutation, never before.** Ordering is
  strictly:
  1. Validate
  2. Mutate `self.vectors` (and policy, if applicable)
  3. Push to `self.pending_events`

  A failed validate step never leaks a pending event. This is
  tested via the atomicity test: on `Err(..)`, `pending_events`
  length is unchanged.
- **No `peek_events` in Phase 18.** Read-only inspection of the
  pending buffer is deferred until Phase 22 adds a pyo3-visible
  accessor; exposing it now would risk a silent contract drift
  with Python.
- **Python parity.** `tinyquant_cpu.corpus.Corpus.drain_events()`
  clears `_pending_events` via `.clear()` then returns the snapshot.
  Rust semantics match exactly via `mem::take`.

- [ ] **Step 13: Run workspace tests, clippy, fmt**

Must pass both `cargo test -p tinyquant-core --features "std"` and
`cargo test -p tinyquant-core --no-default-features --features "alloc"`.
See [[#CI integration|§CI integration]] for the exact invocations.

- [ ] **Step 14: Update prelude and module exports**

`tinyquant-core/src/corpus/mod.rs`:

```rust
mod compression_policy;
mod corpus;
mod entry_meta_value;
mod events;
mod vector_entry;

#[cfg(not(feature = "std"))]
pub(crate) mod vector_id_map;

pub use self::compression_policy::{CompressionPolicy, StorageTag};
pub use self::corpus::{BatchReport, Corpus};
pub use self::entry_meta_value::EntryMetaValue;
pub use self::events::{CorpusEvent, ViolationKind};
pub use self::vector_entry::VectorEntry;
// CorpusError re-exported from crate::errors for ergonomic access.
pub use crate::errors::CorpusError;
```

And `tinyquant-core/src/lib.rs` prelude addition:

```rust
pub mod prelude {
    // ... existing exports ...
    pub use crate::corpus::{
        BatchReport, CompressionPolicy, Corpus, CorpusError, CorpusEvent,
        EntryMetaValue, StorageTag, VectorEntry, ViolationKind,
    };
}
```

**Granular exports, not `pub use crate::corpus::events::*;`.** The
glob re-export was considered and rejected: it would hide future
`events.rs` additions from PR review, making it easier to
accidentally ship a variant that has no Python counterpart. Keeping
the list explicit forces every new type to pass through the prelude
gate.

- [ ] **Step 15: Commit**

```bash
git add rust/crates/tinyquant-core
git commit -m "feat(tinyquant-core): add Corpus aggregate, VectorEntry, and domain events"
```

## Acceptance criteria

- Aggregate lifecycle (create → insert → decompress → drain events)
  produces the expected events in Python order.
- Duplicate id and dimension mismatch rejected with exact error
  variants; failed inserts leave no pending events.
- All three compression policies round-trip within the fidelity
  bars of the [[#Three-policy round-trip matrix|§policy matrix]].
- `insert_batch` is atomic — pre-state checksum equals post-state
  checksum on failure.
- `Corpus::iter()` matches the committed `insertion_order.json`
  fixture for the seeded Python scenario.
- `no_std` build (`--no-default-features --features alloc`) still
  green through the full test suite using the `VectorIdMap`
  fallback.
- Clippy + fmt clean under the crate's pedantic lint wall.

## CI integration

Cross-checked against [[design/rust/phase-14-implementation-notes|Phase 14
Implementation Notes]] §L1–L2.

**Required `cargo` invocations** (both must succeed before merge):

```bash
cargo test -p tinyquant-core --features "std"
cargo test -p tinyquant-core --no-default-features --features "alloc"
```

The second command exercises the `VectorIdMap` fallback; without it
the `no_std` code path is unreachable in CI and will bit-rot.

**Insertion-order fixture drift gate:**

```bash
git diff --exit-code rust/crates/tinyquant-core/tests/fixtures/corpus/
```

Run this step after `cargo xtask fixtures refresh-all` (if the
Python fixture generator is invoked). A non-empty diff means the
Python side drifted and the Rust assertion would have silently
passed after a refresh. Force the diff failure before merge.

**LFS hydration — Phase 14 L2 carryover.** The Phase 14 notes record
that `actions/checkout@v4` pulls LFS pointer files by default unless
`with: { lfs: true }` is set. The `insertion_order.json` fixture
is:

- **Not** LFS-tracked. It is plain UTF-8 JSON under 4 KiB.
- Committed directly to Git.
- Explicitly listed in `.gitattributes` (if a broad LFS glob exists
  for `tests/fixtures/**`) with a `!filter` entry to exempt it.

This sidesteps the Phase 13 / Phase 14 LFS-on-CI failure mode
entirely: there is no LFS blob to hydrate. Phase 19 or later, if
it introduces large binary fixtures, can opt back into LFS with
the `lfs: true` checkout setting.

**CI health check — Phase 14 L1 carryover.** Phase exit requires a
green run of the workflow that owns the new code:

```bash
gh run list --workflow rust-ci.yml --branch <branch> --limit 5
```

At least the top entry must be `completed / success`. Local tests
green is **not** sufficient. Add this check to the phase playbook
exit checklist.

### Clippy profile gotchas

Ported from [[design/rust/phase-14-implementation-notes|Phase 14 L7]].
The crate denies
`clippy::pedantic + clippy::nursery + clippy::unwrap_used + clippy::expect_used + clippy::panic`.
Known friction points for Phase 18:

- **`clippy::indexing_slicing`** — the staging buffer iteration
  after a validation failure must use `iter().skip(i)` or
  `iter().take(i)`, never `buf[i..]`. A narrow `#[allow]` on the
  single rollback helper is acceptable if the iterator form
  costs clarity.
- **`clippy::cast_possible_truncation`** on `timestamp as u64` when
  normalizing the `Timestamp` alias at the event boundary. Use a
  narrow `#[allow(clippy::cast_possible_truncation)]` on the exact
  line with a `// SAFETY: …` rustdoc comment.
- **`clippy::missing_fields_in_debug`** on hand-written `Debug` for
  `VectorEntry` — the metadata map is debug-formatted via a
  pretty-printer helper; mention every field in the `debug_struct`
  builder or tag the impl with
  `#[allow(clippy::missing_fields_in_debug)]` and justify in a
  comment.
- **`clippy::unwrap_used` / `clippy::expect_used` ban** — the drain
  path MUST use `core::mem::take(&mut self.pending_events)` and
  not `self.pending_events.clone().into_iter().collect()`, which
  would need `.expect` somewhere in the Arc pipeline.
- **`clippy::redundant_pub_crate`** — inside `pub(crate) mod corpus { ... }`
  the inner items should be declared `pub`, not `pub(crate)`. Phase
  14 hit this when the author wrote `pub(crate) fn` inside a
  `pub(crate) mod`; Clippy rewrote the inner to `pub`. Start with
  `pub` and save the churn.

Each of these should be in the PR description's "known friction"
list before the first Clippy run.

## Risks

- **R18.1: insertion order drifts from Python.** The Rust
  `IndexMap` preserves insertion order, but the Python side could
  change its iteration contract (very unlikely given
  [PEP 468](https://peps.python.org/pep-0468/), but
  `dict.popitem(last=False)` etc. could be slipped into the future
  corpus). Mitigation: the committed `insertion_order.json` fixture
  + parity test. Any drift breaks the test loudly.
- **R18.2: `ahash` / `indexmap` pull `std` by default.** Both
  crates ship with default features that tug `std` into
  `no_std + alloc` builds. Mitigation: `default-features = false`
  in `tinyquant-core/Cargo.toml`, and the fallback `VectorIdMap`
  for the `no_std` build. CI runs both feature configurations.
- **R18.3: `drain_events` races with concurrent reads.** `Corpus`
  is `Send` but not `Sync` (see
  [[design/rust/memory-layout|Memory Layout]] §Thread safety) —
  callers must wrap in a `Mutex` for cross-thread use. Mitigation:
  document `!Sync` in the rustdoc; if the compiler-level bound is
  not enforced by the current field set, add a
  `_not_sync: PhantomData<*const ()>` phantom field with a
  `#[allow(dead_code)]` comment.
- **R18.4: `EntryMetaValue` deep clone is expensive.** A naive
  clone on a deeply nested `Object` could be O(n) if `Arc` is
  forgotten at any level. Mitigation: `Arc` is mandatory at every
  collection variant; bench a 1k-node round-trip before accepting
  the implementation. The bench goes into `tinyquant-bench` and
  runs non-blocking.
- **R18.5: `insert_batch` rollback cost on large batches.** The
  staging buffer allocates O(n) temporary state; a 100k-vector
  insert would briefly peak at 2× the steady-state footprint.
  Mitigation: document the peak in the rustdoc; large-batch users
  should chunk to ~10k per call. Tested with a 10k synthetic batch
  in `tests/corpus_aggregate.rs`.
- **R18.6: Python `drain_events` tuple ordering.** Python returns a
  Python `list[CorpusEvent]` in insertion order; Rust returns
  `Vec<CorpusEvent>` in insertion order. The two need to match
  exactly including relative ordering of `Created` versus
  `VectorsInserted` (Created always first). Mitigation: a
  round-trip fixture in `tinyquant-py/tests/test_parity.py` that
  calls both runtimes and asserts variant-sequence equality.

## Out of scope

- pgvector adapter ([[plans/rust/phase-19-brute-force-pgvector|Phase 19]]).
- `SearchBackend` trait wiring inside `Corpus` — Phase 19.
- Higher-level `Corpus::save_to_file` wrapper — the TQCV
  primitives landed in [[plans/rust/phase-17-zero-copy-mmap|Phase 17]],
  but exposing a one-call corpus-save API is deferred to Phase 19
  (pgvector adapter uses it) or Phase 22 (final API polish).
- Async mutations — `Corpus` stays sync-only; any async wrapping
  is the caller's responsibility.
- `CorpusEvent::VectorsRemoved` / `PolicyChanged` / `CorpusCleared`
  — not in Python, not in Rust this phase.
- `insert_batch_lenient` (soft-skip mode) — tracked as a Phase 22
  follow-up.

## See also

- [[plans/rust/phase-17-zero-copy-mmap|Phase 17]]
- [[plans/rust/phase-19-brute-force-pgvector|Phase 19]]
- [[design/domain-layer/aggregates-and-entities|Aggregates and Entities]]
- [[design/domain-layer/domain-events|Domain Events]]
- [[design/behavior-layer/corpus-management|Corpus Management]]
- [[design/rust/type-mapping|Type Mapping]]
- [[design/rust/error-model|Error Model]]
- [[design/rust/testing-strategy|Testing Strategy]]
- [[design/rust/feature-flags|Feature Flags]]
- [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]]
