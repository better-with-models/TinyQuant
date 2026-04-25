# src/tinyquant_cpu/backend

This directory is the `tinyquant_cpu.backend` sub-package. It re-exports the search backend classes from `tinyquant_rs._core.backend` (the Rust PyO3 extension) under the `tinyquant_cpu.backend` namespace. The shim contains no logic of its own.

## What lives here

- `__init__.py` — resolves `sys.modules["tinyquant_cpu._core"].backend` and binds:
  - `SearchBackend` — protocol placeholder (no runtime pyclass; typing only).
  - `SearchResult` — result record returned by a search operation.
  - `BruteForceBackend` — exhaustive nearest-neighbour search adapter backed by `tinyquant-bruteforce`.

## How this area fits the system

Same registration pattern as `codec/` and `corpus/`. Changing exported names requires matching changes in `rust/crates/tinyquant-py/src/backend.rs` and `register_backend` in `lib.rs`. The `BruteForceBackend` wraps `tinyquant_bruteforce::BruteForceBackend` from the `tinyquant-bruteforce` crate; the Rust-side `SearchBackend` protocol is a trait, so it has no corresponding Python class at runtime.

## Common edit paths

- `__init__.py` — when `tinyquant-py` adds a new backend adapter class or renames an existing one; keep `__all__` in sync.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
