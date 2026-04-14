# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-io`

This crate owns all serialization, file I/O, and memory-mapping for TinyQuant.
It depends on `tinyquant-core` and adds the Level-1 binary wire format
(bit-packed `CompressedVector` encode / decode via `compressed_vector::to_bytes`
and `from_bytes`), zero-copy views (`CompressedVectorView`) over serialized
records, the Level-2 TQCV corpus file container (`codec_file` — header, reader,
writer, metadata), an optional memory-mapped reader (`mmap` feature via
`memmap2`), and an optional `rayon`-parallel decode path. It is consumed by
`tinyquant-cli`, `tinyquant-bench`, and any downstream crate that reads or
writes `.tqcv` corpus files.

## What this area contains

- primary responsibility: Level-1 wire-format encode / decode, bit-packing, zero-copy views, Level-2 TQCV corpus file container, and optional mmap reader
- main entrypoints: `src/lib.rs` (public re-exports: `to_bytes`, `from_bytes`, `CompressedVectorView`), `src/codec_file/` (TQCV reader/writer), `src/compressed_vector/` (wire format)
- common changes: updating the wire format header, adding a new codec file metadata field, extending the mmap reader for a new access pattern, enabling or tuning rayon parallel decode

## Layout

```text
tinyquant-io/
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
