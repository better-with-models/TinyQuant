# src

This directory is the Python source root for the `tinyquant_cpu` developer shim. In the fat wheel produced by Phase 24.2 this package is replaced by the template entry point in `scripts/packaging/templates/`; for local development it simply re-homes the already-installed `tinyquant_rs._core` extension under `sys.modules["tinyquant_cpu._core"]` so that the per-sub-package shims resolve correctly. See `docs/plans/rust/phase-24-python-fat-wheel-official.md` §Shim-layer contract for the invariants both the dev shim and the fat-wheel shim must satisfy.

## What lives here

- `tinyquant_cpu/` — the `tinyquant_cpu` Python package (see [`tinyquant_cpu/README.md`](tinyquant_cpu/README.md)).

This directory exists because `pyproject.toml` configures `packages = [{include = "tinyquant_cpu", from = "src"}]`; it is the conventional `src/` layout used by Hatch/setuptools.

## How this area fits the system

`pip install -e .` (dev mode) makes `tinyquant_cpu` importable from this directory. The package imports `tinyquant_rs._core` and re-registers it, so the Rust extension must be built first with `maturin develop`. In the fat wheel, `src/tinyquant_cpu/__init__.py` is replaced; the sub-package shims (`codec/`, `corpus/`, `backend/`) are unchanged in both cases.

## Common edit paths

- `tinyquant_cpu/__init__.py` — only if the shim contract changes (e.g. Phase 24 template diverges).
- `tinyquant_cpu/codec/__init__.py`, `corpus/__init__.py`, `backend/__init__.py` — when the Rust extension adds or renames a class.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
