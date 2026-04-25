# rust/crates/tinyquant-core

`tinyquant-core` is the CPU-only vector quantization codec — the pure-logic heart of TinyQuant. It is `no_std` (requires `alloc` only), contains no I/O, no file-system access, and no platform-specific code. All encoding, decoding, rotation, codebook operations, corpus management, and the search-backend trait live here. The sibling crate `tinyquant-io` adds serialization on top of this one.

## What lives here

- `src/lib.rs` — crate root; declares the six top-level modules and feature gates
- `src/types.rs` — primitive type aliases (`VectorId`, `ConfigHash`, `CorpusId`, `Vector`, `VectorSlice`)
- `src/errors.rs` — `CodecError`, `CorpusError`, `BackendError` enums
- `src/prelude.rs` — convenience glob re-exports for downstream crates
- `src/codec/` — `CodecConfig`, `RotationMatrix`, `Codebook`, quantize/dequantize, residual helpers, `Codec` service, SIMD dispatch
- `src/corpus/` — `Corpus` aggregate root, `VectorEntry`, `CompressionPolicy`, `CorpusEvent`, `VectorIdMap`
- `src/backend/` — `SearchBackend` trait and `SearchResult`
- `tests/` — integration test suite (codebook, codec service, corpus, SIMD parity, serialization fixture parity)
- `examples/` — `dump_codec_fixture` and `dump_rotation_fixture` developer binaries

## How this area fits the system

`tinyquant-core` is a dependency of every other crate in the workspace. `tinyquant-io` reads and writes the types it defines. Backend crates (`tinyquant-bruteforce`, `tinyquant-pgvector`) implement the `SearchBackend` trait it declares. The Python FFI layer calls into compiled versions of these codec and corpus primitives.

The crate enforces a hard `no_std + alloc` boundary — any new module that brings in `std` must be gated on `#[cfg(feature = "std")]` and must not appear in the default feature set.

## Common edit paths

- **New codec primitives** — add to `src/codec/` and re-export from `src/codec/mod.rs` and `src/prelude.rs`
- **Error variants** — `src/errors.rs`
- **Corpus domain rules** — `src/corpus/aggregate.rs`, `src/corpus/compression_policy.rs`
- **SIMD kernels** — `src/codec/kernels/`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
