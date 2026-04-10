---
title: Rust Port — Benchmark Harness and Performance Budgets
tags:
  - design
  - rust
  - criterion
  - performance
  - benchmarks
date-created: 2026-04-10
status: draft
category: design
---

# Rust Port — Benchmark Harness and Performance Budgets

> [!info] Purpose
> Turn the goals in [[design/rust/goals-and-non-goals|Goals and
> Non-Goals]] into a measurable, reproducible benchmark regime with
> CI regression gates.

## Harness: `tinyquant-bench`

Criterion is the canonical benchmark framework. It gives us:

- Nanosecond-resolution wall-clock measurement
- Statistical analysis (median, MAD, outlier filtering)
- HTML reports with regression flagging
- JSON dump for machine-readable regression detection

### Fixtures and determinism

All benchmarks load the same deterministic fixtures via
`tinyquant-bench::fixtures`:

```rust
// tinyquant-bench/src/fixtures.rs
pub struct GoldCorpus {
    pub dim: usize,
    pub count: usize,
    pub vectors: Vec<f32>,     // row-major, length = dim * count
    pub config: CodecConfig,
    pub codebook: Codebook,
}

impl GoldCorpus {
    pub fn load(dim: usize, count: usize, bit_width: u8) -> Self {
        let path = format!(
            "tests/fixtures/gold_{dim}_{count}_bw{bit_width}.bin"
        );
        // Loads f32 LE from disk. File is produced by xtask fixtures build.
        // If missing, synthesizes deterministically from seed 42.
        load_or_synthesize(&path, dim, count, bit_width)
    }
}
```

Fixture files are stored in Git LFS and checked into
`rust/crates/tinyquant-bench/tests/fixtures/`. The `xtask fixtures
build` command regenerates them from the Python gold corpus, so
benchmarks always compare against the same data.

### Benchmark modules

```text
tinyquant-bench/benches/
├── codec_compress_single.rs
├── codec_compress_batch.rs
├── codec_decompress_single.rs
├── codec_decompress_batch.rs
├── codec_compress_batch_parallel.rs
├── rotation_build_cold.rs
├── rotation_build_warm.rs
├── codebook_train.rs
├── serialization_roundtrip.rs
├── bit_pack_unpack.rs
├── zero_copy_view_iteration.rs
├── brute_force_search.rs
└── parity_vs_python.rs        # Not run in CI; runs against a live pyo3 process
```

### One benchmark file shape (representative)

```rust
// tinyquant-bench/benches/codec_compress_single.rs
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main, Throughput};
use tinyquant_bench::fixtures::GoldCorpus;
use tinyquant_core::codec::Codec;

fn bench_compress_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec/compress/single");
    group.sample_size(200);

    for &(dim, bit_width) in &[(768u32, 4u8), (1536u32, 4u8), (1536u32, 8u8), (768u32, 2u8)] {
        let corpus = GoldCorpus::load(dim as usize, 1024, bit_width);
        let vector = &corpus.vectors[..dim as usize];
        let codec = Codec::new();

        group.throughput(Throughput::Bytes((dim * 4) as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("d{dim}_bw{bit_width}")),
            &(),
            |b, _| {
                b.iter(|| {
                    codec.compress(
                        criterion::black_box(vector),
                        criterion::black_box(&corpus.config),
                        criterion::black_box(&corpus.codebook),
                    ).unwrap()
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_compress_single);
criterion_main!(benches);
```

### Parallelism bench structure

```rust
// tinyquant-bench/benches/codec_compress_batch_parallel.rs
fn bench_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec/compress/batch_parallel");
    group.sample_size(30);

    let corpus = GoldCorpus::load(1536, 10_000, 4);
    let codec = Codec::new();

    for &threads in &[1usize, 2, 4, 8, 16] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        group.bench_with_input(
            BenchmarkId::from_parameter(threads),
            &threads,
            |b, _| {
                b.iter(|| {
                    pool.install(|| {
                        codec.compress_batch(
                            &corpus.vectors,
                            10_000,
                            1536,
                            &corpus.config,
                            &corpus.codebook,
                            tinyquant_io::rayon_parallelism(),
                        ).unwrap()
                    })
                });
            },
        );
    }
}
```

