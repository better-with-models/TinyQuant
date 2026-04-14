# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-core/src`

This directory is the top-level module tree for `tinyquant-core`. `lib.rs`
declares the four public modules (`backend`, `codec`, `corpus`, `errors`,
`prelude`, `types`) and enforces the `no_std` + `deny(warnings)` policy. Flat
files (`types.rs`, `errors.rs`, `prelude.rs`) hold cross-cutting primitives that
every sub-module imports. The three subdirectories contain all domain logic. All
other crates in the workspace ultimately import from here via `tinyquant-core`.

## What this area contains

- primary responsibility: crate root — `no_std` declarations, cross-cutting types (`VectorId`), shared error enum, and the public prelude re-export
- main entrypoints: `lib.rs` (module declarations and feature gates), `errors.rs` (unified error types), `types.rs` (`VectorId` newtype), `prelude.rs` (convenience re-exports)
- common changes: adding a new top-level module, extending the shared error enum, updating the prelude, adjusting `no_std` / feature flag wiring

## Layout

```text
src/
├── backend/
├── codec/
├── corpus/
├── errors.rs
├── lib.rs
├── prelude.rs
├── README.md
└── types.rs
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
