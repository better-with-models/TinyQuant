---
title: "Fix: RotationMatrix rebuilt per vector in compress_batch_parallel"
tags:
  - plans
  - rust
  - fix
  - rotation-cache
  - batch
date-created: 2026-04-14
status: draft
category: planning
---

# Fix: RotationMatrix rebuilt per vector in compress_batch_parallel

> [!important] Root cause
> `compress_batch_parallel` calls `Codec::new().compress()` per row, which
> calls `RotationMatrix::from_config(config)` → `Self::build(seed, dim)` — a
> full O(d³) SVD — on every vector. For d=1536 this dominates all other work
> by orders of magnitude and makes the batch path algorithmically O(N·d³)
> instead of the intended O(d³ + N·d²).

> [!note] Reference docs
> - [[design/rust/parallelism|Parallelism and Concurrency]]
> - [[design/rust/benchmark-harness|Benchmark Harness]]
> - [[plans/rust/phase-21-rayon-batch-benches|Phase 21 Plan]]

## Problem

### Code locations

| File | Line | Problem |
|------|------|---------|
| `crates/tinyquant-core/src/codec/batch.rs` | ~101 | `Codec::new().compress(slice, config, codebook)` called per row |
| `crates/tinyquant-core/src/codec/service.rs` | ~61 | `let rotation = RotationMatrix::from_config(config)` inside single-vector `compress()` |
| `crates/tinyquant-core/src/codec/rotation_matrix.rs` | ~49 | `from_config` calls `Self::build(seed, dim)` with no cache lookup |

### Measured impact

On branch `fix/rotation-cache-compress-path`, with d=1536 and 256-vector batch:

| Metric | Value |
|--------|-------|
| Rust batch throughput (release, t=1) | ~1.3 vec/s |
| Python reference throughput | ~3,600 vec/s |
| Slowdown factor | ~2,800× |

Rust kernel benchmarks (quantize, dequantize, cosine) are already 5–195× faster
than Python. The regression is entirely in the per-vector SVD, which costs
roughly 750 ms per call at d=1536 on the bench machine.

### Why this is a correctness error, not a performance oversight

`compress_batch_parallel` exists specifically to amortize per-batch setup costs
across the row dimension. A rotation matrix is deterministically fixed by
`(seed, dim)` — both are constants for a given batch call. Computing it N times
instead of once is functionally equivalent but algorithmically wrong: it makes
the batch path O(N·d³) rather than O(d³ + N·d²). The `RotationCache` type was
introduced in Phase 13 precisely for this purpose but was never wired into the
batch path.

## Scope

This fix is **Rust-only**. The Python reference implementation (`tinyquant_py_reference`)
uses `@functools.lru_cache` on `_cached_build` and is correct. No changes to
the Python reference are proposed.

## Fix plan

### Step 1 — Pre-compute rotation before the parallel loop in `batch.rs`

```rust
// Before (per-vector SVD, O(N·d³)):
rows.par_iter().map(|slice| {
    Codec::new().compress(slice, config, codebook)
})

// After (one SVD before the loop, O(d³ + N·d²)):
let rotation = RotationMatrix::from_config(config);
rows.par_iter().map(|slice| {
    codec.compress_with_rotation(slice, config, codebook, &rotation)
})
```

### Step 2 — Add `Codec::compress_with_rotation` in `service.rs`

Add a package-private (or `pub(crate)`) helper that accepts a pre-built
`&RotationMatrix` instead of calling `from_config` internally:

```rust
pub(crate) fn compress_with_rotation(
    &self,
    vector: &[f32],
    config: &CodecConfig,
    codebook: &Codebook,
    rotation: &RotationMatrix,
) -> Result<CompressedVector, CodecError> { … }
```

The public `compress()` API is unchanged; it continues to call `from_config`
internally for single-vector callers.

### Step 3 — Mirror the fix in `decompress_batch_into`

`decompress_into` in `service.rs:151` has the same pattern. Apply the same
pre-compute-and-pass approach in whatever batch decompress path calls it.

### Step 4 — Wire `RotationCache` for repeated cross-batch calls (optional)

If the same `(seed, dim)` config is used across many independent batch calls
(e.g. corpus-wide compression), `RotationCache` can be populated after Step 1
so the one-time SVD cost is amortized across all batches. This is lower
priority than Steps 1–3, which fix the per-row regression.

## Acceptance criteria

- [ ] Single-vector `Codec::compress()` public API unchanged
- [ ] `compress_batch_parallel` pre-computes rotation exactly once per call
- [ ] `decompress_batch_into` pre-computes rotation exactly once per call
- [ ] All existing Phase 15/16 fixture-parity tests pass unchanged
- [ ] Calibration tests pass with `--release` (score fidelity and compression
      ratio gates unchanged)
- [ ] Rust batch throughput on 256×1536 in release mode reaches at minimum
      1,000 vec/s (i.e. eliminates the per-vector SVD regression)

## Not in scope

- Changes to `tinyquant_py_reference`
- Changes to Python calibration tests
- Changing the public `Codec::compress` signature
- Adding new public API surface beyond `compress_with_rotation` (which is
  `pub(crate)`)
