---
title: Rust Port — Serialization Format
tags:
  - design
  - rust
  - serialization
  - binary
  - parity
date-created: 2026-04-10
status: draft
category: design
---

# Rust Port — Serialization Format

> [!info] Purpose
> Lock down the wire format so that `Rust::to_bytes(cv) ==
> Python::to_bytes(cv)` byte for byte, and define the additional
> stream-level container the Rust port introduces for mmap-friendly
> corpus files.

## Level 1: single-vector format (Python parity)

This is the format Python writes and reads today, reproduced here so
that the Rust code and the Python source are easy to cross-check.

```
offset  size  field                       notes
------  ----  --------------------------  -------------------------------
  0     1     version                     0x01
  1     64    config_hash                 UTF-8, null-padded if shorter
 65     4     dimension                   u32 little-endian
 69     1     bit_width                   one of {2, 4, 8}
 70     P     packed_indices              P = ceil(dim * bit_width / 8)
70+P    1     residual_flag               0x00 (absent) or 0x01 (present)
70+P+1  4*    residual_length             u32 little-endian, if flag=0x01
70+P+5* R     residual_payload            R bytes, if flag=0x01
```

The header is **70 bytes** as emitted by `struct.pack("<B64sIB", ...)`.
See the audit note in
[[design/rust/numerical-semantics|Numerical Semantics]] for why this
contradicts an off-by-one comment in the Python source.

### Packing indices (LSB-first)

For `bit_width` in `{2, 4}`, multiple indices fit in a byte. Packing
is **LSB-first**: the first index occupies the low bits of the first
byte, the next index continues at the next bit position, crossing
byte boundaries as needed.

Reference scalar implementation (matches Python `_pack_indices`):

```rust
// tinyquant-io/src/compressed_vector/pack.rs
pub fn pack_indices(indices: &[u8], bit_width: u8, out: &mut [u8]) {
    if bit_width == 8 {
        out.copy_from_slice(indices);
        return;
    }
    let total_bits = indices.len() * usize::from(bit_width);
    let n_bytes = (total_bits + 7) / 8;
    debug_assert_eq!(out.len(), n_bytes);
    out.fill(0);
    let mut bit_pos = 0usize;
    for &idx in indices {
        let byte_idx = bit_pos / 8;
        let bit_off = bit_pos % 8;
        out[byte_idx] |= (idx as u16).wrapping_shl(bit_off as u32) as u8;
        if bit_off + usize::from(bit_width) > 8 {
            out[byte_idx + 1] |= idx >> (8 - bit_off);
        }
        bit_pos += usize::from(bit_width);
    }
}
```

Unpacking is the inverse. A parity test compares output against
Python for all 4096 possible inputs of length 16 at 4-bit, plus
randomized tests at 2-bit and 8-bit.

### Handling short config hashes

Python pads with `b"\x00"`:

```python
hash_bytes = self.config_hash.encode("utf-8")[:_HASH_BYTES].ljust(_HASH_BYTES, b"\x00")
```

Rust:

```rust
let mut hash_buf = [0u8; 64];
let encoded = config_hash.as_bytes();
let len = encoded.len().min(64);
hash_buf[..len].copy_from_slice(&encoded[..len]);
```

And on read-back, Python strips trailing NULs before decoding:

```python
config_hash = hash_raw.rstrip(b"\x00").decode("utf-8", errors="replace")
```

Rust mirrors this, returning `Arc<str>` via
`core::str::from_utf8(&buf[..trim_end])`. Invalid UTF-8 in the config
hash returns `IoError::InvalidUtf8`, not a lossy fallback. The
Python `errors="replace"` path is documented as Python-only
tolerance and is not matched, because any invalid UTF-8 in a hash
field indicates file corruption that Rust must surface.

### `to_bytes` implementation

