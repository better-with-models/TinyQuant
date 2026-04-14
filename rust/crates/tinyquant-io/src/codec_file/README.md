# rust/crates/tinyquant-io/src/codec_file

The Level-2 TQCV corpus file container. A TQCV file begins with a fixed 24-byte header (magic `TQCV`/`TQCX`, version, vector count, dimension, bit-width, residual flag, config-hash length), followed by an optional metadata blob, and then a sequence of length-prefixed Level-1 records. Files in progress use the tentative magic `TQCX`; `finalize()` back-patches the count and flips the magic to `TQCV`.

## What lives here

| File | Public items |
|---|---|
| `header.rs` | `CorpusFileHeader`, encode/decode, `FIXED_HEADER_SIZE = 24`, magic constants |
| `metadata.rs` | `MetadataBlob<'a>` — opaque borrowed metadata bytes |
| `reader.rs` | `CodecFileReader<R: Read + Seek>` — streaming reader |
| `writer.rs` | `CodecFileWriter` — streaming writer with tentative-to-final promotion |
| `mod.rs` | Re-exports `CorpusFileHeader`, `MetadataBlob`, `CodecFileReader`, `CodecFileWriter` |

## How this area fits the system

The Python CLI and benchmarking tools ingest TQCV files written by `CodecFileWriter`. The memory-mapped reader in `../mmap/` is an alternative to `CodecFileReader` for read-heavy workloads. Both readers validate the header and reject files with unknown version bytes or reserved-field violations.

Write protocol: `CodecFileWriter::create` → one or more `append` calls → `finalize`. Aborting without `finalize` leaves a `TQCX` file that readers will reject as tentative.

## Common edit paths

- **Header field additions** — `header.rs`; increment `FORMAT_VERSION`, update encode/decode, update `CorpusFileHeader` struct
- **Reader streaming logic** — `reader.rs`
- **Writer finalization** — `writer.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
