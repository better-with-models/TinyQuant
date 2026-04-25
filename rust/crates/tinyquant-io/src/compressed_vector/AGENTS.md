# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-io/src/compressed_vector`

This module implements the Level-1 binary wire format for `CompressedVector` as
specified in `docs/design/rust/serialization-format.md`. `to_bytes.rs` encodes a
`CompressedVector` to the wire format (header + bit-packed index payload).
`from_bytes.rs` decodes it back. `pack.rs` and `unpack.rs` implement the
bit-packing and unpacking of variable-width (2-, 4-, or 8-bit) index arrays.
`header.rs` defines the fixed-size record header and exposes `HEADER_SIZE`. The
public API (`to_bytes`, `from_bytes`, `HEADER_SIZE`) is re-exported from
`tinyquant_io` directly. `zero_copy::CompressedVectorView` builds on top of
this format without copying.

## What this area contains

- primary responsibility: Level-1 wire-format encode (`to_bytes`) and decode (`from_bytes`), bit-pack / unpack for 2 / 4 / 8-bit index arrays, and the binary record header
- main entrypoints: `mod.rs` (re-exports `from_bytes`, `to_bytes`, `HEADER_SIZE`), `pack.rs` / `unpack.rs` (bit manipulation), `header.rs` (header layout constants)
- common changes: updating the header when the wire format version changes, fixing an edge case in bit-pack / unpack, adjusting `HEADER_SIZE` when new header fields are added

## Layout

```text
compressed_vector/
├── from_bytes.rs
├── header.rs
├── mod.rs
├── pack.rs
├── README.md
├── to_bytes.rs
└── unpack.rs
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
