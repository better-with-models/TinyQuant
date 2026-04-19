---
title: "Phase 16 Implementation Notes"
tags:
  - design
  - rust
  - phase-16
  - serialization
  - io
date-created: 2026-04-11
category: design
---

# Phase 16 Implementation Notes

## What landed

Phase 16 delivers the **`tinyquant-io` serialization crate** with Python
byte-level parity for `CompressedVector`. The crate establishes the canonical
on-disk and over-the-wire layout for every compressed vector produced by
TinyQuant.

### Concrete deliverables

- `rust/crates/tinyquant-io/src/compressed_vector/header.rs` — fixed-size
  header layout: 4-byte magic (`TQCV`), 1-byte format version, 1-byte bit-width,
  2-byte dimension (LE u16), 4-byte config hash (LE u32), 1-byte flags (residual
  present bit), 3-byte reserved (zero-padded).
- `rust/crates/tinyquant-io/src/compressed_vector/to_bytes.rs` — serializer.
  Writes header followed by packed quantization indices, then optional FP16
  residual payload.
- `rust/crates/tinyquant-io/src/compressed_vector/from_bytes.rs` — deserializer.
  Validates magic and version, checks allocation bounds, returns typed
  `TinyQuantIoError` variants on any malformed input.
- `rust/crates/tinyquant-io/src/compressed_vector/pack.rs` /
  `unpack.rs` — bit-pack / bit-unpack for bw ∈ {2, 4, 8} indices into a
  dense byte array. Indices are packed LSB-first within each byte (index 0
  occupies the least-significant bits of byte 0).
- `rust/crates/tinyquant-io/src/errors.rs` — `IoError` enum with
  `Truncated`, `UnknownVersion`, `InvalidBitWidth`, `InvalidUtf8`,
  `LengthMismatch`, `BadMagic`, `InvalidHeader`, `Decode`, `Io` variants.
- `rust/crates/tinyquant-io/tests/bit_pack_exhaustive.rs` — exhaustive round-trip
  test covering every index value at every bit-width, verifying no information is
  dropped by the pack/unpack cycle.
- `tests/fixtures/case_01/` and `case_02/` — Python-generated binary fixtures with
  companion `config_hash.txt`. Consumed by cross-language parity tests in
  `tests/parity/`.
- `pyproject.toml` updated: `scripts/dump_serialization.py` added under
  `[tool.tinyquant.scripts]`; mypy strict annotations added to the dump tool.

## Deviations from the plan

### 1. Allocation bound check on deserialize

The plan did not specify upper-bound checking during `from_bytes`. During
implementation it became clear that a malformed `dim` field (e.g., `0xFFFF`)
would cause `from_bytes` to attempt a `65535 * bw / 8`-byte allocation before
any content validation. The bound check (`dim * bw / 8 ≤ available_bytes`) was
added as a safety gate before any heap allocation. This was later reinforced by
the `fix(tinyquant-io): add decode-time allocation bounds` commit (`01df699`)
which tightened the check to also cover the residual payload length.

### 2. Reserved header bytes zero-padded on write, ignored on read

The plan described 3 reserved bytes without specifying their handling. The
implementation writes zeros and ignores any non-zero values on read (rather than
rejecting them). This keeps forward compatibility: a future format version may
use those bytes without breaking readers built against Phase 16.

### 3. Python fixture generation moved to `scripts/`

The plan implied parity fixtures would be hand-authored. Instead, the fixture
generator lives at `scripts/dump_serialization.py` so the fixtures can be
regenerated deterministically when the Python reference implementation changes.
The generator is invoked manually; fixture binaries are committed to LFS under
`.gitattributes`.

### 4. `cast_possible_truncation` allow in `to_bytes`

The serializer uses `dimension as u8` for the `bit_width` field. The only
cast that requires a lint suppression is in the `encode_header` call where
`bit_width` (a `u8` from `CodecConfig`) is passed directly — no truncation
occurs. In `from_bytes` the dimension is a `u32` and the config hash is a
`&str`, so no width-narrowing cast is needed there either.

## Wire format

```
offset  size  field
------  ----  -----
0       1     format version: 0x01
1       64    config_hash (UTF-8, NUL-padded to 64 bytes)
65      4     dimension (LE u32)
69      1     bit_width: 2 | 4 | 8
--- header end (70 bytes) ---
70      ceil(dim * bw / 8)   packed indices (LSB-first)
+       dim * 2              residual (FP16 LE, if present)
```

The `TQCV` magic bytes appear only in the Level-2 corpus file container
header (Phase 17), not in individual `CompressedVector` blobs. Residual
presence is indicated by the calling context (e.g., a `flags` byte in the
corpus container), not by a field within the `CompressedVector` header itself.

## Parity guarantee

The Python `dump_serialization.py` script calls
`tinyquant_py_reference.Codec().compress(...)` and writes the resulting bytes
using the same header layout. The Rust `bit_pack_exhaustive` test and the Python
fixture parity tests together guarantee that any `CompressedVector` produced by
the Python reference can be deserialized by the Rust crate and vice versa.

## Risks carried forward

- **R-IO-1**: The format version is fixed at `0x01`. A future schema change
  (e.g., wider dimension field) requires a version bump and a migration path.
  Phase 17's corpus file container wraps multiple `CompressedVector` blobs, so
  any version-bump work must coordinate across both layers.
- **R-IO-2**: The `case_01` / `case_02` fixtures are frozen at the Phase 16
  Python reference state. If the Python reference is updated in a way that
  changes byte output, the fixtures must be regenerated and the parity test
  will fail as intended.
