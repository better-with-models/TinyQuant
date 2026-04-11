---
title: Rust Port — Risks and Mitigations
tags:
  - design
  - rust
  - risks
  - mitigations
  - planning
date-created: 2026-04-10
status: draft
category: design
---

# Rust Port — Risks and Mitigations

> [!info] Purpose
> Surface the things most likely to go sideways so that each phase
> plan can address them deliberately instead of discovering them
> during execution.

## Risk matrix

| # | Risk | Likelihood | Impact | Owner |
|---|---|---|---|---|
| R1 | NumPy PCG64 + Ziggurat stream is not bit-reproducible in pure Rust | **High** | **High** (kills byte parity on rotation) | codec lead |
| R2 | `np.linalg.qr` vs `faer::qr` sign conventions diverge | Medium | High | codec lead |
| R3 | `np.quantile` default interpolation edge cases produce 1-ULP differences | Low | Medium | codec lead |
| R4 | `half::f16::from_f32` rounding mode mismatch with NumPy | Low | Medium | IO lead |
| R5 | pyo3 GIL reacquisition in batch paths costs more than the speedup | Medium | Medium | bindings lead |
| R6 | `faer` matmul performance falls short of BLAS on mid-range CPUs | Medium | High | perf lead |
| R7 | Parity fixture drift — Python test data changes, Rust fixtures stale | High | Medium | CI lead |
| R8 | `rayon` thread pool contention with downstream consumers (better-router) | Low | Medium | integration lead |
| R9 | `#[repr(C)]` layout differences between compilers in the C ABI | Low | High | ABI lead |
| R10 | SIMD kernel bugs invisible to unit tests on non-x86 runners | Medium | Medium | testing lead |
| R11 | `no_std` build breakage from new transitive dependencies | Medium | Medium | crates lead |
| R12 | MSRV drift in third-party deps forces consumer upgrades | Medium | Low | release lead |
| R13 | cbindgen generates different header text across versions → CI churn | Medium | Low | ABI lead |
| R14 | Wheel build failures on Windows/aarch64 | Medium | Medium | release lead |
| R15 | Fuzz target crashes in `from_bytes` path reveal latent Python bugs | Low | Medium | IO lead |
| R16 | Feature-flag combinatorial explosion leaves untested combinations | Medium | Low | testing lead |
| R17 | Parity test runtime balloons and blocks CI | Low | Medium | testing lead |
| R18 | Python project's header-size comment discrepancy (70 vs 71 bytes) is a symptom of undocumented format drift | Medium | High | IO lead |

## Detailed mitigations

### R1 — NumPy RNG stream not reproducible in pure Rust

**Problem.** NumPy's `default_rng(seed).standard_normal((d, d))` uses
PCG64 backed by a Ziggurat transform. Implementation details differ
subtly from the community crates (`rand_pcg`, `rand_distr`), so
drawing from an identically-seeded PCG64 in Rust gives a different
byte stream.

**Mitigation.**

1. Accept that byte parity on the rotation matrix is infeasible.
2. Define a *canonical* deterministic path inside Rust
   (`ChaChaGaussianStream`) and commit to it as the parity
   reference for `rust-v0.1.0`.
3. Capture the canonical Python-generated rotation matrices for the
   gold fixture `(seed, dim)` pairs into fixture files, regenerate
   Rust matrices, and assert they match within 1e-12 on f64 bytes
   (this works because both paths use `faer::qr` on the same seed →
   same input matrix).
4. For new `(seed, dim)` pairs not in the fixture set, the Rust
   implementation uses its canonical path and downstream Python
   consumers that need exact parity opt in to the pyo3-backed
   canonical generator instead of NumPy's.
5. Parity tests assert **effect-level** parity for the legacy NumPy
   path: rotated vectors agree to 1e-5 absolute and downstream
   cosine similarities agree to 1e-5.

**Status.** Landed in Phase 13 (2026-04-10) as a **Rust-canonical**
snapshot strategy, not a Python-generated one: the canonical
`ChaChaGaussianStream` + `faer::qr` + Haar sign correction pipeline is
frozen into `rust/crates/tinyquant-core/tests/fixtures/rotation/*.f64.bin`
(LFS-tracked) and every build re-diffs against the snapshot in
`tests/rotation_fixture_parity.rs`. Because NumPy PCG64 and the Rust
canonical stream can never agree byte-for-byte, the "effect-level
parity for the legacy NumPy path" sub-point has been explicitly
deferred to Phase 15+ (legacy-vs-canonical cosine parity harness).
See [[design/rust/phase-13-implementation-notes|Phase 13 Implementation Notes]]
for the full reasoning and the carryover list.

### R2 — QR sign conventions

**Problem.** Both LAPACK and `faer` return a valid QR decomposition,
but the sign of each column of `Q` is not uniquely determined. The
Python code applies `sign(diag(R))` as a correction; `faer` does
not apply such a correction automatically.

**Mitigation.** Replicate Python's correction explicitly in the Rust
path:

