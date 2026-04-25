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
> - [[design/rust/numerical-semantics|Numerical Semantics]]
> - [[design/rust/testing-strategy|Testing Strategy]]
> - [[design/rust/ci-cd|CI/CD]]
> - [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]]

## Parallelism determinism contract

> [!important] Invariant
> `compress_batch` MUST produce byte-identical `CompressedVector`s
> regardless of which `Parallelism` driver is passed. Specifically:
>
> - `Parallelism::Serial`
> - `Parallelism::Custom(rayon_parallelism())` with **1** thread
> - `Parallelism::Custom(rayon_parallelism())` with **4** threads
> - `Parallelism::Custom(rayon_parallelism())` with **`nproc`** threads
>
> all produce the same `Vec<CompressedVector>`, byte-for-byte, at
> every index.

### Why this matters

Phase 16's fixture-parity gates are evaluated against the **serial**
path (the Python reference implementation walks one vector at a
time). If Phase 21's parallel path takes a different reduction order
on any per-vector computation, the fixture-parity gates silently
break on runs that happen to use the rayon driver. This is the same
class of nondeterminism documented in **Phase 14 Lesson L4**
(faer + pulp ISA-dependent reductions): a code path that is
"equivalent up to rounding" is not equivalent for bit-exact fixtures.

The Phase 21 contract is stricter than Phase 14's: we do not tolerate
any bitwise divergence across thread counts, because unlike
dim=768 rotations (an isolated mis-reduction), the batch path is on
the hot path for every production caller and is covered by multiple
fixture gates.

### How we enforce it

1. **No shared reduction.** The parallel path writes into
   `&mut [MaybeUninit<CompressedVector>]` at **independent indices**.
   There is no cross-worker reduction step.
2. **Per-vector purity.** Each vector's compression is a pure,
   order-independent function of `(vector, config, codebook,
   rotation)`. All sums inside `compress_one` happen inside a single
   worker and therefore always run in the same order.
3. **Read-only shared state.** The codebook, config, and rotation
   matrix are passed as `Arc<T>` and only read; no worker mutates
   them.
4. **Thread-count sweep test.** A dedicated test
   (`tests/batch_determinism.rs`) runs `compress_batch` under
   `[1, 2, 4, 8, rayon::current_num_threads()]` threads and asserts
   `result[t_a][i] == result[t_b][i]` as bytes, for every pair
   `(t_a, t_b)` and every index `i`. See Step 10.
5. **Loom stress test (feature-gated).** Under `--features loom` a
   smaller test runs on a 4-vector batch and asks `loom` to explore
   atomic orderings on the `SyncPtr` writes and the `first_error`
   mutex. This is advisory, not gating, because loom + criterion
   interactions are fragile.

### What we accept as non-deterministic

- **Wall-clock time.** Scheduling is non-deterministic by design.
- **The order in which workers observe `first_error`.** Fail-fast is
  best-effort; a slightly later worker may still complete before it
  sees the error flag. This is fine because the fail-fast path
  discards all results on error.

Anything else is a bug and must be gated on regression tests before
Phase 21 exits.

## Prerequisites

- Phase 20 complete (SIMD kernels landed).
- Phase 16 fixture parity gates green on the serial path (they will
  continue to run against `Parallelism::Serial` specifically).
- Phase 14 rotation orthogonality gate green (Lesson L4 caveat:
  bit-exact rotation checks remain skipped at dim=768).

## Deliverables

### Files to create/modify

| File | Purpose | Visibility |
|------|---------|------------|
| `rust/crates/tinyquant-core/src/codec/batch.rs` | Parallel batch internals (`MaybeUninit` + `SyncPtr` recipe) | `pub(crate)` |
| `rust/crates/tinyquant-core/src/codec/batch_error.rs` | `PartialInit<T>` helper for safe drop on error | `pub(crate)` |
| `rust/crates/tinyquant-io/src/parallelism.rs` | `rayon_parallelism()` helper, feature-gated on `rayon` | `pub`, `#[cfg(feature = "rayon")]` |
| `rust/crates/tinyquant-core/tests/batch_parallel.rs` | Parallel correctness vs serial | test binary |
| `rust/crates/tinyquant-core/tests/batch_determinism.rs` | Thread-count sweep, runs on every PR (not `#[ignore]`) | test binary |
| `rust/crates/tinyquant-bench/tests/calibration.rs` | Score fidelity, recall, ratio gates (`#[ignore]`) | test binary |
| `rust/crates/tinyquant-bench/src/calibration/mod.rs` | Shared helpers (corpus loader, metric wrappers) | `pub` |
| `rust/crates/tinyquant-bench/src/calibration/pearson.rs` | Welford-based Pearson ρ | `pub(crate)` |
| `rust/crates/tinyquant-bench/src/calibration/neighbor_recall.rs` | Top-k recall helper | `pub(crate)` |
| `rust/crates/tinyquant-bench/baselines/main.json` | Committed budget (CI-on-main capture) | data |
| `rust/crates/tinyquant-bench/baselines/schema.json` | JSON schema used by `--validate` | data |
| `rust/crates/tinyquant-bench/fixtures/calibration/openai_10k_d1536.f32.bin` | LFS-tracked fixture | data, **LFS** |
| `rust/crates/tinyquant-bench/fixtures/calibration/openai_1k_d768.f32.bin` | PR-speed LFS fixture | data, **LFS** |
| `rust/crates/tinyquant-bench/fixtures/calibration/manifest.json` | SHA256 manifest for drift detection | data |
| `rust/crates/tinyquant-bench/benches/codec_compress_batch_parallel.rs` | Scaling criterion bench (group: `codec/compress/batch_parallel`) | bench binary |
| `rust/crates/tinyquant-bench/benches/codec_decompress_batch_parallel.rs` | Scaling criterion bench (group: `codec/decompress/batch_parallel`) | bench binary |
| `rust/crates/tinyquant-bench/benches/memory_compress_batch.rs` | dhat memory gates (peak RSS, alloc count) | bench binary |
| `rust/xtask/src/cmd/bench.rs` | `--capture-baseline`, `--check-against`, `--diff`, `--validate` | xtask |
| `scripts/calibration/gen_openai_sample.py` | Advisory fixture regen script | Python, advisory |
| `.github/workflows/rust-ci.yml` | `rust-ci/bench-budget` + `rust-ci/calibration` jobs | CI |

