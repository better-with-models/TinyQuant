# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-fuzz/src`

This directory contains the fuzz target implementations for `tinyquant-fuzz`. `lib.rs` is the entry point where `libfuzzer-sys` fuzz targets are registered. New targets are added here as each TinyQuant serialization surface (codec round-trip, corpus loading, etc.) is ready to be fuzzed. The file currently contains the crate-level docstring; concrete target functions are added in later phases alongside the corresponding codec implementations.

## What this area contains

- primary responsibility: fuzz target implementations in `lib.rs` — each target exercises one serialization surface (corpus or codec path) using arbitrary byte input from `libfuzzer-sys`
- main entrypoints: `lib.rs` (all fuzz targets live here; open this first for any fuzz-target addition or modification)
- common changes: adding new `fuzz_target!` functions for newly stabilized codec or corpus serialization paths

## Layout

```text
src/
├── lib.rs
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
- [Root AGENTS.md](../../../../AGENTS.md)
