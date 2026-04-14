# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-io/src/codec_file`

This module implements the Level-2 TQCV corpus file container that wraps a
sequence of Level-1 wire-format records with a file-level header and optional
metadata. `header.rs` defines `CorpusFileHeader` (magic bytes, version, record
count, dimension, bit-width, and config hash). `metadata.rs` defines
`MetadataBlob` for opaque per-file metadata. `writer.rs` provides
`CodecFileWriter` for streaming `CompressedVector` records into a `.tqcv` file.
`reader.rs` provides `CodecFileReader` for sequential decoding. This module is
consumed by `tinyquant-cli` corpus commands, the `gen_corpus_fixture` example,
and the mmap reader in `mmap/`.

## What this area contains

- primary responsibility: Level-2 TQCV corpus file — `CorpusFileHeader`, `MetadataBlob`, `CodecFileWriter` (streaming write), and `CodecFileReader` (sequential read)
- main entrypoints: `mod.rs` (re-exports all four public items), `writer.rs`, `reader.rs`, `header.rs`
- common changes: adding a new header field, extending `MetadataBlob`, changing the magic bytes or file version, adjusting reader / writer buffer logic

## Layout

```text
codec_file/
├── header.rs
├── metadata.rs
├── mod.rs
├── reader.rs
├── README.md
└── writer.rs
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
