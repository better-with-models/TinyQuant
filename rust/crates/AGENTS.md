# AGENTS.md — Guide for AI Agents Working in `rust/crates`

This directory is the Cargo workspace containing all 11 TinyQuant Rust crates. Each crate has a distinct responsibility and its own `AGENTS.md`. The workspace is built from `rust/Cargo.toml`; a change to any crate here may affect CI, the C ABI header, the Python extension wheel, or the `tinyquant` binary. Cross-crate dependency changes and version bumps are the most common operations that touch this directory rather than a single sub-crate.

## What this area contains

- primary responsibility: houses the 11-crate Cargo workspace — `tinyquant-bench`, `tinyquant-bruteforce`, `tinyquant-cli`, `tinyquant-core`, `tinyquant-fuzz`, `tinyquant-gpu-wgpu`, `tinyquant-io`, `tinyquant-js`, `tinyquant-pgvector`, `tinyquant-py`, and `tinyquant-sys`
- main entrypoints: the crate subdirectory matching the area of interest; `rust/Cargo.toml` for workspace-level dependency and version management
- common changes: adding a new crate, updating inter-crate dependency versions, adjusting workspace-level feature flags

## Layout

```text
crates/
├── tinyquant-bench/
├── tinyquant-bruteforce/
├── tinyquant-cli/
├── tinyquant-core/
├── tinyquant-fuzz/
├── tinyquant-gpu-wgpu/
├── tinyquant-io/
├── tinyquant-js/
├── tinyquant-pgvector/
├── tinyquant-py/
├── tinyquant-sys/
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
