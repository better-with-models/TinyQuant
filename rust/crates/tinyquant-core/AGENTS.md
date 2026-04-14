# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-core`

This is the `no_std` (alloc-only) core library for TinyQuant. It defines every
domain type used across the workspace: `CodecConfig`, `Codebook`, `Codec`,
`RotationMatrix`, `CompressedVector`, the `Corpus` aggregate root, and the
`SearchBackend` trait. It contains no I/O, no file-system access, and no
platform-specific code. Every other crate in the workspace depends on this crate.
Changes here most often involve adding codec pipeline variants, extending corpus
domain logic, or landing new SIMD kernel paths behind feature gates.

## What this area contains

- primary responsibility: the pure-Rust, `no_std` domain model — codec, corpus, backend trait
- main entrypoints: `src/lib.rs` (module map), `src/codec/mod.rs`, `src/corpus/mod.rs`, `src/backend/mod.rs`
- common changes: codec config and codebook tuning, corpus event additions, SIMD kernel gating, residual / rotation updates

## Layout

```text
tinyquant-core/
├── examples/
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
