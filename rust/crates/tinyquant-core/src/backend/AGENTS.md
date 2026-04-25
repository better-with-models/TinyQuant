# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-core/src/backend`

This module defines the uniform `SearchBackend` trait that every concrete search
backend must implement, plus the `SearchResult` value object returned by
`SearchBackend::search`. The trait specifies `ingest`, `search`, `remove`, `len`,
`is_empty`, and `dim` methods with dimension-locking and Python-parity semantics.
Concrete implementations live in `tinyquant-bruteforce` and `tinyquant-pgvector`;
this module contains only the contract, not any implementation. It is a pure
`no_std` / alloc module.

## What this area contains

- primary responsibility: `SearchBackend` trait definition and `SearchResult` value object (score + `VectorId`, ordered descending by cosine similarity)
- main entrypoints: `protocol.rs` (both public items), `mod.rs` (re-exports `SearchBackend` and `SearchResult`)
- common changes: extending the trait with a new method, adjusting `SearchResult` ordering semantics, adding a new `BackendError` variant in the shared errors module

## Layout

```text
backend/
├── mod.rs
├── protocol.rs
└── README.md
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
- [Root AGENTS.md](../../../../../AGENTS.md)
