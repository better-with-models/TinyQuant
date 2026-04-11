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
> exhaustive bit-pack coverage, a fuzz harness, and a CI wiring that
> inherits every lesson learned during Phase 14.

> [!note] Reference docs
> - [[design/rust/serialization-format|Serialization Format]]
> - [[design/rust/numerical-semantics|Numerical Semantics]] §Serialization, §Header-size audit
> - [[design/rust/error-model|Error Model]] §IoError
> - [[design/rust/crate-topology|Crate Topology]] §tinyquant-io
> - [[design/rust/testing-strategy|Testing Strategy]] §Fixtures, §Fuzz
> - [[design/rust/ci-cd|CI/CD]] §rust-ci.yml
> - [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]] §Lessons L1–L7

## Prerequisites

- Phase 15 complete (in-memory `CompressedVector` available in
  `tinyquant-core::codec::compressed_vector`).
- `tinyquant-core::codec::Codebook` byte-parity fixtures landed in
  Phase 14 (verifies the fixture pipeline shape we're about to reuse).
- Workspace MSRV confirmed at 1.81 in both `rust/rust-toolchain.toml`
  and `rust/Cargo.toml` (`rust-version = "1.81"`). Re-verify before
  step 1; see Lesson L6.
- `rust-ci.yml` has a clean history on `main` touching
  `rust/crates/tinyquant-core/**` — see Lesson L1 gate.

## Deliverables

### Byte layout (Level-1: single `CompressedVector`)

All multi-byte fields are **little-endian**. The header has no padding
(Python uses `struct.pack("<B64sIB", ...)` — `<` disables struct
alignment). There is no alignment requirement on the payload sections
either; the parser reads byte-by-byte and does not assume the slice
start is aligned.

| offset | length        | field              | encoding               | notes |
|-------:|--------------:|--------------------|------------------------|-------|
| 0      | 1             | `version`          | `u8`                   | Always `0x01` for Level-1. `0x00` and `0x02..` are rejected by `from_bytes`. |
| 1      | 64            | `config_hash`      | UTF-8, NUL-padded      | Short hashes pad with `b"\x00"` up to 64 bytes; over-long inputs are truncated at 64 bytes on write. On read, trailing NUL bytes are stripped before UTF-8 decoding. |
| 65     | 4             | `dimension`        | `u32` LE               | Range `1..=u32::MAX`; `0` is rejected by the constructor, not by `from_bytes`. |
| 69     | 1             | `bit_width`        | `u8`                   | One of `{2, 4, 8}`. Any other value → `IoError::InvalidBitWidth`. |
| **70** | `P`           | `packed_indices`   | LSB-first bitstream    | `P = (dim * bit_width + 7) / 8`. For `bit_width == 8`, this is a plain byte copy. |
| 70+P   | 1             | `residual_flag`    | `u8`                   | `0x00` (absent) or `0x01` (present); any other value → `IoError::InvalidResidualFlag` (treated as `IoError::Decode` via `CodecError::InvalidResidualFlag`). |
| 70+P+1 | 4 (optional)  | `residual_length`  | `u32` LE               | Only present when `residual_flag == 0x01`. Equals `R` below. |
| 70+P+5 | `R` (optional)| `residual_payload` | raw bytes              | `R = 2 * dim` when residual is `fp16` per [[design/rust/numerical-semantics|Numerical Semantics]] §Residual. The parser does not assume `R == 2*dim`; it trusts the length prefix, then verifies the result against the codec contract. |

The header is **exactly 70 bytes**. The `_HEADER_SIZE = 71` comment in
`src/tinyquant_cpu/compressed_vector.py` is off by one; Rust trusts
`struct.calcsize("<B64sIB") == 70`. Step 1 freezes this in a test and
Step 5 freezes it in a `const HEADER_SIZE: usize = 70`.

### Fixture contract

Python-generated fixtures live at:

```text
rust/crates/tinyquant-io/tests/fixtures/compressed_vector/
├── manifest.json                 # case metadata (see schema below)
├── case_01/
│   ├── config.json               # bit_width, dim, residual, config_hash
│   ├── indices.u8.bin            # unpacked u8 indices (raw)
│   ├── residual.u8.bin           # raw residual bytes (present only if residual=on)
│   ├── config_hash.txt           # canonical config hash string (no trailing newline)
│   └── expected.bin              # canonical Python `to_bytes` output
├── case_02/...
└── case_10/...
```

At least **ten** cases ship with Phase 16, covering the minimum
parity sweep:

| id | `bit_width` | `dim` | residual | purpose |
|----|-------------|-------|----------|---------|
| 01 | 4           | 768   | on       | gold happy path |
| 02 | 2           | 768   | off      | tightest bit pack, no residual |
| 03 | 8           | 768   | off      | fast path (`bit_width == 8`, plain copy) |
| 04 | 4           | 1     | off      | minimum-dim edge case |
| 05 | 2           | 17    | on       | sub-byte boundary (`17 * 2 = 34` bits → 5-byte payload with tail bits) |
| 06 | 4           | 15    | off      | odd `dim_mod_2` with 4-bit pack |
| 07 | 8           | 1536  | on       | large dim, `bit_width == 8` |
| 08 | 4           | 768   | off      | same dim as case 01 but no residual |
| 09 | 2           | 16    | off      | baseline for the dim=16 exhaustive test |
| 10 | 4           | 16    | on       | baseline for the dim=16 exhaustive test with residual |

All fixtures are tracked by Git LFS per `.gitattributes` glob
`rust/crates/tinyquant-io/tests/fixtures/**/*.bin`. The
`config_hash.txt` files are plain text and stay out of LFS.

**Python generator invocation** (per case):

```bash
python -m tinyquant_cpu.tools.dump_serialization \
  --bit-width 4 --dim 768 --residual on \
  --seed 42 \
  --out rust/crates/tinyquant-io/tests/fixtures/compressed_vector/case_01/
```

The generator is a small tool we ship under
`src/tinyquant_cpu/tools/dump_serialization.py`; it is a thin wrapper
over the existing `CompressedVector.to_bytes()` API that also emits
`indices.u8.bin`, `residual.u8.bin`, `config_hash.txt`, and the merged
`expected.bin`. If the tool does not yet exist, creating it is part of
Step 11.

**`manifest.json` schema** (one entry per case, committed alongside
the `.bin` files):

```json
{
  "version": 1,
  "cases": [
    {
      "id": "case_01",
      "bit_width": 4,
      "dim": 768,
      "residual": true,
      "seed": 42,
      "config_hash": "…",
      "python_sha": "<hash of the python generator source file>",
      "config_hash_input": "CodecConfig(bit_width=4,seed=42,dimension=768,residual_enabled=True)",
      "expected_sha256": "…",
      "expected_len": 70
    }
  ]
}
```

`python_sha` pins the generator source file so a future change is
caught in the diff; `config_hash_input` captures the canonical
pre-hash string so a human can verify the hash by hand.

**Refresh workflow** — the xtask command that regenerates fixtures:

```bash
cargo xtask fixtures refresh-serialization
```

Under the hood it invokes the Python generator once per case, then
updates `manifest.json` (recomputing `python_sha` and
`expected_sha256`). The command is idempotent: two consecutive runs
leave `git diff rust/crates/tinyquant-io/tests/fixtures/` clean.

**CI drift gate** — after the xtask runs in CI, a post-check asserts:

```bash
git diff --exit-code rust/crates/tinyquant-io/tests/fixtures/
```

A non-empty diff fails the job with a message instructing the author
to regenerate fixtures locally and commit. This matches the Phase 14
codebook fixture pattern (`refresh-codebook` +
`refresh-quantize`).

### Files to create

| File | Purpose | Visibility | Feature-gated |
|------|---------|------------|---------------|
| `rust/crates/tinyquant-io/Cargo.toml` | Crate manifest. Declares `[features]` block: `default = ["std"]`, plus `std`, `simd`, `mmap`, `rayon`, `serde` (see [[design/rust/feature-flags\|Feature Flags]]). | n/a | n/a |
| `rust/crates/tinyquant-io/src/lib.rs` | Re-export surface; `#![deny(...)]` wall matching `tinyquant-core`. | n/a | n/a |
| `rust/crates/tinyquant-io/src/errors.rs` | `IoError` enum (single-file layout; do **not** create `errors/mod.rs`). | `pub` | none |
| `rust/crates/tinyquant-io/src/compressed_vector/mod.rs` | Submodule root. Re-exports `to_bytes`, `from_bytes`, and the header-size constant. Does not re-export `pack`/`unpack` internals. | `pub` | none |
| `rust/crates/tinyquant-io/src/compressed_vector/header.rs` | `encode_header` / `decode_header` + `pub(crate) const HEADER_SIZE: usize = 70` + `pub(crate) const FORMAT_VERSION: u8 = 0x01` + `pub(crate) const HASH_BYTES: usize = 64`. | `pub(crate)` | none |
| `rust/crates/tinyquant-io/src/compressed_vector/pack.rs` | `pack_indices(&[u8], bit_width, &mut [u8])`. | `pub(crate)` | none |
| `rust/crates/tinyquant-io/src/compressed_vector/unpack.rs` | `unpack_indices(&[u8], dim, bit_width, &mut [u8])`. | `pub(crate)` | none |
| `rust/crates/tinyquant-io/src/compressed_vector/to_bytes.rs` | Encoder entry point. | `pub` (re-exported via `mod.rs`) | none |
| `rust/crates/tinyquant-io/src/compressed_vector/from_bytes.rs` | Decoder entry point (owned). | `pub` (re-exported via `mod.rs`) | none |
| `rust/crates/tinyquant-io/tests/header_size_audit.rs` | Asserts the emitted header is 70 bytes. | test | default |
| `rust/crates/tinyquant-io/tests/bit_pack_exhaustive.rs` | Bit-pack coverage matrix (see Step 8). | test | default |
| `rust/crates/tinyquant-io/tests/roundtrip.rs` | Seeded random round-trip loop. | test | default |
| `rust/crates/tinyquant-io/tests/python_parity.rs` | Byte parity against fixtures. | test | default |
| `rust/crates/tinyquant-io/tests/rejection.rs` | `from_bytes` rejection cases. | test | default |
| `rust/crates/tinyquant-io/tests/fixtures/compressed_vector/manifest.json` | Fixture manifest (plain text, not LFS). | fixture | n/a |
| `rust/crates/tinyquant-io/tests/fixtures/compressed_vector/case_*/*.bin` | Python-generated fixture bytes (LFS). | fixture | n/a |
| `rust/crates/tinyquant-fuzz/fuzz_targets/compressed_vector_from_bytes.rs` | libfuzzer target for `from_bytes`. | fuzz target | `fuzzing` |
| `rust/crates/tinyquant-fuzz/corpus/compressed_vector_from_bytes/` | Seed corpus (one Python fixture per case). | fuzz corpus | n/a |
| `rust/xtask/src/cmd/fixtures.rs` | New subcommand `refresh-serialization`. If `cmd/` does not yet exist, create it as a sibling of `main.rs` and wire it from the single-file dispatcher; otherwise place it alongside the existing `fixtures` command. | n/a | n/a |
| `src/tinyquant_cpu/tools/dump_serialization.py` | Python-side fixture generator (only if missing — see Step 11). | n/a | n/a |

Explicitly **not** created:

- `rust/crates/tinyquant-io/src/errors/mod.rs` — we use a single-file
  `errors.rs` layout because the enum fits comfortably in one file
  and the crate is already split along feature lines. Avoid the
  `errors/` directory indirection.
- `rust/crates/tinyquant-io/build.rs` — no build-script code
  generation is needed. `HEADER_SIZE` is a hand-written `const` with
  a matching test, which is clearer and panic-free.

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

A second assertion in the same test freezes the constant:
`assert_eq!(HEADER_SIZE, 70);`. This is the one place the constant
is referenced from a test; any accidental bump to 71 (the Python
comment value) fails both assertions.

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

The `InvalidUtf8` variant intentionally does **not** carry the
`core::str::Utf8Error` source (unlike the design-doc sketch that
had `InvalidUtf8(Utf8Error)`). We drop the source because every
UTF-8 failure on a 64-byte NUL-padded buffer has one meaningful
recovery path (the file is corrupt) and the `Utf8Error` fields
would be the first thing shown in a fuzz crash reduction — keeping
it unit-valued makes `IoError: Eq` possible for the rejection
tests.

### IoError taxonomy

Table row per `IoError` variant. The **wire trigger** column names
the exact byte mutation that each rejection test uses; the **test
file** column points at the function that exercises the variant.

| variant | wire trigger | recovery | test file |
|---------|--------------|----------|-----------|
| `Truncated { needed, got }` | Take any valid `expected.bin` and drop the last byte (`&bytes[..bytes.len()-1]`). Also tested with an `all-zero` 69-byte buffer (header short by one), and with a valid header but missing residual length (drop bytes `70+P+1..`). | Reject the file; the caller must supply the full payload. | `tests/rejection.rs::truncated_*` |
| `UnknownVersion { got }` | Start from case_01, overwrite `bytes[0]` with `0x02`. | Reject; caller should upgrade the reader. | `tests/rejection.rs::unknown_version_0x02` |
| `InvalidBitWidth { got }` | Overwrite `bytes[69]` with `3` (between the valid values `2` and `4`). Also test `0`, `5`, `9`, and `255`. | Reject; file is corrupt or from a future format. | `tests/rejection.rs::invalid_bit_width_*` |
| `InvalidUtf8` | Overwrite `bytes[1]` with `0xFF` (not a valid UTF-8 leading byte), keep the rest of `bytes[1..65]` NUL-padded so the trim logic still runs. | Reject; config hash is corrupt. | `tests/rejection.rs::invalid_utf8_in_hash` |
| `LengthMismatch` | Only surfaces on `CompressedVectorView::unpack_into` when the caller-provided buffer length `!= dim` (deferred to Phase 17). In Phase 16 we still cover the variant with a direct unit test in `unpack.rs`. | Caller passes a correctly sized buffer. | `tests/rejection.rs::view_length_mismatch` (Phase 17 hook; stub test asserts the variant exists) |
| `Decode(CodecError)` | Overwrite `bytes[70+P]` (the residual flag) with `0x02`, which falls through to `CodecError::InvalidResidualFlag`. Also tested by leaving `bit_width = 4` but corrupting an index byte such that `unpack_indices` returns `3` when it should return `2` — surfaced inside `CompressedVector::new`. | Reject; the inner codec reported the fault. | `tests/rejection.rs::decode_invalid_residual_flag` |
| `Io(std::io::Error)` | Not exercised in Phase 16 because `from_bytes` takes `&[u8]`; the variant exists for the Phase 17 `CorpusFileReader::open(path)` code path. Covered by a type-level smoke test that ensures `From<std::io::Error>` compiles. | File-system / permission recovery. | `tests/rejection.rs::io_from_impl_compiles` |

- [ ] **Step 4: Implement `pack.rs` and `unpack.rs`**

Exact code from [[design/rust/serialization-format|Serialization
Format]] §Packing indices. The panic-free discipline from Lesson L7
applies: every slice access in the `if bit_off + bit_width > 8`
cross-byte branch must go through `get` / `get_mut` or a debug
assertion plus a narrow `#[allow(clippy::indexing_slicing)]` on the
function. Prefer the debug-assert path because the function is on
the hot decoder path and `get` adds a bounds check per byte.

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
    let hash_raw = data.get(1..65).ok_or(IoError::Truncated { needed: HEADER_SIZE, got: data.len() })?;
    let trim = hash_raw.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    let config_hash = core::str::from_utf8(&hash_raw[..trim])
        .map_err(|_| IoError::InvalidUtf8)?;
    let dim_bytes: [u8; 4] = data
        .get(65..69)
        .ok_or(IoError::Truncated { needed: HEADER_SIZE, got: data.len() })?
        .try_into()
        .map_err(|_| IoError::Truncated { needed: HEADER_SIZE, got: data.len() })?;
    let dimension = u32::from_le_bytes(dim_bytes);
    let bit_width = data[69];
    Ok((version, config_hash, dimension, bit_width))
}
```

Note the `.get(..).ok_or(...)` pattern instead of slice indexing —
required by `clippy::indexing_slicing`, and it replaces the
`.try_into().unwrap()` that would trip `clippy::unwrap_used`.
Lesson L7 applies.

- [ ] **Step 6: Implement `to_bytes.rs` and `from_bytes.rs`**

Per [[design/rust/serialization-format|Serialization Format]]
§to_bytes and §from_bytes. The only deviation from the design-doc
pseudocode is the `.try_into()` + `?` pattern instead of the
`.try_into().unwrap()` on the `u32` dimension conversion (see
Step 5). Apply the same lint-safe transformation on the residual
length decode.

- [ ] **Step 7: Run header-size audit — expect pass.**

- [ ] **Step 8: Write exhaustive bit-pack coverage**

The single `bit_pack_4bit_dim16_exhaustive` test in the original
draft claimed to enumerate "all 2^16 states" but 16 indices × 4
bits = 64 bits ≈ 1.8e19 total states, far beyond any CI budget.
The pragmatic replacement is an **adjacent-slot coverage matrix**:
for each (bit_width, dim) case, pin all slots to zero except two
adjacent slots, and vary those two slots through every pair. This
gives `(dim - 1) * 2^bit_width * 2^bit_width` assertions per case,
which scales with `dim` and bit_width but stays well under the
5-minute budget for the cases below.

#### Bit-pack coverage matrix

| bit_width | dim values | pair count | cumulative asserts | notes |
|-----------|-----------|------------|--------------------|-------|
| 2 | `{1, 2, 3, 4, 8, 15, 16, 17, 768}` | 16 | ≤ 13 072 | tightest pack; `dim=1` skips the pair loop, just round-trips a single byte |
| 4 | `{1, 2, 3, 4, 8, 15, 16, 17, 768}` | 256 | ≤ 209 152 | default path; dim=16 overlaps with the design-doc example |
| 8 | `{1, 7, 8, 9, 768, 1536}` | 65 536 | ≤ 100 663 296 | plain byte copy; dim values chosen to stress `% 8` boundaries |

The bw=8 cumulative figure is a worst-case ceiling (each dim fully
expanded). In practice we use `prop_filter` to skip trivially
redundant pairs: bw=8 with dim=7 or dim=9 runs 65 536 × 6 = 393 216
assertions total, well under a second.

Formula (from [[design/rust/serialization-format|Serialization
Format]] §Packing indices):

```text
asserts(bit_width, dim) = max(0, dim - 1) * 2^bit_width * 2^bit_width
```

Smoke test covering every (bit_width, dim) in the matrix:

```rust
#[test]
fn bit_pack_coverage_matrix() {
    let cases: &[(u8, &[usize])] = &[
        (2, &[1, 2, 3, 4, 8, 15, 16, 17, 768]),
        (4, &[1, 2, 3, 4, 8, 15, 16, 17, 768]),
        (8, &[1, 7, 8, 9, 768, 1536]),
    ];
    for &(bit_width, dims) in cases {
        let max_idx = 1u16 << bit_width;
        for &dim in dims {
            let packed_len = (dim * bit_width as usize + 7) / 8;
            for i in 0..dim.saturating_sub(1) {
                for a in 0..max_idx {
                    for b in 0..max_idx {
                        let mut indices = vec![0u8; dim];
                        indices[i] = a as u8;
                        indices[i + 1] = b as u8;
                        let mut packed = vec![0u8; packed_len];
                        pack_indices(&indices, bit_width, &mut packed);
                        let mut back = vec![0u8; dim];
                        unpack_indices(&packed, dim, bit_width, &mut back);
                        assert_eq!(indices, back,
                            "bw={bit_width} dim={dim} i={i} a={a} b={b}");
                    }
                }
            }
        }
    }
}
```

Every case packs and unpacks and asserts round-trip equality. A
failure immediately points at the offending (bit_width, dim, slot,
values) tuple.

- [ ] **Step 9: Run — expect pass.**

- [ ] **Step 10: Random round-trip via deterministic ChaCha loop**

Phase 14 Lesson L6 rules out `proptest = "1"` on the current MSRV
1.81: its transitive dep tree pulls `getrandom 0.4.2` →
`edition2024`, which is stable only from Rust 1.85. We therefore
substitute the ChaCha-seeded loop pattern used by
`quantize_indices_always_in_codebook_across_random_inputs` in
`tinyquant-core`. This adds **nothing** to the dep graph because
`rand_chacha` is already a runtime dependency of
`tinyquant-core`, which `tinyquant-io` depends on.

Reference test (abridged for the plan — the real file covers the
full matrix):

```rust
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

