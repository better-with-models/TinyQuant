# AGENTS.md — Guide for AI Agents Working in `tests/integration`

This directory holds integration tests that exercise cross-layer interactions: codec output feeding into corpus storage, corpus feeding into the search backend, pgvector adapter wiring, and serialization round-trips across the Rust/Python boundary. Tests here require more setup than unit tests and may use `conftest.py` fixtures for shared database or file state. Changes here happen when an interface between layers is added or modified.

## What this area contains

- primary responsibility: `test_codec_corpus.py` (codec compress → corpus ingest → corpus decompress round-trip), `test_corpus_backend.py` (corpus retrieval feeding brute-force search), `test_pgvector.py` (pgvector adapter integration), `test_serialization.py` (byte-level parity across Python/Rust serialization); `conftest.py` for shared fixtures
- main entrypoints: `conftest.py` for fixture setup; `test_codec_corpus.py` for the primary cross-layer path
- common changes: adding cross-layer test cases when new pipeline stages land, updating `conftest.py` fixture paths, adjusting pgvector connection parameters

## Layout

```text
integration/
├── __init__.py
├── conftest.py
├── README.md
├── test_codec_corpus.py
├── test_corpus_backend.py
├── test_pgvector.py
└── test_serialization.py
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
