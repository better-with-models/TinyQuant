# rust/crates/tinyquant-gpu-wgpu

`tinyquant-gpu-wgpu` is the wgpu/WGSL GPU-accelerated batch compression backend
for TinyQuant. It ships one crate that is intentionally `publish = false` — it
is a workspace-internal accelerator, not a standalone library.

Minimum Rust version: **1.87** (required for `u32::div_ceil` in stable).

## What lives here

- `src/lib.rs` — crate root; exports `WgpuBackend`, `TinyQuantGpuError`,
  `ComputeBackend` trait, and `GPU_BATCH_THRESHOLD`.
- `src/backend.rs` — `WgpuBackend`: implements `compress_batch`,
  `decompress_batch_into`, `prepare_for_device`, `prepare_corpus_for_device`,
  and `cosine_topk`. Private helpers `create_readback_buf` and `poll_map_read`
  encapsulate the GPU readback pattern shared across all three compute methods.
- `src/context.rs` — `WgpuContext`: instance → adapter → device + queue setup.
- `src/error.rs` — `TinyQuantGpuError` enum; includes `InvalidTopK` (returned
  when `top_k == 0` is passed to `cosine_topk`).
- `src/prepared.rs` — `GpuPreparedState`: device buffers for the rotation matrix
  and codebook, attached to `PreparedCodec` as opaque state. `GpuCorpusState`:
  device buffer for the FP32 corpus used by `cosine_topk`.
- `src/pipelines/` — `rotate`, `quantize`, `residual`, and `search` pipeline builders.
- `shaders/` — WGSL compute shaders: `rotate.wgsl`, `quantize.wgsl`,
  `dequantize.wgsl`, `residual_encode.wgsl`, `cosine_topk.wgsl`.
- `tests/` — integration tests: `context_probe.rs` (FR-GPU-002 adapter
  detection + shader validation), `batch_threshold.rs` (FR-GPU-005),
  `parity_compress.rs` (FR-GPU-003/006 GPU–CPU parity),
  `parity_search.rs` (FR-GPU-007/008 GPU–CPU top-k parity + descending-score order).

## How this area fits the system

`tinyquant-gpu-wgpu` is an optional accelerator layer above `tinyquant-core`.
`tinyquant-core` stays GPU-free; the three methods added to `PreparedCodec`
(`set_gpu_state`, `has_gpu_state`, `gpu_state`) store opaque state via
`Box<dyn Any + Send + Sync>` so the core crate imports nothing from wgpu.

GPU offload is only attempted for batches at or above `GPU_BATCH_THRESHOLD`
(default 512 rows); below that threshold the host↔device transfer overhead
exceeds the compute savings.

The GPU pipeline for compress runs up to three passes:

1. **Rotate** (`rotate.wgsl`) — `input @ R^T` (equivalent to CPU `R @ v`).
2. **Quantize** (`quantize.wgsl`) — nearest-codebook-entry lookup → indices.
3. **Residual encode** (`residual_encode.wgsl`) — when `residual_enabled`,
   stores `rotated − codebook[index]` as packed fp16 pairs.

Decompress runs the reverse (with an optional residual step):

1. **Dequantize** (`dequantize.wgsl`) — `codebook[index]` lookup → values.
2. **Residual decode** (`residual_decode.wgsl`, when the batch carries a
   residual) — unpack fp16 pairs and add in place into `values`.
3. **Inverse-rotate** (`rotate.wgsl` with `R` bound) — `values @ R`
   (equivalent to CPU `R^T @ v`).

## Common edit paths

- **New WGSL shader** — add to `shaders/`, add a corresponding pipeline builder
  in `src/pipelines/`, wire into `backend.rs`.
- **New error variant** — `src/error.rs`; update `backend.rs` call sites.
- **Corpus search pipeline** — the `cosine_topk` pipeline is lazily built and
  cached in `WgpuBackend::search_pipeline` on first call. Compress and
  decompress pipelines are still rebuilt per-call; caching those is deferred
  to Phase 28 (TODO comments in `backend.rs`).

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
