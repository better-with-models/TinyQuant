---
title: "Phase 19 Implementation Notes"
tags:
  - design
  - rust
  - phase-19
  - backend
  - search
  - pgvector
date-created: 2026-04-11
category: design
---

# Phase 19 Implementation Notes

## What landed

Phase 19 delivers the **`SearchBackend` trait**, the **`BruteForceBackend`**
reference implementation, and the **`PgvectorAdapter`** anti-corruption layer
adapter that bridges `tinyquant-core`'s FP32-only search contract to a live
PostgreSQL + pgvector database.

### Concrete deliverables

- `rust/crates/tinyquant-core/src/backend/protocol.rs` — `SearchBackend` trait
  with `ingest(&mut self, &[(VectorId, Vec<f32>)])`, `search(&self, &[f32],
  usize) -> Vec<SearchResult>`, and `remove(&mut self, &[VectorId])`. The trait
  is deliberately FP32-only: no `CompressedVector` crosses the boundary.
- `rust/crates/tinyquant-core/src/backend/mod.rs` — re-exports `SearchBackend`
  and `SearchResult` from `tinyquant_core::prelude`.
- `rust/crates/tinyquant-bruteforce/src/backend.rs` — `BruteForceBackend`:
  exhaustive cosine similarity over an in-memory `HashMap<VectorId, Vec<f32>>`.
  Stores vectors normalized to unit length at ingest time, which makes search a
  pure dot-product sweep.
- `rust/crates/tinyquant-bruteforce/src/similarity.rs` — `cosine_similarity`
  kernel: L2-normalize, then dot product. Operates on `f32` slices; no SIMD in
  this phase (SIMD kernels added in Phase 20).
- `rust/crates/tinyquant-bruteforce/src/store.rs` — `VectorStore`: insertion-
  ordered map that preserves ingest order for deterministic search tiebreaking.
- `rust/crates/tinyquant-bruteforce/tests/backend.rs` — 308 lines covering
  ingest, search ranking, remove, clear, top-k clamping, and the golden fixture
  comparison (`brute_force_golden.json`).
- `rust/crates/tinyquant-pgvector/src/adapter.rs` — `PgvectorAdapter`: wraps a
  `sqlx::PgPool`. `ingest` builds a pgvector `INSERT ... ON CONFLICT UPDATE`
  batch; `search` runs an `<=>` operator query (L2 distance); `remove` deletes
  by `vector_id`. Dimensionality is stored at `new()` time and validated on
  every ingest call.
- `rust/crates/tinyquant-pgvector/src/sql.rs` — raw SQL strings extracted into
  constants to keep `adapter.rs` readable and to enable future parameterization.
- CI: `rust-ci.yml` gains a `pgvector-integration` job that spins up a
  `pgvector/pgvector:pg16` service container and runs
  `cargo test -p tinyquant-pgvector`.

## Deviations from the plan

### 1. Sort order: descending score, ascending vector_id tiebreaker

The plan described `SearchResult` as "`vector_id + score`" without specifying
sort semantics. The initial implementation sorted by score ascending (smallest
distance first), but the brute-force golden fixture comparison revealed that
cosine similarity results should be returned highest-first. Two fix commits
(`82e4e06`, `e17195f`) corrected:

- `SearchResult: Ord` to sort descending by score, then ascending by
  `vector_id` for equal scores (deterministic ordering across equal-cosine
  results).
- `BruteForceBackend::search` to call `.sort_unstable()` after collecting
  candidates, relying on the derived `Ord` impl.

### 2. `shift_remove` instead of `remove` in VectorStore

The plan did not specify how remove interacts with insertion order. The first
implementation used `IndexMap::swap_remove` (O(1) but disrupts order). The fix
commit switched to `IndexMap::shift_remove` (O(n) but preserves order) because
the golden fixture comparison requires deterministic search results even after
removes. The cost is acceptable: remove is not on the hot path.

### 3. `ensure_index` signature change

The initial `PgvectorAdapter::ensure_index` took `dim: u32`. The fix commits
changed it to take no argument (uses `self.dim`) to eliminate a caller-side
error class where the caller could pass a different dimension than the one the
adapter was constructed with.

### 4. `Ord`/`PartialEq` consistency

The initial `SearchResult` derived `PartialOrd` on `(score, vector_id)` but
implemented `Eq` manually for approximate float equality. This violated the
`Ord: Eq` contract. The fix changed to exact float comparison in the sort
(relying on NaN-free scores from normalized vectors) and removed the manual `Eq`
impl.

## FP32 boundary

The `SearchBackend` trait has no method that accepts `CompressedVector`. This is
a deliberate architectural constraint: the backend layer must never know about
the codec's internal representation. All decompression happens inside the corpus
layer before vectors reach the backend. This constraint is tested statically in
`tinyquant-core/tests/backend_trait.rs` (Gap GAP-BACK-001, added in Phase 21.5).

## Risks carried forward

- **R-BACK-1**: `BruteForceBackend` normalizes at ingest. If a caller inserts a
  zero vector, L2 normalization produces NaN. The current implementation does not
  validate for zero vectors at ingest. A future phase should add a guard.
- **R-BACK-2**: The pgvector `<=>` operator is L2 distance, not cosine
  similarity. For unit-normalized vectors the two are equivalent (L2² = 2 − 2·cos),
  but if un-normalized vectors are ever inserted the results will differ from
  `BruteForceBackend`. Normalization is the caller's responsibility.
- **R-BACK-3**: The `PgvectorAdapter` integration test requires a live Docker
  daemon. The job is marked `continue-on-error: false` on `main` and on PRs
  targeting `develop`.
