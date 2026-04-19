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
  dense byte array. Endian-safe: indices are packed MSB-first within each byte.
- `rust/crates/tinyquant-io/src/errors.rs` — `TinyQuantIoError` enum with
  `MagicMismatch`, `UnsupportedVersion`, `AllocationTooLarge`, `Truncated`,
  `UnexpectedEof` variants.
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

The `dim as u16` cast in the serializer triggers Clippy's
`cast_possible_truncation` lint. A narrow `#[allow]` at the call site was
chosen over a checked cast with `unwrap` — the dimension is bounded at
construction time by `CodecConfig::new` (max dim fits in u16), so the
truncation can never occur in practice. A comment explains the invariant.

## Wire format

```
offset  size  field
------  ----  -----
0       4     magic: b"TQCV"
4       1     format version: 0x01
5       1     bit_width: 2 | 4 | 8
6       2     dim (LE u16)
8       4     config_hash (LE u32)
12      1     flags: bit 0 = residual_present
13      3     reserved (zeros)
--- header end (16 bytes) ---
16      ceil(dim * bw / 8)   packed indices
+       dim * 2              residual (FP16 LE, if flags.bit0 set)
```

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
