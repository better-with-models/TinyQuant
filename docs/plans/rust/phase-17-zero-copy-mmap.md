---
title: "Phase 17: Zero-copy Views and Mmap Corpus Files"
tags:
  - plans
  - rust
  - phase-17
  - zero-copy
  - mmap
date-created: 2026-04-10
status: draft
category: planning
---

# Phase 17: Zero-copy Views and Mmap Corpus Files

> [!info] Goal
> Add `CompressedVectorView<'a>` (zero-copy parse), the Level-2
> corpus file container (`TQCV` magic), and an mmap reader that
> iterates a multi-gigabyte corpus file without allocating per
> vector.

> [!note] Reference docs
> - [[design/rust/serialization-format|Serialization Format]] §Level 2, §CompressedVectorView
> - [[design/rust/memory-layout|Memory Layout]] §Zero-copy, §Mmap alignment
> - [[design/rust/feature-flags|Feature Flags]] — `mmap`, `simd`, `dhat-heap` gating
> - [[design/rust/error-model|Error Model]] §`IoError`
> - [[design/rust/testing-strategy|Testing Strategy]] §Fuzz, §Property tests, dhat allocation audits
> - [[design/rust/ci-cd|CI/CD]] §`rust-ci.yml` jobs, LFS hydration
> - [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]] §L1, §L2, §L6, §L7

