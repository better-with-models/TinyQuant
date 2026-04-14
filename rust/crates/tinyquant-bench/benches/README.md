# rust/crates/tinyquant-bench/benches

Criterion benchmark entry points for `tinyquant-bench`. Each file is a
separate Criterion benchmark binary declared in `Cargo.toml`. Run with
`cargo bench`.

## What lives here

| File | Role |
|---|---|
| `batch_parallel.rs` | Throughput benchmarks for batched and Rayon-parallel search |
| `simd_kernels.rs` | Micro-benchmarks for individual SIMD cosine-similarity kernels |
| `smoke.rs` | Minimal benchmark confirming the harness compiles and runs |

## How this area fits the system

These files pull in the `tinyquant-bench` library crate (for calibration
helpers) and depend directly on `tinyquant-bruteforce` and `tinyquant-core`
for the backends under test. Criterion writes HTML reports to
`target/criterion/`; no output files are committed to the repo.

## Common edit paths

- Batch/parallel throughput work: `batch_parallel.rs`
- SIMD kernel iteration: `simd_kernels.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