Scaling charts from this benchmark tell us when parallelism stops
helping (typically around 8-12 threads on the rotate stage,
depending on cache).

## Performance budgets (tied to CI gates)

Each bench has a budget threshold. The `xtask bench` command runs
criterion, parses the JSON output, and compares to a baseline.

### Baseline file

```json
// rust/crates/tinyquant-bench/baselines/main.json
{
  "generated_at": "2026-04-10T00:00:00Z",
  "host": "ci-x86_64-avx2",
  "rust_version": "1.78.0",
  "measurements": {
    "codec/compress/single/d1536_bw4": { "median_ns": 18000, "budget_ns": 20000 },
    "codec/compress/single/d1536_bw8": { "median_ns": 22000, "budget_ns": 25000 },
    "codec/decompress/single/d1536_bw4": { "median_ns": 9000, "budget_ns": 10000 },
    "codec/compress/batch/n10000_d1536_bw4": { "median_ns": 80_000_000, "budget_ns": 120_000_000 },
    "codec/decompress/batch/n10000_d1536_bw4": { "median_ns": 40_000_000, "budget_ns": 60_000_000 },
    "serialization/to_bytes/d1536_bw4": { "median_ns": 150, "budget_ns": 250 },
    "serialization/from_bytes/d1536_bw4": { "median_ns": 200, "budget_ns": 300 },
    "rotation/build/cold/d1536": { "median_ns": 35_000_000, "budget_ns": 50_000_000 },
    "rotation/build/warm/d1536": { "median_ns": 40, "budget_ns": 80 },
    "codebook/train/n100000_bw4": { "median_ns": 5_000_000, "budget_ns": 8_000_000 }
  }
}
```

`budget_ns` is the "must not exceed" value that CI enforces. It's
typically 1.5× the observed median to allow for CI noise without
false positives. `median_ns` is an informational anchor.

### CI gate mechanics

```bash
# xtask bench subcommand
$ cargo xtask bench --baseline main --crate tinyquant-bench
$ cargo xtask bench --check-against main
```

`--check-against main` reads `baselines/main.json`, runs the
benchmarks, and exits non-zero if any measurement's median exceeds
its budget. The CI workflow runs `xtask bench --check-against main`
on every PR.

Baseline updates require:

1. A deliberate PR titled `perf(baseline): ...`.
2. The baseline JSON committed in the same PR as the performance
   improvement.
3. Reviewer sign-off that the new budgets are realistic and not
   papering over a regression.

### Host variability

Criterion's median is stable to ~2% on bare metal but CI runners have
higher jitter. The strategies to reduce false positives:

1. **Pin the CI runner class.** `runs-on: ubuntu-22.04-x86_64-avx2`
   or a self-hosted runner with `target-cpu=native`. Budgets are
   tuned to that class.
2. **Warm up before measurement.** Criterion's warmup is fine for
   numerics, but for SIMD kernels we add 100 extra iterations via
   `group.warm_up_time(Duration::from_millis(500))`.
3. **Disable frequency scaling** (`cpupower frequency-set -g
   performance` inside the runner where possible).
4. **Bench only on `main`** — PR benches are advisory-only; the
   gate is "budgets not exceeded on main after merge." If a merge
   regresses, the next PR is blocked until a fix or baseline update.

### Flame graphs

For deep-dive profiling, `xtask flamegraph bench codec/compress/single/d1536_bw4`
runs `cargo flamegraph` (via `perf` on Linux) and emits an SVG into
`target/flamegraphs/`. These are not CI-gated; they're developer
tooling.

## Python vs Rust end-to-end comparison

