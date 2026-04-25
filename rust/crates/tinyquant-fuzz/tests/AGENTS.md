# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-fuzz/tests`

This directory contains the smoke tests for `tinyquant-fuzz`. Because fuzz targets cannot be run as ordinary `cargo test` binaries, `smoke.rs` provides a lightweight always-run check that the fuzz crate compiles correctly and that its public surface is reachable under normal test conditions. Changes here most often involve updating the smoke test when new fuzz targets are added to `src/lib.rs`.

## What this area contains

- primary responsibility: `smoke.rs` — a minimal always-run test that verifies the `tinyquant-fuzz` crate compiles and its entry points are reachable without invoking the actual fuzzer
- main entrypoints: `smoke.rs` (the only test file; open this first for any smoke-test change)
- common changes: extending coverage when new fuzz targets are registered in `src/lib.rs`

## Layout

```text
tests/
├── README.md
└── smoke.rs
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
