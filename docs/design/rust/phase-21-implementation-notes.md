---
title: "Phase 21 Implementation Notes"
tags:
  - design
  - rust
  - phase-21
  - performance
  - benchmarks
  - calibration
date-created: 2026-04-12
category: design
---

# Phase 21 Implementation Notes

## What landed

Phase 21 delivers **Rayon parallel batch compression**, **calibration quality
gates**, and the **`cargo xtask bench`** subcommand. The phase has three
largely independent delivery tracks that were merged together.

### Track A — Parallel batch path

- `rust/crates/tinyquant-core/src/codec/batch.rs` — `compress_batch_parallel`
  using `MaybeUninit<CompressedVector>` output slots and an `AtomicPtr` write
  fence. Each Rayon worker writes into its own pre-allocated slot; the main
  thread collects after the worker pool drains. No mutex on the hot path.
- `rust/crates/tinyquant-core/src/codec/batch_error.rs` — `PartialInit<T>`:
  a safe wrapper around `ManuallyDrop<T>` + `AtomicBool` initialization flags.
  Provides guaranteed cleanup (calls `drop` on every initialized slot) if the
  batch is abandoned mid-run due to an error.
- `rust/crates/tinyquant-core/src/codec/parallelism.rs` — `Parallelism` enum
  (`Serial | Custom(fn(usize, &(dyn Fn(usize) + Sync + Send))`), first
  introduced in Phase 15 as a stub. Phase 21 adds the `Custom` variant's
  rayon wiring.
- `rust/crates/tinyquant-io/src/parallelism.rs` — `rayon_parallelism() ->
  Parallelism` convenience constructor that returns a `Custom` driver backed
  by Rayon's global thread pool.
- `tinyquant-core/tests/batch_determinism.rs` — asserts byte-identical output
  across `Serial` and `Custom(rayon)` with thread counts `[1, 2, 4, 8, N_CPU]`.
- `tinyquant-core/tests/batch_parallel.rs` — error path and partial-success
  tests for `PartialInit` cleanup behaviour.

### Track B — Calibration quality gates

- `rust/crates/tinyquant-bench/src/calibration/pearson.rs` — `PearsonOnline`
  struct implementing Welford's online algorithm for streaming correlation
  coefficient computation. Avoids numerical cancellation that plagues the
  naive two-pass formula.
- `rust/crates/tinyquant-bench/src/calibration/neighbor_recall.rs` —
  `mean_recall_at_k`: computes pairwise cosine neighbors of the gold corpus,
  then measures recall of TinyQuant's compressed-space neighbor lists.
- `rust/crates/tinyquant-bench/src/calibration/mod.rs` — `GoldCorpus` LFS
  loader with SHA-256 drift detection. Two fixtures committed to LFS:
  `openai_10k_d1536.f32.bin` (10 000 × 1536) and `openai_1k_d768.f32.bin`
  (1 000 × 768).
- `rust/crates/tinyquant-bench/tests/calibration.rs` — four threshold suites:
  - bw4 + residual: ρ ≥ 0.998, recall ≥ 0.95, ratio ≥ 7.0
  - bw4 − residual: ρ ≥ 0.98, recall ≥ 0.85, ratio ≥ 8.0
  - bw2 + residual: ρ ≥ 0.95, recall ≥ 0.80, ratio ≥ 14.0
  - bw8 + residual: ρ ≥ 0.999, recall ≥ 0.98, ratio ≥ 4.0

  All tests are `#[ignore]`-gated (require the gold corpus fixture) and run
  only in the dedicated CI `calibration` job.

### Track C — xtask bench

- `rust/xtask/src/cmd/bench.rs` — three subcommands:
  - `capture-baseline`: runs Criterion, writes `baselines/{name}.json` with
    per-group mean latency, `budget_pct` (default 110), host `cpu_model`, and
    `git_commit`.
  - `check-against`: compares current Criterion run against a committed
    baseline; fails the job if any group regresses beyond `budget_pct`.
  - `diff <from> <to>`: prints a per-group Δ% table by comparing two committed
    baseline JSON files. No Criterion run needed.
- `rust/crates/tinyquant-bench/baselines/main.json` — initial baseline at
  commit `156ffae`; four batch-parallel groups.
- `rust/crates/tinyquant-bench/baselines/schema.json` — JSON schema for
  baseline files.
