# AGENTS.md — Guide for AI Agents Working in `rust/xtask/src`

This directory contains the full source of the `cargo xtask` task runner. `main.rs` owns the top-level dispatch table and all fixture-refresh logic (calling Python scripts and Rust example binaries). The `cmd/` sub-modules implement the heavier subcommands (bench budget checks, CI guard-sync, publish-guard drift checks, SIMD audit) that warrant their own files. CI workflows invoke xtask verbs rather than scripting cargo directly, so changes here can affect multiple workflow files.

## What this area contains

- primary responsibility: `main.rs` (dispatch, fixture refresh functions, `docs_check_ci_parity`), `cmd/bench.rs` (criterion baseline capture/compare), `cmd/guard_sync.rs` (publish-guard equality check for `rust-release.yml`), `cmd/guard_sync_python.rs` (publish-guard contract check for `python-fatwheel.yml`), `cmd/matrix_sync.rs` (CLI smoke matrix diff vs `rust-release.yml`), `cmd/simd.rs` (SIMD audit subcommand)
- main entrypoints: `main.rs` for the top-level `match task` dispatch; `cmd/bench.rs` for the benchmark budget logic used by CI
- common changes: adding a `refresh-*` arm in `fixtures()`, updating `REQUIRED_CI_JOBS`, adding a new `cmd/` module

## Layout

```text
src/
├── cmd/
├── main.rs
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