```rust
pub fn to_bytes(cv: &CompressedVector) -> Vec<u8> {
    let packed_len = ((cv.dimension() as usize) * (cv.bit_width() as usize) + 7) / 8;
    let residual = cv.residual();
    let residual_len = residual.map_or(0, |r| r.len());
    let total = 70 + packed_len + 1 + if residual.is_some() { 4 + residual_len } else { 0 };
    let mut out = Vec::with_capacity(total);

    // Header
    out.push(0x01);
    let mut hash_buf = [0u8; 64];
    let hash = cv.config_hash().as_bytes();
    let hlen = hash.len().min(64);
    hash_buf[..hlen].copy_from_slice(&hash[..hlen]);
    out.extend_from_slice(&hash_buf);
    out.extend_from_slice(&cv.dimension().to_le_bytes());
    out.push(cv.bit_width());

    // Packed indices
    let mut packed = vec![0u8; packed_len];
    pack_indices(cv.indices(), cv.bit_width(), &mut packed);
    out.extend_from_slice(&packed);

    // Residual
    if let Some(r) = residual {
        out.push(0x01);
        out.extend_from_slice(&(r.len() as u32).to_le_bytes());
        out.extend_from_slice(r);
    } else {
        out.push(0x00);
    }
    debug_assert_eq!(out.len(), total);
    out
}
```

Zero temporaries other than the `hash_buf` stack array, the `packed`
scratch vector, and the output. A micro-optimized version writes
directly into `out` without the `packed` detour and is what ships;
the above is the readable reference.

### `from_bytes` implementation (owned)

```rust
pub fn from_bytes(data: &[u8]) -> Result<CompressedVector, IoError> {
    if data.len() < 70 {
        return Err(IoError::Truncated { needed: 70, got: data.len() });
    }
    let version = data[0];
    if version != 0x01 {
        return Err(IoError::UnknownVersion { got: version });
    }
    let hash_raw = &data[1..65];
    let hash_end = hash_raw.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    let config_hash = core::str::from_utf8(&hash_raw[..hash_end])
        .map_err(|e| IoError::InvalidUtf8(e))?;
    let dimension = u32::from_le_bytes(data[65..69].try_into().unwrap());
    let bit_width = data[69];
    if !matches!(bit_width, 2 | 4 | 8) {
        return Err(IoError::InvalidBitWidth { got: bit_width });
    }

    let packed_len = ((dimension as usize) * (bit_width as usize) + 7) / 8;
    let body_needed = 70 + packed_len + 1;
    if data.len() < body_needed {
        return Err(IoError::Truncated { needed: body_needed, got: data.len() });
    }

    let mut indices = vec![0u8; dimension as usize].into_boxed_slice();
    unpack_indices(&data[70..70 + packed_len], dimension as usize, bit_width, &mut indices);

    let flag = data[70 + packed_len];
    let mut offset = 70 + packed_len + 1;
    let residual = if flag == 0x01 {
        if data.len() < offset + 4 {
            return Err(IoError::Truncated { needed: offset + 4, got: data.len() });
        }
        let rlen = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if data.len() < offset + rlen {
            return Err(IoError::Truncated { needed: offset + rlen, got: data.len() });
        }
        let mut r = vec![0u8; rlen].into_boxed_slice();
        r.copy_from_slice(&data[offset..offset + rlen]);
        Some(r)
    } else {
        None
    };

    CompressedVector::new(
        indices,
        residual,
        Arc::from(config_hash),
        dimension,
        bit_width,
    ).map_err(IoError::Decode)
}
```

Any change to this function must be paired with a parity-test update.

### `CompressedVectorView<'a>` (zero-copy)

The zero-copy variant borrows from the input slice and never
allocates. Unpacking happens lazily per index:

```rust
pub struct CompressedVectorView<'a> {
    pub config_hash: &'a str,
    pub dimension: u32,
    pub bit_width: u8,
    packed_indices: &'a [u8],
    residual: Option<&'a [u8]>,
}

impl<'a> CompressedVectorView<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, IoError> { /* … */ }

    pub fn packed_indices(&self) -> &'a [u8] { self.packed_indices }

    pub fn residual(&self) -> Option<&'a [u8]> { self.residual }

    /// Unpack into a caller-provided buffer. Zero allocations.
    pub fn unpack_into(&self, out: &mut [u8]) -> Result<(), IoError> {
        if out.len() != self.dimension as usize {
            return Err(IoError::LengthMismatch);
        }
        unpack_indices(self.packed_indices, self.dimension as usize, self.bit_width, out);
        Ok(())
    }
}
```

The benchmark `bench_decompress_batch_zero_copy` reads an mmapped
corpus file, unpacks into a pre-allocated scratch buffer, and runs
through the full decode path without any heap allocation after
startup.

## Level 2: corpus file container (new — Rust side only)

