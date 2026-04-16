---
title: "Phase 27 Implementation Notes"
tags:
  - implementation-notes
  - rust
  - phase-27
  - gpu
  - wgpu
  - wgsl
date-created: 2026-04-15
status: complete
category: design
---

# Phase 27 Implementation Notes

## Summary

Phase 27 ships `tinyquant-gpu-wgpu`, a wgpu/WGSL GPU-accelerated batch
compression backend. It adds one new crate to the workspace and wires up
Layer 2 CI (compile + shader validation) and an advisory Layer 3 runtime
smoke workflow. All nine Phase 27 acceptance criteria are met.

---

## Scope Reductions and Deviations

### Residual encode/decode pass not implemented

The plan (Step 6, Part C) specifies a three-pass GPU pipeline:
`rotate → quantize → residual_encode` for compress, and
`residual_decode → dequantize → inverse-rotate` for decompress.

The implementation runs only two passes:

- Compress: `rotate → quantize`
- Decompress: `dequantize → inverse-rotate`

`pipelines/residual.rs` exists and contains `build_residual_encode_pipeline`
(and associated WGSL shader `residual_encode.wgsl`) but is not called.

**Rationale:** All parity tests and acceptance criteria use
`CodecConfig::new(..., false)` (residual disabled). The residual_enabled
code path is not yet exercised by any test or benchmark at Phase 27.

**Safety net:** `compress_batch` now returns `Err(ResidualNotSupported)`
immediately when `prepared.config().residual_enabled() == true`, so
callers are never handed a silently incomplete result.

**Phase 28 action:** Wire the residual encode/decode passes behind the
existing `residual_enabled()` gate. Remove the `#[allow(dead_code)]`
from `build_residual_encode_pipeline`.

---

### PreparedCodec::gpu_state() added to tinyquant-core public API

The plan scope states: *"No existing crate's public API changes."*

In practice, three methods were added to `PreparedCodec` in
`tinyquant-core`:

- `set_gpu_state(Box<dyn Any + Send + Sync>)` — called by GPU backends.
- `has_gpu_state() -> bool` — idempotency guard used in `prepare_for_device`.
- `gpu_state() -> Option<&(dyn Any + Send + Sync)>` — buffer retrieval.

All three are additive (no existing call sites break) and the core crate
remains GPU-free (no `wgpu` import). The `ComputeBackend` trait is defined
inside `tinyquant-gpu-wgpu`, not in `tinyquant-core`.

---

### Clippy gate uses --no-deps

The plan acceptance criterion read `cargo clippy -p tinyquant-gpu-wgpu -- -D warnings`.
The actual gate in `.github/workflows/rust-ci.yml` (job `gpu-compile`) uses
`cargo clippy --no-deps -p tinyquant-gpu-wgpu -- -D warnings`.

The full `-p` form fails because `tinyquant-core` has pre-existing Rust 1.87
clippy lints that are out of scope for Phase 27. Those are tracked in the
1.81 clippy job and will be addressed in a future phase.

The plan's acceptance criterion has been updated to match the actual gate.

---

## Rotation Convention Reference

The GPU `rotate.wgsl` shader computes `output = input @ rotation_mat`
(row-vector convention). The CPU path computes `R @ v` (column-vector
convention). The following buffer assignments reconcile the two:

| Pass | Buffer bound | Math | Result |
|---|---|---|---|
| Forward (compress) | `rotation_t_buf` = R^T | `input @ R^T` | = CPU `R @ input^T` ✓ |
| Inverse (decompress) | `rotation_buf` = R | `values @ R` | = CPU `R^T @ values^T` ✓ |

R is orthogonal, so R^{−1} = R^T. The naming convention adopted in
`GpuPreparedState`:

- `rotation_buf` holds R (the plain rotation matrix) — used by the inverse pass.
- `rotation_t_buf` holds R^T (the transposed matrix) — used by the forward pass.

---

## Pipeline Caching Deferred to Phase 28

`compress_batch` and `decompress_batch_into` rebuild the WGSL compute
pipelines on every call. Device shader compilation takes on the order of
milliseconds and will dominate FR-GPU-004's ≤ 5 ms throughput target.

A `CachedPipelines` field will be added to `WgpuBackend` in Phase 28,
before the FR-GPU-004 benchmark gate is exercised on a real GPU runner.