A special benchmark `parity_vs_python` runs the same workload in both
implementations and reports the speedup ratio:

```rust
// tinyquant-bench/benches/parity_vs_python.rs
fn bench_parity_vs_python(c: &mut Criterion) {
    let mut group = c.benchmark_group("parity_vs_python");
    group.sample_size(10);

    let corpus = GoldCorpus::load(1536, 1000, 4);

    group.bench_function("rust/compress_batch", |b| {
        let codec = Codec::new();
        b.iter(|| {
            codec.compress_batch(
                &corpus.vectors, 1000, 1536,
                &corpus.config, &corpus.codebook,
                Parallelism::Serial,
            ).unwrap()
        });
    });

    group.bench_function("python/compress_batch", |b| {
        // Spawn a subprocess: python -c "import timeit; ..."
        // or use inline pyo3 to call into tinyquant_cpu.
        b.iter_custom(|iters| {
            let secs = run_python_compress_batch(iters, &corpus);
            std::time::Duration::from_secs_f64(secs)
        });
    });

    group.finish();
}
```

This bench is documented but not CI-gated because it requires a
Python interpreter with `tinyquant_cpu` installed. It runs weekly as
a scheduled workflow and posts the ratio chart to the repo's
`docs/design/rust/benchmark-results/` folder for historical tracking.

## Memory benchmarks

A secondary set of benches measures peak RSS and allocation count:

```rust
use dhat::{Dhat, DhatAlloc};

#[global_allocator]
static ALLOCATOR: DhatAlloc = DhatAlloc;

#[test]
fn dhat_compress_batch_10000() {
    let _dhat = Dhat::start_heap_profiling();
    let corpus = GoldCorpus::load(1536, 10_000, 4);
    let codec = Codec::new();
    let _ = codec.compress_batch(
        &corpus.vectors, 10_000, 1536,
        &corpus.config, &corpus.codebook,
        Parallelism::Serial,
    ).unwrap();
    // Dhat prints a summary; we assert alloc count ≤ target
}
```

Memory gates:

| Benchmark | Peak heap | Max allocs |
|---|---|---|
| `compress_batch_10000_bw4_res_on` | ≤ 250 MiB | ≤ 20 015 |
| `compress_batch_10000_bw4_res_on` (packed) | ≤ 220 MiB | ≤ 15 |
| `decompress_batch_10000_bw4_res_on` | ≤ 120 MiB | ≤ 20 005 |
| `zero_copy_view_iter_10000` | ≤ 8 KiB | ≤ 4 |

The "packed" variant uses `compress_batch_packed`, which avoids
per-vector allocations.

## Reporting

Every CI run uploads:

1. `target/criterion/report/` — HTML report
2. `target/criterion/report.json` — machine-readable results
3. `target/flamegraphs/*.svg` — when flamegraph mode is enabled

The per-PR comment (via `jonnynabors/criterion-bot` or an xtask
script) highlights:

- Any measurement over budget (fails the PR)
- Any measurement more than 10% slower than the baseline (warn)
- Any measurement more than 10% faster (info — suggest baseline
  update)

## Determinism audit

Criterion itself is deterministic given a fixed seed for random
inputs, but the benchmark harness needs to pin:

- `CARGO_PROFILE_BENCH_DEBUG=0` (no debug symbols in bench profile)
- `CARGO_PROFILE_BENCH_LTO=fat`
- `CARGO_PROFILE_BENCH_CODEGEN_UNITS=1`
- `RUSTFLAGS="-C target-feature=+avx2,+fma"` on the x86_64 runner

These are set in `rust/.cargo/config.toml` under the `[profile.bench]`
section and reinforced by `xtask bench` at runtime.

## See also

- [[design/rust/goals-and-non-goals|Goals and Non-Goals]]
- [[design/rust/memory-layout|Memory Layout]]
- [[design/rust/simd-strategy|SIMD Strategy]]
- [[design/rust/ci-cd|CI/CD]]
