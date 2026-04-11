---
title: "Phase 20: SIMD Kernels and Runtime Dispatch"
tags:
  - plans
  - rust
  - phase-20
  - simd
  - performance
date-created: 2026-04-10
status: draft
category: planning
---

# Phase 20: SIMD Kernels and Runtime Dispatch

> [!info] Goal
> Add portable SIMD kernels for quantize, dequantize, residual
> (f16 conversion), and cosine similarity, with runtime ISA dispatch
> and byte-identical scalar fallbacks.

> [!note] Reference docs
> - [[design/rust/simd-strategy|SIMD Strategy]]
> - [[design/rust/memory-layout|Memory Layout]]
> - [[design/rust/numerical-semantics|Numerical Semantics]] §Quantization parity
> - [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]] §Lessons learned (L1–L7)

## Determinism contract

> [!danger] Load-bearing invariant
> Phase 20 is the phase where TinyQuant earns or loses cross-platform
> parity forever. Everything below is the contract the kernels must
> hold. Any SIMD kernel that violates it is broken, not "close
> enough". We do not relax parity for speed, ever.

### The invariant

For every kernel in the set

```
{ quantize_2bit, quantize_4bit, quantize_8bit,
  dequantize,
  cosine,
  residual_encode, residual_decode }
```

the output must be:

1. **Byte-identical to the scalar reference** on the same inputs, on
   **every** supported `(target_triple, RUSTFLAGS)` pair listed in the
   [target-feature matrix](#target-feature-matrix) below.
2. **Byte-identical regardless of `RAYON_NUM_THREADS`** — `1`, `2`,
   `$(nproc)`, and any value in between must produce the same bytes.
3. **Byte-identical regardless of whether `std::simd` / `pulp` /
   runtime dispatch picks AVX2, AVX-512, NEON, or scalar** on the
   current host CPU. The dispatch decision changes *speed*, never
   *output*.

The scalar reference — `codec::kernels::scalar::*` — is the canonical
source of truth. If SIMD and scalar disagree by even one byte on one
input, the SIMD kernel is the one that is wrong.

### Why this matters — Phase 14 L4 repeated

Phase 14 Lesson L4
([[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]]
§L4) documented that `faer` + `pulp` runtime ISA selection on
GitHub's `ubuntu-22.04` runner pool produced **different f64 bytes on
different runner hosts** for the rotation matrix builder at
`dim = 768`. The Phase 14 Test job got lucky; the subsequent docs PR
did not. The root cause was that two runners in the same pool had
different SIMD ISAs (AVX2 vs AVX-512) and `pulp` dispatched to
different code paths per process, producing structurally different
f64 outputs from the same QR routine. `RAYON_NUM_THREADS: "1"` was a
misleading workaround — the bug was still there, hiding behind
runner luck.

We cannot afford the same class of bug at the Phase 20 codec layer:

- Phase 16 byte-parity fixtures would silently rot as soon as two
  runners disagreed.
- Phase 21 determinism gates would then "certify" that disagreement.
- Phase 22 release parity against the Python reference would break
  for exactly half the install base.

Phase 20 is therefore where the determinism contract moves **into
code** — not into workflow env vars, not into runner pinning, not
into prose. The structural guard against an L4 repeat is a test
matrix that forces every `DispatchKind` variant on every host.

### How we enforce it

1. **Scalar reference exists for every SIMD kernel.** Every file
   under `codec/kernels/{avx2,neon,avx512,portable}.rs` has a
   matching scalar implementation in `codec/kernels/scalar.rs`. The
   scalar path is panic-free, `no_std`-compatible, and is the one
   we diff against.
2. **Identical reduction order.** If the scalar path computes
   `sum = a[0] + a[1] + ... + a[n-1]` left-to-right, the SIMD path
   uses the same tree shape. Horizontal sums inside a vector lane
   follow a fixed reduction tree that the scalar counterpart
   mirrors explicitly — no `_mm256_hadd_ps` with implementation-
   defined order, no "whatever `Iterator::sum` happens to do". See
   [[design/rust/numerical-semantics|Numerical Semantics]]
   §Reduction order contract.
3. **Dispatch is selected once per process.** `dispatch::current()`
   caches the detected ISA at first call via `OnceLock`; it cannot
   be toggled mid-run. See
   [Dispatch cache contract](#dispatch-cache-contract).
4. **CI runs the parity suite under three RUSTFLAGS combinations on
   x86_64:**
   - `""` (baseline, scalar fallback)
   - `"-C target-feature=+avx2,+fma"` (AVX2 path)
   - `"-C target-feature=+avx2,+fma,+avx512f"` (portable path where
     available — advisory only until a later phase promotes it)
5. **CI runs the parity suite on aarch64** via `cross` with the NEON
   path enabled.
6. **`simd_parity_under_every_dispatch`** is the structural guard.
   This test iterates over every `DispatchKind` variant, calls
   `dispatch::force(kind)` to pin the cache, and runs the full
   scalar-vs-SIMD diff on the Phase 14 training corpus plus the
   Phase 16 seeded proptest inputs. If any variant on any runner
   disagrees, the phase does not ship.

### What we accept as non-deterministic

Only the benchmark *timing numbers* (nanoseconds per op, throughput
GB/s, cache-miss counts). Everything observable through a kernel's
output slice is frozen. Criterion noise bands are allowed to move;
indices and f32 bit patterns are not.

## Prerequisites

- Phase 19 complete (brute-force backend exists so cosine kernel
  has a consumer).
- Phase 14 `Codebook` + scalar `quantize`/`dequantize` stable and
  frozen as the parity reference (it is).
- `std::simd` available on the pinned toolchain (stable in 2026).
- Phase 13 rotation fixture debt from L4 cleared, **or** the Phase
  20 work explicitly avoids depending on `RotationMatrix::build` in
  its parity fixtures. Phase 20 uses the Phase 14 `codebook_10k_d64`
  corpus plus Phase 16 seeded inputs, so it does not inherit the
  rotation debt directly — but the determinism contract in this
  phase is the general-purpose fix.

## Deliverables

### Files to create

| File | Purpose | Module visibility | Feature-gated |
|------|---------|-------------------|---------------|
| `rust/crates/tinyquant-core/src/codec/dispatch.rs` | ISA detection + cache + `force` override | `pub(crate)` | `simd` |
| `rust/crates/tinyquant-core/src/codec/kernels/mod.rs` | Module root, re-exports the selected path | `pub(crate)` | — |
| `rust/crates/tinyquant-core/src/codec/kernels/scalar.rs` | Canonical reference kernels | `pub(crate)` | always |
| `rust/crates/tinyquant-core/src/codec/kernels/portable.rs` | `std::simd` portable path | `pub(crate)` | `simd` |
| `rust/crates/tinyquant-core/src/codec/kernels/avx2.rs` | AVX2 intrinsics, `cfg(target_arch = "x86_64")` | `pub(crate)` | `simd` |
| `rust/crates/tinyquant-core/src/codec/kernels/neon.rs` | NEON intrinsics, `cfg(target_arch = "aarch64")` | `pub(crate)` | `simd` |
| `rust/crates/tinyquant-core/src/codec/kernels/avx512.rs` | AVX-512 intrinsics, off by default | `pub(crate)` | `simd` + `avx512` |
| `rust/crates/tinyquant-core/tests/simd_parity.rs` | Big parity test (scalar vs every dispatch) | integration test | `simd` |
| `rust/crates/tinyquant-core/tests/simd_dispatch_cache.rs` | `OnceLock` caching + `force` override behavior | integration test | `simd` |
| `rust/crates/tinyquant-bruteforce/src/similarity_simd.rs` | Dispatched cosine kernel | `pub(crate)` | `simd` |
| `rust/crates/tinyquant-bench/benches/simd_kernels.rs` | Criterion benches, scalar vs dispatched | bench target | `simd` |
| `rust/crates/tinyquant-bench/baselines/simd_kernels.json` | Captured baseline numbers for Phase 21 | data | — |
| `rust/xtask/src/cmd/simd.rs` | `cargo xtask simd audit` — runs parity suite across the matrix | xtask | — |

### Target-feature matrix

This is the authoritative list for CI jobs and for the parity test's
forced-dispatch sweep. Phase 14 L3 (design-doc-vs-CI drift) applies:
every row here must map to a real job in `rust-ci.yml`, and the
xtask drift check ([CI integration](#ci-integration)) enforces it.

| target_triple | RUSTFLAGS | dispatched kernel | CI job name | parity test |
|---|---|---|---|---|
| `x86_64-unknown-linux-gnu` | *(empty)* | scalar fallback | `rust-ci/simd-scalar` | required |
| `x86_64-unknown-linux-gnu` | `-C target-feature=+avx2,+fma` | `avx2` | `rust-ci/simd-avx2` | required |
| `x86_64-unknown-linux-gnu` | `-C target-feature=+avx2,+fma,+avx512f` | `portable` (AVX-512) | `rust-ci/simd-avx512-advisory` | advisory only until a later phase |
| `aarch64-unknown-linux-gnu` | *(empty)* | `neon` | `rust-ci/simd-neon` | required |
| `aarch64-apple-darwin` | *(empty)* | `neon` | `rust-ci/simd-neon-darwin` | required |
| `x86_64-apple-darwin` | `-C target-feature=+avx2,+fma` | `avx2` | `rust-ci/simd-avx2-darwin` | required |
| `x86_64-pc-windows-msvc` | `-C target-feature=+avx2,+fma` | `avx2` | `rust-ci/simd-avx2-windows` | required |

"Advisory only" means the job runs and reports, but a red result does
not block the phase. AVX-512 is kept off the default path until the
downclocking concern (R20.7) is resolved in a later phase.

### Kernel coverage matrix

Which kernel has which implementation, and which paths fall back to
another path rather than having their own specialization.

| kernel | bit_width | dim classes | scalar ref | portable | avx2 | neon | avx512 |
|---|---|---|---|---|---|---|---|
| `quantize_2bit` | 2 | any | yes | yes | yes | fallback→portable | fallback→portable |
| `quantize_4bit` | 4 | any | yes | yes | yes (hand-tuned) | yes (hand-tuned) | fallback→portable |
| `quantize_8bit` | 8 | any | yes | yes | yes | yes | fallback→portable |
| `dequantize` | 2/4/8 | any | yes | yes | yes (`vpgatherdd` when safe) | yes | fallback→portable |
| `cosine` | — | 64 / 256 / 768 / 1536 | yes | yes | yes (FMA) | yes (FMA) | fallback→portable |
| `residual_encode` | — | any | yes | yes | yes (via `half::f16`) | yes (via `half::f16`) | fallback→portable |
| `residual_decode` | — | any | yes | yes | yes | yes | fallback→portable |

"fallback→portable" means the dispatch table routes that
`(kernel, isa)` pair to the portable `std::simd` implementation
rather than to a hand-specialized intrinsic path. Every fallback
still runs the parity test; it does not get to skip the diff just
because it shares code with another row.

## Steps (TDD order)

- [ ] **Step 1: Failing scalar-vs-portable parity test**

```rust
#[test]
fn quantize_portable_matches_scalar_on_random_inputs() {
    let entries: Vec<f32> = (0..16).map(|i| i as f32 * 0.1).collect();
    let values: Vec<f32> = (0..10_000).map(|i| ((i as f32).sin() + 1.0) * 0.8).collect();
    let mut idx_scalar = vec![0u8; values.len()];
    let mut idx_simd = vec![0u8; values.len()];
    kernels::scalar::quantize_4bit(&values, &entries, &mut idx_scalar);
    kernels::portable::quantize_4bit(&values, &entries, &mut idx_simd);
    assert_eq!(idx_scalar, idx_simd);
}
```

Similar tests for `8bit` quantize, dequantize (gather), cosine, and
residual conversion.

> [!tip] Phase 14 L5 reminder
> Do not hand-type expected f32 literals. Always read the expected
> entry back out of the codebook under test. `9.0_f32 * 0.1_f32`
> is `0.90000004`, not `0.9`, and that bit us once already.

- [ ] **Step 2: Implement `dispatch.rs`**

Per [[design/rust/simd-strategy|SIMD Strategy]] §Runtime dispatch.

### Dispatch cache contract

- `dispatch::current() -> DispatchKind` is cached via
  `std::sync::OnceLock<DispatchKind>`. The first caller runs CPUID
  detection (x86_64) or reads `hwcap` (aarch64); every subsequent
  caller loads the cached value.
- **First-call cost:** microseconds for CPUID detection; negligible
  against any kernel it gates.
- **Thread safety:** `OnceLock` initialization races are fine
  because every racer computes the *same* answer from the same
  host CPU — the worst case is some wasted CPUID cycles, never a
  wrong result.
- **Override for tests:** `dispatch::force(kind: DispatchKind)` sets
  the cache **before** first call. It panics if called after the
  cache is already populated, because silently overriding a
  previously-observed value would break the "same value for the
  lifetime of the process" guarantee that the rest of the code
  relies on.
- This is the mechanism `simd_parity_under_every_dispatch` uses:
  each forced variant runs in its own `#[test]` function, and each
  `#[test]` gets a fresh process (one of Rust's default harness
  guarantees for `--test-threads=1` plus spawned subprocesses via
  the test helper). If a test runs in the same process as a
  previous test that already populated the cache, the helper
  spawns a child with `std::process::Command` to get a fresh
  `OnceLock`.
- `dispatch::current()` returns the **same value for the entire
  lifetime of the process** once observed. This is the invariant
  L4 tried to enforce with `RAYON_NUM_THREADS: "1"` and failed.

- [ ] **Step 3: Implement `kernels/scalar.rs`**

Extracted from Phase 14's `Codebook::quantize_into` and the existing
`scalar_quantize` / `scalar_dequantize` helpers. This is the
canonical reference; **no behavior changes** to what Phase 14 shipped.
If the scalar path needs a semantics tweak, that is a Phase 14
bugfix PR against the Codebook, not a Phase 20 change.

- [ ] **Step 4: Implement `kernels/portable.rs`**

Using `std::simd::f32x8` for the quantize-4bit linear search. See
[[design/rust/simd-strategy|SIMD Strategy]] §Portable SIMD fallback.
The portable path must use the same reduction tree as scalar — no
implementation-defined horizontal sums.

- [ ] **Step 5: Implement `kernels/avx2.rs`**

`#[target_feature(enable = "avx2,fma")]` kernels with `// SAFETY:`
comments for every `unsafe` block. Must produce byte-identical
output to scalar on every input in the parity fixture.

- [ ] **Step 6: Implement `kernels/neon.rs`**

Aarch64-only path using `core::arch::aarch64` intrinsics or portable
`std::simd` (which lowers to NEON). Byte-identical to scalar.

- [ ] **Step 7: Wire dispatch into `Codebook::quantize_into`**

The public API stays identical; internally it calls
`dispatch::quantize(...)`. Crucially, the dispatched version must
still match the scalar reference fixtures from Phase 14 byte-for-
byte — the Phase 14 integration tests must continue to pass
unchanged.

- [ ] **Step 8: Run parity tests — expect pass** (possibly after
iteration).

- [ ] **Step 9: Cosine similarity SIMD kernel**

In `tinyquant-bruteforce/src/similarity_simd.rs`, add `cosine_avx2`
and `cosine_portable`. `BruteForceBackend::search` uses the
dispatched variant. Reduction tree matches scalar.

- [ ] **Step 10: SIMD residual conversion**

`compute_residual_simd` and `apply_residual_simd` using
`half::f16::from_f32` via `std::simd`. Parity against scalar,
including NaN bit patterns (see [NaN semantics](#nan-semantics)).

- [ ] **Step 11: Allocation audit on hot path**

Confirm no allocations inside `dispatch::quantize`, cosine, or
residual kernels. Use `#[global_allocator]` instrumentation in a
bench-only test or inspect via `cargo asm` for the hot loops.

- [ ] **Step 11a: Miri audit of unsafe blocks**

- Every `unsafe { ... }` block in `kernels/avx2.rs` and
  `kernels/neon.rs` has a `// SAFETY:` comment explaining why the
  precondition (target-feature availability, pointer validity,
  length bounds) is upheld.
- `cargo +nightly miri test -p tinyquant-core --features ""` runs
  clean on the scalar path only. SIMD intrinsics are not supported
  by Miri's interpreter, so the Miri job deliberately runs with
  `--features ""` (no `simd` feature) and exercises the scalar
  kernels plus the dispatch scaffolding only.
- Explicit CI job **`rust-ci/miri-scalar`** is added to the matrix
  and is a required check.
- Address-sanitizer / thread-sanitizer runs on the SIMD kernels are
  **out of scope for Phase 20**. ASAN/TSAN instrumentation of
  `core::arch::x86_64` intrinsics is historically flaky (false
  positives on `_mm256_loadu_*` when the trailing byte lands near a
  page boundary). We document the blind spot and accept it; the
  parity test + Miri-on-scalar pair covers the realistic failure
  modes.

- [ ] **Step 12: Benchmark suite**

Add `tinyquant-bench/benches/simd_kernels.rs` with:

- **Criterion config:** 100 iterations per bench, confidence 95%,
  noise threshold 2%, warmup 3s, measurement 5s.
- **Captured metrics:** median, mean, stddev, p99.
- **Baseline file path:**
  `rust/crates/tinyquant-bench/baselines/simd_kernels.json`.
- **Regression policy:** Phase 20 **captures the initial baseline
  only**. It is a direction, not a gate. Phase 21 adds the budget
  gate and will re-capture on CI-on-main to establish the real
  reference.
- **What gets measured:**
  - Quantize at `bw = 4` for `dim ∈ {64, 256, 768, 1536}`.
  - Cosine at `dim = 1536`.
  - Residual encode at `dim = 1536`.
  - Residual decode at `dim = 1536`.
  - Parametric sweep: quantize-4bit for `dim in 64..=2048 step 64`,
    to feed Phase 21's piecewise-linear model.
- **Paths compared per bench:** scalar vs portable vs avx2 (on
  x86_64) / scalar vs portable vs neon (on aarch64).

> [!note] Phase 14 L4 reminder — baseline is not a determinism test
> A benchmark baseline is allowed to drift with host CPU and
> toolchain. A *parity* result is not. Phase 21's budget gate will
> distinguish the two; Phase 20's job here is to collect numbers so
> Phase 21 has something to gate against.

- [ ] **Step 13: Update CI to run parity tests with
`RUSTFLAGS="-C target-feature=+avx2,+fma"`**

Add the full target-feature matrix above as jobs in `rust-ci.yml`.
See [CI integration](#ci-integration) for the drift-check and LFS
hydration rules inherited from Phase 14 L2/L3.

- [ ] **Step 14: NaN tie-break test**

Feed NaN values through every kernel and assert the SIMD path
matches scalar bit-for-bit. The semantics are locked below.

### NaN semantics

> [!warning] R20.3 — NaN min/max divergence
> `_mm256_min_ps` returns the second operand on a quiet-NaN
> comparison; `f32::min` in Rust returns the non-NaN operand. These
> disagree. We lock the scalar reference to match `_mm256_min_ps`'s
> behavior explicitly by writing out the branch, not by calling
> `.min`.

- **Quantize:** if `values[i].is_nan()`, the scalar reference
  returns index `0` — the lowest codebook entry. Every SIMD kernel
  must match. The implementation uses `if v.is_nan() { 0 } else { … }`
  rather than relying on `f32::min` / `_mm256_min_ps` corner-case
  behavior.
- **Dequantize:** no NaN is possible from
  `codebook.entries[index]` because entries are sanitized at train
  time (Phase 14 `Codebook::train` rejects NaN/Inf inputs in its
  quantile computation). The parity test still feeds NaN indices
  via an `unsafe` path to confirm the out-of-range index rejection
  stays consistent across SIMD/scalar.
- **Cosine:** if either input vector contains any NaN, the scalar
  reference returns `0.0` (this matches the Python reference). SIMD
  paths must lower to the same value. Implementation: mask-scan for
  NaN before the FMA loop, early-return `0.0` on any NaN. Do not
  try to handle NaN inside the reduction — that way lies L4.
- **Residual encode:** `f16::from_f32(f32::NAN).to_bits()` equals
  `0x7e00` on both x86_64 and aarch64 (verified in the `half` crate
  test suite). The parity test asserts this fixed bit pattern
  explicitly so that any future `half` bump that changes the
  payload is caught immediately.
- **Residual decode:** propagates the NaN bit pattern exactly.
  `f16` bits → `f32` bits round-trip preserves the qNaN payload in
  both directions.

### Alignment audit

- SIMD loads in the AVX2 path use `_mm256_loadu_ps` everywhere, not
  `_mm256_load_ps`. The `u` (unaligned) variant sidesteps the
  32-byte alignment requirement of the aligned load at a negligible
  cost on Haswell+ hosts. Every load site has a `// SAFETY:`
  comment pointing at this contract.
- **Debug-mode assertion:** in `debug_assertions` builds the AVX2
  kernels assert `ptr as usize % 4 == 0` (f32 alignment, which is
  guaranteed by the slice types anyway) — this is a sanity check
  against accidental byte-offset casts, not an alignment gate.
- **`Codebook::entries` alignment.** Phase 14 stores entries as
  `Arc<[f32]>` — heap-allocated. On glibc and jemalloc this is
  16-byte aligned in practice; it is **not** guaranteed 32-byte.
  We document explicitly that Phase 20 kernels use unaligned loads
  for codebook entries and do not attempt to re-allocate them for
  alignment.
- **Residual `Box<[u8]>`** is only 1-byte aligned. `f16::from_le_bytes`
  does not care about alignment, and the SIMD f16 conversion path
  goes through byte-by-byte reads into a scalar register before
  broadcasting, so no alignment requirement propagates to the
  residual buffer.

- [ ] **Step 15: Run workspace tests, clippy, fmt.**

- [ ] **Step 16: Commit**

```bash
git add rust/crates/tinyquant-core/src/codec/kernels \
        rust/crates/tinyquant-core/src/codec/dispatch.rs \
        rust/crates/tinyquant-core/tests/simd_parity.rs \
        rust/crates/tinyquant-core/tests/simd_dispatch_cache.rs \
        rust/crates/tinyquant-bruteforce/src/similarity_simd.rs \
        rust/crates/tinyquant-bench/benches/simd_kernels.rs \
        rust/crates/tinyquant-bench/baselines/simd_kernels.json \
        rust/xtask/src/cmd/simd.rs \
        .github/workflows/rust-ci.yml
git commit -m "feat(kernels): add SIMD dispatch with scalar parity and benchmarks"
```

### Clippy profile gotchas

Ported from [[design/rust/phase-14-implementation-notes|Phase 14
Implementation Notes]] §L7. The crate denies `clippy::pedantic +
nursery + unwrap_used + expect_used + panic + indexing_slicing +
cognitive_complexity`. Phase 20 adds SIMD-specific gotchas on top.

- **`unsafe_code` is allowed only in the three SIMD intrinsic files**
  — `kernels/avx2.rs`, `kernels/neon.rs`, `kernels/avx512.rs`. Every
  other file in `tinyquant-core` keeps `#![forbid(unsafe_code)]`.
  `dispatch.rs` stays safe code (the `OnceLock` does the heavy
  lifting).
- **`cast_ptr_alignment`** fires on intrinsic casts from `*const f32`
  to `*const __m256`. Wrap each occurrence with a narrow
  `#[allow(clippy::cast_ptr_alignment)]` **plus** a `// SAFETY:`
  comment referencing the unaligned-load contract in the
  [Alignment audit](#alignment-audit) section above.
- **`cast_precision_loss` / `cast_possible_truncation`** on
  `as f32` — narrow `#[allow]` on **benchmark helpers only**
  (`benches/simd_kernels.rs`). Production kernels must not lose
  precision implicitly.
- **`missing_safety_doc`** on every `unsafe fn` or
  `#[target_feature]` fn. The `/// # Safety` block names the
  required target feature (`avx2`, `fma`, `neon`) and any
  length/alignment precondition.
- **`indexing_slicing`** in the hot loop — pre-check bounds once
  and use `get_unchecked` inside the tight loop with a `// SAFETY:`
  comment. **Phase 14 reviewers preferred `chunks_exact(LANES)`**
  over `get_unchecked` for the readable path; document the choice
  per kernel. The rule: `chunks_exact` for the scalar tail loop and
  the portable path; `get_unchecked` only where `chunks_exact`
  generated a branch the optimizer could not remove.
- **`bool_to_int_using_if`** — use `u32::from(!slice.is_empty())`,
  not `if slice.is_empty() { 0 } else { 1 }`.
- **`trivially_copy_pass_by_ref`** — `f32`/`f64` helpers take
  values, not references.

## Acceptance criteria

- Every SIMD kernel is byte-identical to the scalar reference on
  10 000 random inputs **and** on the Phase 14 `codebook_10k_d64`
  training corpus **and** on seeded ChaCha proptest inputs.
- `simd_parity_under_every_dispatch` is green under every
  `DispatchKind` variant that the host CPU supports.
- `simd_dispatch_cache` confirms `OnceLock` semantics and that
  `dispatch::force` panics if called after the cache is populated.
- Runtime dispatch selects the best available ISA, caches it, and
  returns the same value for the lifetime of the process.
- Benchmark speedup over scalar is **at least 3×** on quantize and
  **5×** on cosine similarity at dim 1536. If not, the kernel is
  accepted with a note and Phase 21 tracks the delta.
- No allocations in the hot path.
- Miri runs clean on the scalar-only build via `rust-ci/miri-scalar`.
- Clippy + fmt clean under the full pedantic+nursery profile, with
  only the narrow `#[allow]` attributes listed above.
- CI matrix green per the
  [target-feature matrix](#target-feature-matrix) — scalar, AVX2,
  NEON Linux, NEON Darwin, AVX2 Darwin, AVX2 Windows. The
  AVX-512 advisory row is allowed to be red without blocking.
- **No `#[ignore]` on any cross-runner SIMD parity test.** This is
  a hard exit criterion because [[plans/rust/phase-22-pyo3-cabi-release|Phase 22]]
  §R22.10 depends on it — the release workflow refuses to tag if
  any `simd_parity_*` test is ignored. The only permitted ignore
  gate is the `avx512-advisory` feature flag on the AVX-512
  advisory row.
- **Phase 14 §CI follow-ups cleared.** The scalar deterministic
  `RotationMatrix::build` (use `faer::Parallelism::None`), the
  `1.78 → 1.81` toolchain bump in `rust-ci.yml`, and the
  `gh run list --workflow rust-ci.yml --branch main --limit 5`
  phase-exit check are either done in this phase or have a named
  follow-up PR merged before Phase 21 begins.

## CI integration

New top-level section driven by Phase 14 L1, L2, L3, L4.

- **New matrix entries in `rust-ci.yml`**, one per row of the
  [target-feature matrix](#target-feature-matrix). Each job runs
  `cargo test -p tinyquant-core --features simd` plus the parity
  integration test under the target triple and `RUSTFLAGS` combo.
- **LFS hydration — Phase 14 L2.** The SIMD parity tests re-use the
  Phase 14 `codebook_10k_d64_seed42` training fixture and the
  `bw ∈ {2, 4, 8}` quantize fixtures, all of which live in Git LFS.
  Every job's `actions/checkout@v4` step **must** carry
  `with: { lfs: true }`. The existing `rust-ci.yml` change from
  commit `13e888d` already does this for the current jobs; the new
  matrix rows inherit the pattern via a shared composite action.
- **Design-doc-vs-CI drift check — Phase 14 L3.** Add an xtask
  subcommand `cargo xtask simd audit` (file
  `rust/xtask/src/cmd/simd.rs`). It parses the target-feature
  matrix out of `docs/design/rust/simd-strategy.md` and the parity
  test list out of `docs/plans/rust/phase-20-simd-kernels.md` (this
  file), then greps `.github/workflows/rust-ci.yml` to confirm
  every row has a matching job. A missing job is a hard fail. This
  is the L3 remedy we promised.
- **Cross-runner parity check — Phase 14 L4.** Under **no
  circumstances** pin `RAYON_NUM_THREADS`, `JOBS`, runner label,
  or any other workflow-level environment variable as a workaround
  for a determinism bug. If the parity test fails across runners,
  **the kernel is broken**, not the workflow. The fix is in the
  kernel's reduction order or its NaN handling, not in
  `.github/workflows/`.
- **Runner-label host-CPU coverage — Phase 14 L4.** The
  `simd_parity_under_every_dispatch` test must be observed green on
  **at least two different host CPUs** before Phase 20 exits. We
  track this via runner labels in the workflow (GitHub exposes
  `runner.name` and we log `/proc/cpuinfo` headers at job start).
  A phase exit checklist item confirms two distinct CPU signatures
  in the last five successful runs on `main`.
- **CI health check — Phase 14 L1.** Phase 20 exit requires:
  1. Green runs across the full target-feature matrix on `main`
     after the merge push (not just on the PR branch — L1).
  2. `gh run list --workflow rust-ci.yml --branch main --limit 5`
     shows zero `completed failure` entries.
  3. The `simd_parity_under_every_dispatch` test is green on ≥2
     distinct host CPUs, as logged by the runner-label check above.

## Risks

The Phase 20 risk register is deliberately longer than most — this
is the load-bearing phase for determinism and performance both.

- **R20.1: Cross-runner SIMD determinism regression (L4 repeat).**
  A SIMD kernel ends up with a reduction order that differs by host
  CPU, and we only notice after Phase 22 ships. *Mitigation:*
  `simd_parity_under_every_dispatch` forced-variant sweep +
  mandatory scalar fallback + 2-host-CPU coverage gate on CI. Every
  reduction tree is hand-written and documented, not inferred from
  `Iterator::sum`.
- **R20.2: `std::simd` is not yet stable at the pinned toolchain.**
  If the MSRV slips before Phase 20 starts, `core::simd` stays
  nightly-gated. *Mitigation:* fall back to `core::arch::x86_64`
  intrinsics wrapped in a safe facade in `kernels/avx2.rs`, and to
  `core::arch::aarch64` in `kernels/neon.rs`. The portable path
  then becomes a fallback for non-x86/non-arm targets only.
  Revisit on each MSRV bump.
- **R20.3: NaN handling drifts between `_mm256_min_ps` and scalar
  `.min`.** `f32::min` in Rust returns the non-NaN operand when one
  side is NaN; `_mm256_min_ps` returns the second operand. These
  disagree. *Mitigation:* the scalar reference in
  `kernels/scalar.rs` uses an explicit `if v.is_nan() { … } else { … }`
  branch — **not** `.min` and **not** `std::cmp::Ord` (f32 is not
  Ord anyway). The parity test locks the behavior.
- **R20.4: f16 intrinsics missing on pre-Ivy Bridge.**
  `_mm256_cvtps_ph` requires F16C, which is not present on Sandy
  Bridge or earlier. *Mitigation:* use `half::f16` software
  conversion path on the SIMD residual kernels too — the scalar
  reference already does. Document intrinsic f16 conversion as a
  Phase 22 micro-optimization if we decide to raise the x86
  baseline.
- **R20.5: `cargo fuzz` becomes infeasible on SIMD kernels.**
  `cargo-fuzz`'s LLVM coverage instrumentation fights AVX2 target-
  feature gating. *Mitigation:* Phase 14 L6 applies — use
  ChaCha-seeded proptest via `rand_chacha::ChaCha20Rng::seed_from_u64(…)`
  instead of the `proptest` crate (which also pulled an MSRV-
  bumping dep in L6). Deterministic seeded loops cover the same
  invariant space.
- **R20.6: Bench baseline captured on a dev machine drifts from CI
  numbers.** The baseline committed in Phase 20 is a *direction*,
  not a *gate*. *Mitigation:* Phase 21 captures the CI-on-`main`
  baseline and adds the budget gate. Phase 20 explicitly says in
  the acceptance criteria that the 3×/5× targets are soft.
- **R20.7: AVX-512 kernel regresses perf due to downclocking on
  older Xeons.** Skylake-X through Cascade Lake throttle core
  frequency when 512-bit vector registers are active, so an
  "always-on" AVX-512 path can be slower end-to-end than AVX2.
  *Mitigation:* keep AVX-512 **off by default** behind the
  `avx512` feature flag; advisory CI job only. Document as a Phase
  22+ decision.
- **R20.8: Miri cannot instrument SIMD intrinsics.** Miri's
  interpreter has no model for `_mm256_loadu_ps` et al. *Mitigation:*
  scalar-only Miri job (`rust-ci/miri-scalar`). Document the blind
  spot in [Step 11a](#step-11a-miri-audit-of-unsafe-blocks) — the
  parity test is the real safety net for the SIMD path.

## Out of scope

- GPU kernels (CUDA, Metal, ROCm, SYCL, WebGPU).
- AVX-512 as the **default** dispatched path — it stays behind the
  `avx512` feature flag and the advisory CI job.
- Bf16 kernels (f16 only, via the `half` crate).
- ARM SVE / SVE2 (NEON only on aarch64 in Phase 20).
- Autotuning across kernel variants at runtime (the dispatch
  decision is static per-process, CPUID-only).
- Multi-socket NUMA-aware dispatch (single-socket assumption for
  the entire codec layer).
- Address-sanitizer / thread-sanitizer runs on SIMD kernels (see
  R20.8 / Step 11a).
- f16 intrinsic conversion via `_mm256_cvtps_ph` — software path
  only in Phase 20 (R20.4).

## See also

- [[plans/rust/phase-19-brute-force-pgvector|Phase 19]]
- [[plans/rust/phase-21-rayon-batch-benches|Phase 21]]
- [[design/rust/simd-strategy|SIMD Strategy]]
- [[design/rust/numerical-semantics|Numerical Semantics]]
- [[design/rust/memory-layout|Memory Layout]]
- [[design/rust/goals-and-non-goals|Goals and Non-Goals]]
- [[design/rust/testing-strategy|Testing Strategy]]
- [[design/rust/ci-cd|CI/CD]]
- [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]]