#[test]
fn roundtrip_identity_chacha_256() {
    let mut rng = ChaCha20Rng::seed_from_u64(16_000);
    for _ in 0..256 {
        let dim = 1 + (rng.next_u32() as u32 % 2048);
        let bit_width = [2u8, 4, 8][(rng.next_u32() as usize) % 3];
        let residual_on = rng.next_u32() & 1 == 1;
        let n = dim as usize;
        let indices = vec![0u8; n].into_boxed_slice();
        let residual = if residual_on {
            Some(vec![0u8; n * 2].into_boxed_slice())
        } else {
            None
        };
        let cv = CompressedVector::new(
            indices, residual,
            Arc::from("deadbeef"), dim, bit_width,
        ).unwrap();
        let bytes = to_bytes(&cv);
        let decoded = from_bytes(&bytes).unwrap();
        assert_eq!(decoded.indices().len(), cv.indices().len());
        assert_eq!(decoded.bit_width(), cv.bit_width());
        assert_eq!(decoded.dimension(), cv.dimension());
        assert_eq!(decoded.has_residual(), cv.has_residual());
    }
}
```

**Case count gating.** The base loop is 256 batches. A companion
function reads the `TINYQUANT_PROPTEST_CASES` environment variable
(accepted even though we don't use proptest — the name is the
contract Phase 16+ tests agree on) and overrides the base:

| env | iterations | CI profile |
|-----|-----------:|------------|
| unset or `256` | 256 | PR (default) |
| `4096` | 4 096 | nightly (`rust-nightly.yml`) |
| `1` | 1 | smoke for editor save hooks |

**Shrinker hint note.** Because we are not using proptest, there is
no automatic shrinker. When the ChaCha loop fails, the seed
(`16_000`) and the iteration counter uniquely identify the case;
the test prints them in the `assert_eq!` message. To manually
minimize: re-run with `TINYQUANT_PROPTEST_CASES=1` after replacing
the seed with the failing iteration's draw, shrink `dim` first
(bisect between `1` and the failing value), then shrink
`bit_width` (prefer 8 → 4 → 2 because wider pack is simpler). Commit
the minimized case as a plain `#[test]` in `tests/regressions.rs`.