- CI: `bench-budget` job runs `xtask bench --check-against main` on every PR
  to `develop` (advisory) and on every push to `main` (hard gate with
  `continue-on-error: false`).
- CI: `calibration` job runs `cargo test --workspace --ignored`; a second
  `calibration-arm64` job runs on `ubuntu-24.04-arm` with
  `continue-on-error: true` for ISA coverage.
- `scripts/calibration/gen_openai_sample.py` — helper to regenerate the LFS
  calibration fixtures from a local OpenAI embedding cache.

## Deviations from the plan

### 1. `PartialInit` uses `AtomicBool`, not `ManuallyDrop` alone

The plan sketched `MaybeUninit` with a plain `bool` flag per slot. The
implementation uses `AtomicBool` because Rayon workers touch the flag from
multiple threads (writer sets `true`; the main-thread collector reads it
during cleanup). Plain `bool` would be UB. The `Relaxed` ordering is sufficient
because the flag is only read after the Rayon scope has drained (the scope
join provides the happens-before edge).

### 2. `rows * cols` overflow guard

A reviewer found that the `rows * cols` product in `compress_batch_with` could
overflow on contrived inputs (e.g., `rows = usize::MAX / 2 + 1`). The fix added
a `checked_mul` guard that returns `BatchError::DimensionOverflow` before any
allocation.

### 3. `Parallelism::Custom` documents fn-pointer limitation

Rayon thread pools are constructed from closures, but the `Parallelism` enum
needs to be `Send + Sync` to cross the batch API boundary. Using
`fn() -> ThreadPool` (a function pointer, not a closure) satisfies that
constraint but prevents capturing state. The limitation is documented in the
module docstring because callers who need a pre-configured pool should
construct it outside and pass it via `Arc<Mutex<ThreadPool>>` in a future
extension.

### 4. Calibration thresholds from empirical baseline runs

The plan specified placeholder threshold values. During implementation the
actual values were derived from three independent calibration runs against the
`openai_10k_d1536` fixture (described in commit `59af898`). The bw4+residual
ρ ≥ 0.998 threshold in particular had to be tightened from an initial 0.995
to match the floor seen in all three runs.

### 5. `budget_pct` default changed from 115 to 110

The plan listed a 15% regression budget. Review feedback pointed out that the
TinyQuant codec is deterministic (same input always produces the same byte
output), so runtime variance in the benchmark comes entirely from system noise.
A 10% budget captures genuine regressions while tolerating realistic CI noise
on shared runners.

### 6. Calibration matrix removed from rust-ci.yml

The `calibration-matrix` job that appeared during early phase-21 work was a
temporary workaround for exploring threshold parameters across multiple
configurations. It was removed in commit `a8a3da3` (merged via `431fa8d`)
before the phase landed on `main`, replaced by the two focused jobs
(`calibration` + `calibration-arm64`).

## Determinism contract

The batch path is deterministic by construction: Rayon workers write into
pre-allocated non-overlapping slots; the output order is the input order. The
`batch_determinism.rs` test verifies this for thread counts up to `n_cpu`.
The determinism guarantee is identical to the `Serial` path because
`compress_batch_parallel` compresses each vector independently (no
inter-vector state beyond the shared `Codebook` and `CodecConfig`, both of
which are read-only during the batch).

## Risks carried forward

- **R-BATCH-1**: The `PartialInit` cleanup path drops initialized slots on
  error. If `CompressedVector::drop` panics, the second panic during unwinding
  will abort the process. This is acceptable: `CompressedVector` has no
  destructor side effects in the current implementation.
- **R-BATCH-2**: The calibration fixtures are LFS objects. On runners that
  lack LFS credentials (e.g., fork PRs) the `calibration` job skips gracefully
  because `GoldCorpus::load` returns `Err` when the file is an LFS pointer.
  The `#[ignore]` gate is the primary mechanism; the LFS skip is a secondary
  defence.
- **R-BATCH-3**: `cpu_model()` reads `/proc/cpuinfo` on Linux, `sysctl` on
  macOS, and `wmic` on Windows. If none of these succeed (unusual container
  environments) the baseline `host.cpu_model` field is set to `"unknown"`.
  The `check-against` comparison still runs; the CPU model is informational
  only.
