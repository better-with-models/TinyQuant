---
title: "Phase 21.5: Rust Test Gap Remediation"
tags:
  - plans
  - rust
  - phase-21.5
  - testing
  - gaps
date-created: 2026-04-15
status: draft
category: planning
---

# Phase 21.5: Rust Test Gap Remediation

> [!info] Goal
> Close every P0 and P1 Rust testing gap, plus the P2 and P3 gaps
> that do not require the gold corpus fixture, before Phase 22 (the
> crates.io / PyPI release). No new features ship. Every deliverable
> is a test or a small test-helper — no production source changes.
>
> All gaps are listed in [[requirements/testing-gaps|testing-gaps.md]].
> Phase 26 handles the calibration-fixture-dependent gaps
> (GAP-QUAL-001–003, GAP-QUAL-004, GAP-DECOMP-004).

> [!note] Reference docs
> - [[requirements/testing-gaps|Testing Gaps]]
> - [[requirements/codec|Codec Requirements]] — FR-COMP-006, FR-COMP-007, FR-DECOMP-003, FR-COMP-004
> - [[requirements/corpus|Corpus Requirements]] — FR-CORP-001, FR-CORP-002, FR-CORP-006, FR-CORP-007
> - [[requirements/backend|Backend Protocol Requirements]] — FR-BACK-001, FR-BACK-003, FR-BACK-004, FR-BACK-005
> - [[requirements/quality|Quality Requirements]] — FR-SER-003
> - [[design/rust/testing-strategy|Testing Strategy]]
> - [[design/rust/error-model|Error Model]]

## Prerequisites

- Phase 21 complete (Rayon batch paths, calibration `#[ignore]` tests
  landed, benchmark harness working).
- `cargo test --workspace` green on `main`.
- LFS fixtures for dim=64 already present (used by existing codec
  service tests; no new LFS objects required for this phase).

## Scope

This phase adds tests only. No production source files are modified.
If a gap reveals a genuine bug (a test added here fails for a reason
other than missing coverage), that bug is fixed in this phase before
proceeding. Any fix that touches production code is documented in the
commit message with a `fix:` prefix rather than `test:`.

## Deliverables

### Files to modify

| File | Gaps closed | Notes |
|---|---|---|
| `rust/crates/tinyquant-core/tests/codec_service.rs` | GAP-COMP-006, GAP-COMP-007, GAP-DECOMP-003, GAP-COMP-004 | Add integration tests alongside existing codec service tests |
| `rust/crates/tinyquant-core/tests/corpus_aggregate.rs` | GAP-CORP-001, GAP-CORP-002, GAP-CORP-007 | Add corpus-level immutability and ID uniqueness tests |
| `rust/crates/tinyquant-core/tests/compression_policy.rs` | GAP-CORP-006 | Add FP16 precision bound assertion |
| `rust/crates/tinyquant-core/tests/corpus_search.rs` (new) | GAP-BACK-004, GAP-BACK-005 | Spy adapter + batch partial-success |
| `rust/crates/tinyquant-core/tests/backend_trait.rs` | GAP-BACK-001 | Add architecture assertion: no `CompressedVector` in trait |
| `rust/crates/tinyquant-pgvector/src/wire.rs` | GAP-BACK-003 | Extract dimensionality unit under `#[cfg(test)]` |
| `rust/crates/tinyquant-pgvector/tests/wire_unit.rs` (new) | GAP-BACK-003 | Offline format round-trip without PostgreSQL |
| `rust/crates/tinyquant-io/tests/zero_copy.rs` | GAP-SER-003 | Extend dhat-heap assertion to enforce ≤ 4 KiB budget |

### No new crates, no new LFS objects, no CI matrix changes

All tests run under existing CI jobs. The dhat-heap test extends the
existing `test-mmap-dhat` job that already runs in `rust-ci.yml`.

---

## Part A — Codec service integration tests (GAP-COMP-006, GAP-COMP-007, GAP-DECOMP-003, GAP-COMP-004)

All new tests go in `tinyquant-core/tests/codec_service.rs`, which
already contains the compress/decompress round-trip infrastructure.

### Step 1 — Failing tests (red)

