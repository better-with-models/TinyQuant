---
title: "Phase 19: Brute-force Backend and Pgvector Adapter"
tags:
  - plans
  - rust
  - phase-19
  - backend
  - pgvector
date-created: 2026-04-10
status: draft
category: planning
---

# Phase 19: Brute-force Backend and Pgvector Adapter

> [!info] Goal
> Implement the `SearchBackend` trait plus the reference
> `BruteForceBackend` in `tinyquant-bruteforce`, and the anti-corruption
> `PgvectorAdapter` in `tinyquant-pgvector` mirroring the Python
> adapter semantics. Every behavior comes from
> [[design/behavior-layer/backend-protocol|Backend Protocol]]; every
> Rust type comes from [[design/rust/type-mapping|Type Mapping]]
> §Backend layer.

> [!note] Reference docs
> - [[design/rust/type-mapping|Type Mapping]] §Backend layer, §SearchBackend
> - [[design/rust/error-model|Error Model]] §`BackendError`
> - [[design/rust/crate-topology|Crate Topology]] §`tinyquant-bruteforce`, §`tinyquant-pgvector`
> - [[design/rust/feature-flags|Feature Flags]] §`tinyquant-pgvector`
> - [[design/rust/testing-strategy|Testing Strategy]] §Integration tests
> - [[design/rust/ci-cd|CI/CD]] §job matrix
> - [[design/behavior-layer/backend-protocol|Backend Protocol]] (parity contract)
> - [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]] §L1–L4

## Prerequisites

- Phase 18 complete (corpus aggregate + `CorpusEvent` stream).
- `SearchResult` type name and field layout frozen in
  [[design/rust/type-mapping|Type Mapping]].
- `BackendError::{Empty, InvalidTopK, Adapter}` already reserved in
  [[design/rust/error-model|Error Model]] — this phase only **uses**
  them, no new variants.

## Deliverables

### Files to create

| File | Purpose | Module visibility | Feature-gated |
|---|---|---|---|
| `rust/crates/tinyquant-core/src/backend/mod.rs` | Module root; re-exports protocol | `pub` | — |
| `rust/crates/tinyquant-core/src/backend/protocol.rs` | `SearchBackend` trait, `SearchResult` | `pub` | — |
| `rust/crates/tinyquant-core/tests/backend_trait.rs` | dyn-safety + `Send+Sync` assertions | test-only | — |
| `rust/crates/tinyquant-bruteforce/Cargo.toml` | Manifest, `features = ["std" (default), "simd", "dashmap"]` | — | — |
| `rust/crates/tinyquant-bruteforce/src/lib.rs` | Public re-exports | `pub` | — |
| `rust/crates/tinyquant-bruteforce/src/backend.rs` | `BruteForceBackend` struct + trait impl | `pub` | — |
| `rust/crates/tinyquant-bruteforce/src/store.rs` | `OwnedStore` wrapping `IndexMap<VectorId, Arc<[f32]>>` | `pub(crate)` | — |
| `rust/crates/tinyquant-bruteforce/src/similarity.rs` | Scalar cosine kernel (reference for Phase 20) | `pub(crate)` | — |
| `rust/crates/tinyquant-bruteforce/src/errors.rs` | Thin re-export of `BackendError` (see below) | `pub(crate)` | — |
| `rust/crates/tinyquant-bruteforce/tests/backend.rs` | Unit + golden fixture tests | test-only | — |
| `rust/crates/tinyquant-bruteforce/tests/fixtures/brute_force_golden.json` | 100 vectors + 5 queries + expected top-10 ids | fixture | — |
| `rust/crates/tinyquant-bruteforce/benches/store_clone.rs` | Micro-bench: `Arc<[f32]>` vs `Vec<f32>` | bench | — |
| `rust/crates/tinyquant-pgvector/Cargo.toml` | Manifest, `features = ["test-containers"]` | — | — |
| `rust/crates/tinyquant-pgvector/src/lib.rs` | Re-exports | `pub` | — |
| `rust/crates/tinyquant-pgvector/src/adapter.rs` | `PgvectorAdapter` | `pub` | — |
| `rust/crates/tinyquant-pgvector/src/sql.rs` | Prepared statement constants + table-name validator | `pub(crate)` | — |
| `rust/crates/tinyquant-pgvector/src/wire.rs` | FP32 ↔ pgvector text format | `pub(crate)` | — |
| `rust/crates/tinyquant-pgvector/tests/adapter.rs` | Integration (testcontainers) | test-only | `test-containers` |
| `rust/crates/tinyquant-pgvector/tests/testcontainers/image.sha256` | Pinned digest for `pgvector/pgvector:pg16` | fixture | `test-containers` |
| `rust/crates/tinyquant-pgvector/tests/fixtures/0001_create_vectors_table.sql` | Schema migration applied at setup | fixture | `test-containers` |

> [!note] `errors.rs` scope
> The adapter crates do **not** define new error enums. `BruteForceBackend`
> returns `BackendError::InvalidTopK` / `BackendError::Adapter(Arc<str>)`;
> `PgvectorAdapter` wraps `postgres::Error` into
> `BackendError::Adapter(Arc::from(format!("pg: {e}")))`. The `errors.rs`
> file exists only to centralize those `From` conversions and to keep
> `lib.rs` short. This matches the single-error-per-crate rule from
> [[design/rust/error-model|Error Model]].

### Ingest / remove behavior matrix

This table is the black-box contract `BruteForceBackend` must satisfy.
Every row is a unit test in `tests/backend.rs`.

