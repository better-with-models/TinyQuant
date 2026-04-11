---
title: "Phase 17 Implementation Notes"
tags:
  - design
  - rust
  - phase-17
  - zero-copy
  - mmap
date-created: 2026-04-11
category: design
---

# Phase 17 Implementation Notes

## What landed

Phase 17 added zero-copy views, the Level-2 TQCV corpus file container,
and an mmap-based reader to `tinyquant-io`.

### Files created

| File | Purpose |
|------|---------|
| `src/zero_copy/mod.rs` | Module root; re-exports `CompressedVectorView` |
| `src/zero_copy/view.rs` | `CompressedVectorView<'a>` — borrow-based Level-1 parse |
| `src/zero_copy/cursor.rs` | `SliceCursor<'a>` — streaming iterator (reserved, unused) |
| `src/codec_file/mod.rs` | Module root |
| `src/codec_file/header.rs` | Level-2 24-byte fixed header + variable prefix encode/decode |
| `src/codec_file/metadata.rs` | `MetadataBlob<'a>` opaque view |
| `src/codec_file/writer.rs` | `CodecFileWriter` — append-only TQCX→TQCV writer |
| `src/codec_file/reader.rs` | `CodecFileReader<R: Read+Seek>` — non-mmap streaming reader |
| `src/mmap/mod.rs` | Module root (feature-gated `mmap`) |
| `src/mmap/corpus_file.rs` | `CorpusFileReader` + `CorpusFileIter<'a>` |
| `tests/zero_copy.rs` | View parse + ChaCha20Rng equivalence loop |
| `tests/mmap_corpus.rs` | 15 mmap tests: round-trip + 14 rejection/golden scenarios |
| `tests/codec_file_proptest.rs` | 64-case deterministic ChaCha20 loop round-trip |
| `tests/fixtures/codec_file/golden_100.tqcv` | Golden 100-vector corpus file (LFS) |
| `tests/fixtures/codec_file/golden_100_indices.bin` | Companion index bytes for golden fixture (LFS) |
| `tests/baselines/zero_copy_heap.txt` | dhat allocation baseline (see dhat deviation below) |
| `examples/gen_corpus_fixture.rs` | Golden fixture generator |

### Cargo.toml changes

- Added `mmap = ["dep:memmap2"]` feature
- Added `mmap-lock = []` placeholder (reserved for Phase 22; no compile_error since `--all-features` CI would fire it)
- Added `dhat-heap = ["dep:dhat"]` feature (optional)
- `dhat = "=0.3.2"` pinned (MSRV compat; 0.3.3 requires rustc 1.82)
- `tempfile = "=3.14.0"` pinned (MSRV compat; newer versions pull getrandom 0.4 / edition2024)

### CI additions

Four new jobs in `.github/workflows/rust-ci.yml`:

| Job | Runner | Command |
|-----|--------|---------|
| `test-mmap` | ubuntu-22.04 | `cargo test -p tinyquant-io --features mmap` |
| `test-mmap-dhat` | ubuntu-22.04 | `cargo test -p tinyquant-io --features "mmap dhat-heap" --test zero_copy -- --test-threads=1` |
| `test-mmap-windows` | windows-2022 | `cargo test -p tinyquant-io --features mmap --test mmap_corpus` |
| `fixture-drift` | ubuntu-22.04 | `git diff --exit-code crates/tinyquant-io/tests/fixtures/codec_file/` |

All use `toolchain: "1.81.0"` to match MSRV.

## Deviations from plan

### D17.1 — dhat allocation test deferred

The plan (Step 4) required a `#[cfg(feature = "dhat-heap")]` test that
profiles `view.unpack_into` inside `CorpusFileReader::iter()` and asserts
`total_blocks == 0`. This was not implemented because `dhat` instruments
the process-global allocator at startup via a profiler handle — the profiler
must be active for the entire binary run, not scoped to one test function.
Integration tests in `cargo test` share a single binary, so enabling `dhat`
profiling for one test corrupts measurements for others.