```rust
// GAP-COMP-006: dimension mismatch rejected at compress
#[test]
fn compress_dimension_mismatch_returns_error_and_no_output() {
    let config = CodecConfig::new(4, 42, 768, false).unwrap();
    let codebook = fixture_codebook(&config);
    let short_vec = vec![0f32; 512]; // wrong: config expects 768
    let result = Codec::new().compress(&short_vec, &config, &codebook);
    assert!(
        matches!(result, Err(CodecError::DimensionMismatch { .. })),
        "expected DimensionMismatch, got {result:?}"
    );
}

// GAP-COMP-007: config hash embedded in CompressedVector
#[test]
fn compress_embeds_config_hash_in_output() {
    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let codebook = fixture_codebook(&config);
    let vector = fixture_vector(64, 0);
    let cv = Codec::new().compress(&vector, &config, &codebook).unwrap();
    assert_eq!(
        cv.config_hash().as_ref(),
        config.hash().as_ref(),
        "CompressedVector.config_hash must equal CodecConfig.hash()"
    );
}

// GAP-DECOMP-003: config mismatch rejected at decompress
#[test]
fn decompress_config_mismatch_returns_error_and_no_output() {
    let config_a = CodecConfig::new(4, 42, 64, false).unwrap();
    let config_b = CodecConfig::new(4, 99, 64, false).unwrap(); // different seed
    let codebook_a = fixture_codebook(&config_a);
    let codebook_b = fixture_codebook(&config_b);
    let cv = Codec::new()
        .compress(&fixture_vector(64, 0), &config_a, &codebook_a)
        .unwrap();
    let result = Codec::new().decompress(&cv, &config_b, &codebook_b);
    assert!(
        matches!(result, Err(CodecError::ConfigMismatch { .. })),
        "expected ConfigMismatch, got {result:?}"
    );
}

// GAP-COMP-004: residual payload length == dim * 2 bytes (FP16)
#[test]
fn compress_with_residual_sets_correct_payload_length() {
    let dim: usize = 64;
    let config = CodecConfig::new(4, 42, dim as u32, true).unwrap(); // residual on
    let codebook = fixture_codebook(&config);
    let cv = Codec::new()
        .compress(&fixture_vector(dim, 0), &config, &codebook)
        .unwrap();
    let residual = cv.residual().expect("residual should be present");
    assert_eq!(
        residual.len(),
        dim * 2,
        "FP16 residual payload must be dim * 2 bytes; got {}",
        residual.len()
    );
}
```

### Step 2 — Make tests pass

These tests should pass without production code changes if the
implementations are correct. If any fail, investigate before adding
workarounds — a failing test here indicates a real bug in the codec
pipeline.

---

## Part B — Corpus aggregate tests (GAP-CORP-001, GAP-CORP-002, GAP-CORP-007)

### Step 3 — Failing tests (red)

Add to `tinyquant-core/tests/corpus_aggregate.rs`:

```rust
// GAP-CORP-001: each Corpus gets a unique corpus_id
#[test]
fn each_corpus_gets_unique_id() {
    let ids: Vec<CorpusId> = (0..100)
        .map(|_| {
            let config = CodecConfig::new(4, 42, 8, false).unwrap();
            let codebook = fixture_codebook(&config);
            Corpus::new(config, codebook, CompressionPolicy::Passthrough).id()
        })
        .collect();
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(
        ids.len(),
        unique.len(),
        "all 100 corpus IDs must be distinct"
    );
}

// GAP-CORP-002: CodecConfig is frozen at the Corpus level after creation
//
// If Corpus has no mutation API (config is private + only set in ::new),
// this test documents the structural guarantee and serves as a
// compile-time canary: if someone adds a set_config method, the test
// must be updated to assert it is rejected.
#[test]
fn corpus_config_is_frozen_after_creation() {
    let config = CodecConfig::new(4, 42, 8, false).unwrap();
    let codebook = fixture_codebook(&config);
    let corpus = Corpus::new(config.clone(), codebook, CompressionPolicy::Passthrough);
    // Assert the stored config equals the creation config and cannot
    // be replaced via any public API.
    assert_eq!(corpus.config().hash(), config.hash());
    // If Corpus::set_config or similar exists, assert it returns
    // Err(CorpusError::Immutable) here. Currently there is no such
    // method, so this test documents the structural guarantee only.
    // If the type system enforces immutability (no setter), this test
    // still guards against accidental addition of a setter: any new
    // set_config method would need to be exercised here.
}

// GAP-CORP-007: policy change on a populated corpus is rejected
#[test]
fn policy_change_on_populated_corpus_is_rejected() {
    let config = CodecConfig::new(4, 42, 8, false).unwrap();
    let codebook = fixture_codebook(&config);
    let mut corpus = Corpus::new(
        config.clone(),
        codebook.clone(),
        CompressionPolicy::Passthrough,
    );
    corpus
        .insert("v0", fixture_vector(8, 0).as_slice())
        .expect("first insertion must succeed");
    // Any API that changes the compression policy on a populated
    // corpus must return CorpusError::PolicyImmutable.
    // If no such API exists (structural guarantee), document it.
    let result = corpus.set_compression_policy(CompressionPolicy::Compress);
    assert!(
        matches!(result, Err(CorpusError::PolicyImmutable)),
        "expected PolicyImmutable, got {result:?}"
    );
}
```

