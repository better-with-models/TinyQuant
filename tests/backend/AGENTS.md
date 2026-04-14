# AGENTS.md — Guide for AI Agents Working in `tests/backend`

This directory tests the brute-force search backend in isolation. Tests verify correctness of nearest-neighbour retrieval against known query/corpus pairs without exercising the codec pipeline. CI runs these tests on every push. Changes here most often happen when the `BruteForceBackend` API changes or new distance metrics are added.

## What this area contains

- primary responsibility: `test_brute_force.py` — unit tests for the brute-force search backend (result ordering, top-K correctness, edge cases such as empty corpus or single-vector corpus)
- main entrypoints: `test_brute_force.py`
- common changes: adding test cases for new distance metrics, updating assertions when the `SearchResult` schema changes

## Layout

```text
backend/
├── __init__.py
├── README.md
└── test_brute_force.py
```

## Common workflows

### Update existing behavior

1. Read the local README and the files you will touch before editing.
2. Follow the local invariants before introducing new files or abstractions.
3. Update nearby docs when the change affects layout, commands, or invariants.
4. Run the narrowest useful verification first, then the broader project gate.

### Add a new file or module

1. Confirm the new file belongs in this directory rather than a sibling.
2. Update the layout section if the structure changes in a way another agent must notice.
3. Add or refine local docs when the new file introduces a new boundary or invariant.

## Invariants — Do Not Violate

- keep this directory focused on its stated responsibility
- do not invent APIs, workflows, or invariants that the code does not support
- update this file when structure or safe-editing rules change

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../AGENTS.md)
