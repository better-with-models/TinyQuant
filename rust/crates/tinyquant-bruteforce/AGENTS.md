# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-bruteforce`

This crate implements the linear-scan `BruteForceBackend` that satisfies the `SearchBackend` trait from `tinyquant-core`. It computes cosine similarity against every stored vector on each query and is suitable for corpora up to approximately 100 k vectors. It is used as the reference backend in integration tests and as a correctness baseline for more advanced index backends. Changes here most often involve adjusting similarity computation, adding SIMD-accelerated paths gated on the `simd` feature, or refining error handling.

## What this area contains

- primary responsibility: linear-scan `BruteForceBackend` implementing `SearchBackend` from `tinyquant-core`, including cosine similarity, vector storage, and an optional SIMD-accelerated kernel
- main entrypoints: `src/lib.rs` (re-exports `BruteForceBackend`, `BackendError`, and the `SearchBackend` trait), `src/backend.rs` (core `SearchBackend` implementation)
- common changes: adjusting similarity logic, gating SIMD paths behind the `simd` feature flag, updating error variants in `errors.rs`

## Layout

```text
tinyquant-bruteforce/
├── src/
├── tests/
├── Cargo.toml
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
- [Root AGENTS.md](../../../AGENTS.md)