> [!note] Structural vs behavioral enforcement
> If `Corpus` has no `set_compression_policy` method (i.e., immutability
> is enforced purely by the absence of a setter), adjust the
> GAP-CORP-002 and GAP-CORP-007 tests to document that fact with a
> comment and an assertion on the existing `config()` / `policy()`
> getters. The gap is still closed: the test now explicitly states
> _why_ no runtime rejection is needed and will fail if a setter is
> accidentally added.

### Step 4 — Make tests pass

No production code changes expected. If `set_compression_policy` does
not exist, replace the test body with the structural documentation
variant described in the note above.

---

## Part C — FP16 precision bound (GAP-CORP-006)

### Step 5 — Failing test (red)

Add to `tinyquant-core/tests/compression_policy.rs`:

```rust
// GAP-CORP-006: FP16 policy preserves elements within 2^-13 * |original|
#[test]
fn fp16_policy_precision_within_bound() {
    let dim: usize = 64;
    let config = CodecConfig::new(4, 42, dim as u32, false).unwrap();
    let codebook = fixture_codebook(&config);
    let mut corpus = Corpus::new(
        config,
        codebook,
        CompressionPolicy::Fp16,
    );
    let original = fixture_vector(dim, 7); // deterministic, avoids zeros
    corpus.insert("v0", &original).unwrap();
    let retrieved = corpus.decompress_all();
    let retrieved_v = &retrieved["v0"];

    for (i, (&o, &r)) in original.iter().zip(retrieved_v.iter()).enumerate() {
        if o == 0.0 {
            assert_eq!(r, 0.0, "element {i}: zero must round-trip exactly");
            continue;
        }
        let bound = 2f32.powi(-13) * o.abs();
        let delta = (o - r).abs();
        assert!(
            delta <= bound,
            "element {i}: |{o} - {r}| = {delta} exceeds FP16 bound {bound}"
        );
    }
}
```

### Step 6 — Make test pass

No production code changes expected. The FP16 round-trip precision
follows from IEEE 754 half-precision semantics. If the test fails,
investigate whether `Corpus::decompress_all` under the FP16 policy
applies an unintended transformation.

---

## Part D — Batch error isolation and query passthrough (GAP-BACK-004, GAP-BACK-005)

These two gaps require new test infrastructure: a spy adapter that
records what reaches the backend boundary.

### Step 7 — Create `tests/corpus_search.rs`