```rust
let (q, r) = faer::linalg::qr::no_pivoting::compute(&matrix);
for j in 0..dim {
    let sign = if r[(j, j)] >= 0.0 { 1.0 } else { -1.0 };
    for i in 0..dim {
        q[(i, j)] *= sign;
    }
}
```

A unit test checks that `sign(r.diagonal()) = 1` after the correction,
mirroring Python.

### R3 — `np.quantile` interpolation edge cases

**Problem.** NumPy's `method="linear"` (the default) uses the
formula `(N - 1) * q`, which can produce subtly different f64 values
for input arrays with specific structure (e.g., repeated values at
the quantile boundaries).

**Mitigation.** Replicate NumPy's exact formula element-by-element
in the Rust path. If a corner case appears, fall back to
pre-captured fixture codebooks for the gold seeds. A proptest
explores 256 seeds per bit width and asserts byte-level parity; any
failure is captured as a regression test.

### R4 — `f16::from_f32` rounding mode

**Problem.** The `half` crate's default is round-to-nearest-even,
which matches NumPy. The risk is that a future `half` release changes
the default or adds an unsafe fast-rounding path.

**Mitigation.**

1. Pin `half` to an exact minor version in workspace deps.
2. Use `half::f16::from_f32` (the explicit RNE function), not any
   `f32_to_f16_fast` variant.
3. Property test: 1 M random f32 samples round-trip through both
   `np.float16` (via pyo3) and `f16::from_f32` and must match
   byte-for-byte.

### R5 — pyo3 GIL reacquisition cost

**Problem.** `Python::allow_threads` releases the GIL, but every
`PyObject` touch (including `PyArray1::from_vec`) needs the GIL
back. If the return path of a batch compresses 10 000 objects one
at a time, the GIL bouncing dominates.

**Mitigation.**

1. `compress_batch` returns a single `Py<PyList>` built under a
   single GIL acquisition at the end, not per-vector acquisitions.
2. A dedicated `compress_batch_view` method returns a NumPy uint8
   array directly (shape `(n, packed_len)`), avoiding per-vector
   Python objects altogether. Consumers that don't need
   `CompressedVector` instances per-row get the fastest path.
3. Benchmark `bench_batch_rust_vs_python_return` validates the GIL
   cost is acceptable.

### R6 — `faer` matmul performance

**Problem.** `faer` is well-optimized but not as fast as vendor BLAS
(MKL, OpenBLAS with AVX2 tuning) on all CPUs. If the rotation
matvec/matmul falls short of our budget, we need a fallback.

**Mitigation.**

1. Benchmark `faer` on representative CPUs in phase 13. If it meets
   the budget, ship it.
2. If it falls short, add an optional `blas` feature that links
   against the system BLAS (via `cblas-sys`) and uses `cblas_sgemm`.
   The feature is opt-in, off by default, and the parity test
   asserts effect-level parity only.
3. A hand-tuned AVX2 matvec kernel is a fallback for the single-
   vector hot path if `faer` falls short and we don't want the BLAS
   dependency.

### R7 — Parity fixture drift

**Problem.** Python tests change, new test vectors are added, or
`_quantize.py` gains a new behavior. Rust fixtures stale without
noticing.

**Mitigation.**

1. `xtask fixtures check` runs in every PR and compares the
   in-repo fixture hashes against a Python-generated reference.
2. When fixture refresh is needed, the author runs
   `xtask fixtures refresh` which invokes Python and regenerates the
   fixtures. The PR diff shows the fixture changes explicitly;
   reviewers approve them.
3. A tracking issue in the Python side assigns an owner to any
   Python change that would require a Rust fixture refresh.

### R8 — `rayon` thread pool contention

**Problem.** If a downstream app (better-router) already uses
`rayon` with a custom pool, our batch operations might fight over
threads.

**Mitigation.**

1. Document the `rayon::ThreadPoolBuilder` pattern in
   `tinyquant-sys` and `tinyquant-py`:
   ```rust
   pool.install(|| codec.compress_batch(...))
   ```
2. The C ABI's `tq_set_num_threads` installs a dedicated pool
   keyed to `tinyquant-sys` only, isolating it from the consumer's
   pool.
3. `tinyquant-py` respects `RAYON_NUM_THREADS` and an explicit
   `set_num_threads(n)` call.

### R9 — `#[repr(C)]` layout across compilers

**Problem.** `#[repr(C)]` is stable between two rustc-produced
`cdylib`s, but mixing a `cdylib` from one rustc with a C consumer
built by a different clang version might have alignment quirks on
platforms with non-standard ABIs.

**Mitigation.**

1. Avoid aggregates in the C ABI. Every exported function takes
   primitive types, opaque handle pointers, and `*mut
   TinyQuantError`. The only aggregate exposed is
   `TinyQuantError { kind: u32, message: *mut c_char }`, which is
   identically laid out everywhere relevant.
2. Do not embed `f64` fields in C ABI structs on 32-bit targets.
3. The `tinyquant-sys` test suite includes a C consumer build
   (`gcc` on Linux, `cl.exe` on Windows, `clang` on macOS) that
   actually calls the ABI — if the layouts diverge, this test
   fails.

### R10 — SIMD kernel bugs invisible on non-x86 runners

