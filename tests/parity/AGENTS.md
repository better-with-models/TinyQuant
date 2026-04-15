# AGENTS.md — Guide for AI Agents Working in `tests/parity`

`tests/parity/` verifies behavior that must agree across TinyQuant implementations.

## Layout

```text
tests/parity/
├── conftest.py
├── test_cross_impl.py
└── README.md
```

## Common Workflows

### Extend a parity check

1. Use the real implementations under test; avoid mocks that can hide drift.
2. Add the assertion to the narrowest parity scenario possible.
3. Re-run `pytest -m parity` after touching Rust, the Python shim, or the
   frozen oracle.

## Invariants — Do Not Violate

- Parity tests protect cross-implementation behavior, not one implementation's
  internal preferences.
- Keep fixtures and expectation wording neutral so the suite can compare
  implementations fairly.
- If a parity contract changes intentionally, update the relevant rollout docs
  with the test change.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../AGENTS.md)
- [tests/reference/AGENTS.md](../reference/AGENTS.md)