**Re-entry path.** When the workspace MSRV moves past 1.85 (Phase
22+ likely), re-introduce `proptest` as a dev-dependency and wrap
the ChaCha loop with a `proptest!` block — do not delete the
deterministic loop, expand it. The ChaCha version stays as a
regression floor.

- [ ] **Step 10a: IoError taxonomy sanity test**

Exercise each variant listed in the IoError taxonomy table above
inside `tests/rejection.rs`. The byte-mutation column is the exact
input each test feeds to `from_bytes`. Run before the Python parity
test so that fixture bytes don't accidentally mask a missing
variant.

- [ ] **Step 11: Python parity fixture test**

Capture the 10 `(cv, bytes)` cases from the fixture contract table
via `cargo xtask fixtures refresh-serialization`. If the Python
generator module does not yet exist, create
`src/tinyquant_cpu/tools/dump_serialization.py` as part of this
step — it is the smallest change possible: walk the CLI args, build
a `CompressedVector`, call `.to_bytes()`, write four output files.

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

Repeat for every case in the manifest. Use `include_bytes!` for the
small fixtures in the matrix (largest is case_07 at ~4.6 KiB of
payload); switch to runtime `std::fs::read` only if total linked
fixture bytes per integration-test binary exceed 64 KiB (see Phase
14 note about the 2.5 MiB training corpus, which did need
`std::fs::read`). Phase 16 fixtures do not.