```rust
// rust/crates/tinyquant-core/tests/corpus_search.rs
//
// Tests for corpus→backend boundary contracts:
// - FR-BACK-004: error isolation (partial-success batch)
// - FR-BACK-005: query vector passthrough (no rotation/quantization)

use std::sync::{Arc, Mutex};
use tinyquant_core::{
    backend::{SearchBackend, SearchResult},
    corpus::{Corpus, CompressionPolicy},
    errors::BackendError,
};

/// Spy adapter that records every query vector and every ingested vector.
/// All `search` calls return an empty result set — the spy is not a real
/// similarity engine.
struct SpyBackend {
    ingested: Arc<Mutex<Vec<(String, Vec<f32>)>>>,
    queries:  Arc<Mutex<Vec<Vec<f32>>>>,
}

impl SpyBackend {
    fn new() -> Self {
        Self {
            ingested: Arc::new(Mutex::new(Vec::new())),
            queries:  Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn queries_seen(&self) -> Vec<Vec<f32>> {
        self.queries.lock().unwrap().clone()
    }

    fn ingested_count(&self) -> usize {
        self.ingested.lock().unwrap().len()
    }
}

impl SearchBackend for SpyBackend {
    fn ingest(&mut self, vectors: &[(String, Vec<f32>)]) -> Result<(), BackendError> {
        self.ingested.lock().unwrap().extend_from_slice(vectors);
        Ok(())
    }

    fn search(
        &self,
        query: &[f32],
        _top_k: usize,
    ) -> Result<Vec<SearchResult>, BackendError> {
        self.queries.lock().unwrap().push(query.to_vec());
        Ok(vec![])
    }

    fn remove(&mut self, _ids: &[String]) -> Result<(), BackendError> { Ok(()) }
}

/// GAP-BACK-004: a batch where one vector is corrupt delivers the other
/// N-1 vectors to the backend and reports the corrupt id in the error.
///
/// "Corrupt" here means: a VectorEntry whose stored bytes cannot be
/// decompressed (e.g., an impossible quantization index or a truncated
/// residual). The fixture injects a raw corrupt VectorEntry directly
/// rather than going through compress(), which would not produce invalid
/// data.
#[test]
fn batch_decompress_error_isolation_delivers_good_vectors() {
    // Build a corpus with 10 valid compressed vectors.
    let config = CodecConfig::new(4, 42, 8, false).unwrap();
    let codebook = fixture_codebook(&config);
    let mut corpus = Corpus::new(
        config.clone(),
        codebook.clone(),
        CompressionPolicy::Compress,
    );
    for i in 0..10u32 {
        corpus
            .insert(&format!("v{i}"), &fixture_vector(8, i))
            .unwrap();
    }

    // Inject one corrupt VectorEntry (impossible indices) at id "v5".
    corpus.inject_corrupt_entry("v5").expect(
        "inject_corrupt_entry is a test-only helper that replaces the stored \
         bytes with an undecompressable payload",
    );

    let mut backend = SpyBackend::new();
    let result = corpus.decompress_batch_into(&mut backend);

    // The operation should return a partial-success error, not a total failure.
    let err = result.expect_err("corrupt entry must cause a partial error");
    assert!(
        err.affected_ids().contains(&"v5".to_string()),
        "corrupt vector_id must appear in error report"
    );

    // Nine good vectors must still have been ingested.
    assert_eq!(
        backend.ingested_count(),
        9,
        "9 good vectors must be delivered despite one corrupt entry"
    );
}

/// GAP-BACK-005: query vector reaches the backend element-wise unchanged.
#[test]
fn query_vector_passes_through_to_backend_unchanged() {
    let config = CodecConfig::new(4, 42, 8, false).unwrap();
    let codebook = fixture_codebook(&config);
    let mut corpus = Corpus::new(
        config.clone(),
        codebook.clone(),
        CompressionPolicy::Compress,
    );
    corpus.insert("v0", &fixture_vector(8, 0)).unwrap();

    let query = fixture_vector(8, 99);
    let spy = SpyBackend::new();
    corpus.search(&query, &spy, 1).unwrap();

    let seen = spy.queries_seen();
    assert_eq!(seen.len(), 1, "spy must have recorded exactly one query");
    assert_eq!(
        seen[0], query,
        "query delivered to backend must be element-wise identical to original"
    );
}
```

> [!note] `inject_corrupt_entry` test helper
> This is a `#[cfg(test)]` method on `Corpus` (or a `pub(crate)` method
> on the inner storage type) that replaces a stored `VectorEntry`'s
> byte payload with an impossible quantization sequence. It must not be
> visible in the public API. If `Corpus` does not have access to the
> raw stored bytes (e.g., they are behind an immutable `VectorEntry`),
> the helper can insert a `VectorEntry` constructed with
> `VectorEntry::dangerous_test_only_corrupt(id)`. Document the helper
> in the test file, not in production source.

