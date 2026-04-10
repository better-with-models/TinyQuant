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
> - [[design/rust/serialization-format|Serialization Format]] §Level 2
> - [[design/rust/memory-layout|Memory Layout]] §Zero-copy

## Prerequisites

- Phase 16 complete (`to_bytes` / `from_bytes` in place).

## Deliverables

### Files to create

| File | Purpose |
|------|---------|
| `rust/crates/tinyquant-io/src/zero_copy/mod.rs` | Module root |
| `rust/crates/tinyquant-io/src/zero_copy/view.rs` | `CompressedVectorView<'a>` |
| `rust/crates/tinyquant-io/src/zero_copy/cursor.rs` | Stream iterator |
| `rust/crates/tinyquant-io/src/mmap/mod.rs` | Module root |
| `rust/crates/tinyquant-io/src/mmap/corpus_file.rs` | `CorpusFileReader`, `CorpusFileIter` |
| `rust/crates/tinyquant-io/src/codec_file/mod.rs` | Module root |
| `rust/crates/tinyquant-io/src/codec_file/writer.rs` | Append-only writer |
| `rust/crates/tinyquant-io/src/codec_file/reader.rs` | Streaming reader (non-mmap) |
| `rust/crates/tinyquant-io/tests/zero_copy.rs` | View tests |
| `rust/crates/tinyquant-io/tests/mmap_corpus.rs` | mmap round-trip |

## Steps (TDD order)

- [ ] **Step 1: Failing view parse test**

```rust
#[test]
fn view_parses_serialized_compressed_vector_without_alloc() {
    // Serialize a CompressedVector with bit_width=4, dim=32.
    // Parse it via CompressedVectorView::parse; assert every field
    // matches without allocating.
}
```

- [ ] **Step 2: Implement `view.rs`**

Per [[design/rust/serialization-format|Serialization Format]]
§CompressedVectorView. All fields borrow from the input `&'a [u8]`.
No allocation inside `parse`; no allocation inside `unpack_into`
(caller-provided buffer).

- [ ] **Step 3: Proptest — view parse agrees with from_bytes**

```rust
proptest! {
    #[test]
    fn view_parse_agrees_with_from_bytes(/* ... */) {
        let owned = from_bytes(&bytes)?;
        let view = CompressedVectorView::parse(&bytes)?;
        prop_assert_eq!(view.dimension, owned.dimension());
        prop_assert_eq!(view.bit_width, owned.bit_width());
        let mut unpacked = vec![0u8; view.dimension as usize];
        view.unpack_into(&mut unpacked)?;
        prop_assert_eq!(unpacked.as_slice(), owned.indices());
    }
}
```

- [ ] **Step 4: Allocation audit test**

Use `dhat` to assert zero heap allocations after startup during
`view.unpack_into`. (Feature-gated; run under `--features dhat-heap`.)

- [ ] **Step 5: Failing Level-2 file write/read test**

```rust
#[test]
fn corpus_file_write_then_read_round_trip() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let cvs: Vec<CompressedVector> = build_gold_corpus(100, 64, 4);

    let mut writer = CodecFileWriter::create(tmp.path(), &config_hash, 64, 4).unwrap();
    for cv in &cvs {
        writer.append(cv).unwrap();
    }
    writer.finalize().unwrap();

    let reader = CorpusFileReader::open(tmp.path()).unwrap();
    assert_eq!(reader.header().vector_count, 100);
    let count = reader.iter().count();
    assert_eq!(count, 100);
    for (view, expected) in reader.iter().zip(cvs.iter()) {
        let view = view.unwrap();
        assert_eq!(view.dimension, expected.dimension());
        let mut unpacked = vec![0u8; view.dimension as usize];
        view.unpack_into(&mut unpacked).unwrap();
        assert_eq!(unpacked.as_slice(), expected.indices());
    }
}
```

- [ ] **Step 6: Implement `codec_file/writer.rs`**

```rust
pub struct CodecFileWriter {
    file: std::fs::File,
    count: u64,
    header_offset_count: u64,
}

impl CodecFileWriter {
    pub fn create(path: &Path, config_hash: &str, dim: u32, bit_width: u8) -> Result<Self, IoError> {
        let mut file = std::fs::File::create(path)?;
        // Write placeholder header — count will be back-patched in finalize.
        let header = build_header(config_hash, dim, bit_width, 0);
        file.write_all(&header)?;
        Ok(Self { file, count: 0, header_offset_count: 8 })
    }

    pub fn append(&mut self, cv: &CompressedVector) -> Result<(), IoError> {
        let bytes = to_bytes(cv);
        self.file.write_all(&(bytes.len() as u32).to_le_bytes())?;
        self.file.write_all(&bytes)?;
        self.count += 1;
        Ok(())
    }

    pub fn finalize(mut self) -> Result<(), IoError> {
        self.file.seek(SeekFrom::Start(self.header_offset_count))?;
        self.file.write_all(&self.count.to_le_bytes())?;
        self.file.flush()?;
        Ok(())
    }
}
```

- [ ] **Step 7: Implement `mmap/corpus_file.rs`**

Uses `memmap2::Mmap`. `iter()` returns a borrowed iterator of
`CompressedVectorView<'_>` values produced from the mmapped bytes
without allocation.

- [ ] **Step 8: Run mmap test — expect pass.**

- [ ] **Step 9: Magic + version rejection tests**

- Wrong magic (`b"XXXX"`) → `IoError::Decode(...)` or a new
  `IoError::BadMagic` variant.
- Unknown `format_version` → rejection.
- Truncated header → `IoError::Truncated`.

- [ ] **Step 10: Empty file handling**

`CorpusFileReader::open` on a truncated file returns
`IoError::Truncated` before any iteration.

- [ ] **Step 11: Level-2 bench**

Add `tinyquant-bench/benches/zero_copy_view_iteration.rs`: load a
10 000-vector file via mmap and iterate without unpacking; record
throughput. Not CI-gated yet.

- [ ] **Step 12: Run workspace tests, clippy, fmt.**

- [ ] **Step 13: Commit**

```bash
git add rust/crates/tinyquant-io rust/crates/tinyquant-bench
git commit -m "feat(tinyquant-io): add zero-copy view, corpus file writer, and mmap reader"
```

## Acceptance criteria

- Zero-copy view parses CompressedVector bytes without heap
  allocation (dhat verifies).
- Level-2 corpus files round-trip a 100-vector sample; header
  `vector_count` field is back-patched correctly on `finalize`.
- mmap iterator yields views without copying.
- All rejection tests produce the expected `IoError` variants.
- Clippy + fmt clean.

## Out of scope

- Metadata section layout beyond "application-defined opaque bytes"
  (phase 18 decides metadata encoding for the corpus aggregate).
- Columnar Level-3 format (reserved for a later release).

## See also

- [[plans/rust/phase-16-serialization-parity|Phase 16]]
- [[plans/rust/phase-18-corpus-aggregate|Phase 18]]
- [[design/rust/serialization-format|Serialization Format]]
- [[design/rust/memory-layout|Memory Layout]]
