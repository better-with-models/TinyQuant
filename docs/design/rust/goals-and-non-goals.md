---
title: Rust Port — Goals and Non-Goals
tags:
  - design
  - rust
  - goals
  - performance
date-created: 2026-04-10
status: draft
category: design
---

# Rust Port — Goals and Non-Goals

> [!info] Purpose
> Pin down what "ultra-high performance" means for this port so that
> every subsequent design decision can be judged against a crisp
> yardstick. Without this, the port risks optimizing for the wrong
> bottleneck.

## Performance goals (hard numbers, not aspirations)

All numbers are measured on a single x86_64 core at fixed clock,
against the Python reference on the same machine, using real OpenAI
`text-embedding-3-small` vectors (dim 1536) unless otherwise stated.
The gold corpus is the same 10 000-vector fixture the Python
calibration tests use.

| Metric | Python reference | Rust target | Rust stretch |
|---|---|---|---|
| Single-vector `compress` (dim 1536, 4-bit + residual) | ~180 µs | ≤ 18 µs (10×) | ≤ 6 µs (30×) |
| Single-vector `decompress` | ~95 µs | ≤ 9 µs (10×) | ≤ 3 µs (30×) |
| Batch `compress_batch` of 10 000 vectors | ~1.9 s | ≤ 80 ms (24×) | ≤ 25 ms (75×) · ≤ 5 ms GPU (380×) |
| Batch `decompress_batch` of 10 000 vectors | ~0.95 s | ≤ 40 ms (24×) | ≤ 12 ms (80×) · ≤ 2 ms GPU (475×) |
| `CompressedVector::to_bytes` (dim 1536) | ~5 µs | ≤ 150 ns (30×) | ≤ 80 ns |
| `CompressedVector::from_bytes` (dim 1536) | ~8 µs | ≤ 200 ns (40×) | ≤ 100 ns |
| `Codebook::train` on 100k values | ~45 ms | ≤ 5 ms (9×) | ≤ 1.5 ms |
| Rotation matrix build (dim 1536, first call) | ~110 ms | ≤ 35 ms (3×) | ≤ 15 ms |
| Rotation matrix build (warm cache hit) | ~400 ns | ≤ 40 ns (10×) | ≤ 20 ns |

> [!note] Why the rotation build target is only 3×
> Matrix generation is dominated by QR decomposition, which is already
> BLAS-backed in NumPy. The Rust path uses `faer`/`nalgebra` or calls
> into the same BLAS — we accept modest speedup here and focus the SIMD
> budget on the hot path (quantization, rotation application,
> serialization).

### Fidelity goals (inherited from Python, non-negotiable)

| Metric | Threshold | Source |
|---|---|---|
| Pearson ρ, 4-bit + residual | ≥ 0.995 | [[design/behavior-layer/score-fidelity]] VAL-01 |
| Pearson ρ, 4-bit no residual | ≥ 0.98 | VAL-01 |
| Pearson ρ, 2-bit + residual | ≥ 0.95 | VAL-01 |
| Top-10 neighbor overlap on 10k gold corpus | ≥ 80% | behavior layer |
| Compression ratio, 4-bit | ≥ 7× | VAL-02 |
| Compression ratio, 4-bit + residual | ≥ 5× | VAL-02 |
| Round-trip determinism (bit-identical bytes across calls) | 100% | VAL-03 |
| Byte-identical `to_bytes` vs Python reference | 100% | parity gate (this port) |

The parity gate is the binding Rust-specific requirement: for any
`(CodecConfig, Codebook, vector)` triple supported by both
implementations, `Rust::compress(...).to_bytes() == Python::compress(...).to_bytes()`.

## Non-performance goals

- **Zero-copy reads** — an `mmap`ped file of serialized vectors can be
  iterated without allocating per vector. The Python implementation
  cannot do this because every `from_bytes` builds a new NumPy array.
- **Stable C ABI** — a `tinyquant-sys` crate exposes the codec to
  non-Python consumers (better-router Rust agents, C++ glue) as both
  a `cdylib` and a `staticlib` with versioned headers.
- **Drop-in pyo3 binding** — a `tinyquant-py` crate produces a Python
  wheel whose classes behave like `tinyquant_cpu.codec.*` to the byte
  level, so downstream code can migrate by changing the import.
- **Standalone CLI binary** — a `tinyquant-cli` crate produces a
  single-file `tinyquant` (or `tinyquant.exe`) executable that
  ships on GitHub Releases for every Tier-1/Tier-2 target and in a
  multi-arch container image on GHCR. The CLI is the operator and
  CI-job experience and does not require installing a Rust or
  Python toolchain.
- **`no_std` core** — the `tinyquant-core` crate compiles without `std`
  (using `alloc`), enabling future WASM and embedded targets without a
  rewrite.
- **Deterministic builds** — same source, same toolchain → same binary
  output (`SOURCE_DATE_EPOCH`, no `build.rs` timestamps).

## Non-goals (explicit)

- **No new algorithms.** The Rust port does not change the math. No
  learned codebooks, no KMeans, no product quantization, no ANN
  indexes. Those are separate initiatives.
- **No GPU acceleration in `tinyquant-core`.** The core codec stays
  CPU-only and `no_std` — the deployability story of "runs in any
  container, no drivers required" is preserved. GPU offload lives in
  *optional, additive crates* (`tinyquant-gpu-wgpu`, `tinyquant-gpu-cuda`)
  that depend on core but are never pulled in by default. See
  [[design/rust/gpu-acceleration|GPU Acceleration Design]] for the
  full plan. The GPU crates carry their own MSRV and are not wired into
  `rust-ci.yml` until Phase 27.
- **No async runtime.** The codec is CPU-bound; making it `async` adds
  overhead and obscures ownership. Async concerns belong in the HTTP
  layer (better-router), not the codec.
- **No dynamic dispatch in the hot path.** `dyn SearchBackend` is fine
  at the corpus→backend boundary, but `Codec::compress` must be
  monomorphic.
- **No hidden global state.** No lazy statics, no thread-locals, no
  `OnceCell`-backed singletons in the codec. The rotation matrix cache
  is the single exception and is documented explicitly.
- **No breaking behavior changes relative to Python.** Any deviation is
  a bug until the Behavior Layer document is updated and the Python
  implementation is updated to match.
- **No feature creep.** Additional knobs (e.g., 1-bit mode, rotation
  matrix types other than Haar QR) require a behavior-layer change
  before they can land.

## What "ultra-high performance" means operationally

Four things, in order of importance:

1. **Throughput on the batch path** — because that's how production
   systems actually use the codec. A 24× batch speedup matters more
   than a 30× single-vector speedup.
2. **Tail latency on the single-vector path** — because routing systems
   care about p99, not average. Warm caches matter.
3. **Zero-copy IO** — because reading a gigabyte of compressed vectors
   should be limited by the SSD, not by Python object allocation.
4. **Memory footprint during batch ops** — because pushing a 1M-vector
   corpus through the codec on an 8 GiB box should work without OOM.

If a proposed optimization buys us none of these four things, it does
not belong in this port.

## See also

- [[design/rust/README|Rust Port Overview]]
- [[design/rust/simd-strategy|SIMD Strategy]]
- [[design/rust/gpu-acceleration|GPU Acceleration Design]]
- [[design/rust/benchmark-harness|Benchmark Harness and Performance Budgets]]
- [[design/behavior-layer/score-fidelity|Score Fidelity]]
