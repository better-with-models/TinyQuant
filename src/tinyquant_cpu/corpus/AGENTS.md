# AGENTS.md — Guide for AI Agents Working in `src/tinyquant_cpu/corpus`

This sub-package re-exports the corpus layer of the `tinyquant_cpu` Python
shim. It exposes `Corpus`, `CompressionPolicy`, `VectorEntry`, and the domain
events to callers of `tinyquant_cpu.corpus`. The `Corpus` class orchestrates
compress-on-insert and search via a pluggable `Backend`.

## What this area contains

- primary responsibility: expose `Corpus`, `CompressionPolicy`, `VectorEntry`, and domain events under `tinyquant_cpu.corpus`
- main entrypoints: `__init__.py` (re-exports), `corpus.py` (`Corpus` class), `compression_policy.py`, `vector_entry.py`, `events.py`
- common changes: updating `Corpus` when the `Backend` protocol changes; adding new domain events to `events.py`

## Layout

```text
corpus/
├── __init__.py
├── compression_policy.py
├── corpus.py
├── events.py
├── README.md
└── vector_entry.py
```

## Common workflows

### Update existing behavior

1. Read `corpus.py` before touching `compression_policy.py` — the policy is consumed by `Corpus.insert`.
2. New domain events go in `events.py` and must be re-exported from `__init__.py`.
3. Run `pytest tests/corpus/` after any change.

### Add a new domain event

1. Define the event dataclass in `events.py`.
2. Re-export from `__init__.py`.
3. Add a test in `tests/corpus/` that exercises the event emission path.

## Invariants — Do Not Violate

- `Corpus` must never import a concrete `Backend` implementation; it depends only on the `Backend` protocol.
- `VectorEntry` is a value object; do not add mutable state or methods with side effects.
- Domain events must be immutable dataclasses.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../AGENTS.md)