| Call | Preconditions | Expected outcome |
|---|---|---|
| `ingest(&[(id_a, vec_of_dim=768), ...])` | empty backend, 3 vectors of dim 768 | `Ok(())`; `dim` locked to 768; `len == 3` |
| `ingest(&[(id_x, vec_of_dim=512)])` | backend with `dim=768` | `Err(BackendError::Adapter("dimension mismatch: expected 768, got 512"))` |
| `ingest(&[(existing_id, new_vec)])` | backend already contains `existing_id` | `Ok(())`; stored vector is overwritten (Python parity) |
| `ingest(&[])` | any | `Ok(())`; no-op |
| `remove(&[unknown_id])` | any | `Ok(())`; silent no-op (Python parity) |
| `remove(&[])` | any | `Ok(())`; silent no-op |
| `remove(&[id_a])` | backend contains `id_a` | `Ok(())`; `id_a` is gone; iteration order of survivors is preserved |
| `search(q, 10)` | empty backend | `Ok(vec![])` |
| `search(q, 0)` | any | `Err(BackendError::InvalidTopK)` |
| `search(q, 10)` | backend size 3 | `Ok(vec_of_len_3)` sorted descending |
| `search(q_dim_512, 10)` | backend with `dim=768` | `Err(BackendError::Adapter("dimension mismatch: expected 768, got 512"))` |
| `search(q, 10)` | backend size 1000 | `Ok(vec_of_len_10)` sorted descending |

> [!note] Python parity on overwrite/no-op
> Python's `BruteForceBackend.ingest` uses `dict[id] = vec`, which
> overwrites silently. Python's `remove_ids` iterates with `discard`,
> which is a no-op on missing keys. Rust must match both behaviors
> byte-for-byte because
> [[design/behavior-layer/backend-protocol|Backend Protocol]] rule 3
> (error isolation) assumes ingestion is idempotent.

### SearchBackend dyn-safety

The `SearchBackend` trait **must** be `dyn`-compatible so the corpus
layer can hold `Box<dyn SearchBackend>` without knowing the concrete
implementation. Audit checklist:

1. **No generic methods.** All methods take concrete borrowed types.
   `ingest` takes `&[(VectorId, Vec<f32>)]`, `search` takes `&[f32]`
   plus `usize`, `remove` takes `&[VectorId]`. No `impl Trait`,
   no `<T>` parameters.
2. **No `Self: Sized` bounds.** The trait uses `&self` / `&mut self`
   exclusively.
3. **No async.** Phase 19 ships blocking only. If we add async in a
   later phase it goes on a separate trait (`AsyncSearchBackend`) or
   via an erased `Pin<Box<dyn Future>>` return type — *not* as a
   generic async fn, which would break dyn-compatibility.
4. **No `where Self: Copy`** or similar disqualifying bounds.
5. **Metric is not a type parameter.** Cosine is hard-coded for
   this phase. Other metrics (L2, dot-product) ship as **separate
   trait impls on separate backend types** (e.g., a future
   `BruteForceL2Backend`) rather than as a `<M: Metric>` type
   parameter. Rationale: a metric type parameter would either break
   dyn-safety or force `Box<dyn SearchBackend<CosineMetric>>` at
   every call site, both of which are worse than the duplication.

Assertions enforced by `tinyquant-core/tests/backend_trait.rs`:

```rust
use tinyquant_core::backend::{SearchBackend, SearchResult};
use static_assertions::{assert_impl_all, assert_obj_safe};

// Object-safety: the trait must be usable as `dyn SearchBackend`.
assert_obj_safe!(SearchBackend);

// A coercion smoke test also catches later regressions where a
// method signature drifts and breaks dyn-safety.
fn assert_dyn_safe(_: Box<dyn SearchBackend>) {}

// SearchResult must be freely sharable across threads.
assert_impl_all!(SearchResult: Send, Sync, Clone);
```

`SearchResult` is `Send + Sync` because its fields are:

- `vector_id: Arc<str>` — `Arc` is `Send + Sync` when `T: Send + Sync`,
  and `str` is both;
- `score: f32` — `Copy`, trivially `Send + Sync`.

No interior mutability, no raw pointers, no `Rc`. The assertion above
is load-bearing: if anyone changes `vector_id` to `Rc<str>` to save
an atomic, this test fails at compile time.

## Steps (TDD order)

### Part A — `SearchBackend` trait + `SearchResult`

- [ ] **Step 1: Failing `SearchResult` tests**

```rust
#[test]
fn search_result_ordering_is_descending_by_score() {
    let mut v = vec![
        SearchResult { vector_id: Arc::from("a"), score: 0.5 },
        SearchResult { vector_id: Arc::from("b"), score: 0.9 },
        SearchResult { vector_id: Arc::from("c"), score: 0.1 },
    ];
    v.sort();
    assert_eq!(v[0].vector_id.as_ref(), "b");
    assert_eq!(v[2].vector_id.as_ref(), "c");
}

#[test]
fn search_result_tie_breaks_preserve_insertion_order() {
    let mut v = vec![
        SearchResult { vector_id: Arc::from("first"),  score: 0.5 },
        SearchResult { vector_id: Arc::from("second"), score: 0.5 },
        SearchResult { vector_id: Arc::from("third"),  score: 0.5 },
    ];
    v.sort(); // stable
    assert_eq!(v[0].vector_id.as_ref(), "first");
    assert_eq!(v[1].vector_id.as_ref(), "second");
    assert_eq!(v[2].vector_id.as_ref(), "third");
}

#[test]
fn search_result_nan_is_treated_as_equal_for_sort_stability() {
    let mut v = vec![
        SearchResult { vector_id: Arc::from("nan"), score: f32::NAN },
        SearchResult { vector_id: Arc::from("one"), score: 1.0 },
    ];
    v.sort();
    // 'one' must come first because NaN ties at Equal.
    assert_eq!(v[0].vector_id.as_ref(), "one");
}
```

