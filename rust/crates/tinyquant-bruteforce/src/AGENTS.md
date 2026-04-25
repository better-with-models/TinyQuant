# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-bruteforce/src`

This directory contains all source modules for `tinyquant-bruteforce`. `backend.rs` is the primary file and contains the `BruteForceBackend` struct with its `SearchBackend` implementation. `store.rs` manages vector storage, `similarity.rs` provides the scalar cosine similarity computation, `similarity_simd.rs` provides the SIMD-accelerated variant (compiled only when the `simd` feature is enabled), and `errors.rs` defines `BackendError`. Changes here most often involve modifying similarity logic, refining storage layout, or adding new error variants.

## What this area contains

- primary responsibility: `backend.rs` (`BruteForceBackend` and `SearchBackend` impl), `store.rs` (vector storage), `similarity.rs` (scalar cosine similarity), `similarity_simd.rs` (SIMD cosine similarity, `simd` feature-gated), `errors.rs` (`BackendError` definition)
- main entrypoints: `backend.rs` (start here for any query-path or ingest-path changes), `lib.rs` (public re-exports and feature-gated module declarations)
- common changes: modifying cosine similarity computation in `similarity.rs` or `similarity_simd.rs`, updating `BackendError` variants, adjusting storage internals in `store.rs`

## Layout

```text
src/
├── backend.rs
├── errors.rs
├── lib.rs
├── README.md
├── similarity.rs
├── similarity_simd.rs
└── store.rs
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
- [Root AGENTS.md](../../../../AGENTS.md)
