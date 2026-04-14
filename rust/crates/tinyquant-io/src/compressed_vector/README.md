# rust/crates/tinyquant-io/src/compressed_vector

The Level-1 binary wire format for individual `CompressedVector` records. The format is defined in `docs/design/rust/serialization-format.md`. A record begins with a 70-byte header (version byte, 64-byte NUL-padded config hash, 4-byte dimension, 1-byte bit-width), followed by bit-packed indices, a residual flag byte, and an optional FP16 residual section.

## What lives here

| File | Role |
|---|---|
| `header.rs` | 70-byte header encode/decode; defines `FORMAT_VERSION = 0x01` and `HEADER_SIZE = 70` |
| `pack.rs` | `pack_indices`: LSB-first bit-packing for bit-widths 2, 4, 8 |
| `unpack.rs` | `unpack_indices`: inverse of `pack_indices` |
| `to_bytes.rs` | `to_bytes(cv) -> Vec<u8>`: full record serialization |
| `from_bytes.rs` | `from_bytes(data) -> Result<CompressedVector, IoError>`: full record deserialization |
| `mod.rs` | Module declarations; re-exports `from_bytes`, `to_bytes`, `HEADER_SIZE` |

## How this area fits the system

`to_bytes` and `from_bytes` are the primary entry points for individual record I/O and are re-exported from the crate root. The `codec_file` writer calls `to_bytes` for each record it writes; the `zero_copy` module provides a non-allocating alternative to `from_bytes` for read-only views.

Bit-widths 2, 4, and 8 are the only supported values (`SUPPORTED_BIT_WIDTHS`). Any change to the header layout must increment `FORMAT_VERSION` and add a migration path.

## Common edit paths

- **Header layout change** — `header.rs`; update `HEADER_SIZE` constant, encode/decode functions, and the zero-copy view in `../zero_copy/view.rs`
- **Bit-packing logic** — `pack.rs` and `unpack.rs`; changes must maintain bit-exact symmetry

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
