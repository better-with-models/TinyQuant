# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-core/tests/common`

This directory provides shared test helpers used by sibling integration test files
in `tests/`. Because Cargo does not compile files under `tests/common/` as
standalone test binaries, `mod.rs` is included via `mod common;` in each test
file that needs it. It supplies deterministic input generation (seeded RNG via
`StdRng`), shared codebook entry builders, and the per-kernel parity runner used
by all `simd_parity_*` tests. It is gated behind `#[cfg(feature = "simd")]` and
has `#[allow(dead_code)]` since not every test file uses every helper.

## What this area contains

- primary responsibility: shared test helpers — deterministic `f32` vector generation, SIMD dispatch parity runner, and reusable fixture builders for the `simd_parity_*` integration tests
- main entrypoints: `mod.rs` (single file; exposes `make_entries` and parity runner)
- common changes: adding a new helper when a new parity test file needs common setup, extending the entry generator for a new bit-width

## Layout

```text
common/
├── mod.rs
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
- [Root AGENTS.md](../../../../../AGENTS.md)
