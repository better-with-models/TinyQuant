# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-bench/src`

This directory contains the library source for `tinyquant-bench`. `lib.rs` is the crate root and exposes the `calibration` submodule, which provides the neighbor-recall and Pearson ρ helpers used in both the calibration integration tests and CI quality gates. The `calibration/` subdirectory holds the module entry point and the two metric implementations. Changes here most often involve extending the calibration metrics or adding new public helpers for use in `benches/` or `tests/`.

## What this area contains

- primary responsibility: crate root (`lib.rs`) and the `calibration` submodule (`mod.rs`, `neighbor_recall.rs`, `pearson.rs`) that implement the quality-gate metrics
- main entrypoints: `lib.rs` (re-exports and module declarations), `calibration/mod.rs` (submodule root)
- common changes: extending calibration metric logic, adding new metric functions, adjusting metric signatures consumed by `tests/calibration.rs`

## Layout

```text
src/
├── calibration/
│   ├── mod.rs
│   ├── neighbor_recall.rs
│   └── pearson.rs
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