- [ ] **Step 2: Implement `protocol.rs`**

Per [[design/rust/type-mapping|Type Mapping]] §SearchBackend.

```rust
impl Ord for SearchResult {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // Descending by score; NaN collapses to Equal so stable sort
        // keeps insertion order. See parity notes below.
        other.score.partial_cmp(&self.score).unwrap_or(core::cmp::Ordering::Equal)
    }
}

impl PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
```

#### SearchResult ordering semantics

| Aspect | Rule | Rationale |
|---|---|---|
| Direction | Descending by `score` | Python `__lt__` returns `self.score > other.score` |
| Impl | `other.score.partial_cmp(&self.score)` | Flips direction by swapping operands |
| NaN handling | `.unwrap_or(Equal)` | Keeps `sort` total; never panics |
| Tie-break | Insertion order (stable sort) | Matches Python's `sorted(...)` determinism |
| Canonical status of NaN | **Never produced by scoring.** If seen, it is a backend bug and a `debug_assert!` fires in debug builds. | See §Cosine kernel parity contract |
| Sort function choice | `sort` (stable) — **not** `sort_unstable_by` | Tie-break determinism wins over ~2× throughput |

> [!warning] Don't regress to `sort_unstable_by`
> The scalar path is not the performance bottleneck — the cosine
> kernel dominates. Swapping in `sort_unstable_by` would gain ~2% on
> a 100 k search and lose the deterministic tie-break, which the
> Python parity tests rely on. Risk R19.5 tracks this as a recurring
> temptation. Do not land a "performance" change here without
> measuring end-to-end on the Phase 20 SIMD path and updating the
> Python parity fixtures in lock-step.

### Part B — Brute-force backend

#### Store element type

`OwnedStore` uses `IndexMap<VectorId, Arc<[f32]>>`, not
`IndexMap<VectorId, Vec<f32>>`. Rationale:

1. **`remove` returns the old slice without reallocation.**
   `IndexMap::shift_remove` returns the old value; with `Arc<[f32]>`
   that's a bitwise move of a fat pointer, no `Drop` of a `Vec`.
2. **`search` hands out borrowed slices to the SIMD kernel in
   Phase 20 without lifetime contortions.** `&self.store[id]` gives
   `&Arc<[f32]>`, and `Arc::as_ref` gives `&[f32]` cheaply.
3. **Cloning a store entry is O(1).** The Arc header bumps a
   refcount; the f32 payload is not copied. `ingest` of a 10 k × 768
   corpus that already owns its data (e.g., handed in from a corpus
   decompression step) costs only 10 k atomic increments, not
   10 k × 768 × 4 bytes of memcpy.
4. **Memory overhead is 16 bytes per entry** (the Arc header: strong
   count, weak count). At `dim = 768` that's 3 072 bytes of payload
   per entry, so the overhead is ~0.5%. Below `dim = 32` the
   overhead becomes meaningful (~12%) but our target dim is ≥ 128
   per [[design/behavior-layer/backend-protocol|Backend Protocol]]
   scenarios.

Micro-bench `benches/store_clone.rs` proves the point:

```rust
// Pseudocode — real bench is criterion-based.
//
//   arc_store_clone_entry    ≈ 14 ns  (atomic inc)
//   vec_store_clone_entry    ≈ 920 ns (alloc + memcpy of 768 f32)
//   arc_store_remove_entry   ≈ 22 ns
//   vec_store_remove_entry   ≈ 25 ns  (dealloc)
//
// On the ingest path that matters: 10k-vector ingest is ~140 µs
// with Arc, ~9.2 ms with Vec. 65× win.
```

If the bench ever inverts (e.g., `IndexMap` changes its internal
representation), revisit the decision — but keep the `Arc<[f32]>`
store as the default, because the **search** path (Phase 20) also
needs cheap borrowed-slice handoff.

- [ ] **Step 3: Failing ingest/search tests**

```rust
#[test]
fn ingest_then_search_returns_top_k_descending() { /* … */ }

#[test]
fn identical_query_scores_near_one() { /* … */ }

#[test]
fn orthogonal_query_scores_near_zero() { /* … */ }

#[test]
fn empty_backend_returns_empty_list() { /* … */ }

#[test]
fn remove_ids_unknown_is_silent() { /* … */ }

#[test]
fn remove_ids_empty_list_is_silent() { /* … */ }

#[test]
fn search_with_fewer_vectors_than_top_k_returns_all() { /* … */ }

#[test]
fn search_with_top_k_zero_returns_invalid_top_k() { /* … */ }

#[test]
fn search_with_wrong_query_dim_returns_adapter_error() { /* … */ }

#[test]
fn ingest_with_mismatched_dim_returns_adapter_error() { /* … */ }

#[test]
fn ingest_overwrites_existing_id_silently() { /* … */ }

#[test]
fn golden_fixture_top_10_matches_python() {
    // Loads tests/fixtures/brute_force_golden.json, ingests 100
    // vectors, runs 5 queries, asserts top-10 ids and scores
    // (tolerance 1e-7) match the fixture.
}
```