### Batch write recipe

The parallel batch path uses the `MaybeUninit` + `SyncPtr` pattern
from [[design/rust/parallelism|Parallelism]] §Batch GEMM. The
canonical sketch (production code should tighten drop discipline via
`PartialInit<T>` — see Step 3b):

```rust
use core::mem::MaybeUninit;
use core::ptr::NonNull;

/// SAFETY: SyncPtr wraps a NonNull raw pointer that workers use to
/// write into a `Vec<MaybeUninit<T>>` at non-overlapping indices.
/// Thread safety is upheld by the discipline that every worker `i`
/// writes exclusively to slot `i` and never reads from any slot.
struct SyncPtr<T>(NonNull<T>);
unsafe impl<T: Send> Send for SyncPtr<T> {}
unsafe impl<T: Sync> Sync for SyncPtr<T> {}

pub fn compress_batch_parallel(
    vectors: &[&[f32]],
    config: &CodecConfig,
    codebook: &Codebook,
    parallelism: Parallelism,
) -> Result<Vec<CompressedVector>, CodecError> {
    let mut out: Vec<MaybeUninit<CompressedVector>> =
        Vec::with_capacity(vectors.len());
    // SAFETY: set_len is sound because we either fully initialize
    // every slot before return, or drop any initialized slot on
    // the error path (see PartialInit below).
    unsafe { out.set_len(vectors.len()); }
    let out_ptr = SyncPtr(NonNull::new(out.as_mut_ptr()).unwrap());

    let first_error = std::sync::Mutex::new(None);

    parallelism.for_each_row(vectors.len(), |i| {
        if first_error.lock().unwrap().is_some() { return; }
        match Codec::new().compress(vectors[i], config, codebook) {
            Ok(cv) => unsafe {
                // SAFETY: each worker writes to a unique index i;
                // no worker reads from any slot, so there is no
                // data race. The SyncPtr is not aliased across
                // different slots.
                out_ptr.0.as_ptr().add(i).write(cv);
            },
            Err(e) => {
                let mut slot = first_error.lock().unwrap();
                if slot.is_none() { *slot = Some((i, e)); }
            }
        }
    });

    if let Some((_i, e)) = first_error.into_inner().unwrap() {
        // SAFETY: the drop discipline is handled by the PartialInit
        // helper in production code; this sketch leaks initialized
        // slots on the error path, which is unacceptable for
        // CompressedVector (owns heap allocations).
        // See Step 3b for the real implementation.
        unsafe { out.set_len(0); }
        return Err(e);
    }
    // SAFETY: every slot is initialized.
    let out: Vec<CompressedVector> = unsafe {
        let (ptr, len, cap) = (
            out.as_mut_ptr() as *mut CompressedVector,
            out.len(),
            out.capacity(),
        );
        core::mem::forget(out);
        Vec::from_raw_parts(ptr, len, cap)
    };
    Ok(out)
}
```

> [!warning] Drop discipline
> The sketch above **leaks** any `CompressedVector`s already written
> when the error path triggers. `CompressedVector` owns heap
> allocations (the packed code bytes), so this is a real leak, not
> just a style issue. Production code **must** use the
> `PartialInit<T>` helper (Step 3b) or an `AtomicBitmap` to track
> which slots were initialized and drop them before returning.

### Parallel error handling

We ship two modes:

| Mode | Signature | Semantics | Default? |
|------|-----------|-----------|----------|
| Fail-fast | `fn compress_batch(...) -> Result<Vec<CompressedVector>, CodecError>` | First error wins; later workers observe the flag via `Mutex<Option<...>>`, skip their own work, and partial results are dropped via `PartialInit`. | **Yes** |
| Collect-all | `fn compress_batch_collect(...) -> Vec<Result<CompressedVector, CodecError>>` | Every worker records its own result; the caller inspects each slot. Never drops partial results. | No |

Fail-fast is the default because it matches Python's exception
semantics (one `raise` unwinds the batch) and because our batch
sizes are small enough (≤ 100k vectors) that the wasted work on
error is bounded. Collect-all exists for the Phase 22 Python
binding where it lets PyO3 return a mixed-result `list`.

