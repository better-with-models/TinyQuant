# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-core/examples`

This directory contains developer-facing example binaries that generate golden
fixture files consumed by integration tests and downstream tooling.
`dump_codec_fixture.rs` runs the full codec pipeline with configurable seeds,
row/column counts, and bit widths, writing raw `f32` input and `u8` index
binaries so that regressions in the codec pipeline are detectable without
Python round-trips. `dump_rotation_fixture.rs` writes a canonical little-endian
`f64[dim × dim]` rotation matrix binary used by `rotation_fixture_parity` tests
and any future backend that needs Rust-reference rotation data. Both examples
require `feature = "std"` and are excluded from `no_std` builds.

## What this area contains

- primary responsibility: fixture-generation binaries — `dump_codec_fixture.rs` (full codec pipeline golden output) and `dump_rotation_fixture.rs` (rotation matrix binary)
- main entrypoints: `dump_codec_fixture.rs` (run with `cargo run -p tinyquant-core --example dump_codec_fixture --features std`), `dump_rotation_fixture.rs` (analogous invocation)
- common changes: updating output paths or file formats when the wire format or rotation algorithm changes, adding a new example when a new fixture type is needed

## Layout

```text
examples/
├── dump_codec_fixture.rs
├── dump_rotation_fixture.rs
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
