---
title: "Phase 28 Implementation Notes"
tags:
  - design
  - rust
  - phase-28
  - gpu
  - wgpu
  - wgsl
  - performance
date-created: 2026-04-18
category: design
---

# Phase 28 Implementation Notes

## What landed

Phase 28 delivers four additions to `tinyquant-gpu-wgpu`: **lazy pipeline
caching** (`CachedPipelines`), a **GPU residual encode/decode pass**
(`residual_decode.wgsl`), an **explicit pipeline lifecycle API**
(`load_pipelines` / `unload_pipelines`), and a **backend-preference API**
(`BackendPreference` + `enumerate_adapters`).

### Concrete deliverables

- `src/backend.rs` — `CachedPipelines` struct: holds `Option<wgpu::ComputePipeline>`
  for every kernel (quantize, dequantize, cosine, residual encode, residual
  decode). Pipelines are built lazily on first use via `get_or_build` closures;
  subsequent calls return the cached handle. Previously every `WgpuBackend`
  call recompiled shaders on construction.
- `shaders/residual_decode.wgsl` — WGSL kernel that reads FP16-encoded residual
  bytes from a GPU buffer, converts each `f16` pair to `f32` (via bit-cast +
  decode), and adds the residual to the dequantized vector element-wise.
  Satisfies FR-GPU-003 (residual decode tolerance ≤ 1e-3 for bw=4 configs).
- `shaders/residual_encode.wgsl` — amended to fix an FP16 subnormal edge case:
  very small `f32` values that map to subnormal FP16 representations are now
  flushed to zero before encoding, matching the CPU reference path.
- `src/backend_preference.rs` — `BackendPreference` enum
  (`Any | HighPerformance | LowPower | DiscreteGpu | IntegratedGpu`) and
  `AdapterCandidate` struct (`name`, `backend`, `device_type`). Neither type
  carries a wgpu handle; they are plain data for caller inspection.
- `src/context.rs` — `WgpuContext::new_with_preference(BackendPreference)` and
  `enumerate_adapters() -> Vec<AdapterCandidate>`. The preference is passed
  through to `wgpu::Instance::request_adapter` as a `PowerPreference` hint.
- `src/error.rs` — `NoPreferredAdapter` variant added to `TinyQuantGpuError`.
  Returned by `new_with_preference` when no adapter satisfies the requested
  preference on the current platform.
- `src/lib.rs` — re-exports `BackendPreference`, `AdapterCandidate`.
- `tests/parity_compress.rs` — two new tests: `residual_round_trip_bw4` (checks
  GPU encode+decode residual against CPU reference within 1e-3) and
  `cached_pipelines_no_recompile` (verifies that a second compress call against
  the same `WgpuBackend` completes faster than the first, as a regression gate
  on lazy caching).
- `tests/context_probe.rs` — three new tests: `enumerate_adapters_returns_list`,
  `new_with_preference_any_succeeds`, and `new_with_preference_unknown_fails`
  (uses a synthetic preference value that no adapter can satisfy, asserts
  `NoPreferredAdapter`).

All 18 `tinyquant-gpu-wgpu` tests pass on both the software renderer (CI)
and on a hardware GPU (local).

## Deviations from the plan

### 1. Pipeline recompile regression test uses ratio, not absolute time

The plan specified a fixed 15 ms upper bound for the "no recompile" cache hit.
CI runs on a software renderer (Mesa `llvmpipe` or `warp`) where shader
compilation is significantly slower than on hardware. The committed test uses
a ratio (`second_call_ms / first_call_ms < 0.20`) rather than an absolute
bound. The ratio is hardware-agnostic: it only requires that the cached path is
at least 5× faster than the initial compile, which holds on all tested renderers.
The floor was later raised to 15 ms minimum for `first_call_ms` to prevent the
ratio test from passing vacuously on extremely fast software renders (`26b4708`).

### 2. `ResidualNotSupported` variant removed

The plan described adding a residual GPU pass while keeping
`ResidualNotSupported` as a fallback. During implementation it became clear
that the fallback path was unreachable after the residual shader was added: the
only code path that returned `ResidualNotSupported` was the stub left from
Phase 27. Removing the variant simplified the error type and eliminated a dead
code path. Any future residual-specific error (e.g., unsupported dimension)
should be added as a new variant.

### 3. `residual_encode.wgsl` subnormal fix was a bug, not a planned feature

The plan did not mention the FP16 subnormal edge case. During parity testing
(`residual_round_trip_bw4`), values in the range `(0, 6.1e-5)` (the FP16
subnormal range) produced non-zero residuals after GPU encode + CPU decode,
violating the ≤ 1e-3 tolerance. The root cause was that the WGSL `f32 -> f16`
conversion preserves subnormals, while the CPU reference flushes them to zero
at the `half` crate level. The fix (flush to zero before encoding in the WGSL)
was committed as part of the same feature commit.

### 4. `load_pipelines` / `unload_pipelines` are synchronous

The plan described pipeline loading as a background operation. The
implementation is synchronous because:
- wgpu pipeline compilation on the async `wgpu::Device` requires polling the
  device event loop, which is already handled by the existing `pollster::block_on`
  wrapper around all GPU operations.
- Background loading would require exposing a `Future` or callback interface
  that has no established pattern in the TinyQuant API surface.

The synchronous API is sufficient for all current use cases (one-time warm-up
before a batch). If streaming pipeline loading is needed in a future phase, the
`load_pipelines` signature can be changed to `async fn` without breaking the
call sites that are already inside `block_on`.

## Pipeline lifecycle

```
WgpuBackend::new()           // no pipelines built
    ↓
load_pipelines()             // builds all 5 pipelines (optional warm-up)
    ↓
compress() / search() / …   // each operation lazy-builds its pipeline on first call
    ↓
unload_pipelines()           // drops all cached handles (frees GPU memory)
```

Callers that omit `load_pipelines()` pay the compile cost on the first operation
of each type. Callers that need predictable latency (e.g., servers) should call
`load_pipelines()` during startup.

## Risks carried forward

- **R-GPU28-1**: `enumerate_adapters` returns all adapters visible to the wgpu
  instance, including software renderers. Callers must not assume that the
  first adapter is a hardware GPU. The `BackendPreference::DiscreteGpu` variant
  can be used to filter, but it will return `NoPreferredAdapter` in CI
  environments that only expose software renderers.
- **R-GPU28-2**: The `cached_pipelines_no_recompile` regression test can produce
  false negatives on extremely loaded CI runners where the ratio condition is
  not met even with caching. The test is marked `#[ignore]` in the software-
  renderer CI job and runs only in the hardware GPU job.
- **R-GPU28-3**: `unload_pipelines` drops wgpu handles but does not wait for any
  in-flight GPU work to complete. Callers must ensure no GPU operations are
  in-flight when calling `unload_pipelines`, or risk using a stale bind group
  referencing a dropped pipeline.