## Steps (TDD order)

### Part A — Rayon batch paths

- [ ] **Step 1: Failing parallel batch test**

```rust
// tests/batch_parallel.rs
#[test]
fn compress_batch_rayon_matches_serial() {
    // Build 256 random vectors with a fixed seed.
    // compress_batch(Parallelism::Serial) vs
    // compress_batch(Parallelism::Custom(rayon_parallelism())).
    // Assert byte-identical result per index.
}
```

- [ ] **Step 2: Implement `tinyquant-io::parallelism::rayon_parallelism()`**

```rust
#[cfg(feature = "rayon")]
pub fn rayon_parallelism() -> tinyquant_core::codec::Parallelism {
    tinyquant_core::codec::Parallelism::Custom(|count, body| {
        use rayon::prelude::*;
        (0..count).into_par_iter().for_each(|i| body(i));
    })
}
```

Tests that need a specific thread count build a local pool (see
**R21.6**): `ThreadPoolBuilder::new().num_threads(n).build().unwrap()`
and call `pool.install(|| rayon_parallelism())`.

- [ ] **Step 3: Consolidate batch API and wire parallel path**

Phase 15 shipped `Codec::compress_batch` (5-arg, serial) plus
`Codec::compress_batch_with` (6-arg, takes `Parallelism`) as an
interim split. The design docs
([[design/rust/parallelism|Parallelism]] and
[[design/rust/ffi-and-bindings|FFI and Bindings]]) both assume a
single 6-arg `compress_batch(..., parallelism)` signature. Phase 21
is the right time to consolidate:

- `Codec::compress_batch` takes `Parallelism` as its final
  argument (matching the design sketch in
  [[design/rust/parallelism|parallelism.md §The `Parallelism`
  type]]).
- `compress_batch_with` is removed; every in-tree caller is
  updated to pass `Parallelism::Serial` (or `rayon_parallelism()`
  when the caller lives in a `rayon`-enabled crate).
- The 5-arg convenience wrapper is gone — the argument is
  explicit, so no caller accidentally opts out of parallelism.

Land the parallel implementation in a new `batch.rs` module using
the `MaybeUninit` + `SyncPtr` recipe above, with a narrow
`#[allow(unsafe_code)]` on the impl block and a module-level
`// SAFETY:` doc comment covering the invariants listed in the
§Parallelism determinism contract.

- [ ] **Step 3b: `PartialInit<T>` helper for safe drop on error**

Create `codec/batch_error.rs`:

```rust
/// Owns a `Vec<MaybeUninit<T>>` plus an `AtomicBitmap` of
/// initialized slots. `Drop` walks the bitmap and drops every
/// initialized slot. The parallel batch path writes into a
/// `PartialInit<CompressedVector>` and calls `.into_vec()` on
/// success (which asserts full initialization).
pub(crate) struct PartialInit<T> { /* ... */ }
```

The parallel path now uses `PartialInit<CompressedVector>` instead
of a raw `Vec<MaybeUninit<_>>`. On error, `PartialInit::drop`
cleans up automatically — no leaks. See R21.1.

- [ ] **Step 4: Run parallel correctness test — expect pass.**

- [ ] **Step 5: Implement `compress_batch_packed`**

Zero-allocation variant returning `BatchedCompressedVectors` that
hand out `CompressedVectorView` borrows. The packed layout writes
all codes into a single contiguous buffer at known offsets, so the
parallel path writes into pre-sliced `&mut [u8]` segments and
doesn't need `PartialInit` at all.

- [ ] **Step 5b: Wire parallel path into `decompress_batch_into`**

[[design/rust/parallelism|Parallelism]] §Principles #2 requires
**both** `compress_batch` and `decompress_batch_into` to use rayon.
`decompress_batch_into` writes into `&mut [f32]` — the caller owns
the output buffer, so parallelism is simpler than the compress
path: no `MaybeUninit` / `SyncPtr` / `PartialInit` dance, just a
per-row `split_at_mut` over the output chunks.

```rust
pub fn decompress_batch_into(
    &self,
    compressed: &[CompressedVector],
    config: &CodecConfig,
    codebook: &Codebook,
    output: &mut [f32],
    parallelism: Parallelism,
) -> Result<(), CodecError> {
    let cols = config.dimension() as usize;
    // Pre-chunk the output so workers write disjoint slices.
    let chunks: Vec<&mut [f32]> = output.chunks_mut(cols).collect();
    let first_error = std::sync::Mutex::new(None);
    parallelism.for_each_row(compressed.len(), |i| {
        if first_error.lock().unwrap().is_some() { return; }
        // SAFETY argument: `chunks[i]` is disjoint per index.
        // The closure captures `chunks` by &mut; since rayon
        // borrows &mut [&mut [f32]] is not `Sync`, we instead
        // pass indexed ptr writes via the same SyncPtr trick.
        // See Step 5b note on Sync-shape.
        if let Err(e) = self.decompress_into(
            &compressed[i], config, codebook, chunks_ptr_at(i),
        ) {
            let mut slot = first_error.lock().unwrap();
            if slot.is_none() { *slot = Some(e); }
        }
    });
    if let Some(e) = first_error.into_inner().unwrap() {
        return Err(e);
    }
    Ok(())
}
```

