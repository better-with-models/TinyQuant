# AGENTS.md — Guide for AI Agents Working in `tests/reference/tinyquant_py_reference/backend`

`backend/` holds the frozen reference backend surface for the pure-Python oracle.

## Layout

```text
tests/reference/tinyquant_py_reference/backend/
├── adapters/
├── brute_force.py
├── protocol.py
├── __init__.py
└── README.md
```

## Common Workflows

### Touch the reference backend

1. Keep changes narrowly scoped to parity or historical-behavior preservation.
2. Preserve protocol and adapter naming unless a migration plan requires more.
3. Re-run parity or backend tests after editing this package.

## Invariants — Do Not Violate

- This backend is historical reference behavior, not the authoritative runtime.
- Adapter and protocol surfaces here must stay stable for the oracle tests.
- Do not add new backend features here ahead of Rust.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../AGENTS.md)
- [adapters/AGENTS.md](./adapters/AGENTS.md)
