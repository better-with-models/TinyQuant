# rust/crates/tinyquant-bench

`tinyquant-bench` is the Criterion benchmark harness for TinyQuant. It also
provides calibration helpers — neighbor-recall and Pearson ρ — that act as
quality gates in both the bench suite and the `tests/` directory. Benchmark
modules and fixture loading infrastructure are being populated in Phase 21
alongside the real codec implementation.

## What lives here

| Path | Role |
|---|---|
| `src/` | Library crate root (`lib.rs`) and `calibration/` sub-module |
| `benches/` | Criterion entry points: `batch_parallel.rs`, `simd_kernels.rs`, `smoke.rs` |
| `tests/` | Quality-gate integration tests: `calibration.rs`, `smoke.rs` |

## How this area fits the system

The calibration helpers in `src/calibration/` are consumed by both the Criterion
benchmarks and the integration tests. Benchmarks exercise `BruteForceBackend`
and (when added) future backends; the calibration tests enforce minimum recall
and correlation thresholds that must pass in CI.

## Common edit paths

- Adding or adjusting quality thresholds: `src/calibration/neighbor_recall.rs`, `src/calibration/pearson.rs`
- New benchmark workloads: `benches/batch_parallel.rs` or `benches/simd_kernels.rs`
- CI quality-gate tests: `tests/calibration.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
