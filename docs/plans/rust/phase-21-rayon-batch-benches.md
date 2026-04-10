---
title: "Phase 21: Rayon Batch Paths, Calibration, and Benchmarks"
tags:
  - plans
  - rust
  - phase-21
  - rayon
  - calibration
  - benchmarks
date-created: 2026-04-10
status: draft
category: planning
---

# Phase 21: Rayon Batch Paths, Calibration, and Benchmarks

> [!info] Goal
> Wire the `Parallelism::Custom` rayon driver into batch codec
> paths, land the score-fidelity and compression-ratio calibration
> tests (mirroring Python's `tests/calibration/`), and enforce
> `bench --check-against main` budgets in CI.

> [!note] Reference docs
> - [[design/rust/parallelism|Parallelism and Concurrency]]
> - [[design/rust/benchmark-harness|Benchmark Harness]]
> - [[design/rust/goals-and-non-goals|Goals and Non-Goals]]

## Prerequisites

- Phase 20 complete (SIMD kernels landed).

## Deliverables

### Files to create/modify

| File | Purpose |
|------|---------|
| `rust/crates/tinyquant-core/src/codec/batch.rs` | Parallel batch internals |
| `rust/crates/tinyquant-io/src/parallelism.rs` | `rayon_parallelism()` helper |
| `rust/crates/tinyquant-core/tests/batch_parallel.rs` | Parallel correctness |
| `rust/crates/tinyquant-bench/tests/calibration.rs` | Score fidelity gates |
| `rust/crates/tinyquant-bench/baselines/main.json` | Budget file |
| `rust/crates/tinyquant-bench/benches/batch_parallel.rs` | Parallel scaling |
| `rust/xtask/src/cmd/bench.rs` | `xtask bench --check-against` |
| `.github/workflows/rust-ci.yml` | Add bench budget gate |

## Steps (TDD order)

### Part A — Rayon batch paths

- [ ] **Step 1: Failing parallel batch test**

```rust
#[test]
fn compress_batch_rayon_matches_serial() {
    // Build 256 random vectors.
    // compress_batch(serial) vs compress_batch(rayon).
    // Byte-identical result per index.
}
```

- [ ] **Step 2: Implement `tinyquant-io::parallelism::rayon_parallelism()`**

```rust
pub fn rayon_parallelism() -> tinyquant_core::codec::Parallelism {
    tinyquant_core::codec::Parallelism::Custom(|count, body| {
        use rayon::prelude::*;
        (0..count).into_par_iter().for_each(|i| body(i));
    })
}
```

- [ ] **Step 3: Wire parallel path into `compress_batch`**

Use the `MaybeUninit` + `SyncPtr` pattern from
[[design/rust/parallelism|Parallelism]] §Batch GEMM.

- [ ] **Step 4: Run parallel correctness test — expect pass.**

- [ ] **Step 5: Implement `compress_batch_packed`**

Zero-allocation variant returning `BatchedCompressedVectors` that
hand out `CompressedVectorView` borrows.

- [ ] **Step 6: Batch bench scaling**

```rust
// benches/batch_parallel.rs
for &threads in &[1, 2, 4, 8, 16] {
    let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
    group.bench_with_input(BenchmarkId::from_parameter(threads), ...);
}
```

### Part B — Calibration suite

- [ ] **Step 7: Failing score fidelity test**

Mirrors Python `tests/calibration/test_score_fidelity.py`:

```rust
#[test]
#[ignore = "calibration; run with --ignored"]
fn pearson_rho_4bit_with_residuals_meets_threshold() {
    let corpus = GoldCorpus::load(768, 10_000, 4);
    // Sample 1000 pairs, compute original pairwise cosine and
    // reconstructed pairwise cosine, then Pearson correlation.
    let rho = compute_pearson_correlation(&corpus);
    assert!(rho >= 0.995, "rho={rho}");
}
```

Similar tests for:

- 4-bit without residuals (ρ ≥ 0.98).
- 2-bit with residuals (ρ ≥ 0.95).
- Top-10 neighbor overlap (≥ 80%).
- Passthrough perfect fidelity (ρ == 1.0).

- [ ] **Step 8: Implement helper**

`compute_pearson_correlation` lives in `tinyquant-bench::calibration`
and is shared between tests and benchmarks.

- [ ] **Step 9: Compression ratio gate**

```rust
#[test]
#[ignore = "calibration"]
fn compression_ratio_4bit_meets_threshold() {
    let corpus = GoldCorpus::load(1536, 10_000, 4);
    let total_raw = corpus.vectors.len() * 4;
    let codec = Codec::new();
    let cvs = codec.compress_batch(
        &corpus.vectors, 10_000, 1536,
        &corpus.config, &corpus.codebook,
        Parallelism::Serial,
    ).unwrap();
    let total_compressed: usize = cvs.iter().map(|cv| cv.size_bytes()).sum();
    let ratio = total_raw as f64 / total_compressed as f64;
    assert!(ratio >= 5.0, "ratio={ratio}"); // 4-bit + residuals
}
```

- [ ] **Step 10: Determinism gate**

```rust
#[test]
fn full_pipeline_is_deterministic_across_calls() { /* … */ }
```

### Part C — Benchmark budget gate

- [ ] **Step 11: Implement `xtask bench --check-against main`**

Parses `target/criterion/*/new/estimates.json`, loads
`baselines/main.json`, and exits non-zero if any median exceeds the
budget.

- [ ] **Step 12: Capture initial baseline**

```bash
cargo bench -p tinyquant-bench
cargo xtask bench --capture-baseline main
```

Commit `baselines/main.json`.

- [ ] **Step 13: Update `rust-ci.yml` to run `xtask bench
--check-against main` on pushes to `main`**

Add the stage as advisory on PRs and blocking on main.

- [ ] **Step 14: Run all calibration tests**

```bash
cargo test --workspace --all-features -- --ignored
```

Confirm every calibration gate passes.

- [ ] **Step 15: Run standard tests (not ignored)**

```bash
cargo test --workspace --all-features
```

Expect no regressions.

- [ ] **Step 16: Commit**

```bash
git add rust/crates/tinyquant-core/src/codec/batch.rs \
        rust/crates/tinyquant-io/src/parallelism.rs \
        rust/crates/tinyquant-bench \
        rust/xtask .github/workflows/rust-ci.yml
git commit -m "feat(rust): add rayon batch paths, calibration gates, and bench budgets"
```

## Acceptance criteria

- Parallel batch compress matches serial byte-for-byte.
- Pearson ρ, compression ratio, and determinism gates all green on
  the gold fixture.
- `xtask bench --check-against main` fails on regression, passes on
  clean main.
- Every performance goal from
  [[design/rust/goals-and-non-goals|Goals and Non-Goals]] is either
  met or has an open risk entry.
- Clippy + fmt clean.

## Out of scope

- Python bindings — phase 22.
- C ABI — phase 22.
- Release publishing — phase 22.

## See also

- [[plans/rust/phase-20-simd-kernels|Phase 20]]
- [[plans/rust/phase-22-pyo3-cabi-release|Phase 22]]
- [[design/rust/parallelism|Parallelism]]
- [[design/rust/benchmark-harness|Benchmark Harness]]
