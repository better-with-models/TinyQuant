# AGENTS.md — Guide for AI Agents Working in `tests/corpus`

This directory holds unit tests for the corpus layer: compression policies, corpus file I/O, event handling, and `VectorEntry` serialization. Tests exercise the corpus independently of the brute-force search backend. Changes here happen when corpus file format, compression policy semantics, or event schema changes.

## What this area contains

- primary responsibility: `test_compression_policy.py` (policy enum and selection logic), `test_corpus.py` (corpus construction, insertion, and retrieval), `test_events.py` (corpus event schema and dispatch), `test_vector_entry.py` (`VectorEntry` serialization and round-trip)
- main entrypoints: `test_corpus.py` for the primary corpus API; `test_compression_policy.py` for policy invariants
- common changes: adding tests for new compression policies, updating serialization assertions when the `VectorEntry` or corpus file format changes

## Layout

```text
corpus/
├── __init__.py
├── README.md
├── test_compression_policy.py
├── test_corpus.py
├── test_events.py
└── test_vector_entry.py
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
