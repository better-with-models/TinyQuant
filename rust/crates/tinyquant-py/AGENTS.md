# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-py`

This crate is the PyO3 binding layer that compiles to the `tinyquant_rs._core`
Python extension module. It exposes `CodecConfig`, `Codebook`, `Codec`,
`CompressedVector`, and `RotationMatrix` to Python callers and is the Rust half
of the fat wheel produced in Phase 24. The Python shim package in
`src/tinyquant_cpu/` re-exports from this extension module.

## What this area contains

- primary responsibility: PyO3 extension module exposing the TinyQuant codec API to Python as `tinyquant_rs._core`
- main entrypoints: `src/lib.rs` (PyO3 module root with `#[pymodule]`), individual `*_py.rs` files for each exported type
- common changes: adding PyO3 `#[pymethods]` when the core codec API adds new methods; updating maturin build config

## Layout

```text
tinyquant-py/
├── src/
├── Cargo.toml
└── README.md
```

## Common workflows

### Update existing behavior

1. Read `src/lib.rs` and the relevant `*_py.rs` file before adding or changing a `#[pymethod]`.
2. All Python-visible types must derive `#[pyclass]`; keep the surface consistent with the Python reference API.
3. Build and test with `maturin develop` in the project virtual environment.
4. Run the parity test suite (`pytest -m parity`) to confirm Python ↔ Rust agreement.

### Add a new exported type

1. Create a new `*_py.rs` file following the existing pattern.
2. Register the type in `src/lib.rs` via `m.add_class::<MyType>()?`.
3. Add `///` doc comments; PyO3 surfaces them as Python `__doc__` strings.

## Invariants — Do Not Violate

- The Python-visible API must remain compatible with the pure-Python reference implementation's interface.
- Do not expose internal Rust types that are not part of the stable codec surface.
- Parity tests (`tests/python/test_parity.py`) must pass before merging any API change.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../AGENTS.md)
