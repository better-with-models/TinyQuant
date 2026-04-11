---
title: "Phase 20 Implementation Notes"
tags:
  - design
  - rust
  - phase-20
  - simd
  - performance
date-created: 2026-04-11
category: design
---

# Phase 20 Implementation Notes

## What landed

Phase 20 delivers the **runtime SIMD dispatch framework** for
TinyQuant's quantize / dequantize / cosine / residual kernels, with
byte-for-byte scalar parity guaranteed on every target-feature matrix
row. The actual hand-written AVX2 / NEON intrinsic bodies are
intentionally deferred to Phase 22 — the framework exists now so that
subsequent intrinsic work lands as isolated "swap the body of
`kernels::avx2::cosine`" patches without touching dispatch,
benchmarks, CI, or tests.

### Concrete deliverables

- `tinyquant-core/src/codec/kernels/{mod,scalar,portable,avx2,neon}.rs`
  — kernel modules. `scalar.rs` is the canonical reference;
  `portable.rs` re-exports scalar on MSRV 1.81; `avx2.rs` / `neon.rs`
  provide `#[target_feature(enable = "...")]` wrappers that currently
  delegate to scalar.
- `tinyquant-core/src/codec/dispatch.rs` — `DispatchKind` enum +
  `current()` / `force()` backed by `std::sync::OnceLock`. Probes
  AVX2+FMA via `is_x86_feature_detected!` on x86_64; unconditionally
  picks NEON on aarch64 (ARMv8 base ISA guarantee).
- `tinyquant-core/src/codec/simd_api.rs` — the public façade that
  downstream crates and tests use instead of reaching into the private
  `kernels::*` modules.
- `Codebook::quantize_into` / `dequantize_into` now route through
  `dispatch::current()` when compiled with `feature = "simd"`.
- `tinyquant-bruteforce` gains a `simd` feature that forwards cosine
  similarity to `tinyquant_core::codec::simd_api::cosine`.
- `tinyquant-bench/benches/simd_kernels.rs` — Criterion benches for
  quantize / dequantize / cosine / residual at dim=1536, gated on a
  new `tinyquant-bench/simd` feature. Placeholder baseline under
  `baselines/simd_kernels.json`.
- `xtask simd audit` — string-grep over `.github/workflows/rust-ci.yml`
  to guarantee the Phase 20 target-feature matrix stays wired into CI.
- CI jobs: `simd-scalar`, `simd-avx2` (RUSTFLAGS `+avx2,+fma`),
  `simd-neon` (ubuntu-22.04-arm runner), `miri-scalar` (nightly),
  plus a `simd-audit` job running the xtask.
- Per-kind parity integration tests
  (`tests/simd_parity_{scalar,avx2,neon}.rs` + shared
  `tests/common/mod.rs`) and a `tests/simd_dispatch_cache.rs` cache
  smoke test.

## Deviations from the plan

### 1. No `std::simd`, no `pulp` for the "portable" path

The plan referenced `core::simd` / `std::simd` as a "portable" kernel
target. `std::simd` (portable-simd) stabilized in **Rust 1.82**,
above the TinyQuant workspace MSRV of **1.81**. Using it now would
either:

- break the MSRV (non-starter), or
- force an `#[cfg(msrv >= 1.82)]` fork that nobody runs (useless).

Resolution: `kernels/portable.rs` re-exports `kernels/scalar::*`
verbatim. The module exists so that downstream call-sites can
reference `kernels::portable::cosine` without each one re-implementing
the "no AVX2 and no NEON" cfg thicket. When the MSRV rises to 1.82+,
that file becomes the place where a real `f32x8`-based implementation
lands.

### 2. AVX2 / NEON wrappers currently delegate to scalar

The determinism contract in `docs/plans/rust/phase-20-simd-kernels.md`
demands byte-for-byte scalar parity on every target-feature matrix
row. The safest way to satisfy that contract while establishing the
dispatch framework is: **ship wrappers that call scalar**, carry the
`#[target_feature(enable = "avx2,fma")]` / `#[target_feature(enable =
"neon")]` annotation, and measure them in the parity tests. Zero
speedup, but zero risk of ever producing a non-parity byte.

When Phase 22 adds real intrinsic bodies, the parity tests still
pass because they compare every dispatch path against the scalar
reference — the intrinsic implementations must continue to return the
exact same bytes, or the test suite fails.

### 3. `dispatch::force` is not `#[cfg(test)]`

The plan suggested gating `force` behind `#[cfg(test)]`. That fails
in practice because Rust **integration tests** (files under `tests/`)
compile against the library with `cfg(not(test))`, so anything
marked `#[cfg(test)]` inside `src/` is invisible to them. The
resolution is `#[doc(hidden)]` + a blunt docstring warning: the
helper is technically reachable by application code, but it is
clearly labeled as a test hook and the `OnceLock` contract (single
successful set per process) makes misuse loudly fail.