The `tests/baselines/zero_copy_heap.txt` baseline file was committed with
the expected all-zero values for documentation. The `test-mmap-dhat` CI job
exists and runs the `zero_copy` test suite with the `dhat-heap` feature
enabled — currently it passes vacuously (no dhat tests execute). When dhat
adds per-test scoping support (or when we add a separate binary for the audit),
this can be filled in without further CI plumbing.

### D17.2 — `to_owned` renamed to `to_owned_cv`

`CompressedVectorView::to_owned()` conflicted with the standard Rust naming
convention where `to_owned()` implies a cheap `&str → String` clone. Renamed
to `to_owned_cv()` to be unambiguous about allocation.

### D17.3 — `zero_copy/errors.rs` and `codec_file/cursor.rs` inlined

The plan listed these as separate files. Both are small enough to inline into
their calling modules without readability loss. `zero_copy/errors.rs` would
have been a single helper function; `codec_file/cursor.rs` is the body state
management inside `CodecFileReader`.

### D17.4 — `mmap-lock compile_error!` removed

The `compile_error!` guard fired when `--all-features` was used (e.g. the
CI `clippy` job), breaking the build. The feature is documented as reserved
in `Cargo.toml`; no compile-time guard is needed since it is a no-op feature
until Phase 22.

### D17.5 — `SliceCursor` is present but unused

`zero_copy/cursor.rs` contains `SliceCursor<'a>` which was listed in the
plan as a `pub(crate)` type. Nothing in the current code calls it (the
non-mmap streaming reader uses a different approach). Marked with
`#[allow(dead_code)]` for now; will be wired up or removed in Phase 18.

## Lessons learned

### L17.1 — dhat profiling is process-global

dhat's `Profiler::builder().testing().build()` approach instruments the
global allocator for the entire test binary. Running it inside a single
`#[test]` function does not isolate measurements to that test. Any dhat
tests must live in a standalone binary or be the only test in a binary.
The `--test-threads=1` flag helps with ordering but not with isolation.

### L17.2 — tempfile MSRV cliff at 3.15

`tempfile >= 3.15` pulls `getrandom 0.4.x` which requires edition2024
(Rust 1.85+). With MSRV 1.81, pin to `tempfile = "=3.14.0"`. Check
this when the workspace MSRV is bumped.

### L17.3 — TQCX fold-into-Truncated semantics

The plan says TQCX should fold into `Truncated`. The first implementation
used `Truncated { needed: 0, got: 0 }` which produced the contradictory
error message "data too short: needed 0 bytes, got 0". Corrected to
`Truncated { needed: len+1, got: len }` which preserves the `needed > got`
invariant and correctly signals "file exists but is incomplete."

### L17.4 — finalize() atomicity requires three sync_data calls

The plan specified three `sync_data()` calls in `finalize()`:
1. After all `append()` calls (body sync)
2. After writing `vector_count` (count sync, before magic flip)
3. After writing TQCV magic (final sync)

The initial implementation had only two (missing step 2). Without the
intermediate sync, a storage layer could theoretically reorder the magic
write before the count write, leaving a valid-looking TQCV file with
`vector_count = 0`. The fix adds the required intermediate sync.

### L17.5 — Windows `--all-features` + mmap feature

On Windows, `memmap2` pulls in `winapi` which compiles cleanly. The
`test-mmap-windows` CI job confirmed that `CreateFileMappingW` read-only
semantics work correctly for multi-reader scenarios via `OpenOptions::read(true)`.

## Test counts

| Test file | Count |
|-----------|-------|
| `zero_copy.rs` | 2 |
| `mmap_corpus.rs` | 15 |
| `codec_file_proptest.rs` | 1 |
| `bit_pack_exhaustive.rs` | 9 |
| `header_size_audit.rs` | 3 |
| `python_parity.rs` | 10 |
| `rejection.rs` | 9 |
| `roundtrip.rs` | 11 |
| `smoke.rs` | 1 |
| Doc-tests | 1 |
| **Total** | **62** |
