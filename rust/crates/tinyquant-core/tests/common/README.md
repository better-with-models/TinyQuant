# rust/crates/tinyquant-core/tests/common

Shared test helpers for the SIMD parity integration tests. Cargo does not compile files inside `tests/common/` as standalone integration test binaries; `mod.rs` is included from each sibling test file via `mod common;`. The module provides deterministic input generation (seeded `StdRng`) and per-kernel parity run helpers.

## What lives here

- `mod.rs` — the only file; gated on `#[cfg(feature = "simd")]` and `#[allow(dead_code)]`

The module uses `rand::rngs::StdRng` with a fixed seed for reproducible input vectors, and imports `DispatchKind` and `simd_api` from `tinyquant_core` to drive per-kernel comparisons.

## How this area fits the system

Every `simd_parity_*.rs` test file in the parent directory includes this module. Adding a new helper function here makes it available to all parity tests without duplication.

This directory is intentionally scoped to SIMD parity helpers. Non-SIMD test utilities should live inline in the test files that need them.

## Common edit paths

- **New input shape for parity tests** — `mod.rs`, add a new generator function
- **New dispatch kind to test** — update the per-kernel run helper in `mod.rs` and add a `simd_parity_<arch>.rs` file in the parent directory

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
