# AGENTS.md — Guide for AI Agents Working in `rust/xtask`

This crate is the `cargo xtask` task runner for the TinyQuant Rust workspace. It is invoked as `cargo xtask <command>` from the `rust/` directory. CI workflows, the release workflow, and local developer scripts all call into xtask rather than scripting raw `cargo` invocations. Changes here most often happen when adding new fixture-refresh sub-verbs, adjusting benchmark budget checks, or extending CI-parity guards.

## What this area contains

- primary responsibility: `cargo xtask` entry point — `fmt`, `lint`, `test`, `fixtures` (sub-verbs for all fixture categories), `bench`/`bench-budget`, `check-matrix-sync`, `check-publish-guards`, `docs`, `simd`, and `help` commands
- main entrypoints: `src/main.rs` (top-level dispatch and all fixture refresh functions), `src/cmd/` (bench, guard_sync, guard_sync_python, matrix_sync, simd sub-modules)
- common changes: adding a `fixtures refresh-*` sub-verb, updating required CI job names in `REQUIRED_CI_JOBS`, adding a new `cmd/` module for a new xtask subcommand

## Layout

```text
xtask/
├── src/
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
- [Root AGENTS.md](../../AGENTS.md)
