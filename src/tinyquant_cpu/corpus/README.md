# src/tinyquant_cpu/corpus

This directory is the `tinyquant_cpu.corpus` sub-package. It re-exports every corpus class from `tinyquant_rs._core.corpus` (the Rust PyO3 extension) under the `tinyquant_cpu.corpus` namespace. The shim contains no logic of its own.

## What lives here

- `__init__.py` — resolves `sys.modules["tinyquant_cpu._core"].corpus` and binds:
  - `CompressionPolicy` — enum of `COMPRESS`, `PASSTHROUGH`, `FP16` policies.
  - `Corpus` — aggregate root for vector storage.
  - `VectorEntry` — individual stored vector record.
  - Event types: `CorpusCreated`, `VectorsInserted`, `CorpusDecompressed`, `CompressionPolicyViolationDetected`

## How this area fits the system

Same registration pattern as `codec/`: parent `__init__.py` registers `_core`, then this shim reads `.corpus` off it. Changing exported names requires matching changes in `rust/crates/tinyquant-py/src/corpus.rs` and `register_corpus` in `lib.rs`. The event types mirror the domain events emitted by the Rust `Corpus` API.

## Common edit paths

- `__init__.py` — when `tinyquant-py` adds, renames, or removes a corpus class or event type; keep `__all__` in sync.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
