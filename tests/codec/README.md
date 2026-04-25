# tests/codec

This directory contains unit tests for the `codec` bounded context of the Python
reference implementation. Each file corresponds to one module in
`tinyquant_py_reference.codec`. The tests are fast, use small synthetic vectors
(100 vectors at dimension 64), and cover construction, immutability, serialization,
and round-trip behaviour for each value object and the `Codec` domain service.
Several files also include Hypothesis property-based tests for determinism and
batch-vs-individual equivalence.

## What lives here

- `test_codebook.py` — tests for `Codebook`: `train()` entry count (4-bit = 16,
  2-bit = 4), sorted and distinct entries, insufficient-data error, training
  determinism; `quantize()` valid-index range, nearest-entry mapping, exact-value
  lookup, uint8 output dtype; `dequantize()` index-to-entry mapping, float32
  output, out-of-range error; and non-writeable (frozen) entries array.
- `test_codec.py` — tests for `Codec` and the module-level `compress`/`decompress`
  functions: compress output type and field correctness, dimension-mismatch and
  codebook-incompatible errors, residual presence/absence; decompress dtype and
  dimension, config-mismatch and codebook-incompatible errors; round-trip shape
  and bounded MSE, residual reducing reconstruction error; batch compress/decompress
  count and match-individual equivalence; `build_codebook` and `build_rotation`
  helpers; and Hypothesis tests for compress-decompress determinism and
  batch-vs-loop equivalence.
- `test_codec_config.py` — tests for `CodecConfig`: valid construction, unsupported
  bit_width/negative seed/zero or negative dimension errors; frozen field
  immutability; `num_codebook_entries` for 2/4/8 bit; `config_hash` determinism
  and sensitivity to any field change; equality and hash consistency; and a
  Hypothesis test across all supported bit widths.
- `test_compressed_vector.py` — tests for `CompressedVector`: dimension-match
  validation, residual None/present, invalid bit_width; `has_residual` and
  `size_bytes` properties; `to_bytes`/`from_bytes` round-trip including config
  hash region, corrupted-data rejection; and frozen field immutability.
- `test_rotation_matrix.py` — tests for `RotationMatrix`: shape correctness,
  construction determinism, distinct matrices for different seeds/dimensions;
  orthogonality (`R @ R.T ≈ I`), `verify_orthogonality()` pass/fail; `apply`
  changes the vector, `apply_inverse` recovers original, norm preservation,
  dimension-mismatch error; and a Hypothesis round-trip test.
- `__init__.py` — empty package marker.

## How this area fits the system

All tests in this directory use fixtures from `tests/conftest.py` —
`config_4bit`, `config_2bit`, `sample_vectors`, `sample_vector`, and
`trained_codebook`. No corpus or backend imports are needed. These tests define
the behavioural contract for the codec layer that the calibration and E2E suites
depend on.

## Common edit paths

- `test_codec.py` when adding methods to `Codec` (e.g., a new batch helper) or
  changing error types in `tinyquant_py_reference.codec._errors`.
- `test_compressed_vector.py` when the serialization format (header, bit packing,
  residual flag) changes — expected byte offsets and lengths will need updating.
- `test_codec_config.py` when `SUPPORTED_BIT_WIDTHS` changes.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