The same determinism contract (byte-identical output regardless of
thread count) applies. Add a companion test
`tests/batch_determinism.rs::decompress_deterministic_across_threads`
that asserts `decompress_batch_into` produces byte-identical `f32`
slices across `[1, 2, 4, rayon::current_num_threads()]` threads.

The 5-arg `decompress_batch_into` is consolidated the same way
`compress_batch` is: parallelism becomes the sixth required
argument, no `_with` split.

- [ ] **Step 6: Batch bench scaling**

```rust
// benches/codec_compress_batch_parallel.rs
// Group name: "codec/compress/batch_parallel"
// (pinned so baseline keys are stable; matches benchmark-harness.md)
for &threads in &[1, 2, 4, 8, 16] {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    group.bench_with_input(
        BenchmarkId::from_parameter(threads),
        &threads,
        |b, _| {
            pool.install(|| {
                b.iter(|| {
                    let par = rayon_parallelism();
                    compress_batch(&vectors, &cfg, &cb, par)
                })
            });
        },
    );
}
```

- [ ] **Step 6b: Decompress batch bench scaling**

Same structure as Step 6 but for `decompress_batch_into`:

```rust
// benches/codec_decompress_batch_parallel.rs
// Group name: "codec/decompress/batch_parallel"
for &threads in &[1, 2, 4, 8, 16] {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    group.bench_with_input(
        BenchmarkId::from_parameter(threads),
        &threads,
        |b, _| {
            pool.install(|| {
                b.iter(|| {
                    let par = rayon_parallelism();
                    codec.decompress_batch_into(&cvs, &cfg, &cb, &mut out, par)
                })
            });
        },
    );
}
```

- [ ] **Step 6c: Memory benches (dhat)**

[[design/rust/benchmark-harness|Benchmark Harness]] §Memory
benchmarks specifies allocation-count and peak-heap gates that
Phase 21 must land alongside the throughput benches (without them,
the `compress_batch_packed` claim of "single contiguous buffer"
has no regression guard).

`tinyquant-bench/benches/memory_compress_batch.rs` uses
`dhat::DhatAlloc` as a per-bench `#[global_allocator]` and asserts
the following gates on the committed
`openai_10k_d1536.f32.bin` fixture:

| Benchmark | Peak heap | Max allocs |
| --- | --- | --- |
| `compress_batch_10000_bw4_res_on` | ≤ 250 MiB | ≤ 20 015 |
| `compress_batch_10000_bw4_res_on` (packed) | ≤ 220 MiB | ≤ 15 |
| `decompress_batch_10000_bw4_res_on` | ≤ 120 MiB | ≤ 20 005 |

These numbers are copied verbatim from benchmark-harness.md so the
two docs stay aligned; the packed-mode `≤ 15 allocs` is the
regression guard proving the `MaybeUninit` path avoids per-row
allocation. Failure of either gate blocks `rust-ci/bench-budget`.

### Part B — Calibration suite

- [ ] **Step 7: Failing score fidelity test**

Mirrors Python `tests/calibration/test_score_fidelity.py`:

```rust
// tests/calibration.rs
#[test]
#[ignore = "calibration; run with --ignored"]
fn pearson_rho_4bit_with_residuals_meets_threshold() {
    let corpus = GoldCorpus::load_openai_10k_d1536();
    // Sample 1000 pairs, compute original pairwise cosine and
    // reconstructed pairwise cosine, then Pearson correlation
    // via Welford's algorithm (see R21.3).
    let rho = compute_pearson_correlation(&corpus);
    // Threshold matches goals-and-non-goals.md §Fidelity goals
    // (VAL-01): Pearson ρ, 4-bit + residual >= 0.995.
    assert!(rho >= 0.995, "rho={rho}");
}
```

Similar tests for the threshold table below.

#### Calibration fixture generation

- **Source.** A 10,000-vector subsample of real OpenAI embeddings,
  committed at
  `rust/crates/tinyquant-bench/fixtures/calibration/openai_10k_d1536.f32.bin`
  (40 MiB, **LFS-tracked**).
- **Regen script.** Advisory only; humans regenerate, CI never
  regenerates:

  ```bash
  python scripts/calibration/gen_openai_sample.py \
      --seed 42 --count 10000 \
      --out rust/crates/tinyquant-bench/fixtures/calibration/
  ```

- **Drift detection.** SHA256 of every fixture file committed to
  `rust/crates/tinyquant-bench/fixtures/calibration/manifest.json`.
  The calibration test suite loads `manifest.json` at startup and
  asserts every fixture SHA matches before running any gate. A
  mismatch is a hard failure with a message pointing at the
  regeneration runbook.
- **xtask subcommand.** `cargo xtask fixtures refresh-calibration`
  runs the Python script **and** updates `manifest.json`. Advisory,
  does not run in CI (fixture regen requires a human because new
  OpenAI samples can invalidate the thresholds below).
- **LFS hydration (Phase 14 Lesson L2).** The fixtures are ≥ 1 MiB
  each. Every CI job that reads them MUST carry
  `with: { lfs: true }` in its checkout step. Phase 14 L2 is the
  reason we have this rule written down: it took two days to
  diagnose a "fixture missing" red CI after a hydration flag was
  dropped from a refactored workflow.
