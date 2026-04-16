# rust/crates/tinyquant-gpu-wgpu

`tinyquant-gpu-wgpu` is the wgpu/WGSL GPU-accelerated batch compression backend
for TinyQuant. It ships one crate that is intentionally `publish = false` — it
is a workspace-internal accelerator, not a standalone library.

Minimum Rust version: **1.87** (required for `u32::div_ceil` in stable).

## What lives here

- `src/lib.rs` — crate root; exports `WgpuBackend`, `TinyQuantGpuError`,
  `ComputeBackend` trait, and `GPU_BATCH_THRESHOLD`.
- `src/backend.rs` — `WgpuBackend`: implements `compress_batch`,
  `decompress_batch_into`, and `prepare_for_device` via three WGSL compute passes.
- `src/context.rs` — `WgpuContext`: instance → adapter → device + queue setup.
- `src/error.rs` — `TinyQuantGpuError` enum.
- `src/prepared.rs` — `GpuPreparedState`: device buffers for the rotation matrix
  and codebook, attached to `PreparedCodec` as opaque state.
- `src/pipelines/` — `rotate`, `quantize`, and `residual` pipeline builders.
- `shaders/` — WGSL compute shaders: `rotate.wgsl`, `quantize.wgsl`,
  `dequantize.wgsl`, `residual_encode.wgsl`.
- `tests/` — integration tests: `context_probe.rs` (FR-GPU-002 adapter
  detection + shader validation), `batch_threshold.rs` (FR-GPU-005),
  `parity_compress.rs` (FR-GPU-003/006 GPU–CPU parity).

## How this area fits the system

`tinyquant-gpu-wgpu` is an optional accelerator layer above `tinyquant-core`.
`tinyquant-core` stays GPU-free; the three methods added to `PreparedCodec`
(`set_gpu_state`, `has_gpu_state`, `gpu_state`) store opaque state via
`Box<dyn Any + Send + Sync>` so the core crate imports nothing from wgpu.

GPU offload is only attempted for batches at or above `GPU_BATCH_THRESHOLD`
(default 512 rows); below that threshold the host↔device transfer overhead
exceeds the compute savings.

The GPU pipeline for compress runs two passes:

1. **Rotate** (`rotate.wgsl`) — `input @ R^T` (equivalent to CPU `R @ v`).
2. **Quantize** (`quantize.wgsl`) — nearest-codebook-entry lookup → indices.

Decompress runs the reverse:

1. **Dequantize** (`dequantize.wgsl`) — `codebook[index]` lookup → values.
2. **Inverse-rotate** (`rotate.wgsl` with `R` bound) — `values @ R` (equivalent to CPU `R^T @ v`).

Residual encode/decode is deferred to Phase 28; `compress_batch` returns
`Err(ResidualNotSupported)` for `residual_enabled = true` configs.

## Common edit paths

- **New WGSL shader** — add to `shaders/`, add a corresponding pipeline builder
  in `src/pipelines/`, wire into `backend.rs`.
- **New error variant** — `src/error.rs`; update `backend.rs` call sites.
- **Pipeline caching (Phase 28)** — add a `CachedPipelines` field to
  `WgpuBackend` in `src/backend.rs`; remove the per-call
  `build_*_pipeline` calls.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