### Step 8 — Make tests pass

`decompress_batch_into` (or the equivalent `decompress_all` + ingest
pipeline) may need a partial-success error variant if it currently
returns only `Ok` or a total-failure `Err`. Check the current API and
add `CorpusError::PartialDecompressionFailure { affected_ids: Vec<VectorId>, delivered: usize }`
if it does not exist. This is the only production source change
permitted in this phase; if it is needed, it goes in a separate commit
with a `fix:` prefix.

---

## Part E — pgvector offline unit test (GAP-BACK-003)

### Step 9 — Create `tests/wire_unit.rs`

The existing integration test in `tinyquant-pgvector/tests/adapter.rs`
requires a live PostgreSQL container. This step extracts the
dimensionality-preservation check into a unit test that runs without
Docker.

```rust
// rust/crates/tinyquant-pgvector/tests/wire_unit.rs
//
// Offline unit tests for the pgvector wire format (GAP-BACK-003).
// No PostgreSQL connection required.

use tinyquant_pgvector::wire::{format_as_pgvector, parse_pgvector};

/// FR-BACK-003: dimension and element values are preserved through the
/// pgvector text wire format.
#[test]
fn pgvector_wire_preserves_dimension_and_values() {
    let original: Vec<f32> = (0..768).map(|i| i as f32 * 0.001).collect();
    let wire = format_as_pgvector(&original);
    let recovered = parse_pgvector(&wire)
        .expect("valid pgvector text must parse without error");

    assert_eq!(
        original.len(),
        recovered.len(),
        "dimension must be preserved through pgvector wire format"
    );
    for (i, (o, r)) in original.iter().zip(recovered.iter()).enumerate() {
        assert!(
            (o - r).abs() < f32::EPSILON * o.abs().max(1.0),
            "element {i}: {o} != {r} after wire round-trip"
        );
    }
}

/// Vectors at the FP32 extremes survive the wire format.
#[test]
fn pgvector_wire_handles_extreme_fp32_values() {
    let extremes = vec![f32::MAX, f32::MIN_POSITIVE, 0.0f32, -0.0f32, 1.0f32];
    let wire = format_as_pgvector(&extremes);
    let recovered = parse_pgvector(&wire).unwrap();
    assert_eq!(extremes.len(), recovered.len());
    // f32::MAX may lose precision in text format; allow 1 ULP.
    for (o, r) in extremes.iter().zip(recovered.iter()) {
        assert!((o - r).abs() <= o.abs() * f32::EPSILON);
    }
}
```

The `format_as_pgvector` and `parse_pgvector` functions must be made
`pub(crate)` (or `pub` under `#[cfg(test)]`) if they are currently
private. No logic change is needed.

---

## Part F — FP32 boundary architecture test (GAP-BACK-001)

### Step 10 — Extend `backend_trait.rs`

Add to `tinyquant-core/tests/backend_trait.rs`:

```rust
// GAP-BACK-001: SearchBackend trait methods must not accept CompressedVector.
//
// This test documents the structural guarantee. If someone adds a method
// that takes &CompressedVector, this file must be updated — and that
// update triggers review of whether the anti-corruption layer has been
// violated.
//
// The compile-time guarantee is enforced by the trait definition:
// `ingest` takes `&[(VectorId, Vec<f32>)]`, `search` takes `&[f32]`,
// `remove` takes `&[VectorId]`. None of these accept CompressedVector.
//
// We assert this structurally by verifying that CompressedVector does
// not implement Into<Vec<f32>> (which would create an implicit path)
// and is not Send-castable as a search input.
static_assertions::assert_not_impl_all!(
    tinyquant_core::CompressedVector: Into<Vec<f32>>
);
```

---

## Part G — Zero-copy heap measurement (GAP-SER-003)

### Step 11 — Extend the dhat-heap test

The existing `test-mmap-dhat` CI job runs:

```
cargo test -p tinyquant-io --features "mmap dhat-heap" --test zero_copy
```

Extend `tinyquant-io/tests/zero_copy.rs` with a quantitative heap
measurement:

