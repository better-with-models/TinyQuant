# rust/crates/tinyquant-io

`tinyquant-io` provides all serialization, bit-packing, and file I/O for TinyQuant. It depends on `tinyquant-core` for the domain types it encodes and decodes. This crate owns the Level-1 binary wire format for individual `CompressedVector` records and the Level-2 TQCV corpus file container. Optional features add memory-mapped reading (`mmap`) and Rayon-backed batch parallelism (`rayon`).

## What lives here

- `src/lib.rs` — crate root; declares modules, re-exports `to_bytes`, `from_bytes`, `CompressedVectorView`
- `src/errors.rs` — `IoError` enum covering truncation, unknown version, invalid bit-width, and I/O failures
- `src/compressed_vector/` — Level-1 binary wire format: header, bit-pack/unpack, `to_bytes`, `from_bytes`
- `src/codec_file/` — Level-2 TQCV corpus file: 24-byte header, metadata blob, streaming reader and writer
- `src/zero_copy/` — `CompressedVectorView` (zero-copy parse) and `SliceCursor` (streaming iterator)
- `src/mmap/` — `CorpusFileReader` and `CorpusFileIter` backed by `memmap2` (`feature = "mmap"`)
- `src/parallelism.rs` — `rayon_parallelism()` driver for batch compress (`feature = "rayon"`)

## How this area fits the system

Consumers call `to_bytes` / `from_bytes` for individual records, or use `CodecFileWriter` / `CodecFileReader` for corpus files. The Python CLI and benchmarking tools consume TQCV files written by this crate. `tinyquant-core` has no dependency on this crate; the dependency is one-way.

The Level-1 wire format is defined in `docs/design/rust/serialization-format.md`. The Level-2 TQCV header is documented in `src/codec_file/header.rs`. Both formats are versioned (`FORMAT_VERSION = 0x01`); any backward-incompatible change requires a new version byte.

## Common edit paths

- **Wire format change** — `src/compressed_vector/header.rs`, then `pack.rs`/`unpack.rs`, `to_bytes.rs`, `from_bytes.rs`, `zero_copy/view.rs`
- **Corpus file format change** — `src/codec_file/header.rs`, `reader.rs`, `writer.rs`
- **mmap reader** — `src/mmap/corpus_file.rs`
- **Rayon parallelism** — `src/parallelism.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