> [!warning] Design-doc drift noted during Phase 17 planning
> The Level-2 header layout specified in this phase plan (see
> [[#Level-2 file container layout]] below) intentionally differs
> from [[design/rust/serialization-format|Serialization Format]]
> §Level 2 in three places: the `format_version` is encoded as a
> single byte with a 3-byte reserved pad (instead of `u16 LE` +
> `u16 LE flags`), the `config_hash` is length-prefixed rather than
> fixed at 64 null-padded bytes, and the metadata section is
> application-opaque `(u32 LE length, bytes)` rather than
> `(32-byte SHA-256, CBOR)`. The reason for the drift is
> **alignment**: the new layout places the first record header at a
> deterministic 8-byte-aligned offset after a variable-length
> config-hash prefix, which is a hard requirement for the mmap
> safety contract in [[#Mmap safety contract]] below. The Phase 17
> acceptance gate is this plan; a follow-up doc PR is queued to
> reconcile [[design/rust/serialization-format]] §Level 2 before
> Phase 18 starts.

## Prerequisites

- Phase 16 complete (`to_bytes` / `from_bytes` in place, `IoError`
  baseline variants land, fixtures refresh harness exists in
  `xtask`).
- Git LFS configured for `rust/crates/tinyquant-io/tests/fixtures/**`
  (Phase 14 §L2 debt — verify the YAML carries `with: { lfs: true }`
  before merging).

## Deliverables

### Level-2 file container layout

The Level-2 container (`TQCV`) wraps a stream of Level-1 encoded
`CompressedVector` payloads with a fixed-shape header, an opaque
metadata blob, and length-prefixed record entries. The header is
padded so that the first record begins at an 8-byte-aligned file
offset, which is the precondition for the mmap pointer-arithmetic
guarantees in [[#Mmap safety contract]].

**Header (fixed-prefix section, 24 bytes):**

| offset | length | field | encoding | notes |
|--------|--------|-------|----------|-------|
| 0 | 4 | `magic` | ASCII bytes | `b"TQCV"` on a finalized file, `b"TQCX"` on an in-progress writer |
| 4 | 1 | `format_version` | `u8` | starts at `0x01`; readers refuse any other value |
| 5 | 3 | `reserved` | 3× `u8` | must be `0x00 0x00 0x00`; rejected otherwise |
| 8 | 8 | `vector_count` | `u64` LE | back-patched by `finalize()` |
| 16 | 4 | `dimension` | `u32` LE | must be non-zero |
| 20 | 1 | `bit_width` | `u8` | one of `{2, 4, 8}` |
| 21 | 1 | `residual_flag` | `u8` | `0x00` = no residuals throughout; `0x01` = residuals throughout |
| 22 | 2 | `config_hash_len` | `u16` LE | number of UTF-8 bytes in `config_hash`, `≤ 256` |

**Variable-length prefix (header extension):**

| offset | length | field | encoding | notes |
|--------|--------|-------|----------|-------|
| 24 | `config_hash_len` | `config_hash` | UTF-8 bytes | not null-padded; trimmed by length |
| 24 + `config_hash_len` | 4 | `metadata_len` | `u32` LE | opaque body length in bytes, `≤ 16 MiB` |
| 28 + `config_hash_len` | `metadata_len` | `metadata` | opaque bytes | application-defined; see [[#Metadata section]] |
| ... | `pad` | `pad` | `0x00` bytes | 0–7 bytes of trailing zero pad so the body begins at an 8-byte-aligned offset |

**Body (records section):**

Immediately after the header pad, the body contains exactly
`vector_count` length-prefixed records:

```text
[u32 LE record_length][record_length bytes of Level-1 payload]
...repeated vector_count times...
```

Each record's payload is the Level-1 format defined in
[[design/rust/serialization-format|Serialization Format]] §Level 1
(70-byte header + packed indices + residual). The outer `u32`
prefix exists so the mmap iterator can advance without re-parsing
the Level-1 header just to find the next record boundary.

**Alignment rules:**

- The header prefix is a fixed 24 bytes, which is `0 mod 8`.
- The variable-length section `(config_hash, metadata_len, metadata)`
  may have any length; the writer inserts `pad = (8 - (header_end %
  8)) % 8` zero bytes so the body starts at an 8-byte-aligned file
  offset. This is the reason the 3-byte reserved field in the fixed
  prefix exists — it keeps the fixed prefix itself 8-byte-aligned,
  independent of the pad.
- Record payloads are **not** individually padded. The per-record
  alignment contract is only "the `u32` length prefix is u32-aligned
  if the containing page is aligned", which is sufficient for
  `u32::from_le_bytes(try_into().unwrap())` on a 4-byte slice.

**Back-patching policy for `vector_count`:**

1. `CodecFileWriter::create` writes the entire header with
   `vector_count = 0` and the magic bytes set to `b"TQCX"` (the
   "tentative" marker).
2. Each `append(cv)` call writes the record prefix + payload and
   increments an in-memory counter.
3. `finalize()` seeks to offset `8`, writes the final counter as
   `u64` LE, `fsync`s, seeks to offset `0`, writes `b"TQCV"`, and
   `fsync`s a second time. See [[#Atomic finalize rules]].

### Metadata section

The `metadata` bytes are opaque to the Level-2 reader. Phase 18
introduces the `CorpusAggregate` metadata schema and will define
a CBOR encoding on top of this opaque slot; Phase 17 merely
preserves the bytes round-trip and exposes them through
`MetadataBlob<'a>` (see [[#Files to create]]).

### Files to create

| File | Purpose | Module visibility | Feature-gated |
|------|---------|-------------------|---------------|
| `rust/crates/tinyquant-io/src/zero_copy/mod.rs` | Module root, re-exports `CompressedVectorView` | `pub` | — |
| `rust/crates/tinyquant-io/src/zero_copy/view.rs` | `CompressedVectorView<'a>` definition + `parse` | `pub` | — |
| `rust/crates/tinyquant-io/src/zero_copy/cursor.rs` | Stream iterator over a byte slice (non-mmap) | `pub(crate)` | — |
| `rust/crates/tinyquant-io/src/zero_copy/errors.rs` | Parse-error helpers that fold into `IoError` (see note) | `pub(crate)` | — |
| `rust/crates/tinyquant-io/src/mmap/mod.rs` | Module root | `pub` | `#[cfg(feature = "mmap")]` |
| `rust/crates/tinyquant-io/src/mmap/corpus_file.rs` | `CorpusFileReader`, `CorpusFileIter` | `pub` | `#[cfg(feature = "mmap")]` |
| `rust/crates/tinyquant-io/src/codec_file/mod.rs` | Module root | `pub` | — |
| `rust/crates/tinyquant-io/src/codec_file/header.rs` | Shared Level-2 header encode/decode (distinct from the per-CV header in Phase 16) | `pub(crate)` | — |
| `rust/crates/tinyquant-io/src/codec_file/metadata.rs` | Opaque `MetadataBlob<'a>` view | `pub` | — |
| `rust/crates/tinyquant-io/src/codec_file/writer.rs` | `CodecFileWriter` append-only writer | `pub` | — |
| `rust/crates/tinyquant-io/src/codec_file/reader.rs` | Streaming reader (non-mmap, `Read + Seek`) | `pub` | — |
| `rust/crates/tinyquant-io/src/codec_file/cursor.rs` | Non-mmap streaming reader state (buffer management) | `pub(crate)` | — |
| `rust/crates/tinyquant-io/tests/zero_copy.rs` | View parse tests | — | — |
| `rust/crates/tinyquant-io/tests/mmap_corpus.rs` | Mmap round-trip + rejection tests | — | `#[cfg(feature = "mmap")]` |
| `rust/crates/tinyquant-io/tests/codec_file_proptest.rs` | Proptest / ChaCha20Rng loop round-trip (see [[#Step 7: Property coverage]]) | — | — |
| `rust/crates/tinyquant-io/tests/fixtures/codec_file/golden_100.tqcv` | Committed fixture: 100 vectors, dim=64, bw=4, residual off | — | tracked via Git LFS |
| `rust/crates/tinyquant-io/tests/baselines/zero_copy_heap.txt` | dhat heap-stats baseline (see [[#Step 4: Allocation audit]]) | — | — |
| `rust/xtask/src/cmd/fixtures.rs` (extension) | New `refresh-corpus-file` subcommand that regenerates `golden_100.tqcv` | existing | — |

> [!note] Parse-error handling: fold into `IoError`
> Rather than introducing a new error enum in
> `zero_copy/errors.rs`, this phase adds the missing variants
> `BadMagic`, `InvalidHeader`, and `TentativeFile` to the existing
> `IoError` in `tinyquant-io/src/errors.rs` (Phase 16 owner). The
> `zero_copy/errors.rs` file is therefore a thin helper module
> (`pub(crate) fn err_bad_magic(got: [u8; 4]) -> IoError { ... }`),
> not a new error type. This keeps the cross-crate error surface
> stable and avoids a `From` conversion boilerplate at the
> `mmap::corpus_file` → caller boundary.

### `CompressedVectorView<'a>` contract

The view is a **borrow**, not an owned value. It never allocates
on construction, never copies bytes, and its lifetime `'a` is tied
to the input byte slice (in practice, `&'a Mmap`).

```rust
// tinyquant-io/src/zero_copy/view.rs
pub struct CompressedVectorView<'a> {
    pub format_version: u8,
    pub config_hash: &'a str,
    pub dimension: u32,
    pub bit_width: u8,
    pub packed_indices: &'a [u8],
    pub residual: Option<&'a [u8]>,
}

impl<'a> CompressedVectorView<'a> {
    /// Parse a single Level-1 record from the head of `data`.
    /// Returns the parsed view and the unconsumed tail so that
    /// iterators can chain successive calls without re-computing
    /// offsets.
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), IoError> { /* ... */ }

    /// Unpack the indices into a caller-provided buffer. Zero heap
    /// allocation; returns `IoError::LengthMismatch` if the buffer
    /// length does not equal `self.dimension as usize`.
    pub fn unpack_into(&self, out: &mut [u8]) -> Result<(), IoError> { /* ... */ }

    /// Single-shot escape hatch that copies the borrowed bytes into
    /// a fresh `CompressedVector`. **Allocates**; documented as
    /// such. Use only when the caller needs an owned value across a
    /// lifetime boundary.
    pub fn to_owned(&self) -> CompressedVector { /* ... */ }
}
```

**Field lifetime justification:**

- `config_hash: &'a str` — borrowed directly from the Level-1
  header bytes. UTF-8 validity is checked once by `parse`; from
  that point on the slice is a trusted `&str`.
- `packed_indices: &'a [u8]` — borrowed from the record body, no
  copy. `unpack_into` is the only way to materialize unpacked
  indices and it writes into a caller buffer.
- `residual: Option<&'a [u8]>` — borrowed from the record body
  when the residual flag is set, `None` otherwise.

**Safety envelope:**

- `parse` only reads from `data`. It never writes, never casts to
  a mutable pointer, never calls an `unsafe` function. Any `unsafe`
  at this layer is **forbidden**; the mmap crossing lives one
  layer down in `mmap::corpus_file`.
- `parse` returns `(Self, &'a [u8])` rather than `(Self, usize)`
  so the returned tail is lifetime-checked by the compiler and the
  iterator cannot accidentally overrun the mmap region.

**Explicit forbidding of serde derives:**

`CompressedVectorView` must not implement `serde::Serialize` or
`serde::Deserialize`. The view is a borrow and has no owned
representation; any derived `Deserialize` would need to allocate
backing storage and would silently defeat the zero-alloc contract.
A crate-level `#![cfg_attr(feature = "serde", deny(...))]`
construct is not available; instead, a doctest asserts that the
type does not satisfy `Serialize`:

```rust
/// ```compile_fail
/// fn assert_not_serialize<T: serde::Serialize>() {}
/// assert_not_serialize::<tinyquant_io::CompressedVectorView<'_>>();
/// ```
```

### Mmap safety contract

The mmap crossing is the only place in `tinyquant-io` that pulls
memory from an external source into a Rust `&[u8]`. This subsection
defines the invariants that make that crossing sound.

**Truncation mid-iteration.** If the file is truncated after
`CorpusFileReader::open` but before the iterator has consumed every
record, the next `CorpusFileIter::next()` call parses an
insufficient number of bytes and yields exactly one
`Err(IoError::Truncated)`, then `None`. The iterator is fused — it
never yields another error or another `Ok` after a `Truncated` has
been observed.

**Modification-under-feet.** If the file is modified by another
process while the mmap is live, the mapped pages may change
out-of-band. This is **documented as undefined behavior** at the
type level:

```rust
// tinyquant-io/src/mmap/corpus_file.rs
/// # Safety invariants
///
/// The caller guarantees that while the returned
/// [`CorpusFileReader`] is alive, **no other process writes to the
/// backing file**. Violating this guarantee can produce
/// inconsistent parse results and, on some platforms, memory
/// safety violations.
///
/// When the `mmap-lock` feature is enabled, the reader takes an
/// OS-level shared lock (Unix: `flock(LOCK_SH)`; Windows:
/// `LockFileEx(LOCKFILE_FAIL_IMMEDIATELY)`) and converts
/// concurrent writes into an I/O error on the writer side. Without
/// `mmap-lock` the caveat stands.
pub struct CorpusFileReader { /* ... */ }
```

The `mmap-lock` feature is **not** implemented in Phase 17; the
flag is reserved in `Cargo.toml` with a `compile_error!` guard so
that attempting to enable it fails the build with a pointer to
this phase plan.

**Windows-specific behavior.**

- `memmap2::MmapOptions::map` calls `CreateFileMappingW` with
  `PAGE_READONLY`. Phase 17 uses **read-only** mappings only.
- `CreateFileMappingW` can fail with `ERROR_SHARING_VIOLATION` if
  another handle has the file open without the `FILE_SHARE_READ`
  flag. `CorpusFileReader::open` catches this specific OS error
  and returns `IoError::Io(std::io::Error::from_raw_os_error(32))`
  with a message that points at the sharing violation. The Phase
  14 §L4 cross-runner story is the reason we flag this explicitly:
  Windows runners in CI exhibit subtly different sharing semantics
  than developer machines, and hiding the `ERROR_SHARING_VIOLATION`
  inside a generic `std::io::Error` makes failures hard to triage.

**Unix-specific behavior.**

- On `iter()` the reader calls
  `unsafe { libc::madvise(ptr, len, libc::MADV_SEQUENTIAL) }`
  to hint the kernel toward read-ahead.
- On `get(index)` (random access, Phase 18 surface, sketch only
  here) the hint flips to `libc::MADV_RANDOM`.
- Both are **performance hints**, not safety contracts. The
  `unsafe` block carries a `// SAFETY: madvise on a valid,
  read-only mmap region is always sound; failure is ignored
  because MADV_* hints are advisory.` comment.

**Alignment.** The file header is padded so the first record
starts at an 8-byte-aligned offset. The `CorpusFileIter::next`
implementation takes advantage of this by reading the `u32`
length prefix via `u32::from_le_bytes` from a 4-byte sub-slice —
this does **not** require any alignment guarantee from the CPU
(`from_le_bytes` accepts any byte alignment), but the 8-byte
alignment is the hook that Phase 18's random-access
`get(index: u64)` will rely on.

### `CodecFileWriter` and `CodecFileReader`

`codec_file/writer.rs` exposes `CodecFileWriter::create(path,
config_hash, dimension, bit_width, residual, metadata)`,
`append(&cv)`, and a consuming `finalize(self)` — see
[[#Step 6: Implement `codec_file/writer.rs` with atomic finalize]]
for the full signatures and body.

`codec_file/reader.rs` is a streaming, non-mmap reader for
environments without `memmap2` (WASM, `no_std + alloc` hosts that
still want Level-2 playback):

```rust
pub struct CodecFileReader<R: Read + Seek> { /* ... */ }

impl<R: Read + Seek> CodecFileReader<R> {
    pub fn open(reader: R) -> Result<Self, IoError> { /* ... */ }
    pub fn header(&self) -> &CorpusFileHeader { /* ... */ }
    pub fn read_next(&mut self, buf: &mut Vec<u8>) -> Result<Option<()>, IoError> { /* ... */ }
}
```

`read_next` reuses the caller's `Vec<u8>` for per-record scratch,
so callers drive zero-alloc reads by resizing (never clearing)
the existing buffer as the next record grows.

## Steps (TDD order)

> [!tip] Clippy profile gotchas (ported from Phase 14 §L7)
> Phase 17 code runs under the same
> `pedantic + nursery + unwrap_used + expect_used + panic +
> indexing_slicing + cognitive_complexity` profile as Phase 14. Re-read
> these before the first clippy run:
>
> - **`indexing_slicing`** — all slice access in parse code must go
>   through `data.get(..)` / `data.get(0..4).ok_or(Err(...))?`,
>   never `data[..4]`. The Level-2 header parser is full of
>   bounded slice accesses; use `split_at_checked` or `get(..4)`
>   consistently.
> - **`cast_possible_truncation`** on `u64 as usize` in the
>   iterator's `record_length as usize` conversion. On 32-bit
>   targets a `u32` record length can exceed `usize::MAX` only in
>   theory; add a narrow `#[allow(clippy::cast_possible_truncation)]`
>   on the conversion with a `debug_assert!(len as u64 ==
>   len_u32 as u64)` or return `IoError::InvalidHeader` on 32-bit
>   targets via `#[cfg(target_pointer_width = "32")]`.
> - **`cast_precision_loss`** on progress-bar percentage arithmetic
>   in the writer (when fixtures are regenerated by xtask). Narrow
>   `#[allow]` on the single expression is acceptable.
> - **`missing_safety_doc`** on any `unsafe fn` in the mmap layer.
>   Phase 17 should ideally have **zero** `unsafe fn` (only
>   `unsafe { ... }` blocks inside safe fns, with `// SAFETY:`
>   comments).
> - **`trivially_copy_pass_by_ref`** on `u32`/`u64` helper args —
>   take `u32`/`u64` by value.

- [ ] **Step 1: Failing view parse test**

```rust
// tinyquant-io/tests/zero_copy.rs
#[test]
fn view_parses_serialized_compressed_vector_without_alloc() {
    // Serialize a CompressedVector with bit_width=4, dim=32.
    // Parse it via CompressedVectorView::parse; assert every field
    // matches without allocating.
    let cv = build_cv(32, 4, false);
    let bytes = tinyquant_io::to_bytes(&cv);
    let (view, tail) = CompressedVectorView::parse(&bytes).unwrap();
    assert!(tail.is_empty());
    assert_eq!(view.dimension, 32);
    assert_eq!(view.bit_width, 4);
    assert_eq!(view.config_hash, cv.config_hash().as_ref());
}
```

- [ ] **Step 2: Implement `zero_copy/view.rs`**

Per [[design/rust/serialization-format|Serialization Format]]
§CompressedVectorView. All fields borrow from the input `&'a [u8]`.
No allocation inside `parse`; no allocation inside `unpack_into`
(caller-provided buffer).

- [ ] **Step 3: Proptest / ChaCha20Rng loop — view parse agrees with from_bytes**

```rust
// Per Phase 14 §L6, proptest is blocked by MSRV 1.81. Use a
// deterministic ChaCha20Rng loop instead.
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::{RngCore, SeedableRng};

#[test]
fn view_parse_agrees_with_from_bytes() {
    let mut rng = ChaCha20Rng::seed_from_u64(17);
    for _ in 0..256 {
        let dim = 1 + (rng.next_u32() % 512);
        let bw = [2u8, 4, 8][rng.next_u32() as usize % 3];
        let cv = build_cv_rng(&mut rng, dim, bw);
        let bytes = tinyquant_io::to_bytes(&cv);
        let owned = tinyquant_io::from_bytes(&bytes).unwrap();
        let (view, tail) = CompressedVectorView::parse(&bytes).unwrap();
        assert!(tail.is_empty());
        assert_eq!(view.dimension, owned.dimension());
        assert_eq!(view.bit_width, owned.bit_width());
        let mut unpacked = vec![0u8; view.dimension as usize];
        view.unpack_into(&mut unpacked).unwrap();
        assert_eq!(unpacked.as_slice(), owned.indices());
    }
}
```

### Step 4: Allocation audit

Use `dhat` to assert zero heap allocations during
`view.unpack_into` on a pre-loaded mmap.

**Feature-flag setup** — add to `tinyquant-io/Cargo.toml`:

```toml
[features]
# existing entries elided
dhat-heap = ["dep:dhat"]

[dependencies]
dhat = { workspace = true, optional = true }
```

**Test helper:**

```rust
// tinyquant-io/tests/zero_copy.rs
#[cfg(feature = "dhat-heap")]
fn with_dhat_profile<F: FnOnce()>(f: F) -> dhat::HeapStats {
    let _profiler = dhat::Profiler::builder().testing().build();
    f();
    dhat::HeapStats::get()
}
```

**Assertions:**

```rust
#[cfg(feature = "dhat-heap")]
#[test]
fn unpack_into_allocates_zero_blocks_on_mmap() {
    let reader = CorpusFileReader::open(golden_fixture_path()).unwrap();
    let mut out = vec![0u8; 64]; // pre-allocated outside the profile
    let stats = with_dhat_profile(|| {
        for view in reader.iter() {
            let view = view.unwrap();
            view.unpack_into(&mut out).unwrap();
        }
    });
    assert_eq!(stats.total_blocks, 0, "zero-copy path allocated {stats:?}");
}
```

**Parse allocation contract.** `CompressedVectorView::parse` is
**not allowed** to allocate either. It returns a struct whose
fields are `&'a [u8]` / `&'a str` slices plus `Copy` scalars; no
heap interaction. This is explicit in the contract so future
refactors do not silently insert a `Vec::from_slice` somewhere in
the header-validation path.

**Baseline capture file:** the expected dhat heap stats summary is
committed to
`rust/crates/tinyquant-io/tests/baselines/zero_copy_heap.txt` in
the format

```text
# Generated by `cargo test --features dhat-heap
#   unpack_into_allocates_zero_blocks_on_mmap -- --nocapture`
total_blocks: 0
total_bytes: 0
max_blocks: 0
max_bytes: 0
```

The test reads this file and compares rather than asserting
literal zero, so if the fixture size grows in Phase 18 and a
one-time scratch buffer enters the hot loop, the baseline can be
updated with reviewer sign-off.

**CI gating:** the test is guarded by `#[cfg(feature =
"dhat-heap")]` so it does not run in default `cargo test`. The
`rust-ci.yml` matrix gets a dedicated job (see [[#CI integration]])
that runs `cargo test -p tinyquant-io --features "mmap dhat-heap"
--test zero_copy`. The job runs serially on a single core — dhat
profiling is not thread-safe — and therefore lives outside the
main test matrix.

- [ ] **Step 5: Failing Level-2 file write/read test**

```rust
#[test]
fn corpus_file_write_then_read_round_trip() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let cvs: Vec<CompressedVector> = build_gold_corpus(100, 64, 4);

    let mut writer = CodecFileWriter::create(
        tmp.path(),
        &config_hash,
        64,
        4,
        /* residual */ false,
        /* metadata */ &[],
    ).unwrap();
    for cv in &cvs {
        writer.append(cv).unwrap();
    }
    writer.finalize().unwrap();

    let reader = CorpusFileReader::open(tmp.path()).unwrap();
    assert_eq!(reader.header().vector_count, 100);
    assert_eq!(reader.header().dimension, 64);
    assert_eq!(reader.header().bit_width, 4);
    assert_eq!(reader.iter().count(), 100);
    for (view_res, expected) in reader.iter().zip(cvs.iter()) {
        let view = view_res.unwrap();
        assert_eq!(view.dimension, expected.dimension());
        let mut unpacked = vec![0u8; view.dimension as usize];
        view.unpack_into(&mut unpacked).unwrap();
        assert_eq!(unpacked.as_slice(), expected.indices());
    }
}
```

### Step 6: Implement `codec_file/writer.rs` with atomic finalize

```rust
pub struct CodecFileWriter {
    file: std::fs::File,
    count: u64,
    header_len: u64,
    finalized: bool,
}

impl CodecFileWriter {
    pub fn create(
        path: &Path,
        config_hash: &str,
        dimension: u32,
        bit_width: u8,
        residual: bool,
        metadata: &[u8],
    ) -> Result<Self, IoError> {
        let mut file = std::fs::File::create(path)?;
        let header = build_tentative_header(
            config_hash, dimension, bit_width, residual, metadata,
        )?;
        file.write_all(&header)?;
        Ok(Self { file, count: 0, header_len: header.len() as u64, finalized: false })
    }

    pub fn append(&mut self, cv: &CompressedVector) -> Result<(), IoError> {
        let bytes = tinyquant_io::to_bytes(cv);
        let len = u32::try_from(bytes.len()).map_err(|_| IoError::InvalidHeader)?;
        self.file.write_all(&len.to_le_bytes())?;
        self.file.write_all(&bytes)?;
        self.count += 1;
        Ok(())
    }

    pub fn finalize(mut self) -> Result<(), IoError> {
        // Phase 1: flush + sync the body.
        self.file.flush()?;
        self.file.sync_data()?;
        // Phase 2: back-patch vector_count at offset 8, then sync.
        self.file.seek(SeekFrom::Start(8))?;
        self.file.write_all(&self.count.to_le_bytes())?;
        self.file.sync_data()?;
        // Phase 3: flip the magic from TQCX -> TQCV, then final sync.
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(b"TQCV")?;
        self.file.sync_data()?;
        self.finalized = true;
        Ok(())
    }
}
// Drop leaves the file as TQCX if finalize() never ran; the reader refuses it.
```

**Atomic finalize rules in prose:**

1. `finalize` calls `self.file.sync_data()` three times. On Unix,
   `sync_data` calls `fdatasync`. On Windows, it calls
   `FlushFileBuffers` via the `std::fs::File` wrapper.
2. If any `sync_data` call returns an `io::Error`, `finalize`
   returns `IoError::Io(...)` and **the magic bytes remain
   `TQCX`**. A reader that opens the file after a crashed
   `finalize` therefore sees a tentative file and rejects it with
   `IoError::Truncated` (see the variant-mapping table below; the
   reader folds `TQCX` into `Truncated` because the semantic is
   the same from the caller's perspective: "this file is not safe
   to read yet").
3. The `TQCX → TQCV` flip is the single linearization point:
   either the reader sees `TQCV` and a well-formed file, or it
   sees `TQCX` and refuses. There is no in-between state that
   parses partially.

**Crash-injection test:**

```rust
#[test]
fn finalize_interrupted_before_magic_rewrite_is_rejected() {
    // Manually construct a file that mimics a crash after
    // vector_count write but before magic flip:
    //   magic = b"TQCX"
    //   vector_count = 100 (back-patched)
    //   body = 100 valid records
    // Assert CorpusFileReader::open(...) returns IoError::Truncated.
    let tmp = tempfile::NamedTempFile::new().unwrap();
    write_tentative_file_with_valid_body(&tmp, 100);
    let err = CorpusFileReader::open(tmp.path()).unwrap_err();
    assert!(matches!(err, IoError::Truncated { .. }));
}
```

- [ ] **Step 6a: Implement `mmap/corpus_file.rs`**

Uses `memmap2::Mmap`. `iter()` returns a borrowed iterator of
`CompressedVectorView<'_>` values produced from the mmapped bytes
without allocation. See [[#Mmap safety contract]] for the
invariants.

- [ ] **Step 6b: Run mmap round-trip test — expect pass.**

### Step 7: Property coverage

Add `tinyquant-io/tests/codec_file_proptest.rs`:

- Generate `Vec<CompressedVector>` with random `bit_width in {2, 4,
  8}` and random `dim in 1..=512`, encode them via
  `CodecFileWriter`, then iterate via `CorpusFileReader::open().iter()`
  and assert byte-for-byte equality per record (indices slice,
  residual slice, config_hash, bit_width, dimension).
- **Shrinker hints** (documented for the day proptest is unblocked):
  reduce `vector_count` first (fewer records = simpler failure),
  then per-record `dim`, then `bit_width`. Implement the loop with
  the nested-for discipline that matches this ordering so that a
  manual bisect reproduces what a real shrinker would have found.
- **Deterministic seed:** `ChaCha20Rng::seed_from_u64(17)`. Per
  Phase 14 §L6 we cannot use the `proptest` crate while the
  workspace MSRV is pinned at 1.81; the seed is fixed so failures
  reproduce byte-for-byte across runs.

### Step 8: Parity fixture — committed golden file

Generate `rust/crates/tinyquant-io/tests/fixtures/codec_file/golden_100.tqcv`
via `cargo xtask fixtures refresh-corpus-file`. The fixture has:

- `vector_count = 100`
- `dimension = 64`
- `bit_width = 4`
- `residual = false`
- `config_hash = "phase17-golden"`
- `metadata = b""`

A test asserts that `CorpusFileReader::open(golden)` succeeds and
that `reader.iter()` yields 100 views whose unpacked indices match
a sibling fixture `golden_100_indices.bin` (raw `u8` blob, one
record per 64-byte stride).

### Step 9: Rejection tests expanded

| Mutation | Byte offset | Expected `IoError` variant |
|----------|-------------|----------------------------|
| Wrong magic (e.g. `b"XXXX"`) | 0 | `BadMagic { got: [u8; 4] }` (new variant) |
| Tentative magic left as `b"TQCX"` | 0 | `Truncated { needed, got }` (folded from `TentativeFile`) |
| Unknown `format_version` | 4 | `UnknownVersion { got: u8 }` |
| Non-zero `reserved` bytes | 5..=7 | `InvalidHeader` (new variant) |
| Zero `dimension` | 16 | `InvalidHeader` (new variant) |
| `bit_width` not in `{2, 4, 8}` | 20 | `InvalidBitWidth { got: u8 }` |
| `residual_flag` not in `{0, 1}` | 21 | `InvalidHeader` |
| `config_hash_len > 256` | 22 | `InvalidHeader` |
| Excess `metadata_len` beyond file tail | 24 + `config_hash_len` | `Truncated { needed, got }` |
| Record length prefix greater than remaining bytes | variable | `Truncated { needed, got }` |
| Record header `format_version` mismatch (inner Level-1) | variable | `Decode(CodecError::UnknownVersion { got })` |
| Record header `bit_width` mismatch vs file-level `bit_width` | variable | `InvalidHeader` |

Each row corresponds to a test function in
`tinyquant-io/tests/mmap_corpus.rs` (for the mmap path) and
`tinyquant-io/tests/codec_file_proptest.rs` (for the streaming
reader). The mutation is applied by reading a valid fixture into a
`Vec<u8>`, overwriting the target byte(s), writing to a tempfile,
and reopening.

**New `IoError` variants** (owner: Phase 16; added as part of
this phase's first commit):

```rust
#[derive(Debug, thiserror::Error)]
pub enum IoError {
    // existing variants elided
    #[error("bad magic bytes: {got:?}")]
    BadMagic { got: [u8; 4] },

    #[error("invalid header field")]
    InvalidHeader,
    // note: no TentativeFile variant; TQCX is folded into Truncated
}
```

- [ ] **Step 10: Empty-file handling**

`CorpusFileReader::open` on a zero-byte file returns
`IoError::Truncated { needed: 24, got: 0 }` before any iteration.
Same for a file that is shorter than the fixed 24-byte header
prefix.

### Step 11: Bench integration

Add `tinyquant-bench/benches/zero_copy_view_iteration.rs`.
Scenarios:

1. **`from_bytes_owning`** — for each record in a pre-loaded
   `Vec<u8>`, call `from_bytes` and discard the owned
   `CompressedVector`. Measures the allocation-heavy reference path.
2. **`mmap_view`** — mmap the file, iterate `CorpusFileIter`, call
   `unpack_into` into a reused `[u8; MAX_DIM]` scratch buffer. The
   zero-alloc path that is the goal of this phase.
3. **`file_read_reuse`** — `std::fs::File` + `BufReader` + manual
   length-prefixed record read into a reused `Vec<u8>` scratch.
   Representative of a WASM / embedded host that cannot mmap.

Corpus sizes: **1K / 10K / 100K** vectors at `dim=768, bw=4,
residual=off`. The 100K corpus is generated on-demand by the bench
setup harness and cached under `target/tmp/bench-corpus-100k.tqcv`
— not committed to LFS.

Baseline numbers (the first green run's medians) are committed to
`rust/crates/tinyquant-bench/baselines/main.json` under the key
`zero_copy_view_iteration`. The bench budget gate does **not**
fire on this key in Phase 17 — budget enforcement lands in Phase
21. The phase plan notes the deferral explicitly so the author
does not try to wire up a regression alarm before the infrastructure
is ready.

- [ ] **Step 12: Run `cargo xtask fmt`, `cargo xtask lint`, `cargo xtask test`.**
- [ ] **Step 13: Commit**

```bash
git add rust/crates/tinyquant-io rust/crates/tinyquant-bench rust/xtask
git commit -m "feat(tinyquant-io): add zero-copy view, corpus file writer, and mmap reader"
```

## Acceptance criteria

- Zero-copy view parses `CompressedVector` bytes without heap
  allocation (dhat verifies against the committed baseline).
- Level-2 corpus files round-trip a 100-vector golden sample;
  header `vector_count` field is back-patched correctly on
  `finalize`, and the magic bytes flip from `TQCX` to `TQCV`
  atomically.
- Mmap iterator yields views without copying (`memmap2::Mmap` →
  `&[u8]` → `CompressedVectorView<'_>` with no intermediate
  allocation).
- All rejection tests from [[#Step 9: Rejection tests expanded]]
  produce the expected `IoError` variants.
- Crash-injection test demonstrates that a `TQCX` file is refused.
- dhat allocation audit passes: `total_blocks = 0` across the
  full iteration of `golden_100.tqcv`.
- Deterministic ChaCha20 loop covers random bit-widths and dims
  without proptest (Phase 14 §L6 substitute).
- `rust-ci.yml` matrix runs the new `mmap` + `dhat-heap` job green
  on `main`.
- Clippy + fmt clean under the existing denies.

## CI integration

New `rust-ci.yml` jobs (Phase 17 delivers the YAML edit as part
of the PR that lands the code):

| Job | Command | Runner | Gating |
|-----|---------|--------|--------|
| `test-mmap` | `cargo test -p tinyquant-io --features "mmap"` | `ubuntu-22.04` | block |
| `test-mmap-dhat` | `cargo test -p tinyquant-io --features "mmap dhat-heap" -- --test-threads=1` | `ubuntu-22.04` | block (dedicated job, single-threaded) |
| `test-mmap-windows` | `cargo test -p tinyquant-io --features "mmap" --test mmap_corpus` | `windows-2022` | block (catches path-separator + file-locking issues; ties back to Phase 14 §L4) |
| `fixture-drift` | `git diff --exit-code rust/crates/tinyquant-io/tests/fixtures/codec_file/` | any | block |

**LFS hydration check (Phase 14 §L2).** All jobs that read the
`golden_100.tqcv` fixture **must** carry `with: { lfs: true }` on
their `actions/checkout@v4` step. Phase 14 §L2 documented that
this is off by default on GitHub Actions and caused silent test
failures for an entire phase. The Phase 17 PR description includes
a checklist item to verify the YAML on the merge commit.

**CI health check (Phase 14 §L1).** The phase exit criterion is
a green `rust-ci.yml` run on `main` **after** the merge, confirmed
via `gh run list --workflow rust-ci.yml --branch main --limit 5`.
"Green locally" is not sufficient. The author must paste the URL
of the successful run into the phase-completion commit message.

**Windows-specific job rationale.** The `test-mmap-windows` job
exists because:

1. `CreateFileMappingW` sharing semantics differ from POSIX
   `mmap` and are not exercised on `ubuntu-22.04`.
2. Windows path separators (`\\` vs `/`) have bitten Phase 14
   cross-runner parity (§L4). The mmap round-trip test constructs
   a `Path` and must handle both.
3. The `NamedTempFile` on Windows holds an open handle to the
   file, and `CorpusFileReader::open` needs to tolerate that
   (share-read opening).

## Risks

### R17.1 — mmap lifetime vs iterator lifetime mis-design

**Failure mode.** A naive implementation returns `CorpusFileIter`
with a borrow to `Mmap` that outlives the reader, producing a
compile-time error or (worse) a runtime use-after-free if
`unsafe` is used to paper over the borrow checker.

**Likelihood.** High on first draft; self-referential structs are
a classic Rust foot-gun.

**Impact.** Compile-time error is acceptable and caught in CI;
runtime UAF is catastrophic.

**Mitigation.** Use the **self-borrow pattern**: the iterator's
lifetime `'a` is tied to `&'a Mmap`, not `Self`. The reader owns
the mmap and returns `CorpusFileIter<'a>` where `'a` is the
reader's own borrow lifetime. No `unsafe` is used. The
`CorpusFileIter` type is `pub struct CorpusFileIter<'a> { remaining:
&'a [u8], count: u64, errored: bool }` and nothing more.

### R17.2 — Partial write leaves file "valid enough" to iterate

**Failure mode.** A crash during `append` leaves the file with a
correct-looking prefix and garbage at the tail. A naive reader
iterates `vector_count` records and falls off the end into
undefined bytes.

**Likelihood.** Moderate — any crash, power loss, or forced
process kill between `append` calls hits this path.

**Impact.** Silent data corruption returned as `Ok(view)`.

**Mitigation.** The magic-byte trick from [[#Atomic finalize rules]]:
the file starts as `TQCX` and only flips to `TQCV` after every
record is on disk and the count is back-patched. A reader that
sees `TQCX` refuses the file entirely. The crash-injection test in
Step 6 exercises this path.

### R17.3 — Windows file-locking semantics break multi-reader open

**Failure mode.** On Windows, opening a file with the default
`CreateFileW` flags takes an exclusive lock. A second
`CorpusFileReader::open` on the same file from another process
fails with `ERROR_SHARING_VIOLATION` and hangs or errors the
caller.

**Likelihood.** Moderate — multi-process readers are an expected
corpus-serving scenario.

**Impact.** Silent hang (worst case) or opaque error (better
case).

**Mitigation.** `memmap2::MmapOptions::new().populate().map(&file)`
constructs the mmap with read-only share-read semantics. The
`File` handle is obtained via `OpenOptions::new().read(true)` (no
exclusive lock). The `test-mmap-windows` CI job exercises
multi-open and asserts both readers observe the same records.

### R17.4 — dhat feature flag becomes required for CI, blocking parallel jobs

**Failure mode.** If the main `test` job enables
`--all-features`, it pulls in `dhat-heap` which requires
single-threaded execution, serializing the entire test suite and
blowing the CI budget.

**Likelihood.** High if the feature is enabled globally.

**Impact.** CI wall time triples; PR throughput drops.

**Mitigation.** `dhat-heap` is **only** enabled by the dedicated
`test-mmap-dhat` job. The main `test` job uses `--features
"simd,mmap,rayon"` explicitly and does not pass `--all-features`.
A clippy-adjacent grep in `xtask arch-check` verifies no
workspace-wide command enables `dhat-heap`.

### R17.5 — `memmap2` minor version bump changes platform behavior

**Failure mode.** `memmap2` `0.9.x` → `0.10.x` changes the default
mmap flags or drops the `populate()` method. Silent behavioral
change.

**Likelihood.** Low but precedented — the `memmap2` crate has had
two soundness-related API renames.

**Impact.** Silent platform divergence that may only show up on
Windows or macOS runners.

**Mitigation.** Pin `memmap2 = "^0.9"` in the workspace
`[workspace.dependencies]`. Any bump requires a manual review of
the `memmap2` CHANGELOG and a green `test-mmap` + `test-mmap-windows`
run. `cargo-deny` blocks undeclared major/minor bumps.

### R17.6 — Level-2 header layout drifts from the design doc

**Failure mode.** The code in `codec_file/header.rs` drifts from
the table in [[#Level-2 file container layout]] above (and from
[[design/rust/serialization-format]] §Level 2 once the drift noted
at the top of this file is reconciled). Fixture files produced by
one version of the code are unreadable by another.

**Likelihood.** Moderate — the layout has three sources of truth
right now (this phase plan, the design doc, and the code).

**Impact.** Fixture rebuild churn; possible silent corpus
corruption if the drift lands on `main` without a version bump.

**Mitigation.**

1. A **header-size audit test** in
   `tinyquant-io/tests/header_audit.rs` asserts the fixed prefix
   is exactly 24 bytes and that a reference fixture with
   `config_hash_len=14, metadata_len=0` has its body start at
   offset 48 (which is `24 + 14 + 4 + 6` pad-to-8).
2. A **grep check** in `xtask arch-check` searches this file for
   the literal string `| 0 | 4 | `magic` |` and asserts the
   matching row in `codec_file/header.rs` has the same literal
   offset/length pair.
3. The follow-up doc PR (queued at the top of this file) brings
   [[design/rust/serialization-format]] §Level 2 into sync with
   the code.

## Out of scope

- **Level-3 columnar format** — reserved for a later release
  (post-1.0). The `format_version` byte carves out `0x02+` for
  future layouts; `0x01` is the only accepted value in Phase 17.
- **Compression or encryption of file contents.** The Level-2
  container stores Level-1 payloads verbatim. Compressed Level-2
  (e.g. zstd-wrapped records) is deferred; so is encrypted or
  signed Level-2.
- **Network-mounted mmap edge cases (NFS, SMB).** Documented as
  "use at your own risk": the mmap safety contract assumes a
  local file system. `memmap2` on an NFS mount can produce stale
  reads and `mmap`-vs-`write` coherence issues that are outside
  the scope of this phase.
- **Async readers.** `CorpusFileReader` and `CodecFileReader` are
  strictly blocking I/O. `tokio::fs` wrappers are not part of
  Phase 17 and should not be introduced before Phase 22 at the
  earliest.
- **Metadata section layout beyond "application-defined opaque
  bytes".** Phase 18 decides metadata encoding for the corpus
  aggregate (likely CBOR). Phase 17 only preserves the bytes
  round-trip through `MetadataBlob<'a>`.
- **Random-access `get(index: u64)` on the mmap reader.** The
  sketch appears in [[#Mmap safety contract]] only to justify the
  8-byte alignment; the actual implementation lands in Phase 18
  alongside the corpus aggregate.
- **`mmap-lock` feature.** Reserved in `Cargo.toml` with a
  `compile_error!` guard; implementation deferred.

## See also

- [[plans/rust/phase-16-serialization-parity|Phase 16]]
- [[plans/rust/phase-18-corpus-aggregate|Phase 18]]
- [[design/rust/serialization-format|Serialization Format]]
- [[design/rust/memory-layout|Memory Layout]]
- [[design/rust/testing-strategy|Testing Strategy]]
- [[design/rust/feature-flags|Feature Flags]]
- [[design/rust/ci-cd|CI/CD]]
- [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]]