- **PR-speed alternative.** `openai_1k_d768.f32.bin` (1/10 the
  vectors, half the dimension, ~1.5 MiB) runs on every PR. Main
  branch runs the full `openai_10k_d1536` fixture.

#### Calibration thresholds

Thresholds are asserted `>=` (not `==`) so ISA-level numerical
nondeterminism (Phase 14 L4) does not flake the gates. Bit-exact
gates live in the fixture parity tests from Phase 16.

All thresholds below are sourced from
[[design/rust/goals-and-non-goals|goals-and-non-goals.md §Fidelity
goals]] (VAL-01 for ρ, behavior-layer for recall, VAL-02 for
ratio). Plan-only deviations are **tighter** thresholds than the
goals doc; they are called out in the `Source` column. Any
threshold that is tighter than the goals doc is either (a) matched
by a commit that raises the goals doc in the same PR, or (b)
footnoted as an aspirational calibration floor not binding the
parity gate.

> [!warning] Re-baselined 2026-04-14 (R22, interim)
> The threshold table below was re-baselined on 2026-04-14 against
> honest measurements of the codec as it actually ships. The
> previous "aspirational" values for residual-on ratios (4/7/14×)
> are structurally unreachable — the codec currently ships raw
> `f16` residuals, so ratio is capped at `4 / (bw/8 + 2)` for both
> the Rust codec and the Python reference. Promoting those numbers
> back toward 4/7/14× is tracked as Phase 26 residual compression
> (see `docs/plans/rust/calibration-threshold-investigation.md`
> §5 B2).

| Config | ρ lower bound | Top-10 recall | Ratio lower bound | Source | Mirrored Python test |
| --- | --- | --- | --- | --- | --- |
| bw=4, residual=on | ≥ 0.99 | ≥ 0.95 | ≥ 1.50 | measured 2026-04-14 on `openai_1k_d768` (rho=1.000, recall=1.000, ratio=1.600); TODO(phase-26) raise ratio toward 7.0 | `test_pearson_4bit_residual` |
| bw=4, residual=off | ≥ 0.95 | ≥ 0.75 | ≥ 7.50 | measured (rho=0.9573, recall=0.7910, ratio=8.000); rho/recall are scalar-quantizer-inherent ceilings, not targets | `test_pearson_4bit_no_residual` |
| bw=2, residual=on | ≥ 0.99 | ≥ 0.95 | ≥ 1.70 | measured (rho=1.000, recall=1.000, ratio=1.7778); TODO(phase-26) raise ratio toward 14.0 | `test_pearson_2bit_residual` |
| bw=2, residual=off | ≥ 0.50 | ≥ 0.30 | ≥ 15.00 | measured (rho=0.5119, recall=0.3530, ratio=16.000); regression-canary only — 2-bit scalar quantization of 768-dim unit vectors cannot preserve semantic structure without the residual path | `test_pearson_2bit_no_residual` (new) |
| bw=8, residual=on | ≥ 0.99 | ≥ 0.95 | ≥ 1.25 | measured (rho=1.000, recall=1.000, ratio=1.3333); TODO(phase-26) raise ratio toward 4.0 | `test_pearson_8bit_residual` |
| Passthrough | `== 1.0` exact | `== 100%` exact | `== 1.0` exact | VAL-03 round-trip determinism | `test_passthrough_identity` |
| Fp16 | ≥ 0.9999 | n/a | `== 2.0` exact | plan-only (aspirational) | `test_fp16_halfprecision` |

**Calibration vs parity gate.** These thresholds are the
**calibration** floor — evaluated `>=` to tolerate ISA-level
numerical drift (Phase 14 Lesson L4). The bit-exact **parity**
gate from Phase 16
(`Rust::compress(...).to_bytes() == Python::compress(...).to_bytes()`)
is the hard fidelity contract; calibration is the numerical safety
net.

> [!warning] Parallel Python deliverable
> At the time of writing, `tests/calibration/test_pearson_2bit_residual`
> and `test_pearson_8bit_residual` do **not** exist on the Python
> side. They are flagged as a parallel Phase 21 deliverable — the
> Python test file is a prerequisite for merging the Rust calibration
> table, so the Python and Rust thresholds are authored from the same
> fixture and metric implementation. Open issue: #TBD.

- [ ] **Step 8: Implement helpers**

`compute_pearson_correlation` and `compute_top_k_recall` live in
`tinyquant-bench::calibration` and are shared between the
ignored tests and the benchmarks that capture baseline metrics.
Pearson uses **Welford's algorithm** to avoid the numerical issues
of the two-pass formula on n=1000 pairs (see R21.3).

- [ ] **Step 9: Compression ratio gate**

