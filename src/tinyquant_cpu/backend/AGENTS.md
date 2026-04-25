# AGENTS.md — Guide for AI Agents Working in `src/tinyquant_cpu/backend`

This sub-package re-exports the backend layer of the `tinyquant_cpu` Python
shim. In Phase 23 and earlier the pure-Python `BruteForceBackend` is the
primary implementation; from Phase 24 onward the Rust-backed
`tinyquant_rs._core.backend` implementation is preferred and this shim
transparently selects it.

## What this area contains

- primary responsibility: expose `BruteForceBackend`, `SearchResult`, and the `Backend` protocol to callers of `tinyquant_cpu.backend`
- main entrypoints: `__init__.py` (re-exports), `brute_force.py` (pure-Python fallback), `protocol.py` (structural `Backend` protocol)
- common changes: wiring the Rust-backed adapter in `adapters/` when Phase 24 is fully enabled; adding new `Backend` protocol methods

## Layout

```text
backend/
├── adapters/
├── __init__.py
├── brute_force.py
├── protocol.py
└── README.md
```

## Common workflows

### Update existing behavior

1. Read `protocol.py` before changing a method signature — the protocol is the public contract.
2. Pure-Python changes go in `brute_force.py`; Rust-backed changes go in `adapters/`.
3. Run `pytest tests/backend/` after any change.

### Add a new adapter

1. Add the adapter module under `adapters/`.
2. Wire it in `__init__.py` behind the feature flag or import guard.
3. Add or update tests in `tests/backend/`.

## Invariants — Do Not Violate

- `protocol.py` defines the structural protocol; do not add non-protocol methods to it.
- The pure-Python `BruteForceBackend` must remain functional as a fallback when the Rust extension is not installed.
- `__init__.py` must not import Rust extension modules at the top level; guard with `try/except ImportError`.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../AGENTS.md)