- [ ] **Step 12: Rejection tests** — covered by Step 10a's IoError
  taxonomy table. Verify `cargo test -p tinyquant-io rejection`
  yields one passing test per row.

- [ ] **Step 13: Run workspace test/clippy/fmt.**

```bash
cargo xtask fmt
cargo xtask lint
cargo test -p tinyquant-io --all-features
cargo test -p tinyquant-io --no-default-features
cargo build -p tinyquant-core --no-default-features --target thumbv7em-none-eabihf
```

### Clippy profile gotchas (ported from Phase 14)

The `tinyquant-io` crate inherits the same lint wall as
`tinyquant-core`: `clippy::pedantic + nursery + unwrap_used +
expect_used + panic + indexing_slicing + cognitive_complexity`. The
concrete patterns that bit Phase 14 and are especially relevant to
serialization code:

- **`indexing_slicing`.** The draft `decode_header` writes
  `data[65..69]`. Under the lint wall this is rejected. Rewrite to
  `data.get(65..69).ok_or(IoError::Truncated { ... })?` or, when the
  preceding `data.len() < HEADER_SIZE` check has already run, wrap
  the direct slice in a module-local `fn header_slice(data: &[u8]) ->
  &[u8; 70]` safe helper that uses `TryInto` and propagates the
  error once. Prefer the helper because it localizes the
  `#[allow(clippy::indexing_slicing)]` attribute (if needed at all)
  to one function.
