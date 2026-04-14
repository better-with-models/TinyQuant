# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-core/src/corpus`

This module implements the `Corpus` aggregate root and its supporting domain
types. `Corpus` owns an insertion-ordered `BTreeMap` of `VectorEntry` values,
enforces the active `CompressionPolicy` (Compress, Passthrough, or Fp16),
accumulates domain events (`CorpusEvent`), and exposes insert / decompress /
remove / drain-events APIs. Supporting types include `VectorEntry` (stores a
`CompressedVector` plus optional metadata), `CompressionPolicy` / `StorageTag`,
`EntryMetaValue`, `ViolationKind`, and the crate-private `VectorIdMap` (id-to-key
index). `tinyquant-cli`, `tinyquant-io`, and `tinyquant-bench` all consume this
module via `tinyquant-core`.

## What this area contains

- primary responsibility: `Corpus` aggregate root, `VectorEntry`, `CompressionPolicy`, domain events (`CorpusEvent`, `ViolationKind`), and the crate-private `VectorIdMap`
- main entrypoints: `aggregate.rs` (`Corpus`, `BatchReport`), `mod.rs` (public re-exports), `compression_policy.rs`, `events.rs`
- common changes: adding a new `CorpusEvent` variant, extending `CompressionPolicy`, adding metadata fields to `VectorEntry`, adjusting batch-insert atomicity logic

## Layout

```text
corpus/
├── aggregate.rs
├── compression_policy.rs
├── entry_meta_value.rs
├── errors.rs
├── events.rs
├── mod.rs
├── README.md
├── vector_entry.rs
└── vector_id_map.rs
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