```rust
#[test]
#[ignore = "calibration"]
fn compression_ratio_4bit_meets_threshold() {
    let corpus = GoldCorpus::load_openai_10k_d1536();
    let total_raw = corpus.vectors.len() * 1536 * 4;
    let codec = Codec::new();
    // Consolidated 6-arg signature from Step 3 — parallelism is
    // now a required argument on compress_batch.
    let cvs = codec.compress_batch(
        &corpus.vectors, 10_000, 1536,
        &corpus.config, &corpus.codebook,
        Parallelism::Serial,
    ).unwrap();
    let total_compressed: usize = cvs.iter().map(|cv| cv.size_bytes()).sum();
    let ratio = total_raw as f64 / total_compressed as f64;
    // VAL-02: 4-bit + residual >= 5.0 (goals-and-non-goals.md).
    assert!(ratio >= 5.0, "ratio={ratio}");
}
```

- [ ] **Step 10: Determinism gate** (not `#[ignore]` — runs every PR)

```rust
// tests/batch_determinism.rs
#[test]
fn full_pipeline_deterministic_across_threads_and_runs() {
    let seed = 0xDEADBEEF;
    let thread_counts = [1usize, 2, 4, rayon::current_num_threads()];
    let mut runs: Vec<Vec<CompressedVector>> = Vec::new();

    // 10 repeated runs under the default thread count.
    for _ in 0..10 {
        let out = full_pipeline(seed, num_cpus::get());
        if let Some(prev) = runs.last() {
            assert_eq!(prev, &out, "repeat-run divergence");
        }
        runs.push(out);
    }

    // Thread-count sweep.
    let reference = full_pipeline(seed, thread_counts[0]);
    for &t in &thread_counts[1..] {
        let out = full_pipeline(seed, t);
        assert_eq!(reference, out, "thread-count divergence at t={t}");
    }
}
```

`full_pipeline` runs `train → compress_batch → decompress_batch`.
Every step of the pipeline is covered by the determinism assertion.
This test is **not** `#[ignore]`: it runs on every PR and is
considered a hard phase-exit gate.

### Part C — Benchmark budget gate

- [ ] **Step 11: Implement xtask bench commands**

#### xtask bench commands

| Command | Behavior |
|---------|----------|
| `cargo xtask bench --capture-baseline main` | Run criterion, parse `target/criterion/*/new/estimates.json`, write `baselines/main.json` with the current git commit, host info, and rustc version. |
| `cargo xtask bench --check-against main` | Run criterion, load `baselines/main.json`, compute per-group `new_median / baseline_median`, exit non-zero if any exceeds `budget_pct / 100`, print a human-readable table. |
| `cargo xtask bench --diff HEAD~1 HEAD` | Ad-hoc diff: captures two baselines at `HEAD~1` and `HEAD` and prints the delta. Does not write any committed file. |
| `cargo xtask bench --validate` | Assert `baselines/main.json` parses against `baselines/schema.json`. Run as part of `cargo xtask ci`. |

#### Bench budget schema

`baselines/schema.json` is a JSON-Schema draft-07 document. The
committed `baselines/main.json` matches:

```json
{
  "schema_version": 1,
  "captured_at": "2026-04-10T00:00:00Z",
  "git_commit": "abcdef123",
  "host": {
    "os": "linux",
    "arch": "x86_64",
    "cpu_model": "Intel(R) Xeon(R) Platinum 8370C",
    "rustc": "1.81.0"
  },
  "bench_groups": {
    "quantize_4bit/dim_1536": {
      "median_ns": 12345,
      "p99_ns": 14000,
      "budget_pct": 110
    },
    "dequantize_4bit/dim_1536": {
      "median_ns": 8000,
      "p99_ns": 9500,
      "budget_pct": 110
    },
    "batch_parallel/threads_8/dim_1536_n_10000": {
      "median_ns": 4200000,
      "p99_ns": 5000000,
      "budget_pct": 115
    }
  }
}
```

**Budget semantics.** A run fails when
`new_median > baseline_median * (budget_pct / 100)`. The default
`budget_pct` is **110** (10% regression allowed). Above the budget:

- **On `main` push.** Blocks merge. The job is a hard CI gate.
- **On PR.** Advisory — comments on the PR but does not block. This
  lets reviewers see a regression without breaking unrelated PRs.

A regression above 110% that is explainable (e.g. a new feature,
newly-enabled debug assertions) is resolved by adjusting the
baseline in a dedicated commit:

```text
chore(bench): raise quantize_4bit/dim_1536 budget to 115% after adding range checks
```

- [ ] **Step 12: Capture initial baseline**

```bash
cargo bench -p tinyquant-bench
cargo xtask bench --capture-baseline main
```

Commit **only** the CI-on-main capture (see R21.8). The developer
should run this on a CI runner via `act` or on a fresh GitHub
Actions job; do not commit a baseline captured on a developer
laptop, because laptop CPUs are noisier and the numbers will
immediately flake CI.

- [ ] **Step 13: Update `rust-ci.yml`**

See the `## CI integration` section below. Adds two jobs:
`rust-ci/bench-budget` and `rust-ci/calibration`.

- [ ] **Step 14: Run all calibration tests**

```bash
cargo test --workspace --all-features -- --ignored
```

Confirm every calibration gate passes on the committed fixtures.

- [ ] **Step 15: Run standard tests (not ignored)**

```bash
cargo test --workspace --all-features
```

Expect no regressions, and in particular expect
`batch_determinism::full_pipeline_deterministic_across_threads_and_runs`
to pass on every target in the CI matrix.

- [ ] **Step 16: Commit**

