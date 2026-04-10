---
title: "Phase 16: Serialization and Python Byte Parity"
tags:
  - plans
  - rust
  - phase-16
  - serialization
  - parity
date-created: 2026-04-10
status: draft
category: planning
---

# Phase 16: Serialization and Python Byte Parity

> [!info] Goal
> Move `CompressedVector` serialization into `tinyquant-io` with
> byte-identical Python parity for `to_bytes` / `from_bytes`,
> including exhaustive bit-pack tests.

> [!note] Reference docs
> - [[design/rust/serialization-format|Serialization Format]]
> - [[design/rust/numerical-semantics|Numerical Semantics]] §Serialization, §Header-size audit

## Prerequisites

- Phase 15 complete (in-memory `CompressedVector` available).

## Deliverables

### Files to create

| File | Purpose |
|------|---------|
| `rust/crates/tinyquant-io/src/lib.rs` | Module exports |
| `rust/crates/tinyquant-io/src/errors.rs` | `IoError` |
| `rust/crates/tinyquant-io/src/compressed_vector/mod.rs` | submodule root |
| `rust/crates/tinyquant-io/src/compressed_vector/header.rs` | Header encode/decode |
| `rust/crates/tinyquant-io/src/compressed_vector/pack.rs` | Bit pack |
| `rust/crates/tinyquant-io/src/compressed_vector/unpack.rs` | Bit unpack |
| `rust/crates/tinyquant-io/src/compressed_vector/to_bytes.rs` | Encoder |
| `rust/crates/tinyquant-io/src/compressed_vector/from_bytes.rs` | Decoder |
| `rust/crates/tinyquant-io/tests/header_size_audit.rs` | Asserts header is 70 bytes |
| `rust/crates/tinyquant-io/tests/bit_pack_exhaustive.rs` | All 2^16 states for dim=16 bw=4 |
| `rust/crates/tinyquant-io/tests/roundtrip.rs` | Random round-trip |
| `rust/crates/tinyquant-io/tests/python_parity.rs` | Byte parity against fixture |
| `rust/crates/tinyquant-io/tests/fixtures/compressed_vector/*.bin` | Python-generated fixtures |

## Steps (TDD order)

- [ ] **Step 1: Write failing header-size audit test**

```rust
#[test]
fn header_size_is_exactly_70_bytes() {
    // Build a CompressedVector with minimum-size indices and no residual.
    // Confirm serialization header is 70 bytes.
    let cv = CompressedVector::new(
        vec![0u8; 8].into_boxed_slice(),
        None,
        Arc::from("abc"),
        8,
        8,
    ).unwrap();
    let bytes = to_bytes(&cv);
    // 70 header + 8 indices + 1 residual flag = 79
    assert_eq!(bytes.len(), 79);
}
```