- **`cast_precision_loss` / `cast_possible_truncation` /
  `cast_sign_loss`.** The `packed_len = (dim as usize) *
  (bit_width as usize) + 7) / 8` expression is fine because
  `u32 → usize` is lossless on all supported targets, but the
  compiler can't prove it on 16-bit targets. The crate target list
  does not include any 16-bit targets, so add a narrow
  `#[allow(clippy::cast_possible_truncation)]` on the single
  function that performs the cast, with a comment pointing at the
  32-bit-or-wider target assumption. Do not loosen the crate-wide
  profile.
- **`missing_fields_in_debug`** on any hand-written Debug impl for
  the fixture-case helper struct (the table-driven test loader).
  Cover every field or call `.finish_non_exhaustive()`.
- **`unwrap_used` / `expect_used` ban.** `decode_header` cannot
  `.unwrap()` on `data[65..69].try_into()`. Use `?` plus the
  `Truncated` error (see Step 5 rewrite). The `residual_length`
  decode in `from_bytes` has the same shape and needs the same
  rewrite.
- **`trivially_copy_pass_by_ref`.** Any `bit_width` parameter must
  be passed by value (`u8`), not by reference. Same rule for `u32`
  lengths.
- **`bool_to_int_using_if`.** When computing `total` length with
  an optional residual, write
  `let residual_overhead = usize::from(residual.is_some()) * 5 + residual_len;`
  instead of `if residual.is_some() { 5 + residual_len } else { 0 }`.
