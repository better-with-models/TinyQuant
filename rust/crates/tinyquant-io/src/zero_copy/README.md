# rust/crates/tinyquant-io/src/zero_copy

Zero-copy views into Level-1 serialized `CompressedVector` data. `CompressedVectorView` parses a byte slice in-place without heap allocation — all fields borrow from the input. `SliceCursor` provides a streaming iterator over a contiguous byte buffer containing multiple Level-1 records.

## What lives here

| File | Public items |
| --- | --- |
| `view.rs` | `CompressedVectorView<'a>` — zero-copy parse; `unpack_into` materializes indices |
| `cursor.rs` | `SliceCursor<'a>` — streaming iterator over a byte slice of packed Level-1 records |
| `mod.rs` | Re-exports `CompressedVectorView`; `cursor` is crate-internal |

`CompressedVectorView` fields (`format_version`, `config_hash`, `dimension`, `bit_width`, packed byte slice) all borrow from the input. `unpack_into` allocates a `Vec<u8>` for the unpacked indices on demand. `parse` does not implement `serde::Serialize` by design (enforced by a `compile_fail` doc-test).

## How this area fits the system

The `mmap` reader yields `CompressedVectorView` values directly from the mapped file region. Search backends can inspect `config_hash` and `dimension` from the view before deciding whether to allocate a full `CompressedVector` via `from_bytes`.

`CompressedVectorView` is re-exported from the crate root (`tinyquant_io::CompressedVectorView`).

## Common edit paths

- **Adding fields to the Level-1 header** — `view.rs` must be updated in lockstep with `../compressed_vector/header.rs`
- **Cursor iteration logic** — `cursor.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
