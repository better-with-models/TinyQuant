# AGENTS.md — Guide for AI Agents Working in `tests/reference/tinyquant_py_reference/corpus`

`corpus/` contains the frozen reference corpus domain model for the Python oracle.

## Layout

```text
tests/reference/tinyquant_py_reference/corpus/
├── corpus.py
├── compression_policy.py
├── vector_entry.py
├── events.py
├── __init__.py
└── README.md
```

## Common Workflows

### Touch the reference corpus model

1. Keep changes justified by parity preservation or migration support.
2. Preserve naming and event shape unless a plan explicitly changes them.
3. Re-run corpus and integration tests after editing this subtree.

## Invariants — Do Not Violate

- This subtree is part of the frozen oracle, not a feature-development surface.
- Corpus, policy, and event semantics must stay aligned with the historical
  pure-Python contract.
- Keep the role of this subtree explicit in docs and tests.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../AGENTS.md)
- [tests/integration/AGENTS.md](../../../integration/AGENTS.md)
