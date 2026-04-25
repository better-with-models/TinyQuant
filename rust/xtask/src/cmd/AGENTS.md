# AGENTS.md — Guide for AI Agents Working in `rust/xtask/src/cmd`

This directory contains the xtask subcommand modules that are large enough to warrant their own files. Each module exports a `run` function called from `main.rs`. CI workflows and the release workflow call into these verbs and rely on their exit behaviour for pass/fail gating.

## What this area contains

- primary responsibility: `bench.rs` (criterion baseline capture/compare/validate/diff), `guard_sync.rs` (asserts four-way `if:` guard equality across `publish-*` jobs in `rust-release.yml`), `guard_sync_python.rs` (asserts publish-guard contract in `python-fatwheel.yml`), `matrix_sync.rs` (diffs CLI smoke matrix in the plan doc against `rust-release.yml`), `simd.rs` (SIMD feature audit)
- main entrypoints: `bench.rs` for the benchmark budget logic invoked by the `bench-budget` CI step; `guard_sync.rs` and `guard_sync_python.rs` for publish drift detection
- common changes: extending `bench.rs` with new baseline comparison modes, updating the plan-doc path or job-name selectors in `matrix_sync.rs`, adding a new guard check module

## Layout

```text
cmd/
├── bench.rs
├── guard_sync.rs
├── guard_sync_python.rs
├── matrix_sync.rs
├── simd.rs
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