- **`redundant_pub_crate`.** Items inside a `pub(crate) mod` are
  declared `pub`, not `pub(crate)`. The `compressed_vector` module
  is `pub`, so inner helpers remain `pub(crate)`.

Re-read this list before the first `cargo xtask lint` run.

- [ ] **Step 14: Fuzz target**

Add `rust/crates/tinyquant-fuzz/fuzz_targets/compressed_vector_from_bytes.rs`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tinyquant_io::compressed_vector::from_bytes(data);
});
```

**Seed corpus layout:**

```text
rust/crates/tinyquant-fuzz/corpus/compressed_vector_from_bytes/
├── case_01.bin    # copy of rust/crates/tinyquant-io/tests/fixtures/compressed_vector/case_01/expected.bin
├── case_02.bin
├── ...
└── case_10.bin
```

Populate the corpus by a one-shot cp (or symlink on Linux) from the
fixture directory; the corpus files are git-tracked (not LFS — they
are at most a few KiB each and libfuzzer benefits from random-access
reads without LFS pointer juggling).

**Crash-reduction workflow:**

1. On failure, `cargo fuzz tmin -s target/ compressed_vector_from_bytes <crash-input>`
   to shrink the reproducer.
2. Commit the minimized input under
   `rust/crates/tinyquant-io/tests/regressions/compressed_vector_from_bytes/`.
3. Add a unit test in `tests/regressions.rs` that reads the file and
   calls `from_bytes`, asserting the expected `IoError` variant.

**CI schedule:**

- PR workflow: no fuzzing (keeps per-PR budget under the 25-minute
  target per [[design/rust/ci-cd|CI/CD]] §Stages).
- Nightly workflow (`rust-nightly.yml`): 10 minutes per target via
  `cargo fuzz run compressed_vector_from_bytes -- -max_total_time=600`.
  Any crash opens a GitHub issue tagged `fuzz-regression` via the
  nightly workflow's issue-creation step.
- Local smoke: `cargo fuzz run compressed_vector_from_bytes -- -max_total_time=60`
  before commit.

**Integration with `rust-ci.yml`:** the main PR workflow does not
run the fuzz target directly, but it does run a no-op compile check
(`cargo fuzz build compressed_vector_from_bytes`) inside an advisory
job so that a broken target surface shows up on the PR without
blocking. See [[design/rust/ci-cd|CI/CD]] §Quality gates vs
advisories for the advisory-only designation.

- [ ] **Step 15: Commit**

```bash
git add rust/crates/tinyquant-io
git add rust/crates/tinyquant-fuzz/fuzz_targets/compressed_vector_from_bytes.rs
git add rust/crates/tinyquant-fuzz/corpus/compressed_vector_from_bytes
git add rust/xtask/src/cmd/fixtures.rs
git add src/tinyquant_cpu/tools/dump_serialization.py
git commit -m "feat(tinyquant-io): add CompressedVector serialization with Python byte parity"
```

Follow the Phase 14 split-commit pattern if the patch is large:

1. `chore(workspace)` — `.gitattributes` LFS globs + xtask
   subcommand skeleton.
2. `feat(tinyquant-io)` — header + pack + unpack + to_bytes +
   from_bytes.
3. `test(tinyquant-io)` — fixtures + parity tests + rejection
   tests.
4. `feat(tinyquant-fuzz)` — fuzz target + corpus.
5. `docs(rust)` — flip phase plan status if done.

## Acceptance criteria

- Header is exactly 70 bytes (test + const).
- Bit pack/unpack passes the adjacent-slot coverage matrix for
  every (bit_width, dim) entry.
- ChaCha-seeded random round-trip passes 256 iterations on PR,
  4 096 on nightly (`TINYQUANT_PROPTEST_CASES=4096`).
- All ten Python fixture cases produce byte-identical `to_bytes`
  output.
- `from_bytes` rejects every row of the IoError taxonomy table
  with the expected variant.
- Fuzz target compiles and runs 60 seconds locally without
  panicking; advisory build job in `rust-ci.yml` stays green.
- `cargo xtask fmt`, `cargo xtask lint`, `cargo test -p
  tinyquant-io --all-features`, and `cargo test -p tinyquant-io
  --no-default-features` all pass.
- `git diff --exit-code rust/crates/tinyquant-io/tests/fixtures/`
  is clean after `cargo xtask fixtures refresh-serialization`.
- Phase-end CI health check: `gh run list --workflow rust-ci.yml
  --branch main --limit 5` shows at least one successful run whose
  change set touches `rust/crates/tinyquant-io/` (Lesson L1).

## CI integration

### New `rust-ci.yml` job

Name: `tinyquant-io-test`. Commands:

```yaml
- name: Checkout (with LFS)
  uses: actions/checkout@v4
  with:
    lfs: true           # REQUIRED — see Lesson L2

