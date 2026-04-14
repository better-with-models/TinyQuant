# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-core/src/codec/kernels`

This directory contains all architecture-specific and reference kernel
implementations for quantize, dequantize, cosine similarity, and FP16 residual
operations. `scalar.rs` is the canonical reference that every SIMD path must
match byte-for-byte; it is always compiled regardless of features. Under
`feature = "simd"`, `avx2.rs` (x86\_64), `neon.rs` (aarch64), and
`avx512.rs` (x86\_64 + `feature = "avx512"`) provide architecture-specific
wrappers — currently delegating to scalar until full intrinsic paths land.
`portable.rs` documents the intended `core::simd` portable fallback but also
delegates to scalar on MSRV 1.81 where `core::simd` is unavailable. The
`kernels` module itself is `pub(crate)` and is consumed exclusively by
`dispatch.rs` and `simd_api.rs`.

## What this area contains

- primary responsibility: scalar reference kernels plus SIMD-gated architecture-specific kernel modules (AVX2, AVX-512, NEON, portable)
- main entrypoints: `scalar.rs` (canonical reference for `quantize_into`, `dequantize_into`, `cosine`, `compute_residual_into`, `apply_residual_into`), `mod.rs` (feature-gated module declarations)
- common changes: promoting a stub SIMD kernel to a real intrinsic path, adding a new kernel function, extending parity test coverage in sibling `tests/`

## Layout

```text
kernels/
├── avx2.rs
├── avx512.rs
├── mod.rs
├── neon.rs
├── portable.rs
├── README.md
└── scalar.rs
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
- [Root AGENTS.md](../../../../../../AGENTS.md)
