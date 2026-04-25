# AGENTS.md — Guide for AI Agents Working in `tests/reference/tinyquant_py_reference`

`tinyquant_py_reference/` is the frozen pure-Python oracle for parity and migration tests.

## Layout

```text
tests/reference/tinyquant_py_reference/
├── backend/
├── codec/
├── corpus/
├── tools/
├── __init__.py
├── _types.py
├── py.typed
└── README.md
```

## Common Workflows

### Touch the reference package

1. Confirm the change belongs in a frozen oracle before editing.
2. Keep module docstrings and public docstrings intact.
3. Re-run the narrowest parity or packaging test that depends on the changed
   behavior.

## Invariants — Do Not Violate

- This package is test-only and frozen to historical pure-Python behavior.
- Do not add new product features here ahead of the Rust implementation.
- Keep the package layout stable unless tests and parent docs are updated in the
  same change.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../AGENTS.md)
- [tests/parity/AGENTS.md](../../parity/AGENTS.md)
