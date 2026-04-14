# rust/crates/tinyquant-io/src/mmap

Memory-mapped Level-2 TQCV corpus file reader, compiled only when `feature = "mmap"` is enabled. The entire file is mapped read-only via `memmap2`. Iteration yields zero-copy `CompressedVectorView`s that borrow directly from the mapped region, avoiding per-record heap allocation.

## What lives here

| File | Public items |
| --- | --- |
| `corpus_file.rs` | `CorpusFileReader`, `CorpusFileIter` |
| `mod.rs` | Feature-gated re-exports of the above |

`CorpusFileReader::open` validates the Level-2 header immediately. `CorpusFileIter` yields `Result<CompressedVectorView<'_>, IoError>` with the lifetime tied to the reader's mapped region.

## How this area fits the system

This module is an alternative to `../codec_file/reader.rs` for scenarios where the entire corpus is read repeatedly (e.g., benchmarks, search). The `mmap` feature is off by default and must be explicitly enabled in `Cargo.toml`.

Safety note: memory-mapping is inherently unsafe — external modification of the file while a `CorpusFileReader` is alive results in undefined behaviour. Callers must ensure the TQCV file is not truncated or overwritten during iteration.

## Common edit paths

- **Iterator logic** — `corpus_file.rs`, `CorpusFileIter::next`
- **Header validation changes** — delegate to `../codec_file/header.rs`; only the open/map logic lives here

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