**Problem.** An AVX2-only bug might slip past the portable SIMD
path on CI runners that don't have AVX2 enabled.

**Mitigation.**

1. CI matrix includes an x86_64 runner with
   `RUSTFLAGS="-C target-feature=+avx2,+fma"` and another with
   `-C target-feature=+sse4.1` (no AVX). Both run the full test
   suite.
2. For every SIMD kernel, tests run both scalar and vector paths
   on the same inputs and assert byte identity.
3. A nightly job runs on an Apple Silicon runner to exercise the
   NEON kernel on real hardware.

### R11 — `no_std` breakage from transitive deps

**Problem.** Adding a dep that's secretly `std`-only breaks the
`no_std` build silently unless a dedicated CI job catches it.

**Mitigation.**

1. CI job `core-nostd` builds `tinyquant-core` for
   `thumbv7em-none-eabihf` on every PR.
2. `cargo tree -p tinyquant-core --no-default-features` is run and
   its output compared against a committed reference to detect new
   transitive deps.
3. Any new dep added to `tinyquant-core` must be `default-features =
   false` unless it's `std`-specific and guarded behind the `std`
   feature.

### R12 — MSRV drift

**Problem.** A dep bumps its MSRV past ours, forcing us to bump.

**Mitigation.** `cargo +1.78.0 check` job catches this immediately.
When a dep forces an MSRV bump, we pin the dep to the previous
version until we're ready to bump ours, or we bump ours in a minor
release with a clear changelog entry.

### R13 — cbindgen header churn

**Problem.** Running cbindgen across versions produces slightly
different whitespace or ordering, causing CI diffs that aren't
semantic.

**Mitigation.**

1. Pin `cbindgen` to an exact version in `Cargo.toml`.
2. Commit the generated `tinyquant.h` verbatim.
3. CI fails if `cargo build -p tinyquant-sys` produces a diff to
   the committed header. Author re-runs locally and commits.

### R14 — Wheel build failures

**Problem.** `maturin` cross-compiling wheels for aarch64-linux on
x86_64-linux requires `zig` or a dedicated runner, and Windows
wheels need MSVC. Each combination is a separate failure mode.

**Mitigation.**

1. Use `PyO3/maturin-action@v1` with `--zig` for Linux cross builds.
2. Use dedicated `macos-14` runners for arm64 macOS.
3. Use `windows-2022` for Windows x86_64.
4. On first failure, we fall back to building each platform on its
   native runner.

### R15 — Fuzz target crashes

**Problem.** Fuzzing `from_bytes` might reveal a Python bug (e.g.,
Python accepts certain malformed inputs without raising). If Rust
fixes the bug, parity is broken.

**Mitigation.**

1. Parity is defined on *well-formed* inputs. If Rust returns
   `Err(IoError)` for an input Python silently corrupts, that is
   not a parity violation; it's a defensive improvement.
2. `COMPATIBILITY.md` documents the specific inputs where Rust is
   stricter, with rationale.
3. Upstream bug reports filed against the Python side so the fix
   can eventually land there too.

### R16 — Feature-flag combinatorial explosion

**Problem.** N features ⇒ 2^N combinations, many untested.

**Mitigation.** The 9-combination matrix in
[[design/rust/feature-flags|Feature Flags]] covers the interesting
combinations. Adding a new feature requires adding a matrix entry.

### R17 — Parity test runtime

**Problem.** If parity tests grow to cover every combination of
(bit_width, dimension, seed, residual_mode), the full matrix has
thousands of runs and the CI time balloons.

**Mitigation.**

1. Parity tests use `proptest` with bounded iteration counts (256
   cases per proptest block).
2. Exhaustive byte-parity tests are reserved for a small canonical
   set (120 triples for config hash, 12 triples for full compress
   roundtrip).
3. Weekly `rust-parity.yml` runs the full expanded matrix
   (thousands of cases) out-of-band.

### R18 — Header-size discrepancy in Python source

**Problem.** The Python file `compressed_vector.py` has a comment
claiming the header is 71 bytes, but `struct.calcsize("<B64sIB")`
returns 70. This is documented technical debt.

**Mitigation.**

1. The Rust port uses the empirical 70-byte value.
2. Phase 13 includes a ticket to file a PR against the Python side
   fixing the comment.
3. The parity test for header size explicitly asserts 70 and
   exercises the serialized byte stream end-to-end, so any future
   drift is caught.

## Open questions tracked elsewhere

- Is the `legacy` rotation mode worth the complexity? See
  [[design/rust/numerical-semantics|Numerical Semantics]] §3.
- Should `tinyquant-sys` ship pre-built binaries via GitHub
  Releases? See [[design/rust/release-strategy|Release Strategy]].
- Does the Rust port need a separate design for a columnar corpus
  format? See level-3 section in
  [[design/rust/serialization-format|Serialization Format]].

## See also

- [[design/rust/numerical-semantics|Numerical Semantics]]
- [[design/rust/error-model|Error Model]]
- [[design/rust/testing-strategy|Testing Strategy]]
- [[design/rust/ci-cd|CI/CD]]
