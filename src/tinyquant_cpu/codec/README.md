# src/tinyquant_cpu/codec

This directory is the `tinyquant_cpu.codec` sub-package. It re-exports every codec class and function from `tinyquant_rs._core.codec` (the Rust PyO3 extension) under the `tinyquant_cpu.codec` namespace. The shim contains no logic of its own — it exists solely to satisfy the `tinyquant_cpu` public API surface.

## What lives here

- `__init__.py` — resolves `sys.modules["tinyquant_cpu._core"].codec` and binds the following names into the package namespace:
  - Classes: `CodecConfig`, `Codebook`, `RotationMatrix`, `CompressedVector`, `Codec`
  - Functions: `compress`, `decompress`
  - Exceptions: `CodebookIncompatibleError`, `ConfigMismatchError`, `DimensionMismatchError`, `DuplicateVectorError`

## How this area fits the system

Importing `tinyquant_cpu.codec` first triggers the `tinyquant_cpu` parent `__init__.py`, which registers `tinyquant_rs._core` into `sys.modules`. The shim then reads the `.codec` sub-module attribute off that registered extension. Changing the set of exported names requires matching changes in `rust/crates/tinyquant-py/src/codec.rs` and the `register_codec` function in `lib.rs`.

## Common edit paths

- `__init__.py` — when `tinyquant-py` adds, renames, or removes a codec-layer class or function; keep `__all__` in sync.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
