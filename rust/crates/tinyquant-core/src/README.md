# rust/crates/tinyquant-core/src

This directory contains the full source tree for `tinyquant-core`. The crate root (`lib.rs`) declares six public modules; the three with non-trivial internal structure (`codec`, `corpus`, `backend`) each live in a subdirectory. The remaining modules (`types`, `errors`, `prelude`) are single files.

## What lives here

| File/dir | Purpose |
| --- | --- |
| `lib.rs` | Crate root; `#![no_std]`, deny lints, module declarations |
| `types.rs` | Primitive type aliases: `VectorId`, `ConfigHash`, `CorpusId`, `Vector`, `VectorSlice` |
| `errors.rs` | `CodecError`, `CorpusError`, `BackendError` — all `Clone + PartialEq` |
| `prelude.rs` | Glob re-export of all stable public items across the three layers |
| `codec/` | Codec layer: config, rotation, codebook, quantize, service, SIMD kernels |
| `corpus/` | Corpus aggregate root and domain types |
| `backend/` | `SearchBackend` trait and `SearchResult` |

## How this area fits the system

Downstream crates import the items declared here. `tinyquant-io` depends on `CompressedVector` and `CodecConfig`. Test binaries in `../tests/` exercise the public API through `tinyquant_core::prelude`. No module in this tree may introduce `std` outside of a `#[cfg(feature = "std")]` guard.

## Common edit paths

- Adding a type alias: `types.rs`
- Adding or changing error variants: `errors.rs`
- Expanding the prelude: `prelude.rs`
- Deeper codec or corpus changes: the respective subdirectory

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
