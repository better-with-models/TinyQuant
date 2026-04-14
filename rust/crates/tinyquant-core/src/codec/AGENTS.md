# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-core/src/codec`

This module implements the full codec pipeline: deterministic `CodecConfig`
(dimension, bit-width, seed), Gaussian-initialized `Codebook` training,
`RotationMatrix` generation and caching (`RotationCache`), scalar quantize /
dequantize operations, FP16 residual helpers, the `CompressedVector` value object,
and the stateless `Codec` service that wires them together
(rotate → quantize → optional residual on compress; inverse on decompress).
Under `feature = "simd"`, `dispatch.rs` and `simd_api.rs` select the
best-available kernel at runtime. Under `feature = "std"`, batch compression
helpers (`batch.rs`) are available. The `corpus` and `backend` modules depend on
types defined here; so do every crate that imports `tinyquant-core`.

## What this area contains

- primary responsibility: codec domain — `CodecConfig`, `Codebook`, `RotationMatrix`, `RotationCache`, `CompressedVector`, `Codec` service, quantize/dequantize primitives, FP16 residual, SIMD dispatch, and optional batch parallelism
- main entrypoints: `mod.rs` (public re-exports), `service.rs` (`Codec::compress` / `Codec::decompress`), `codec_config.rs`, `codebook.rs`, `rotation_matrix.rs`
- common changes: adjusting codec pipeline steps, adding a new bit-width, tuning rotation or codebook training, wiring a new SIMD kernel through `dispatch.rs` / `simd_api.rs`

## Layout

```text
codec/
├── kernels/
├── codebook.rs
├── codec_config.rs
├── compressed_vector.rs
├── dispatch.rs
├── gaussian.rs
├── mod.rs
├── parallelism.rs
├── quantize.rs
├── README.md
├── residual.rs
└── rotation_cache.rs
└── …
```

## Common workflows

### Update existing behavior

1. Read the local README and the files you will touch before editing.
2. Follow the local invariants before introducing new files or abstractions.
3. Update nearby docs when the change affects layout, commands, or invariants.
4. Run the narrowest useful verification first, then the broader project gate.

### Add a new file or module

1. Confirm the new file belongs in this directory rather than a sibling.
2. Update the layout section if the structure changes in a way another agent must notice.
3. Add or refine local docs when the new file introduces a new boundary or invariant.

## Invariants — Do Not Violate

- keep this directory focused on its stated responsibility
- do not invent APIs, workflows, or invariants that the code does not support
- update this file when structure or safe-editing rules change

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../../AGENTS.md)
