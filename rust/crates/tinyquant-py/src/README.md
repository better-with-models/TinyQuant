# rust/crates/tinyquant-py/src

This directory contains the PyO3 source for the `tinyquant_rs._core` Python extension module. `lib.rs` is the crate root and the single entry point registered by `maturin`; it wires three sub-modules (`codec`, `corpus`, `backend`) into nested `PyModule` objects and promotes every public class and function to the root level so the pure-Python `tinyquant_rs/__init__.py` wrapper can re-export them. The extension is built as an `abi3`-stable shared library (`_core.abi3.so` / `_core.abi3.pyd`); the Python-visible API mirrors `tinyquant_cpu` exactly, per the §Binding 1 contract in `docs/design/rust/ffi-and-bindings.md`.

## What lives here

- `lib.rs` — crate root; declares the `_core` `#[pymodule]`, registers all three sub-modules, and re-exports every public class and function at the module root.
- `codec.rs` — PyO3 wrappers for `CodecConfig`, `Codebook`, `RotationMatrix`, `CompressedVector`, `Codec`, `compress`, and `decompress`.
- `corpus.rs` — PyO3 wrappers for `CompressionPolicy`, `VectorEntry`, `Corpus`, `CorpusCreated`, `VectorsInserted`, `CorpusDecompressed`, and `CompressionPolicyViolationDetected`.
- `backend.rs` — PyO3 wrappers for `SearchBackend` (protocol placeholder), `SearchResult`, and `BruteForceBackend`.
- `errors.rs` — maps Rust error types to the Python exception hierarchy (`DimensionMismatchError`, `ConfigMismatchError`, `CodebookIncompatibleError`, `DuplicateVectorError`).
- `numpy_bridge.rs` — helper conversions between `PyReadonlyArray` and Rust `ndarray` / `Vec<f32>` slices.

## How this area fits the system

`maturin` compiles this crate into the `tinyquant_rs` Python package. The pure-Python shim `src/tinyquant_cpu/__init__.py` imports `tinyquant_rs._core` at dev time and re-registers it under `tinyquant_cpu._core`; in a fat wheel (Phase 24.2) the entry point is replaced but the `_core` sub-module layout is identical. Every class exposed here must match the `tinyquant_cpu` reference Python package — adding or renaming a class also requires updating `src/tinyquant_cpu/{codec,corpus,backend}/__init__.py`. This crate depends on `tinyquant-core`, `tinyquant-bruteforce`, and `tinyquant-io`.

## Common edit paths

- `codec.rs` — most Python API additions (new methods on `Codec` or `Codebook`).
- `corpus.rs` — new corpus events or policy variants.
- `errors.rs` — adding or renaming Python-visible exception classes.
- `lib.rs` — only when a new sub-module or top-level export is introduced.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
