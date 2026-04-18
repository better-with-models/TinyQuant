# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-gpu-wgpu`

This is the wgpu/WGSL GPU-accelerated batch compression backend for TinyQuant.
It is `publish = false` and requires Rust 1.87. It depends on `tinyquant-core`
but does not modify it — GPU state is stored in `PreparedCodec` as opaque
`Box<dyn Any + Send + Sync>` to keep the core crate GPU-free.

## What this area contains

- primary responsibility: implement the `ComputeBackend` trait using wgpu +
  WGSL compute shaders; compress and decompress embedding batches on GPU; and
  perform GPU-resident corpus nearest-neighbour search (`cosine_topk`).
- main entrypoints: `src/lib.rs` (public API surface), `src/backend.rs`
  (`WgpuBackend` — compress/decompress/search/pipeline-lifecycle),
  `src/backend_preference.rs` (`BackendPreference` enum, `AdapterCandidate`
  struct), `src/context.rs` (`WgpuContext::new_with_preference`,
  `enumerate_adapters`).
- common changes: new or updated WGSL shaders, pipeline wiring, error variants,
  GPU buffer layout for new codec or search features.

## Layout

```text
tinyquant-gpu-wgpu/
├── shaders/
│   ├── rotate.wgsl
│   ├── quantize.wgsl
│   ├── dequantize.wgsl
│   ├── residual_encode.wgsl
│   ├── residual_decode.wgsl    ← Phase 28: residual decode pass
│   └── cosine_topk.wgsl        ← Phase 27.5: corpus search scoring
├── src/
│   ├── lib.rs
│   ├── backend.rs
│   ├── backend_preference.rs   ← Phase 28: BackendPreference + AdapterCandidate
│   ├── context.rs
│   ├── error.rs
│   ├── prepared.rs              ← GpuPreparedState + GpuCorpusState
│   └── pipelines/
│       ├── mod.rs
│       ├── rotate.rs
│       ├── quantize.rs
│       ├── residual.rs
│       └── search.rs            ← Phase 27.5: cosine_topk pipeline
├── benches/
│   └── throughput_search.rs    ← FR-GPU-004 criterion bench
├── tests/
│   ├── batch_threshold.rs
│   ├── context_probe.rs
│   ├── parity_compress.rs
│   └── parity_search.rs        ← Phase 27.5: CPU vs GPU top-k parity
├── Cargo.toml
├── README.md
└── AGENTS.md
```

## Common workflows

### Add a new WGSL shader stage

1. Write the shader in `shaders/<name>.wgsl`.
2. Add a `build_<name>_pipeline` function in `src/pipelines/<name>.rs` (or an
   existing module if the stage is logically grouped with one).
3. Wire the pipeline into `compress_batch` or `decompress_batch_into` in
   `src/backend.rs`.
4. Add a shader-compilation test in `tests/context_probe.rs`.

### Add a new error variant

1. Add the variant to `TinyQuantGpuError` in `src/error.rs`.
2. Update any `backend.rs` call site that should produce the new variant.
3. Add a test if the variant is reachable from a public code path.

### Update GPU buffer layout

1. Update `GpuPreparedState` (codec) or `GpuCorpusState` (search) in
   `src/prepared.rs`.
2. Update `prepare_for_device` / `prepare_corpus_for_device` in `src/backend.rs`
   to populate the new field.
3. Update bind group construction in the affected pass in `src/backend.rs`.

### Extend corpus search

1. Corpus is uploaded once via `WgpuBackend::prepare_corpus_for_device`.
   The `GpuCorpusState` (in `src/prepared.rs`) holds the device buffer.
   Re-uploading always replaces the resident corpus; idempotency tracking is
   the caller's responsibility.
2. `cosine_topk` (in `src/backend.rs`) takes `&mut self` — it lazily builds
   and caches the compute pipeline in `WgpuBackend::search_pipeline` on first
   call.  Scores a query against the resident corpus, reads back all `n_rows`
   scores, and selects top-k on the host.
3. Passing `top_k == 0` returns `Err(TinyQuantGpuError::InvalidTopK)`.
4. Results are sorted descending by score, then ascending by `vector_id`
   (string-numeric row index) as a tiebreaker — matching the
   `SearchResult: Ord` contract defined in `tinyquant-core`.
5. Row indices (0-based integers rendered as strings) serve as `vector_id`s.
6. Throughput target: ≤ 5 ms for 10 000 rows × dim=1536 on RTX 3060-class
   hardware (FR-GPU-004).  Run `cargo bench -p tinyquant-gpu-wgpu` on a GPU
   runner to collect evidence.

### Manage the pipeline lifecycle

1. All pipelines (rotate, quantize, dequantize, residual enc/dec, search) are
   lazily compiled on first use and cached in `WgpuBackend::pipelines` /
   `WgpuBackend::search_pipeline`.
2. To precompile all pipelines upfront, call `WgpuBackend::load_pipelines()`
   after construction.  This eliminates first-call shader-compilation latency.
3. To release GPU shader memory when the backend is idle, call
   `WgpuBackend::unload_pipelines()`.  The device/queue remain live; pipelines
   rebuild lazily on next use.
4. `WgpuBackend::pipelines_loaded()` returns `true` only when all six
   pipelines are cached and the backend has a live adapter.

### Select or enumerate adapters

1. Use `WgpuBackend::new_with_preference(pref)` to request a specific backend
   (Vulkan, Metal, DX12) or performance class (HighPerformance, LowPower).
2. Use `WgpuBackend::enumerate_adapters(pref)` to list available adapters
   without creating a device.  The first entry is what
   `new_with_preference(pref)` would select.
3. `BackendPreference` and `AdapterCandidate` are public types re-exported
   from `src/lib.rs`.  Both live in `src/backend_preference.rs`.
4. `BackendPreference::to_backends()` and `to_power_preference()` are
   `pub(crate)` helpers used by `WgpuContext`; do not make them public.
5. `BackendPreference::Software` maps to `wgpu::Backends::GL`
   (`force_fallback_adapter: true` in `RequestAdapterOptions`) — it selects
   a CPU-emulated renderer, never a physical GPU.

## Invariants — Do Not Violate

- `tinyquant-core` must remain GPU-free. Do not add `wgpu` as a dependency of
  `tinyquant-core`. GPU state travels as `Box<dyn Any + Send + Sync>`.
- Every `.rs` file must open with a `//!` module docstring. Public symbols must
  carry their own doc comments.
- After Phase 28: `compress_batch` must succeed (not return
  `ResidualNotSupported`) for `residual_enabled=true` configs.  The
  `ResidualNotSupported` variant remains in `TinyQuantGpuError` for any
  future use but must not be returned by the current compress/decompress path.
- `BackendPreference::to_backends()` and `to_power_preference()` are
  `pub(crate)` — keep them out of the public API.
- Do not invent APIs, invariants, or performance claims not supported by the
  actual code.
- `publish = false` must remain in `Cargo.toml`; this crate is not released
  independently.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../AGENTS.md)
- [docs/plans/rust/phase-27-wgpu-wgsl-kernels.md](../../../docs/plans/rust/phase-27-wgpu-wgsl-kernels.md)
- [docs/plans/rust/phase-27.5-resident-corpus-search.md](../../../docs/plans/rust/phase-27.5-resident-corpus-search.md)
- [docs/plans/rust/phase-28-wgpu-pipeline-caching.md](../../../docs/plans/rust/phase-28-wgpu-pipeline-caching.md)
- [docs/design/rust/phase-27-implementation-notes.md](../../../docs/design/rust/phase-27-implementation-notes.md)
