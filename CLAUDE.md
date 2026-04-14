# CLAUDE.md

> *CPU-only vector quantization codec for embedding storage compression.*

This repository uses [AGENTS.md](AGENTS.md) as the primary operating contract.

If you are working in TinyQuant, read [AGENTS.md](AGENTS.md) first and treat it
as the authoritative source for:

- documentation structure
- markdown policy
- wiki maintenance rules
- pre-commit verification expectations

## Implementation layout (Phase 23+)

The shipping engine is the Rust workspace under `rust/`. The pure-Python
implementation lives under `tests/reference/tinyquant_py_reference/` as
a test-only differential oracle and is no longer shipped on PyPI. The
in-tree `src/tinyquant_cpu/` directory is a Phase 24.1 developer shim
that re-exports the Rust extension (`tinyquant_rs`) under the
`tinyquant_cpu` import name; the same files are templated into the
Phase 24 fat wheel by `scripts/packaging/assemble_fat_wheel.py`. The
last pure-Python release remains `tinyquant-cpu==0.1.1` on PyPI;
Phase 24 reclaims the name with a Rust-backed fat wheel at `0.2.0`.

Cross-implementation parity lives under `tests/parity/` and runs under
`pytest -m parity`. Self-parity is always live; Rust-side parity is
wired on when Phase 24 installs the fat wheel.
