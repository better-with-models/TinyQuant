# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-fuzz`

This crate holds `libfuzzer-sys` fuzz targets for TinyQuant serialization surfaces. New fuzz targets are added as each codec or corpus serialization path is implemented. The crate is not part of the standard `cargo test` run; fuzz targets are exercised separately via `cargo fuzz`. The smoke tests in `tests/` provide a minimal always-run check that the crate compiles and its entry points are reachable. Changes here most often involve adding new fuzz targets for newly stabilized codec surfaces.

## What this area contains

- primary responsibility: `libfuzzer-sys` fuzz target implementations for corpus and codec serialization paths, with a smoke-test layer for CI compilation checks
- main entrypoints: `src/lib.rs` (fuzz target entry points), `tests/smoke.rs` (compilation and reachability check)
- common changes: adding new fuzz targets in `src/lib.rs` as new serialization surfaces are stabilized, updating smoke tests to cover new entry points

## Layout

```text
tinyquant-fuzz/
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