- [ ] **Step 4: Implement `BruteForceBackend`**

```rust
pub struct BruteForceBackend {
    dim: Option<usize>,
    store: OwnedStore, // IndexMap<VectorId, Arc<[f32]>>
}

impl SearchBackend for BruteForceBackend {
    fn ingest(&mut self, vectors: &[(VectorId, Vec<f32>)]) -> Result<(), BackendError> {
        for (id, v) in vectors {
            match self.dim {
                Some(d) if d != v.len() => {
                    return Err(BackendError::Adapter(Arc::from(
                        format!("dimension mismatch: expected {d}, got {}", v.len()),
                    )));
                }
                None => self.dim = Some(v.len()),
                _ => {}
            }
            self.store.insert(id.clone(), Arc::from(v.as_slice()));
        }
        Ok(())
    }

    fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<SearchResult>, BackendError> {
        if top_k == 0 {
            return Err(BackendError::InvalidTopK);
        }
        if let Some(d) = self.dim {
            if d != query.len() {
                return Err(BackendError::Adapter(Arc::from(
                    format!("dimension mismatch: expected {d}, got {}", query.len()),
                )));
            }
        }
        if self.store.is_empty() {
            return Ok(Vec::new());
        }
        let mut results = Vec::with_capacity(self.store.len());
        for (id, v) in self.store.iter() {
            let score = similarity::cosine_similarity(query, v);
            results.push(SearchResult { vector_id: id.clone(), score });
        }
        results.sort(); // stable, descending
        results.truncate(top_k);
        Ok(results)
    }

    fn remove(&mut self, vector_ids: &[VectorId]) -> Result<(), BackendError> {
        for id in vector_ids {
            self.store.shift_remove(id);
        }
        Ok(())
    }
}
```

- [ ] **Step 5: Implement scalar `cosine_similarity` in `similarity.rs`**

```rust
pub(crate) fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    debug_assert!(!a.iter().any(|x| x.is_nan()), "NaN in query; backend bug");
    debug_assert!(!b.iter().any(|x| x.is_nan()), "NaN in stored vector; backend bug");
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (&x, &y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na  += x * x;
        nb  += y * y;
    }
    let denom = (na * nb).sqrt();
    if denom == 0.0 { 0.0 } else { dot / denom }
}
```

#### Cosine kernel parity contract

This is the authoritative reference kernel. Phase 20 SIMD kernels
diff against it byte-for-byte on canned inputs, and the Python
reference diffs against it with a tolerance.

1. **Scalar is canonical.** `scalar_quantize` in Phase 14 played the
   same role for codebook quantization. `cosine_similarity` plays it
   for search scoring. SIMD kernels in Phase 20 **must** match this
   function bit-for-bit on the 1000-input fixture under
   `tests/fixtures/cosine_parity_1000.bin`; if they don't, the SIMD
   kernel is wrong, not the scalar one.
2. **Python parity.** The Python reference is
   `tinyquant_cpu.backend.bruteforce.cosine_similarity`. Parity is
   tested on 1000 vector pairs generated with
   `np.random.default_rng(19).standard_normal((1000, 2, 768))`.
   **Tolerance is `1e-7`**, not zero, because:
   - Python's implementation uses `np.dot(a, b) / (norm(a) * norm(b))`,
     and `np.dot` may compile down to a BLAS `sdot` whose rounding
     depends on the installed BLAS (OpenBLAS vs MKL vs Accelerate).
   - Rust's scalar kernel uses ordered f32 summation, which matches
     OpenBLAS on small dims but not large ones.
   - `1e-7` comfortably fits f32 precision (`~6e-8` ULP at ~1.0) and
     tolerates both rounding paths.
3. **Zero handling.** `cosine(0, 0) == 0.0` (not NaN) is the canonical
   behavior. The explicit `if denom == 0.0` branch implements it.
   Python does the same via `np.where(denom == 0, 0.0, dot / denom)`
   in its adapter wrapper.
4. **NaN/inf propagation.** If debug assertions are enabled and either
   input contains a NaN, the kernel panics with "backend bug" — this
   is the only panic in the backend layer. In release builds, a NaN
   input yields a NaN score; the subsequent sort collapses it to
   Equal via the `SearchResult::cmp` rule above, so the result list
   is still well-ordered. This is a defense-in-depth pair: the
   debug-mode panic catches bugs during development, the
   release-mode collapse prevents sort corruption in production.