```bash
git add rust/crates/tinyquant-core/src/codec/batch.rs \
        rust/crates/tinyquant-core/src/codec/batch_error.rs \
        rust/crates/tinyquant-core/tests/batch_parallel.rs \
        rust/crates/tinyquant-core/tests/batch_determinism.rs \
        rust/crates/tinyquant-io/src/parallelism.rs \
        rust/crates/tinyquant-bench \
        rust/xtask .github/workflows/rust-ci.yml \
        scripts/calibration/gen_openai_sample.py
git commit -m "feat(rust): add rayon batch paths, calibration gates, and bench budgets"
```

### Clippy profile gotchas

Ported from Phase 14; these lints bite during Phase 21 work:

- **`unsafe_code`.** Required in `batch.rs` for the `MaybeUninit`
  + `SyncPtr` pattern. Wrap in a narrow `#[allow(unsafe_code)]` on
  the impl block, paired with a module-level `// SAFETY:` doc block
  that cites the determinism contract.
- **`cast_precision_loss`.** Pearson denominators want
  `len() as f32` — narrow `#[allow(clippy::cast_precision_loss)]`
  on the function, documented as "n ≤ 1000 pairs, well within f32
  integer precision (2^24)".
- **`indexing_slicing`.** Parallel iterator indices look like raw
  slice access. Use `.enumerate()` + a bounds check inside the
  closure, or prefer the packed path (which slices up-front).
- **`missing_safety_doc`.** Every `unsafe fn` needs a doc comment.
  `PartialInit<T>` in particular has three `unsafe fn`s
  (`set_init`, `into_vec`, `drop_initialized`) — each needs its own
  safety contract.
- **`cognitive_complexity`.** The calibration helper that iterates
  over the threshold table will trip this lint. Split into small
  private fns per threshold row.

## Phase 18 deferral: `IndexMap`

[[design/rust/phase-18-implementation-notes|Phase 18 Implementation
Notes]] D18.2 deferred `IndexMap` integration "to Phase 21 per the
plan's own recommendation." The Phase 21 scope (rayon batch paths,
calibration, bench budgets) does **not** benefit from switching the
corpus's `VectorIdMap` to `IndexMap`: the parallel batch path writes
through a raw `Vec<MaybeUninit<_>>` and never touches the
insertion-order map, and the calibration / bench suites read the
corpus sequentially.

**Decision: re-defer `IndexMap` to Phase 22 or later.** The
`VectorIdMap` (pure `no_std`, `BTreeMap` + `Vec` slab) already meets
Phase 18's functional contract, and adopting `IndexMap` would
introduce an external dependency whose default hasher
(`RandomState`) requires `std` — incompatible with the
`tinyquant-core` `no_std` charter. If a future phase needs
O(1) lookup by string key in a `std`-enabled context, revisit under
that phase's explicit plan. The Phase 18 D18.2 note should be
updated to point at this re-deferral rather than leaving a stale
"deferred to Phase 21" claim.

## Rayon pool contention with downstream consumers (R8)