```rust
// GAP-SER-003: heap growth per 1 000 corpus reads must not exceed 4 KiB.
//
// This test runs only when the dhat-heap feature is enabled (CI's
// test-mmap-dhat job). The dhat crate writes heap stats to a JSON file
// after the process exits; we read and assert the budget in a follow-up
// script (`scripts/check_dhat_budget.py`) called by the CI job.
// The test itself records the dhat epoch marker; the budget assertion
// lives outside Rust to keep the test binary simple.
#[cfg(feature = "dhat-heap")]
#[test]
fn zero_copy_heap_growth_within_4kib_per_1000_reads() {
    let _profiler = dhat::Profiler::new_heap();
    let corpus_file = fixture_mmap_corpus(1_000); // 1 000 pre-built vectors

    let before = dhat::HeapStats::get();
    for entry in corpus_file.iter() {
        // Access the slice; do not allocate a Vec.
        let _slice: &[u8] = entry.as_bytes();
    }
    let after = dhat::HeapStats::get();

    let delta_bytes = after.total_bytes - before.total_bytes;
    assert!(
        delta_bytes <= 4096,
        "heap growth for 1 000 corpus reads must be ≤ 4 KiB; got {delta_bytes} bytes"
    );
}
```

Update `.github/workflows/rust-ci.yml` `test-mmap-dhat` job to call
`scripts/check_dhat_budget.py` after the test completes and assert the
budget from the output file. The script already exists for Phase 17;
extend it to parse the `zero_copy_heap_growth` epoch.

---

## Steps summary (ordered)

| Step | Gap(s) closed | File | Est. effort |
|---|---|---|---|
| 1–2 | GAP-COMP-006, GAP-COMP-007, GAP-DECOMP-003, GAP-COMP-004 | `codec_service.rs` | 1–2 h |
| 3–4 | GAP-CORP-001, GAP-CORP-002, GAP-CORP-007 | `corpus_aggregate.rs` | 1–2 h |
| 5–6 | GAP-CORP-006 | `compression_policy.rs` | 0.5 h |
| 7–8 | GAP-BACK-004, GAP-BACK-005 | `corpus_search.rs` (new) | 2–3 h |
| 9 | GAP-BACK-003 | `wire_unit.rs` (new) | 1 h |
| 10 | GAP-BACK-001 | `backend_trait.rs` | 0.5 h |
| 11 | GAP-SER-003 | `zero_copy.rs`, CI script | 1 h |

---

## Acceptance criteria

- [ ] `cargo test --workspace` green (no new failures, no regressions).
- [ ] `cargo test --workspace --features dhat-heap` green (dhat-heap
      tests added in Step 11 pass).
- [ ] Every test added in this phase passes on its first non-trivial run
      (i.e., all tests were genuinely failing before the gap was closed).
- [ ] `cargo clippy -p tinyquant-core -p tinyquant-pgvector -p tinyquant-io
      -- -D warnings` clean.
- [ ] All 13 gaps listed in [[requirements/testing-gaps|testing-gaps.md]]
      §Phase 21.5 are updated to `Gap: None.` in the relevant requirement
      blocks.
- [ ] `docs/requirements/testing-gaps.md` gap summary table updated:
      every closed gap's status column reads "closed — Phase 21.5".

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `inject_corrupt_entry` requires touching private struct internals | Medium | Low | Use `pub(crate)` test helper on `VectorEntryStore`; do not expose in public API |
| GAP-BACK-004 partial-success path does not yet exist in `Corpus` | Medium | Medium | If missing, add `CorpusError::PartialDecompressionFailure` in this phase; document as a bug fix |
| `format_as_pgvector` / `parse_pgvector` currently private | Low | Low | Make `pub(crate)` — no API surface change |
| dhat-heap CI job timing varies across runners | Low | Low | Use delta bytes (not total), which is runner-independent |

## See also

- [[requirements/testing-gaps|Testing Gaps]] — canonical gap descriptions and actions
- [[plans/rust/phase-22-pyo3-cabi-release|Phase 22]] — gated on this phase
- [[plans/rust/phase-26-preparedcodec-calibration|Phase 26]] — closes remaining calibration gaps
- [[design/rust/testing-strategy|Testing Strategy]]
