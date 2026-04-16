# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-gpu-wgpu`

This is the wgpu/WGSL GPU-accelerated batch compression backend for TinyQuant.
It is `publish = false` and requires Rust 1.87. It depends on `tinyquant-core`
but does not modify it — GPU state is stored in `PreparedCodec` as opaque
`Box<dyn Any + Send + Sync>` to keep the core crate GPU-free.

## What this area contains

- primary responsibility: implement the `ComputeBackend` trait using wgpu +
  WGSL compute shaders; compress and decompress embedding batches on GPU.
- main entrypoints: `src/lib.rs` (public API surface), `src/backend.rs`
  (`WgpuBackend` implementation).
- common changes: new or updated WGSL shaders, pipeline wiring, error variants,
  GPU buffer layout for new codec features.

## Layout

```text
tinyquant-gpu-wgpu/
├── shaders/
│   ├── rotate.wgsl
│   ├── quantize.wgsl
│   ├── dequantize.wgsl
│   └── residual_encode.wgsl
├── src/
│   ├── lib.rs
│   ├── backend.rs
│   ├── context.rs
│   ├── error.rs
│   ├── prepared.rs
│   └── pipelines/
│       ├── mod.rs
│       ├── rotate.rs
│       ├── quantize.rs
│       └── residual.rs
├── tests/
│   ├── batch_threshold.rs
│   ├── context_probe.rs
│   └── parity_compress.rs
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

1. Update `GpuPreparedState` in `src/prepared.rs`.
2. Update `prepare_for_device` in `src/backend.rs` to populate the new field.
3. Update bind group construction in the affected pass in `src/backend.rs`.

## Invariants — Do Not Violate

- `tinyquant-core` must remain GPU-free. Do not add `wgpu` as a dependency of
  `tinyquant-core`. GPU state travels as `Box<dyn Any + Send + Sync>`.
- Every `.rs` file must open with a `//!` module docstring. Public symbols must
  carry their own doc comments.
- `compress_batch` must return `Err(ResidualNotSupported)` when
  `prepared.config().residual_enabled() == true`, until Phase 28 wires the
  residual passes.
- Do not invent APIs, invariants, or performance claims not supported by the
  actual code.
- `publish = false` must remain in `Cargo.toml`; this crate is not released
  independently.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../AGENTS.md)
- [docs/plans/rust/phase-27-wgpu-wgsl-kernels.md](../../../docs/plans/rust/phase-27-wgpu-wgsl-kernels.md)
- [docs/design/rust/phase-27-implementation-notes.md](../../../docs/design/rust/phase-27-implementation-notes.md)
