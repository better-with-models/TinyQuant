# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-io/src/zero_copy`

This module provides zero-copy read access to serialized `.tqcv` corpus files.
`CompressedVectorView` maps byte slices directly to on-disk layouts without
deserializing into heap-allocated structs; `SliceCursor` advances through a
byte buffer with bounds-checked reads. Callers are `mmap_corpus.rs` and the
Python parity test.

## What this area contains

- primary responsibility: zero-copy byte-slice views over serialized compressed vectors
- main entrypoints: `view.rs` (`CompressedVectorView`), `cursor.rs` (`SliceCursor`), `mod.rs` (re-exports)
- common changes: adjusting the view layout when the binary format version changes

## Layout

```text
zero_copy/
├── cursor.rs
├── mod.rs
├── README.md
└── view.rs
```

## Common workflows

### Update existing behavior

1. Read `view.rs` and `cursor.rs` before touching the byte-layout logic.
2. Any format change here must be mirrored in `tests/zero_copy.rs` baselines.
3. Update `mod.rs` re-exports when adding a new public type.
4. Run `cargo test -p tinyquant-io zero_copy` before widening the change.

### Add a new file or module

1. Confirm the new file is a zero-copy concern, not a higher-level codec concern.
2. Update the layout section and `mod.rs` if structure changes.
3. Add a corresponding unit test in `tests/zero_copy.rs`.

## Invariants — Do Not Violate

- Views must not allocate; all reads go through `SliceCursor` bounds checks.
- Public types must implement `Copy` or carry only lifetime-bounded slice refs.
- Binary compatibility: changing a view layout is a breaking format change and requires a version bump in the file header.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../../AGENTS.md)