- [ ] **Step 2: Run — expect failure** (`to_bytes` doesn't exist).

- [ ] **Step 3: Implement `errors.rs`**

```rust
use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum IoError {
    #[error("data too short: needed {needed} bytes, got {got}")]
    Truncated { needed: usize, got: usize },
    #[error("unknown format version {got:#04x}")]
    UnknownVersion { got: u8 },
    #[error("invalid bit_width {got} in serialized data")]
    InvalidBitWidth { got: u8 },
    #[error("invalid UTF-8 in config hash")]
    InvalidUtf8,
    #[error("input/output length mismatch")]
    LengthMismatch,
    #[error("decode error: {0}")]
    Decode(#[from] tinyquant_core::errors::CodecError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

- [ ] **Step 4: Implement `pack.rs` and `unpack.rs`**

Exact code from [[design/rust/serialization-format|Serialization
Format]] §Packing indices.

- [ ] **Step 5: Implement `header.rs`**

```rust
pub(crate) const FORMAT_VERSION: u8 = 0x01;
pub(crate) const HASH_BYTES: usize = 64;
pub(crate) const HEADER_SIZE: usize = 70;

pub(crate) fn encode_header(
    out: &mut Vec<u8>,
    config_hash: &str,
    dimension: u32,
    bit_width: u8,
) {
    out.push(FORMAT_VERSION);
    let mut buf = [0u8; HASH_BYTES];
    let src = config_hash.as_bytes();
    let n = src.len().min(HASH_BYTES);
    buf[..n].copy_from_slice(&src[..n]);
    out.extend_from_slice(&buf);
    out.extend_from_slice(&dimension.to_le_bytes());
    out.push(bit_width);
}

pub(crate) fn decode_header(data: &[u8]) -> Result<(u8, &str, u32, u8), IoError> {
    if data.len() < HEADER_SIZE {
        return Err(IoError::Truncated { needed: HEADER_SIZE, got: data.len() });
    }
    let version = data[0];
    let hash_raw = &data[1..65];
    let trim = hash_raw.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    let config_hash = core::str::from_utf8(&hash_raw[..trim]).map_err(|_| IoError::InvalidUtf8)?;
    let dimension = u32::from_le_bytes(data[65..69].try_into().unwrap());
    let bit_width = data[69];
    Ok((version, config_hash, dimension, bit_width))
}
```

- [ ] **Step 6: Implement `to_bytes.rs` and `from_bytes.rs`**

Per [[design/rust/serialization-format|Serialization Format]] §to_bytes and §from_bytes.

- [ ] **Step 7: Run header-size audit — expect pass.**

- [ ] **Step 8: Write exhaustive bit-pack test**

```rust
#[test]
fn bit_pack_4bit_dim16_exhaustive() {
    // Each of 16 indices is 4 bits → 64 bits = u64.
    // Enumerate all 2^16 four-bit-limited sequences of length 16.
    // Actually, with 16 indices x 4 bits each, the state is 2^64 —
    // far too many. Instead enumerate all 16! (too many) OR fix 14
    // slots and vary 2. The pragmatic coverage target: every pair of
    // adjacent slots tested across all bit combinations.
    //
    // Concretely: for each i in 0..15, for every (a, b) in 0..16 x 0..16,
    // set indices[i]=a, indices[i+1]=b, others=0; pack; unpack; assert equality.
    let cb_bw = 4u8;
    for i in 0..15 {
        for a in 0u8..16 {
            for b in 0u8..16 {
                let mut indices = vec![0u8; 16];
                indices[i] = a;
                indices[i+1] = b;
                let mut packed = vec![0u8; 8];
                pack_indices(&indices, cb_bw, &mut packed);
                let mut back = vec![0u8; 16];
                unpack_indices(&packed, 16, cb_bw, &mut back);
                assert_eq!(indices, back, "i={i} a={a} b={b}");
            }
        }
    }
}
```

- [ ] **Step 9: Run — expect pass.**

- [ ] **Step 10: Random round-trip proptest**

```rust
proptest! {
    #[test]
    fn roundtrip_identity(
        dim in 1u32..=2048,
        bit_width in prop::sample::select(vec![2u8, 4, 8]),
        residual_on in any::<bool>(),
    ) {
        let n = dim as usize;
        let indices = vec![0u8; n].into_boxed_slice();
        let residual = if residual_on { Some(vec![0u8; n * 2].into_boxed_slice()) } else { None };
        let cv = CompressedVector::new(indices, residual, Arc::from("deadbeef"), dim, bit_width).unwrap();
        let bytes = to_bytes(&cv);
        let decoded = from_bytes(&bytes).unwrap();
        prop_assert_eq!(decoded.indices().len(), cv.indices().len());
        prop_assert_eq!(decoded.bit_width(), cv.bit_width());
        prop_assert_eq!(decoded.dimension(), cv.dimension());
        prop_assert_eq!(decoded.has_residual(), cv.has_residual());
    }
}
```

- [ ] **Step 11: Python parity fixture test**

Capture 10 `(cv, bytes)` pairs from Python via
`xtask fixtures refresh-serialization`, committing `.bin` files.

```rust
#[test]
fn to_bytes_matches_python_fixture_bw4_d768_res_on() {
    let indices_bytes = include_bytes!("fixtures/compressed_vector/case_01/indices.u8.bin");
    let residual_bytes = include_bytes!("fixtures/compressed_vector/case_01/residual.u8.bin");
    let expected = include_bytes!("fixtures/compressed_vector/case_01/expected.bin");
    let config_hash = include_str!("fixtures/compressed_vector/case_01/config_hash.txt").trim();

    let cv = CompressedVector::new(
        indices_bytes.to_vec().into_boxed_slice(),
        Some(residual_bytes.to_vec().into_boxed_slice()),
        Arc::from(config_hash),
        768,
        4,
    ).unwrap();

    let bytes = to_bytes(&cv);
    assert_eq!(bytes, expected.as_slice());
}
```

Repeat for at least three cases covering bit widths 2, 4, 8 and
both residual states.

- [ ] **Step 12: Rejection tests**

- Truncated header → `IoError::Truncated`.
- Unknown version (byte 0 = 0x02) → `IoError::UnknownVersion`.
- Invalid bit_width (byte 69 = 3) → `IoError::InvalidBitWidth`.
- Missing residual length → `IoError::Truncated`.

- [ ] **Step 13: Run workspace test/clippy/fmt.**

- [ ] **Step 14: Fuzz target**

Add `fuzz_targets/compressed_vector_from_bytes.rs`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tinyquant_io::compressed_vector::from_bytes(data);
});
```

Run `cargo fuzz run compressed_vector_from_bytes -- -max_total_time=60`
locally as a smoke check.

- [ ] **Step 15: Commit**

```bash
git add rust/crates/tinyquant-io rust/crates/tinyquant-fuzz
git commit -m "feat(tinyquant-io): add CompressedVector serialization with Python byte parity"
```

## Acceptance criteria

- Header is exactly 70 bytes.
- Bit pack/unpack passes exhaustive pair-wise coverage.
- Random round-trip proptest passes 256+ cases.
- All Python fixture cases produce byte-identical `to_bytes` output.
- `from_bytes` rejects truncated, unknown-version, and
  invalid-bit-width inputs with the expected `IoError` variant.
- Fuzz target runs 60 seconds without panicking.
- Clippy + fmt clean.

## Out of scope

- Level-2 corpus file container — phase 17.
- Zero-copy view iterator — phase 17.

## See also

- [[plans/rust/phase-15-codec-service-residual|Phase 15]]
- [[plans/rust/phase-17-zero-copy-mmap|Phase 17]]
- [[design/rust/serialization-format|Serialization Format]]