- name: Install toolchain
  uses: dtolnay/rust-toolchain@stable
  with:
    toolchain: "1.81.0"  # MUST match rust/rust-toolchain.toml — see Lesson L3

- run: cargo test -p tinyquant-io --all-features
- run: cargo test -p tinyquant-io --no-default-features
- run: cargo xtask fixtures refresh-serialization
- run: git diff --exit-code rust/crates/tinyquant-io/tests/fixtures/
```

The `git diff --exit-code` step is the fixture drift gate; it fails
the job if the committed fixtures disagree with a fresh refresh.

### Feature-matrix entries

Three entries added to the existing feature-matrix job (see
[[design/rust/feature-flags|Feature Flags]] §CI feature matrix):

| Job | Command |
|---|---|
| `io-default` | `cargo test -p tinyquant-io` |
| `io-no-default` | `cargo test -p tinyquant-io --no-default-features` |
| `io-all-features` | `cargo test -p tinyquant-io --all-features` |

### LFS hydration gate (Lesson L2)

Every `actions/checkout@v4` step in any job that reads fixtures
under `rust/crates/tinyquant-io/tests/fixtures/` must carry
`with: { lfs: true }`. Add a one-line xtask check that fails the
phase-end sanity sweep if a job touching `tinyquant-io` lacks the
flag:

```bash
cargo xtask ci check-lfs-hydration
# Greps .github/workflows/*.yml for any actions/checkout@v4 step
# in a job referenced from tinyquant-io tests and verifies
# `lfs: true` is present.
```

The implementation is a small grep wrapped in the xtask crate — the
check logic lives in `rust/xtask/src/cmd/ci.rs` (create if needed).
Failure message: "LFS hydration missing in rust-ci.yml job `<name>`;
fixtures under tinyquant-io will download as 132-byte pointer files
and silently fail parity tests. See Phase 14 Lesson L2."

### Toolchain version gate (Lesson L3)

`rust-ci.yml` declares the toolchain version on the
`dtolnay/rust-toolchain` action. That string **must** equal the
channel in `rust/rust-toolchain.toml` (currently `1.81`). Add a
second grep to the same xtask:

```bash
cargo xtask ci check-toolchain-version
```

The command reads `rust/rust-toolchain.toml`, extracts `channel`,
then greps every job in `.github/workflows/rust-ci.yml` for
`toolchain:` lines. If any mismatch, it fails with the diff. This
prevents the Phase 13/14 situation where `rust-ci.yml` was still
pinning 1.78 while the workspace had moved to 1.81.

### Post-push gate (Lesson L1)

Before marking Phase 16 complete, run:

```bash
gh run list --workflow rust-ci.yml --branch main --limit 5
```

Confirm at least one `success` entry whose commit touches
`rust/crates/tinyquant-io/**`. A `completed failure` on `main`
blocks the phase-complete transition regardless of local test
results.

## Risks

| id | risk | likelihood | impact | mitigation |
|----|------|------------|--------|------------|
| R16.1 | Python byte layout drifts before fixtures are refreshed | medium | blocking phase | `xtask fixtures refresh-serialization` + CI fixture drift gate (`git diff --exit-code`); fixtures live in LFS with a `manifest.json` pinning `python_sha`. |
| R16.2 | Endian assumption broken on a future big-endian target | low | wire-format incompatible | `u32::from_le_bytes` / `to_le_bytes` is explicit; add a `#[cfg(target_endian = "big")]` compile-time assertion in `lib.rs` and exercise on a big-endian QEMU emulator (`powerpc64-unknown-linux-gnu`) in `rust-nightly.yml`. |
| R16.3 | `proptest` dep tree bumps MSRV above 1.81 | medium | CI red on first edit | Use the ChaCha-seeded loop pattern (see Step 10 and Lesson L6); never introduce `proptest` on MSRV 1.81. Re-entry gated on future MSRV bump. |
| R16.4 | Fuzz target finds a panic in `from_bytes` | medium | parser incorrect | Seed corpus from Python fixtures; nightly `cargo fuzz` with 10-minute budget; `cargo fuzz tmin` on crashes; regression tests under `tests/regressions.rs`. |
| R16.5 | Header size silently changes (70 → 71) | high | wire break | Header-size audit test in Step 1 + `const HEADER_SIZE: usize = 70` frozen in `header.rs`; byte-layout table in Deliverables is the doc gate. |
| R16.6 | Cross-runner SIMD nondeterminism in the serializer path | low | CI flaky | Serializer path is purely byte-shuffling — no float kernels — so pulp / faer SIMD dispatch does not apply. Still, Phase 14 Lesson L4 is explicit about treating "green on first run" with suspicion; Step 13 requires two consecutive green CI runs before commit. |
| R16.7 | LFS pointer hydration fails on a new CI job | medium | silent test-no-op | Lesson L2: `xtask ci check-lfs-hydration` gate in Step 13; every new job must pass. |

## Out of scope

Explicitly deferred to later phases (do **not** attempt in Phase 16):

- **Level-2 corpus file container** (magic `b"TQCV"`, `u16` format
  version, per-vector entries). Phase 17.
- **Zero-copy view iterator** (`CompressedVectorView<'a>`). Phase 17.
- **`serde` Serialize/Deserialize impls** on `CompressedVector` and
  `CompressedVectorView`. Deferred until the pyo3 binding lands
  in Phase 22, and only if a consumer actually needs it. The wire
  format is stable without serde.
- **Compression of the serialized stream** (gzip, zstd, lz4, or
  any other entropy coding applied after `to_bytes`). Explicitly
  **not** a goal: the whole point of fixed-bit-width quantization
  is the compression ratio claim, and wrapping the output in an
  entropy coder would confuse the story. Downstream consumers can
  compress at the storage layer if they want; that's their choice.
- **mmap reader** (`CorpusFileReader`, `CorpusFileIter`). Phase 17.
- **Streaming codec-file writer / reader** (append-only log format,
  Phase 17/18).
- **Parity with the Python `errors="replace"` lossy UTF-8 path** —
  see [[design/rust/serialization-format|Serialization Format]]
  §Handling short config hashes. Rust returns `IoError::InvalidUtf8`.

## See also

- [[plans/rust/phase-15-codec-service-residual|Phase 15]]
- [[plans/rust/phase-17-zero-copy-mmap|Phase 17]]
- [[design/rust/serialization-format|Serialization Format]]
- [[design/rust/error-model|Error Model]]
- [[design/rust/testing-strategy|Testing Strategy]]
- [[design/rust/feature-flags|Feature Flags]]
- [[design/rust/ci-cd|CI/CD]]
- [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]]
