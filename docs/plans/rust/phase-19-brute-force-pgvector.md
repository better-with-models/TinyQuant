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
> adapter semantics.

> [!note] Reference docs
> - [[design/rust/type-mapping|Type Mapping]] §Backend layer
> - [[design/behavior-layer/backend-protocol|Backend Protocol]]

## Prerequisites

- Phase 18 complete (corpus aggregate).

## Deliverables

### Files to create

| File | Purpose |
|------|---------|
| `rust/crates/tinyquant-core/src/backend/mod.rs` | Module root |
| `rust/crates/tinyquant-core/src/backend/protocol.rs` | `SearchBackend` trait, `SearchResult` |
| `rust/crates/tinyquant-bruteforce/src/lib.rs` | Public re-exports |
| `rust/crates/tinyquant-bruteforce/src/backend.rs` | `BruteForceBackend` |
| `rust/crates/tinyquant-bruteforce/src/similarity.rs` | Scalar cosine kernel |
| `rust/crates/tinyquant-bruteforce/src/store.rs` | Owned FP32 store |
| `rust/crates/tinyquant-bruteforce/tests/backend.rs` | Unit tests |
| `rust/crates/tinyquant-pgvector/src/lib.rs` | Re-exports |
| `rust/crates/tinyquant-pgvector/src/adapter.rs` | `PgvectorAdapter` |
| `rust/crates/tinyquant-pgvector/src/sql.rs` | Prepared statement constants |
| `rust/crates/tinyquant-pgvector/src/wire.rs` | FP32 ↔ pgvector format |
| `rust/crates/tinyquant-pgvector/tests/adapter.rs` | Integration (testcontainers) |

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
```

- [ ] **Step 2: Implement `protocol.rs`**

Per [[design/rust/type-mapping|Type Mapping]] §SearchBackend.

Use `Ord` derived via manual `PartialOrd` that flips the direction:

```rust
impl Ord for SearchResult {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        other.score.partial_cmp(&self.score).unwrap_or(core::cmp::Ordering::Equal)
    }
}
```

NaN scores tie at Equal to keep `sort` stable.

### Part B — Brute-force backend

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
fn search_with_fewer_vectors_than_top_k_returns_all() { /* … */ }
```

- [ ] **Step 4: Implement `BruteForceBackend`**

```rust
pub struct BruteForceBackend {
    dim: Option<usize>,
    store: IndexMap<VectorId, Vec<f32>, RandomState>,
}

impl SearchBackend for BruteForceBackend {
    fn ingest(&mut self, vectors: &[(VectorId, Vec<f32>)]) -> Result<(), BackendError> {
        for (id, v) in vectors {
            match self.dim {
                Some(d) if d != v.len() => return Err(BackendError::Adapter(Arc::from("dimension mismatch"))),
                None => self.dim = Some(v.len()),
                _ => {}
            }
            self.store.insert(id.clone(), v.clone());
        }
        Ok(())
    }

    fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<SearchResult>, BackendError> {
        if top_k == 0 {
            return Err(BackendError::InvalidTopK);
        }
        if self.store.is_empty() {
            return Ok(Vec::new());
        }
        let mut results = Vec::with_capacity(self.store.len());
        for (id, v) in &self.store {
            let score = cosine_similarity(query, v);
            results.push(SearchResult { vector_id: id.clone(), score });
        }
        results.sort();
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
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (&x, &y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = (na * nb).sqrt();
    if denom == 0.0 { 0.0 } else { dot / denom }
}
```

- [ ] **Step 6: Run backend tests — expect pass.**

### Part C — Pgvector adapter

- [ ] **Step 7: Failing adapter tests (testcontainers-gated)**

```rust
#[cfg(feature = "test-containers")]
#[test]
fn pgvector_adapter_round_trip() {
    use testcontainers::clients::Cli;
    // spin up pgvector image, run migration, insert, search, remove
}
```

- [ ] **Step 8: Implement `PgvectorAdapter`**

Uses the `postgres` crate for a blocking client with a caller-
supplied `impl Fn() -> Result<Client, postgres::Error> + Send + Sync`
factory.

- [ ] **Step 9: Implement SQL constants**

```rust
// sql.rs
pub(crate) const INSERT_SQL: &str =
    "INSERT INTO {table} (id, embedding) VALUES ($1, $2::vector) \
     ON CONFLICT (id) DO UPDATE SET embedding = EXCLUDED.embedding";

pub(crate) const SEARCH_SQL: &str =
    "SELECT id, 1 - (embedding <=> $1::vector) AS score \
     FROM {table} \
     ORDER BY embedding <=> $1::vector \
     LIMIT $2";

pub(crate) const DELETE_SQL: &str =
    "DELETE FROM {table} WHERE id = ANY($1)";
```

- [ ] **Step 10: Wire format**

`wire.rs` converts `&[f32]` to pgvector's text format
`"[1.0,2.0,3.0]"` (matching Python's `.tolist()` approach).

- [ ] **Step 11: Injection safety**

Table name is validated against `[A-Za-z_][A-Za-z0-9_]{0,62}`.
Values always go through parameterized queries.

- [ ] **Step 12: Run adapter tests (gated on Docker).**

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

## Acceptance criteria

- `SearchBackend` trait is dyn-safe (`Box<dyn SearchBackend>` compiles).
- `BruteForceBackend` passes all mirroring Python brute-force tests
  (identical to orthogonal → score 0, identical → 1.0).
- Results ordered by descending score.
- `BackendError::InvalidTopK` returned on `top_k == 0`.
- `PgvectorAdapter` round-trips against a testcontainers-backed
  pgvector instance (integration test is gated on Docker).
- Clippy + fmt clean.

## Out of scope

- SIMD cosine kernel (phase 20).
- Async postgres support.

## See also

- [[plans/rust/phase-18-corpus-aggregate|Phase 18]]
- [[plans/rust/phase-20-simd-kernels|Phase 20]]
- [[design/behavior-layer/backend-protocol|Backend Protocol]]
