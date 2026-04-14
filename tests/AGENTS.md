# AGENTS.md — Guide for AI Agents Working in `tests`

`tests/` holds the Python test suite for TinyQuant. Tests are organized by
domain area and enforce byte-parity between the Python reference and the
Rust port, as well as pure-Python behavior guarantees for downstream
consumers.

## What this area contains

- primary responsibility: verify codec, backend, corpus, and end-to-end
  behavior for the Python reference implementation and its interop with
  the Rust port.
- main entrypoints:
  - `codec/` — codec round-trips, codebook invariants, quantization edges.
  - `backend/` — brute-force and pgvector adapter behavior.
  - `corpus/` — aggregate-root lifecycle and event emission.
  - `calibration/` — compression-policy and scale calibration.
  - `architecture/` — import-time and layering rules.
  - `integration/`, `e2e/` — cross-module and full-pipeline scenarios.
  - `conftest.py` — shared fixtures.
- common changes: expand coverage when behavior changes, refresh fixtures
  alongside codec format revisions, add regression tests for filed bugs.

## Layout

```text
tests/
├── architecture/
├── backend/
├── calibration/
├── codec/
├── corpus/
├── e2e/
├── integration/
├── conftest.py
├── test_smoke.py
└── __init__.py
```

## Common workflows

### Add a failing test first

1. Write the test in the most specific subdirectory possible.
2. Run it with `pytest tests/<area>/test_name.py -v` and confirm it fails
   for the right reason before implementing the fix.
3. Implement the fix in `src/tinyquant_cpu/` or the matching Rust crate,
   then re-run the narrowest gate.

### Refresh shared fixtures

1. Regenerate via `python scripts/generate_rust_fixtures.py` when the
   codec byte layout changes.
2. Commit fixture updates in the same commit as the behavior change that
   justifies them.

## Invariants — Do Not Violate

- every new `.py` test file opens with a module docstring describing what
  it covers; helper functions keep docstrings per `$python-docstrings`.
- Python tests that exercise codec byte layout must stay consistent with
  Rust parity tests under `rust/crates/tinyquant-core/tests/`.
- do not mock the codec or rotation cache in parity tests; those suites
  must use the real implementation to catch drift.

## See Also

- [Root AGENTS.md](../AGENTS.md)
- [Python package guide](../src/AGENTS.md)
- [Rust parity tests](../rust/crates/tinyquant-core/tests/)
