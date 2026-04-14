# src/tinyquant_cpu

This directory is the root of the `tinyquant_cpu` Python package — the developer shim that bridges the installed `tinyquant_rs` Rust extension to the `tinyquant_cpu` namespace. `__init__.py` imports `tinyquant_rs._core` and registers it under `sys.modules["tinyquant_cpu._core"]`; the three sub-packages (`codec/`, `corpus/`, `backend/`) then pull class and function references off that module entry. The package exposes `__version__` forwarded from the Rust extension.

## What lives here

- `__init__.py` — package entry point; registers `tinyquant_rs._core` under `tinyquant_cpu._core` and re-exports `__version__`.
- `codec/` — codec sub-package shim (see [`codec/README.md`](codec/README.md)).
- `corpus/` — corpus sub-package shim (see [`corpus/README.md`](corpus/README.md)).
- `backend/` — backend sub-package shim (see [`backend/README.md`](backend/README.md)).

## How this area fits the system

End-user code imports from `tinyquant_cpu.codec`, `tinyquant_cpu.corpus`, or `tinyquant_cpu.backend`. Each sub-package shim pulls its names from `sys.modules["tinyquant_cpu._core"]` (set by `__init__.py`), which is the Rust extension compiled by `maturin`. Importing any sub-package first imports `tinyquant_cpu` (the parent), which triggers `_core` registration. In the fat wheel, `__init__.py` is replaced by the arch-selector template, but the sub-packages are unchanged.

## Common edit paths

- `__init__.py` — if the shim registration contract changes.
- `codec/__init__.py` — when a new class or function is added to the Rust codec binding.
- `corpus/__init__.py` — when a new corpus class or event type is added.
- `backend/__init__.py` — when a new backend class is added.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