[[design/rust/risks-and-mitigations|Risks and Mitigations]] §R8
flags that when downstream consumers (better-router Rust agents,
other `rayon`-using services) install their own
`rayon::ThreadPoolBuilder`, our batch operations can fight over the
global pool. Phase 21 honors this by **never constructing a pool
inside the library**: `rayon_parallelism()` uses whatever pool the
caller already installed (`rayon::current_num_threads()` sees the
caller's pool inside a `pool.install(...)` scope). The library
ships no `OnceCell<ThreadPool>` — pool ownership belongs to the
caller, exactly as [[design/rust/parallelism|parallelism.md §Thread
pool configuration defaults]] prescribes.

The integration documentation added in Phase 21 (a new §"Bringing
your own rayon pool" block in
[[design/rust/parallelism|parallelism.md]]) tells downstream
consumers to wrap batch calls in their own `pool.install(...)`
scope when they have one, so TinyQuant inherits rather than
contends. This closes R8 for the CPU-bound codec path.

## Acceptance criteria

- Parallel batch compress **and decompress** match serial
  byte-for-byte on the thread-count sweep test (Step 10).
- Pearson ρ, top-k recall, compression ratio, and determinism gates
  all green on the committed LFS fixture.
- Memory benches (Step 6c) meet the dhat gates from
  benchmark-harness.md §Memory benchmarks, in particular
  `compress_batch_packed` ≤ 15 allocations on a 10 000-vector
  batch.
- `xtask bench --check-against main` fails on a synthetic
  regression (covered by an xtask unit test) and passes on clean
  main.
- Every performance goal from
  [[design/rust/goals-and-non-goals|Goals and Non-Goals]] is either
  met or has an open risk entry in §Risks.
- Clippy + fmt clean, including the narrow `#[allow]` blocks called
  out in §Clippy profile gotchas.
- Both `rust-ci/bench-budget` and `rust-ci/calibration` green on
  `main` (Phase 14 L1: phase exit is gated on green CI, not "green
  locally").

## CI integration

Two new jobs land in `.github/workflows/rust-ci.yml`:

### `rust-ci/bench-budget`

- **Triggers.** `main` pushes; PRs that touch
  `rust/crates/tinyquant-core/**`, `rust/crates/tinyquant-io/**`,
  `rust/crates/tinyquant-bench/**`, or `rust/xtask/**`.
- **Command.** `cargo xtask bench --check-against main`.
- **LFS hydration.** `with: { lfs: true }` on `actions/checkout`
  — the bench suite reads the calibration fixtures for some
  benchmark inputs. **(Phase 14 Lesson L2.)**
- **Mode.** Advisory on PRs; blocking on main.
- **Runner.** `ubuntu-latest-8-cores` for stable median noise.

### `rust-ci/calibration`

- **Triggers.** PRs and `main` pushes, same as the main test job.
- **Command.** `cargo test --workspace --all-features -- --ignored`.
- **LFS hydration.** `with: { lfs: true }` — the calibration
  fixtures are LFS-tracked; without hydration the manifest SHA
  check fails immediately. **(Phase 14 Lesson L2.)**
- **Runner matrix.** Runs on at least **two distinct host CPUs**
  so that ISA-level numerical divergence (Phase 14 L4) is caught
  rather than hidden behind a single-runner baseline. Example:

  ```yaml
  strategy:
    matrix:
      runner: [ubuntu-latest, ubuntu-latest-arm64]
  ```

  The calibration thresholds are evaluated as `>=` (not `==`), so
  ISA nondeterminism does not flake them; the parity tests from
  Phase 16 are the byte-level gate, calibration is the numerical
  gate. **(Phase 14 Lesson L4.)**
- **Mode.** Blocking on main, advisory on PRs that only touch
  `docs/`.

### Design-doc-vs-CI drift check

Per **Phase 14 Lesson L3**, every time a CI job or a design doc
changes, the other side drifts silently unless we guard against it.
Phase 21 adds an xtask subcommand:

```bash
cargo xtask docs check-ci-parity
```

Which greps `docs/design/rust/ci-cd.md` for the bench and
calibration job names and asserts they appear in `rust-ci.yml`. The
check runs as part of `cargo xtask ci` and blocks merge if a job
rename lands in one place but not the other.

### CI health check

**Phase 14 Lesson L1:** Phase 21 does not exit until both
`rust-ci/bench-budget` and `rust-ci/calibration` have been green on
`main` for at least one push after all Phase 21 commits have
landed. This catches the "works locally, fails in CI matrix" class
of bug that bit Phase 14 twice.

## Risks

| ID | Risk | Mitigation |
|----|------|------------|
| R21.1 | Parallel batch drop discipline leaks `CompressedVector` allocations on error | `PartialInit<T>` helper (Step 3b); loom stress test on a 4-vector batch under `--features loom` |
| R21.2 | Calibration fixture drift invalidates thresholds, tests silently pass on wrong data | `manifest.json` SHA256 check loaded at test startup; CI fails hard on mismatch |
| R21.3 | Pearson numerical stability on n=1000 pairs using the two-pass formula | Use Welford's online algorithm in `calibration::pearson` |
| R21.4 | Bench budget flakes under CPU contention on shared runners | Median-only comparison + 3 back-to-back runs averaged + default 10% budget + `--warm-up-time 5 --measurement-time 20` |
| R21.5 | Ignored tests never actually run, calibration rots | Dedicated `rust-ci/calibration` job with `-- --ignored` on every PR and main push |
| R21.6 | Rayon global thread pool state interacts with other tests in the same binary | Every determinism test builds a local `ThreadPoolBuilder::new().build().unwrap()` and runs via `pool.install(...)` |
| R21.7 | Cross-runner SIMD flake in calibration (Lesson L4 repeat) | Thresholds are `>=` not `==`; parity tests are the byte-level gate, calibration is the numerical gate |
| R21.8 | Baseline drift between dev-laptop captures and CI captures | **Only commit baselines captured on a CI runner.** Baselines captured locally are for diagnosis, never committed. `xtask bench --capture-baseline` emits a warning when it detects it is not running under GitHub Actions. |
| R21.9 | Rayon pool contention with downstream consumers (re-cites R8 from risks-and-mitigations.md) | Library never builds or owns a rayon pool; `rayon_parallelism()` inherits the caller's `pool.install(...)` scope. Documentation in parallelism.md §Bringing your own rayon pool tells consumers how to install their pool once and have TinyQuant follow. See §"Rayon pool contention with downstream consumers (R8)" above. |

## Out of scope

- Python bindings — Phase 22.
- C ABI — Phase 22.
- Release publishing — Phase 22.
- Multi-process batching — single process only. Cross-process
  coordination is not needed for embedding codec workloads and is
  explicitly excluded from the Parallelism contract.
- GPU acceleration — no CUDA, no Metal, no Vulkan. The charter is
  CPU-only.
- Adaptive chunk-size tuning — the rayon work-stealing scheduler is
  good enough; we do not add a per-host tuner.

## See also

- [[plans/rust/phase-20-simd-kernels|Phase 20]]
- [[plans/rust/phase-22-pyo3-cabi-release|Phase 22]]
- [[design/rust/parallelism|Parallelism]]
- [[design/rust/benchmark-harness|Benchmark Harness]]
- [[design/rust/testing-strategy|Testing Strategy]]
- [[design/rust/ci-cd|CI/CD]]
- [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]]
