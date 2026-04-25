# rust/crates/tinyquant-io/src

The full source tree for `tinyquant-io`. The crate root (`lib.rs`) declares five unconditional modules plus two feature-gated ones (`mmap`, `rayon`). The two complex subsystems — `compressed_vector` and `codec_file` — live in subdirectories; simpler modules are single files.

## What lives here

| File/dir | Compiled when | Purpose |
| --- | --- | --- |
| `lib.rs` | always | Crate root, top-level re-exports |
| `errors.rs` | always | `IoError` enum |
| `compressed_vector/` | always | Level-1 wire format encode/decode |
| `codec_file/` | always | Level-2 TQCV corpus file container |
| `zero_copy/` | always | `CompressedVectorView`, `SliceCursor` |
| `parallelism.rs` | `feature = "rayon"` | `rayon_parallelism()` driver |
| `mmap/` | `feature = "mmap"` | Memory-mapped corpus file reader |

## How this area fits the system

All public I/O entry points (`to_bytes`, `from_bytes`, `CompressedVectorView`, the `codec_file` reader/writer) are re-exported from `lib.rs`. Downstream users import from the crate root; module paths are implementation detail.

## Common edit paths

- **Serialization logic** — `compressed_vector/` subdirectory
- **Corpus file I/O** — `codec_file/` subdirectory
- **Error variants** — `errors.rs`
- **Zero-copy parsing** — `zero_copy/`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