5. **Numerical stability.** The reference kernel does **not**
   normalize inputs up front (Python doesn't either). If a caller
   pre-normalizes, the result is still correct because
   `dot / (1 * 1) == dot`.

- [ ] **Step 6: Run backend tests — expect pass.**

### Part C — Pgvector adapter

- [ ] **Step 7: Failing adapter tests (testcontainers-gated)**

```rust
#[cfg(feature = "test-containers")]
mod integration {
    use testcontainers::{GenericImage, runners::SyncRunner, core::WaitFor};
    use std::fs;

    fn pinned_image() -> GenericImage {
        // Digest is pinned in tests/testcontainers/image.sha256 to
        // prevent silent drift when pgvector/pgvector:pg16 republishes.
        let digest = fs::read_to_string("tests/testcontainers/image.sha256")
            .expect("pinned digest missing")
            .trim()
            .to_owned();
        GenericImage::new("pgvector/pgvector", &format!("pg16@{digest}"))
            .with_wait_for(WaitFor::message_on_stdout(
                "database system is ready to accept connections",
            ))
            .with_env_var("POSTGRES_PASSWORD", "tinyquant")
            .with_env_var("POSTGRES_DB", "tinyquant_test")
    }

    #[test]
    fn pgvector_adapter_round_trip() {
        let container = pinned_image().start().expect("docker");
        let port = container.get_host_port_ipv4(5432).unwrap();
        // 1. Apply fixtures/0001_create_vectors_table.sql
        // 2. Construct PgvectorAdapter::new(...)
        // 3. ingest → search → remove → search (verify empty)
        // 4. Container drops at end of function (RAII)
    }

    #[test]
    fn pgvector_adapter_rejects_bad_table_name() { /* … */ }

    #[test]
    fn pgvector_adapter_rejects_wrong_dim_at_construction() { /* … */ }

    #[test]
    fn pgvector_adapter_ensure_index_is_idempotent() { /* … */ }
}
```

**Testcontainers harness details:**

- **Image pin.** `pgvector/pgvector:pg16` is pinned to a specific
  SHA-256 digest recorded in
  `rust/crates/tinyquant-pgvector/tests/testcontainers/image.sha256`.
  When the upstream re-tags `pg16` (which they do every few months)
  our CI keeps using the frozen digest. Refreshing the pin is a
  two-line PR that runs the parity suite against the new image.
- **Migration fixture.** `tests/fixtures/0001_create_vectors_table.sql`
  creates the extension, the table, and the index. The test harness
  applies it with a single `client.batch_execute` after the container
  becomes ready.
- **Startup wait.** `WaitFor::message_on_stdout("database system is
  ready to accept connections")` — this exact string is the
  Postgres startup log line across all recent versions.
- **Teardown.** RAII on the `Container` handle. Dropping at end of
  test function stops and removes the container. If a test panics,
  `Drop` still runs because panics unwind by default in tests.
- **Feature gate.** `[features] test-containers = ["dep:testcontainers"]`.
  `default` is empty. Release builds of `tinyquant-pgvector` *must
  not* pull `testcontainers`; `xtask arch-check` verifies this with
  `cargo tree -p tinyquant-pgvector --no-default-features`.

- [ ] **Step 8: Implement `PgvectorAdapter`**

#### PgvectorAdapter contract

```rust
pub struct PgvectorAdapter {
    factory: Box<dyn Fn() -> Result<postgres::Client, postgres::Error> + Send + Sync>,
    table: Arc<str>,      // validated against TABLE_NAME_RE
    dim: u32,             // frozen at construction
    insert_sql: String,   // interpolated once; reused forever
    search_sql: String,
    delete_sql: String,
}

impl PgvectorAdapter {
    pub fn new(
        factory: impl Fn() -> Result<postgres::Client, postgres::Error> + Send + Sync + 'static,
        table: &str,
        dim: u32,
    ) -> Result<Self, BackendError> { /* … */ }

    /// Create `vector` extension + table if missing. Idempotent.
    pub fn ensure_schema(&self) -> Result<(), BackendError> { /* … */ }

    /// Create the IVFFlat index. Idempotent. Optional.
    pub fn ensure_index(&self, lists: u32) -> Result<(), BackendError> { /* … */ }
}
```

Design points:

1. **Connection factory, not pool.** The adapter holds
   `Box<dyn Fn() -> Result<postgres::Client, postgres::Error>
   + Send + Sync>`. Callers that need pooling (r2d2, bb8, native
   Postgres pool) wire their own pool and hand the factory a closure
   that borrows from it. No implicit pool inside the adapter —
   matching Python's `connection_factory: Callable[[], Any]`.
2. **Table name validation** happens at `PgvectorAdapter::new` time
   against the regex `^[A-Za-z_][A-Za-z0-9_]{0,62}$` (see Injection
   safety below). This is **tighter** than Postgres's actual
   identifier rules, which allow Unicode and quoted names — we
   trade expressiveness for a zero-branch injection audit.
3. **Dim is frozen at construction.** `dim: u32` is baked into the
   schema migration SQL (`vector({dim})`) and into every prepared
   statement's parameter binding. Mismatched ingest errors
   immediately with `BackendError::Adapter`. The reason `dim` can't
   be a server-side parameter is that pgvector's type constructor
   `vector(N)` does not accept bound parameters — it's a type
   annotation, not a value, so it must be interpolated into the SQL
   source at `ensure_schema` time. This is the single place where
   our SQL strings contain an interpolated integer; it is safe
   because `dim` is a typed `u32` that can never be a SQL injection
   payload.
4. **Schema migration SQL:**

   ```sql
   CREATE EXTENSION IF NOT EXISTS vector;
   CREATE TABLE IF NOT EXISTS {table} (
       id        TEXT PRIMARY KEY,
       embedding vector({dim})
   );
   ```

   Generated at construction with `format!("...", table = self.table,
   dim = self.dim)` and cached in `self` as an owned `String`.
5. **Prepared statement cache** lives in the adapter's `Client` via
   the underlying `postgres` crate's per-connection statement cache.
   The adapter does not manage it directly. Tracing warning if a
   caller hands the same `Client` to two adapter instances on
   different threads — `postgres::Client` is `!Sync` so this is a
   misuse.
6. **IVFFlat index** gated behind `ensure_index(lists)`:

   ```sql
   CREATE INDEX IF NOT EXISTS {table}_embedding_idx
   ON {table}
   USING ivfflat (embedding vector_cosine_ops)
   WITH (lists = $1);
   ```

   Wait — `lists` is also a DDL parameter, not a bind parameter, so
   it must be interpolated. Same zero-injection argument as `dim`.
   Default `lists = 100` if the caller passes `u32::default()`.

- [ ] **Step 9: Implement SQL constants**

```rust
// sql.rs
use regex::Regex;
use once_cell::sync::Lazy;

pub(crate) static TABLE_NAME_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Za-z_][A-Za-z0-9_]{0,62}$").expect("const regex")
});

pub(crate) fn validate_table_name(name: &str) -> Result<(), BackendError> {
    if TABLE_NAME_RE.is_match(name) {
        Ok(())
    } else {
        Err(BackendError::Adapter(Arc::from(
            format!("invalid table name {name:?}; must match {:?}", TABLE_NAME_RE.as_str()),
        )))
    }
}

pub(crate) const INSERT_SQL_TEMPLATE: &str =
    "INSERT INTO {table} (id, embedding) VALUES ($1, $2::vector) \
     ON CONFLICT (id) DO UPDATE SET embedding = EXCLUDED.embedding";

pub(crate) const SEARCH_SQL_TEMPLATE: &str =
    "SELECT id, 1 - (embedding <=> $1::vector) AS score \
     FROM {table} \
     ORDER BY embedding <=> $1::vector \
     LIMIT $2";

pub(crate) const DELETE_SQL_TEMPLATE: &str =
    "DELETE FROM {table} WHERE id = ANY($1)";
```

#### Injection safety tests

A table of adversarial inputs and their expected handling, each
implemented as a unit test:

| Input | Field | Expected result |
|---|---|---|
| `"users; DROP TABLE users; --"` | `table_name` at `PgvectorAdapter::new` | `Err(BackendError::Adapter(..))` — regex rejects |
| `"users"` | `table_name` | `Ok` |
| `"1users"` | `table_name` | `Err` — must start with letter/underscore |
| `"u$ers"` | `table_name` | `Err` — `$` not allowed |
| (63 × `'a'`) | `table_name` | `Err` — length > 63 |
| (62 × `'a'`) | `table_name` | `Ok` |
| `"'; DELETE FROM vectors; --"` | `VectorId` on `ingest` | `Ok`; parameterized, no DELETE executes; row is visible on next `search` |
| `"' OR 1=1 --"` | `VectorId` on `remove` | `Ok`; no rows other than the literal id are affected |
| `"' OR 1=1 --"` | `ConfigHash` in a future metadata column | parameterized, safe |
| property test: 1000 random strings up to 256 chars | `VectorId` | no error other than `BackendError::Adapter` from Postgres itself; nothing crashes, nothing injects |

Test-plan note: we do **not** add `sqlx` to the dep tree just to
parse our SQL — the project already depends on `postgres` (sync).
Instead, a test spins up the pgvector container, executes each
candidate statement with `EXPLAIN`, and asserts the plan is
well-formed. This is slower but uses tools we already have.

- [ ] **Step 10: Wire format**

`wire.rs` converts `&[f32]` to pgvector's text format
`"[1.0,2.0,3.0]"`.

**Expanded wire-format spec:**

1. **Text format, not binary.** pgvector supports both; we pick text
   in Phase 19 for readability and because Python's adapter also
   uses text. Binary (pgvector ≥ 0.5) is deferred to
   [[plans/rust/phase-22-pyo3-cabi-release|Phase 22]] as an
   optimization knob once parity is stable.
2. **Number formatting.** Each f32 is rendered with `{:.7e}`, i.e.,
   scientific notation with 7 significant digits. Rationale: 7 digits
   is exactly enough to round-trip a f32 through ASCII (IEEE 754
   binary32 has ~7.22 decimal digits of precision). Using `{}` with
   default Rust formatting produces shorter strings but may drop
   precision on edge-case values; `{:.7e}` is the paranoid default.
3. **Parity fixture.** 100 vectors of `dim = 768` drawn from
   `np.random.default_rng(23).standard_normal((100, 768))` are
   encoded in Python with `str(np.array(v, dtype=np.float32).tolist())`
   (minus the numpy wrapping, i.e., `"[" + ",".join(str(x) for x in
   vs) + "]"`) and stored in
   `tests/fixtures/pgvector_wire_100.json`. The Rust adapter encodes
   the same vectors; for each one, the round-trip
   `encode → pgvector → decode` must yield a byte-identical f32 array.
   The **text form itself** is *not* required to be byte-identical
   (Python and Rust format differently) — only the decoded f32s are.
4. **NaN / infinity.** Rejected at the adapter boundary **before**
   wire encoding. `BackendError::Adapter("NaN in vector at index i")`
   — pgvector's text parser does accept `NaN`, but the backend
   protocol (rule 4) forbids sending NaN across the backend
   boundary, so we reject early.
5. **Negative-zero.** f32 `-0.0` round-trips; pgvector treats
   `-0.0 == 0.0` internally but preserves the sign byte in storage.

- [ ] **Step 11: Injection safety hookup**

Wire the `validate_table_name` check from sql.rs into
`PgvectorAdapter::new`. Run the injection-safety test matrix from
Step 9.

- [ ] **Step 12: Run adapter tests (gated on Docker).**

`cargo test -p tinyquant-pgvector --features test-containers` on a
Linux runner with Docker. See §CI integration below for the matrix.

- [ ] **Step 13: Update prelude + re-exports**

```rust
// tinyquant-core prelude
pub use crate::backend::{SearchBackend, SearchResult};

// tinyquant-bruteforce lib
pub use crate::backend::BruteForceBackend;

// tinyquant-pgvector lib
pub use crate::adapter::PgvectorAdapter;
```

- [ ] **Step 14: Commit**

```bash
git add rust/crates/tinyquant-core/src/backend rust/crates/tinyquant-bruteforce rust/crates/tinyquant-pgvector
git commit -m "feat(backend): add SearchBackend trait, brute-force, and pgvector adapter"
```

### Clippy profile gotchas

Phase 14's lint-wall walk-through ([[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]] §L4)
left a map of clippy traps under the crate-wide `deny(clippy::all,
clippy::pedantic, clippy::nursery)` profile. Phase 19 hits the same
ones on new surface:

| Lint | Site | Fix |
|---|---|---|
| `indexing_slicing` | `results[..top_k]` | `results.into_iter().take(top_k).collect()` or `results.truncate(top_k)` (used above) |
| `cast_precision_loss` | `len() as f32` inside a norm computation | narrow `#[allow(clippy::cast_precision_loss)]` with a comment citing the f32 domain |
| `trivially_copy_pass_by_ref` | `fn cosine(&f32, &f32)` helpers | take by value |
| `missing_errors_doc` | public `search` / `ingest` / `remove` | add `/// # Errors` sections documenting each `BackendError` variant |
| `unwrap_used` / `expect_used` | `.partial_cmp(&other.score).unwrap()` | use `.unwrap_or(core::cmp::Ordering::Equal)` explicitly (already in Step 2) |
| `too_many_lines` | `BruteForceBackend::search` | extract `score_all_vectors` private helper if it balloons past ~40 lines |
| `needless_pass_by_value` | `fn ingest(vs: Vec<...>)` | take `&[(VectorId, Vec<f32>)]` per trait (already correct) |
| `option_if_let_else` | `self.dim` checks | leave as `match` for readability; narrow `#[allow]` at the call site |
| `redundant_closure_for_method_calls` | `iter().map(|x| x.clone())` | `.cloned()` |

> [!tip] Port-for-port
> Most of the above fell out of Phase 14. When in doubt, grep
> `rust/crates/tinyquant-core/src/codec/**` for the same lint name
> and copy the annotation style exactly — consistency is worth more
> than micro-optimizing each waiver.

## CI integration

Phase 19 adds two jobs to `rust-ci.yml` and one grep-check to the
existing xtask suite.

### Job: unit (existing matrix, extended)

```yaml
- name: Unit tests
  run: cargo test -p tinyquant-core -p tinyquant-bruteforce --all-features
```

Runs on the existing `linux-x86_64`, `linux-aarch64`, `macos-arm64`,
and `windows-x86_64` matrix rows. No LFS needed — the
`brute_force_golden.json` fixture is small JSON (≈40 KB for 100
vectors × 768 dim + metadata) and lives directly in the repo.

> [!warning] LFS lesson from Phase 14 (L2)
> Phase 14 learned the hard way that **if a test fixture is small,
> don't put it in LFS**. LFS pointer files deserialize as 132 bytes
> of ASCII and every parity assertion fails with a misleading
> length error, as documented in
> [[design/rust/phase-14-implementation-notes|Phase 14 Implementation
> Notes]] §L2. Our 100-vector brute-force fixture is well under the
> LFS threshold and **must not** be marked `*.json filter=lfs` —
> add an explicit negative entry in `.gitattributes` if one creeps
> in:
>
> ```gitattributes
> rust/crates/tinyquant-bruteforce/tests/fixtures/*.json -filter -diff -merge
> ```

### Job: pgvector-integration (new, Linux-only)

```yaml
pgvector-integration:
  name: pgvector integration
  runs-on: ubuntu-22.04
  if: runner.os == 'Linux'
  needs: [fmt, clippy]
  steps:
    - uses: actions/checkout@v4
      with:
        lfs: true  # L1: LFS hydration must be explicit
    - uses: Swatinem/rust-cache@v2
    - name: Pull pinned pgvector image
      run: |
        digest=$(cat rust/crates/tinyquant-pgvector/tests/testcontainers/image.sha256)
        docker pull "pgvector/pgvector@${digest}"
    - name: Run integration tests
      run: cargo test -p tinyquant-pgvector --features test-containers
```

**Platform coverage notes:**

- **Linux only.** Windows GitHub runners do not provide a working
  Docker daemon for Linux containers (the Windows runner's Docker
  is Windows-container mode), and macOS runners do not have Docker
  at all since GitHub removed Docker-in-VM support. Phase 19
  explicitly marks Windows and macOS pgvector-integration runs as
  **not applicable**, documented in `## Risks` R19.4 below.
- **Developer workflow on Windows/macOS.** Local developers can run
  the integration suite against a self-hosted Postgres+pgvector
  instance by setting `TINYQUANT_PG_DSN=postgres://...`; the test
  harness checks that env var first and skips the testcontainers
  path if it's set. This is not a CI gate, just a convenience.

### Job: design-doc drift grep (xtask)

Phase 14 L3 taught us that prose claims about CI can drift from the
actual YAML. Phase 19 adds a grep-check to `xtask arch-check`:

```rust
// xtask/src/cmd/arch.rs
fn check_ci_claims_match_workflow() -> Result<(), XtaskError> {
    let ci_design = fs::read_to_string("docs/design/rust/ci-cd.md")?;
    let workflow  = fs::read_to_string(".github/workflows/rust-ci.yml")?;

    if ci_design.contains("pgvector-integration") && !workflow.contains("pgvector-integration") {
        return Err(XtaskError::Drift(
            "ci-cd.md mentions pgvector-integration but rust-ci.yml does not define it".into(),
        ));
    }
    // ... symmetric check for other jobs
    Ok(())
}
```

Runs on every PR via the existing `architecture` stage. This is the
same kind of guardrail Phase 14 L3 prescribed for Phase 14+; Phase 19
is the first phase to actually wire it in for backend claims.

### Phase exit CI health check

Phase 14 L1 demanded "a green integration run on `main` before phase
exit." Phase 19 exit criteria:

1. `rust-ci.yml` / unit matrix on the Phase 19 branch: all 4 OS rows
   green.
2. `pgvector-integration` job on the Phase 19 branch: green.
3. `rust-ci.yml` on `main` after merge: green within the next scheduled
   run (no "green on branch, red on main" surprises).

If any of these fail, Phase 19 is not considered complete and Phase 20
cannot start. This is a hard gate, not advisory.

## Acceptance criteria

- `SearchBackend` trait is dyn-safe (`Box<dyn SearchBackend>` and
  `assert_obj_safe!` both compile).
- `SearchResult` is `Send + Sync + Clone` (asserted at compile time).
- `BruteForceBackend` passes every row of the
  §Ingest / remove behavior matrix.
- `BruteForceBackend` passes the 100-vector golden fixture top-10
  match against Python, tolerance `1e-7`.
- Scalar `cosine_similarity` passes the 1000-pair Python parity
  fixture, tolerance `1e-7`; `cosine(0, 0) == 0.0`.
- `BackendError::InvalidTopK` returned on `top_k == 0`.
- `PgvectorAdapter::new` rejects all table names from the injection
  safety table.
- `PgvectorAdapter` round-trips against a testcontainers-backed
  pgvector instance on Linux CI with the digest-pinned image.
- `pgvector_wire_100.json` round-trip is f32-exact.
- `cargo tree -p tinyquant-pgvector --no-default-features` does not
  list `testcontainers`.
- Clippy + fmt clean under the Phase 14 profile.
- `xtask arch-check` passes, including the new CI-drift grep.

## Risks

| ID | Risk | Mitigation |
|---|---|---|
| R19.1 | `pgvector/pgvector:pg16` tag drifts silently, breaking integration tests without warning | Pin SHA-256 digest in `tests/testcontainers/image.sha256`; `xtask arch-check` greps the CI YAML for the pinned digest |
| R19.2 | `postgres` crate version bump pulls `tokio` or `tokio-postgres` transitively, breaking the sync-only invariant | Pin `postgres = "=0.19.x"` at the workspace level; `cargo deny` `bans` list forbids `tokio-postgres` from the `tinyquant-pgvector` subgraph |
| R19.3 | Cosine parity with numpy+BLAS is ISA-dependent; exact f32 equality would fail on aarch64 or MKL | Tolerance `1e-7`, documented in §Cosine kernel parity contract; Phase 20 SIMD lessons (L4 of Phase 14) already established "tolerance > 0 for cross-ISA parity" |
| R19.4 | testcontainers is Linux-only on GitHub-hosted runners; Windows/macOS CI never exercises pgvector | Document as known gap; expose `TINYQUANT_PG_DSN` env-var escape hatch for local dev; revisit in Phase 22 if a macOS Docker solution ships |
| R19.5 | `sort_unstable_by` swap regresses tie-break determinism across platforms | Use `sort` (stable); explicit §SearchResult ordering semantics callout; a failing parity test on tied scores is the canary |
| R19.6 | Dim validation fires too late — after the prepared-statement cache has built state against the wrong schema | Fire dim validation in `PgvectorAdapter::new` before any SQL is prepared or executed; unit test `rejects_wrong_dim_at_construction` proves it |
| R19.7 | `Arc<[f32]>` store is slower than `Vec<f32>` on tiny dims — inverted benchmark on `dim ≤ 32` | Document in §Store element type; keep `Arc<[f32]>` anyway because Phase 20 SIMD needs borrowed slices; revisit only if a real workload shows a cliff |

## Out of scope

- **SIMD cosine kernel** — deferred to
  [[plans/rust/phase-20-simd-kernels|Phase 20]]. Scalar kernel is
  canonical and Phase 20 diffs against it.
- **Async postgres / tokio-postgres.** Phase 19 is sync-only; an
  async trait is a future phase (not scheduled).
- **HNSW / IVF index management beyond `ensure_index`.** We create
  an IVFFlat index with a single `lists` parameter and nothing more;
  reindexing, rebuild, and probe-tuning are left to operators.
- **Pluggable metric types** (L2, dot-product, Hamming). Cosine is
  the only scoring function in Phase 19; alternative metrics ship as
  separate backend types in later phases if demand arises.
- **Multi-tenant table isolation** (schemas, search_path, row-level
  security). Each `PgvectorAdapter` instance owns exactly one table.
- **Binary wire format** for pgvector (text only in Phase 19).
- **`cargo-fuzz` targets for the adapter.** Injection safety is
  enforced by the regex + parameterized queries and tested with
  deterministic fixtures; fuzzing the SQL generator adds little
  beyond the random-string property test.

## See also

- [[plans/rust/phase-18-corpus-aggregate|Phase 18]]
- [[plans/rust/phase-20-simd-kernels|Phase 20]]
- [[design/behavior-layer/backend-protocol|Backend Protocol]]
- [[design/rust/type-mapping|Type Mapping]]
- [[design/rust/error-model|Error Model]]
- [[design/rust/testing-strategy|Testing Strategy]]
- [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]]
