# rust/crates/tinyquant-core/src/codec/kernels

Architecture-specific kernel implementations for quantize, dequantize, cosine distance, and residual helpers (Phase 20). The `scalar` submodule is the canonical reference and is always compiled. Under `feature = "simd"`, architecture-specific submodules provide the fast paths. `portable` exists as a documented fallback but delegates to scalar on MSRV 1.81 where `core::simd` is unavailable.

## What lives here

| File | Compiled when | Role |
| --- | --- | --- |
| `scalar.rs` | always | Canonical reference implementation |
| `portable.rs` | `feature = "simd"` | `core::simd` fallback; currently delegates to scalar |
| `avx2.rs` | `feature = "simd"` + `target_arch = "x86_64"` | AVX2 fast path |
| `avx512.rs` | `feature = "simd"` + `feature = "avx512"` + `target_arch = "x86_64"` | AVX-512 fast path |
| `neon.rs` | `feature = "simd"` + `target_arch = "aarch64"` | NEON fast path |
| `mod.rs` | always | Re-exports and feature-gated module declarations |

## How this area fits the system

`../dispatch.rs` selects a kernel at runtime via `DispatchKind` and calls into the appropriate submodule through `../simd_api.rs`. The SIMD parity integration tests in `../../../../tests/simd_parity_*.rs` verify that every kernel produces bit-identical output to `scalar` for the same inputs.

Adding a new intrinsic path requires: a new submodule here, a new `DispatchKind` variant, a dispatch arm in `simd_api.rs`, and a corresponding parity test.

## Common edit paths

- **Correctness fixes or new operations** — start in `scalar.rs`, propagate to architecture submodules
- **AVX2 intrinsics** — `avx2.rs`
- **NEON intrinsics** — `neon.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