The Python library stores corpora as in-memory Python objects and
writes them to disk only via application-level glue (JSON, ad-hoc
binary, or domain-specific encoders). The Rust port introduces a
disciplined streaming container for byte-for-byte reproducible corpus
files.

```
offset  size   field                 notes
------  ----   -------------------   -------------------------------
  0     4      magic                 b"TQCV"
  4     2      format_version        u16 LE, starts at 0x0002
  6     2      flags                 u16 LE
                                     bit 0: residual_present_throughout
                                     bit 1: sorted_by_id
 8      8      vector_count          u64 LE
16     64      config_hash           UTF-8, null-padded
80      4      dimension             u32 LE
84      1      bit_width             u8
85      3      reserved              0x00 0x00 0x00
88     32      meta_digest           SHA-256 of section below
120    N       metadata_section      application-defined CBOR
120+N          per-vector entries    each: <u32 LE length><payload>
                                     where payload is the Level 1 format
```

Consumers read the file via an `CorpusFileReader`:

```rust
// tinyquant-io/src/mmap/corpus_file.rs
use memmap2::Mmap;

pub struct CorpusFileReader {
    mmap: Mmap,
    header: CorpusFileHeader,
    body_offset: usize,
}

impl CorpusFileReader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, IoError> { /* … */ }

    pub fn header(&self) -> &CorpusFileHeader { &self.header }

    pub fn iter(&self) -> CorpusFileIter<'_> { /* … */ }
}

pub struct CorpusFileIter<'a> {
    remaining: &'a [u8],
    count: u64,
}

impl<'a> Iterator for CorpusFileIter<'a> {
    type Item = Result<CompressedVectorView<'a>, IoError>;
}
```

A parity gate asserts that for a synthetic corpus of 10 000 vectors
(same gold fixture Python uses), the Level-1 payloads inside the
file are byte-identical to what Python produces via
`[cv.to_bytes() for cv in corpus.decompress_all()]`. The level-2
header is Rust-only; there is no Python counterpart yet.

## Level 3: future formats

Not part of this port. Reserved format versions:

- `0x0003`: columnar codec file (indices SoA, residuals SoA)
- `0x0004`: sharded codec files with manifest

Each future version adds a new magic tail (`TQCV2`, `TQCV3`, etc.)
and a new reader crate. Adding a new format does **not** break
`from_bytes` on the Level-1 payload.

## Versioning discipline

1. The Level-1 version (`0x01`) can never change without updating
   `tinyquant_cpu` and a major-version bump.
2. The Level-2 version is controlled solely by `tinyquant-io`.
   Backward-compatible additions (new flag bits, new reserved-zero
   fields consumed as optional) do not change the version.
3. Incompatible Level-2 changes bump the `u16` version; readers must
   refuse unknown versions.
4. Any CI run that produces a corpus file of a new version must also
   ship a reader fixture capturing a small sample for regression
   testing.

## Parity tests (exact names)

| Test | Crate | Expected |
|---|---|---|
| `compressed_vector_parity_4bit_residual_on` | `tinyquant-io` | bytes equal to Python fixture |
| `compressed_vector_parity_2bit_residual_off` | `tinyquant-io` | bytes equal to Python fixture |
| `compressed_vector_parity_8bit_residual_off` | `tinyquant-io` | bytes equal to Python fixture |
| `compressed_vector_roundtrip_random_4bit` | `tinyquant-io` | `from_bytes(to_bytes(x)) == x` for 1000 random |
| `compressed_vector_roundtrip_empty_residual` | `tinyquant-io` | residual=None preserved |
| `header_size_is_70` | `tinyquant-io` | emitted header is exactly 70 bytes |
| `bit_pack_parity_4bit_len_16_exhaustive` | `tinyquant-io` | all 2^16 states exhaustively enumerated |
| `unpack_rejects_truncated_data` | `tinyquant-io` | `IoError::Truncated` |
| `unpack_rejects_unknown_version` | `tinyquant-io` | `IoError::UnknownVersion` |
| `unpack_rejects_invalid_bit_width` | `tinyquant-io` | `IoError::InvalidBitWidth` |

Python fixtures live at
`rust/crates/tinyquant-io/tests/fixtures/compressed_vector/`
and are regenerated by `xtask fixtures refresh`.

## See also

- [[design/rust/numerical-semantics|Numerical Semantics]]
- [[design/rust/memory-layout|Memory Layout]]
- [[design/rust/testing-strategy|Testing Strategy]]