### 4. Parity tests split into separate binaries

The plan suggested exercising every `DispatchKind` from a single
`tests/simd_parity.rs` file. That approach collapses because
`OnceLock` state is shared across every `#[test]` in a given
integration binary — only the **first** `force()` call wins, and the
others trip the "already populated" assertion.

Resolution: split into three per-kind test binaries
(`simd_parity_scalar.rs`, `simd_parity_avx2.rs`,
`simd_parity_neon.rs`) that share code via `tests/common/mod.rs`.
Cargo does not compile `tests/common/mod.rs` as a standalone
integration binary, so the shared helpers live there without
polluting test-binary discovery. Each per-kind file is its own
binary, gets its own process, and therefore gets its own fresh
dispatch cache.

### 5. `tinyquant-core/simd` implies `std`

`std::sync::OnceLock` and `is_x86_feature_detected!` both require
`std`. The simplest way to keep `tinyquant-core` `no_std` by default
while still supporting the dispatch cache is:

```toml
[features]
default = []
std  = []
simd = ["std"]
```

combined with `#[cfg(feature = "std")] extern crate std;` in `lib.rs`.
The thumbv7em-none-eabihf cross-compile job runs with
`--no-default-features`, so `simd` is never enabled there and the
dispatch module never appears in that build.

### 6. Bruteforce `simd` feature activates `tinyquant-core/simd`

The plan implied implementing an inline SIMD cosine. Instead,
`tinyquant-bruteforce/simd = ["tinyquant-core/simd"]` routes the
cosine through the core crate's `simd_api::cosine`. One dispatch
cache, one parity-test story, one place where intrinsic bodies need
to land in Phase 22.

## Parity guarantee matrix

| target_triple                          | RUSTFLAGS              | DispatchKind | Expected output |
| -------------------------------------- | ---------------------- | ------------ | --------------- |
| x86_64-unknown-linux-gnu               | (default)              | Scalar       | scalar          |
| x86_64-unknown-linux-gnu               | `+avx2,+fma`           | Avx2         | scalar          |
| aarch64-unknown-linux-gnu              | (default)              | Neon         | scalar          |
| thumbv7em-none-eabihf (`--no-default`) | (default)              | n/a (no_std) | scalar          |
| x86_64-pc-windows-msvc                 | (default)              | Scalar/Avx2* | scalar          |

`*` On Windows the dispatch cache probes AVX2+FMA at runtime; the
`simd-avx2` CI job pins `+avx2,+fma` via `RUSTFLAGS` on Linux.

Every row produces byte-identical output because every SIMD path
currently forwards to the scalar reference. Phase 22 updates this
matrix with a new column recording measured-speedup numbers; the
"expected output" column does not change.

## Dispatch cache semantics

- **Cache is process-local.** Each test binary, each benchmark run,
  each `cargo run` of the CLI gets its own `DISPATCH: OnceLock`.
- **First write wins.** `force()` succeeds only once per process;
  subsequent calls panic with a clear message. `current()` auto-
  detects on first call.
- **Cache is frozen after the first read.** `current()` is the only
  way to trigger detection in production code. Application code
  **must not** call `force()` — that function is documented as a test
  hook and is `#[doc(hidden)]`.

## Forward compatibility

Phase 22 will:

1. Replace the body of `kernels::avx2::*` with hand-written
   `core::arch::x86_64` intrinsic implementations, preserving the
   function signatures exactly.
2. Replace `kernels::neon::*` with `core::arch::aarch64` intrinsics.
3. Re-run the parity tests and capture real benchmark numbers into
   `rust/crates/tinyquant-bench/baselines/simd_kernels.json`.

No dispatch code, no test infrastructure, and no CI jobs need to
change. That is the point of landing the framework first.

## Risks carried forward

- **R-SIMD-1**: `#[target_feature]` on a function that calls a
  non-target-feature function defeats the point of the annotation.
  This is acceptable in Phase 20 because the bodies are literal
  forwards; Phase 22 must inline or duplicate any helper that sits
  on the SIMD hot path.
- **R-SIMD-2**: NEON is assumed available unconditionally on
  aarch64. True for ARMv8, but if we ever target ARMv7 that
  assumption breaks — gate future aarch32 work behind an additional
  feature probe.
- **R-SIMD-3**: The Miri job runs against the scalar path only.
  Miri does not fully model `core::arch` intrinsics, so Phase 22
  intrinsic bodies must be excluded with `#[cfg(not(miri))]` or
  equivalent.
