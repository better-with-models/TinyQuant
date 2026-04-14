# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-io/src`

This is the source root for `tinyquant-io`. `lib.rs` declares the public module
tree, re-exports the top-level `to_bytes` / `from_bytes` / `CompressedVectorView`
convenience symbols, and gates `mmap` and `parallelism` modules behind the
matching Cargo features. `errors.rs` defines `IoError`, the unified error type
for all I/O operations. The four subdirectories (`compressed_vector/`,
`codec_file/`, `mmap/`, `zero_copy/`) each own one layer of the I/O stack.
`parallelism.rs` (feature-gated `rayon`) provides parallel decode helpers.

## What this area contains

- primary responsibility: crate root module tree — module declarations, feature gating for `mmap` and `rayon`, and re-exports of the primary public API
- main entrypoints: `lib.rs` (module map and public re-exports), `errors.rs` (`IoError`), `compressed_vector/` and `codec_file/` subdirectories
- common changes: adding a new top-level module, adjusting feature gates, extending `IoError`, wiring a new public re-export

## Layout

```text
src/
├── codec_file/
├── compressed_vector/
├── mmap/
├── zero_copy/
├── errors.rs
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
