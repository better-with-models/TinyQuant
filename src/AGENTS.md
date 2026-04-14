# AGENTS.md — Guide for AI Agents Working in `src`

`src/` holds the Python reference implementation of TinyQuant. The single
package inside, `tinyquant_cpu`, mirrors the Rust port under `rust/crates/`
and is slated for demotion to a thin reference per the Phase 23–25 plans
in [`docs/plans/rust/`](../docs/plans/rust/).

## What this area contains

- primary responsibility: the Python reference codec, backends, corpus
  aggregate, and tooling that non-Rust consumers use via PyPI.
- main entrypoints:
  - `tinyquant_cpu/codec/` — codec, codebooks, rotation matrix, quantize.
  - `tinyquant_cpu/backend/` — brute-force + pgvector search adapters.
  - `tinyquant_cpu/corpus/` — aggregate root and vector lifecycle.
  - `tinyquant_cpu/tools/` — fixture dump + diagnostic helpers.
- common changes: keep behavior byte-aligned with the Rust crates, add or
  tune reference-only helpers, fix import-time issues surfaced by PyPI users.

## Layout

```text
src/
└── tinyquant_cpu/
    ├── backend/
    │   └── adapters/
    ├── codec/
    ├── corpus/
    ├── tools/
    ├── _types.py
    ├── __init__.py
    └── py.typed
```

## Common workflows

### Update existing behavior

1. Check whether the Rust crate already encodes the behavior you are about
   to change; the Rust side is authoritative.
2. Update the Python reference and matching golden fixtures via
   `python scripts/generate_rust_fixtures.py` when byte layout shifts.
3. Run `pytest tests/<area>/` for the narrowest gate, then the full suite.

### Add a new module

1. Confirm no Rust crate already provides the equivalent surface.
2. Add `"""module docstring"""` at the top of every new `.py` file per
   `$python-docstrings`; public classes and functions get their own
   docstrings too.
3. Update `tinyquant_cpu/__init__.py` only when the symbol is part of the
   public package API.

## Invariants — Do Not Violate

- Python behavior must stay byte-compatible with the Rust codec; parity
  tests under `tests/` are the gate.
- every `.py` file in this tree opens with a module docstring; public
  classes and functions carry their own docstrings.
- do not add new product features here without a matching Rust
  implementation — the Rust port is the system of record.

## See Also

- [Root AGENTS.md](../AGENTS.md)
- [Rust port design](../docs/design/rust/README.md)
- [Rust demotion plans](../docs/plans/rust/)
