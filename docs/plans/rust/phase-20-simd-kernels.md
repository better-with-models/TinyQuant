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

## Prerequisites

- Phase 19 complete (brute-force backend exists so cosine kernel
  has a consumer).
- `std::simd` available on the pinned toolchain (stable in 2026).

## Deliverables

### Files to create

| File | Purpose |
|------|---------|
| `rust/crates/tinyquant-core/src/codec/dispatch.rs` | ISA detection cache |
| `rust/crates/tinyquant-core/src/codec/kernels/mod.rs` | Module root |
| `rust/crates/tinyquant-core/src/codec/kernels/scalar.rs` | Reference scalar path |
| `rust/crates/tinyquant-core/src/codec/kernels/portable.rs` | `std::simd` |
| `rust/crates/tinyquant-core/src/codec/kernels/avx2.rs` | AVX2 specializations |
| `rust/crates/tinyquant-core/src/codec/kernels/neon.rs` | NEON specializations |
| `rust/crates/tinyquant-bruteforce/src/similarity_simd.rs` | SIMD cosine |
| `rust/crates/tinyquant-core/tests/simd_parity.rs` | Scalar-vs-SIMD byte parity |
| `rust/crates/tinyquant-bench/benches/simd_kernels.rs` | Before/after numbers |

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

- [ ] **Step 2: Implement `dispatch.rs`**

Per [[design/rust/simd-strategy|SIMD Strategy]] §Runtime dispatch.

- [ ] **Step 3: Implement `kernels/scalar.rs`**

Extracted from phase 14's `Codebook::quantize_into`. This is the
canonical reference; no changes to its behavior.

- [ ] **Step 4: Implement `kernels/portable.rs`**

Using `std::simd::f32x8` for the quantize-4bit linear search. See
[[design/rust/simd-strategy|SIMD Strategy]] §Portable SIMD fallback.

- [ ] **Step 5: Implement `kernels/avx2.rs`**

`#[target_feature(enable = "avx2,fma")]` kernels with `// SAFETY:`
comments for every `unsafe` block. Must produce byte-identical
output to scalar.

- [ ] **Step 6: Implement `kernels/neon.rs`**

Aarch64-only path using `core::arch::aarch64` intrinsics or portable
`std::simd` (which lowers to NEON).

- [ ] **Step 7: Wire dispatch into `Codebook::quantize_into`**

The public API stays identical; internally it calls
`dispatch::quantize(...)`.

- [ ] **Step 8: Run parity tests — expect pass** (possibly after
iteration).

- [ ] **Step 9: Cosine similarity SIMD kernel**

In `tinyquant-bruteforce/src/similarity_simd.rs`, add `cosine_avx2`
and `cosine_portable`. `BruteForceBackend::search` uses the
dispatched variant.

- [ ] **Step 10: SIMD residual conversion**

`compute_residual_simd` and `apply_residual_simd` using
`half::f16::from_f32` via `std::simd`. Parity against scalar.

- [ ] **Step 11: Allocation audit on hot path**

Confirm no allocations inside `dispatch::quantize` or cosine kernel.

- [ ] **Step 12: Benchmark suite**

Add `tinyquant-bench/benches/simd_kernels.rs` with:

- Scalar vs portable vs AVX2 quantize at 4-bit for dims 64, 768, 1536.
- Scalar vs AVX2 cosine at dim 1536.
- Scalar vs AVX2 residual encode at dim 1536.

Record baseline numbers into `baselines/main.json`.

- [ ] **Step 13: Update CI to run parity tests with
`RUSTFLAGS="-C target-feature=+avx2,+fma"`**

Add a matrix entry for this flag combination in `rust-ci.yml`.

- [ ] **Step 14: NaN tie-break test**

Feed a few NaN values to every kernel and assert they produce the
same index (0 or whatever the scalar path produces) in both scalar
and SIMD.

- [ ] **Step 15: Run workspace tests, clippy, fmt.**

- [ ] **Step 16: Commit**

```bash
git add rust/crates/tinyquant-core/src/codec/kernels \
        rust/crates/tinyquant-core/src/codec/dispatch.rs \
        rust/crates/tinyquant-bruteforce/src/similarity_simd.rs \
        rust/crates/tinyquant-bench/benches/simd_kernels.rs \
        .github/workflows/rust-ci.yml
git commit -m "feat(kernels): add SIMD dispatch with scalar parity and benchmarks"
```

## Acceptance criteria

- Every SIMD kernel is byte-identical to the scalar reference on
  10 000 random inputs.
- Runtime dispatch selects the best available ISA and caches it.
- Benchmark speedup over scalar is at least 3× on quantize and 5×
  on cosine similarity at dim 1536.
- No allocations in the hot path.
- Miri runs clean on the `unsafe` blocks.
- Clippy + fmt clean.

## Out of scope

- GPU kernels.
- AVX-512 specialization (reserved for a later release after AVX2
  is proven).

## See also

- [[plans/rust/phase-19-brute-force-pgvector|Phase 19]]
- [[plans/rust/phase-21-rayon-batch-benches|Phase 21]]
- [[design/rust/simd-strategy|SIMD Strategy]]
