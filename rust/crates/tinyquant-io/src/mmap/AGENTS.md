# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-io/src/mmap`

This module provides a memory-mapped reader for TQCV corpus files, compiled only
when `feature = "mmap"` is enabled (backed by `memmap2`). `corpus_file.rs`
defines `CorpusFileReader` and `CorpusFileIter`, which open a `.tqcv` file,
validate its header, and expose zero-copy iteration over `CompressedVectorView`
records without loading the entire file into heap memory. The optional
`mmap-lock` feature can be combined to lock the mapping in RAM. This module is
consumed by performance-critical read paths in `tinyquant-cli` and
`tinyquant-bench`.

## What this area contains

- primary responsibility: memory-mapped TQCV corpus file reader — `CorpusFileReader` (open + validate header) and `CorpusFileIter` (zero-copy iteration over `CompressedVectorView` records)
- main entrypoints: `corpus_file.rs` (both public types), `mod.rs` (feature-gated re-exports)
- common changes: adjusting mapping or locking strategy, extending `CorpusFileIter` with random-access or seek, adapting to header format changes in `codec_file/header.rs`

## Layout

```text
mmap/
├── corpus_file.rs
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
