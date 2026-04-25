# rust/crates/tinyquant-py

PyO3 binding crate that compiles to the `tinyquant_rs._core` Python extension
module. This is the Rust half of the Phase 24 fat wheel.

## What lives here

| File / Directory | Purpose |
| --- | --- |
| `src/` | PyO3 module source — `lib.rs` (module root) and per-type `*_py.rs` files |
| `Cargo.toml` | Rust crate manifest with PyO3 and maturin dependencies |

## How this area fits the system

`tinyquant-py` wraps `tinyquant-core` and exposes the codec API to Python
without going through the pure-Python reference implementation. The compiled
`.so` / `.pyd` is bundled into the fat wheel assembled by
`scripts/packaging/assemble_fat_wheel.py`. The Python shim package at
`src/tinyquant_cpu/` imports from `tinyquant_rs._core` at runtime and
re-exports the public symbols.

## Common edit paths

- **`src/lib.rs`** — add new `#[pymodule]` registrations when a new type is exported.
- **`src/<type>_py.rs`** — implement `#[pymethods]` for individual types.
- **`Cargo.toml`** — bump PyO3 or maturin versions here.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
