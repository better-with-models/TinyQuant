---
title: "Unit Tests: Codec"
tags:
  - qa
  - unit-tests
  - codec
date-created: 2026-04-08
status: active
category: qa
---

# Unit Tests: Codec

> [!info] Class under test
> [[classes/codec|Codec]] — stateless compression/decompression service.

## Test file

```text
tests/codec/test_codec.py
```

## Test cases

### compress

| Test | Asserts |
|------|---------|
| `test_compress_returns_compressed_vector` | Result type is `CompressedVector` |
| `test_compress_indices_in_valid_range` | All indices in `[0, 2^bit_width)` |
| `test_compress_config_hash_matches` | Result `config_hash` matches input config |
| `test_compress_dimension_match` | Result `dimension` matches config |
| `test_compress_deterministic` | Same input → identical output (byte-level) |
| `test_compress_dimension_mismatch_raises` | Wrong-length vector → `DimensionMismatchError` |
| `test_compress_codebook_incompatible_raises` | Mismatched bit_width → `CodebookIncompatibleError` |
| `test_compress_with_residual` | `residual_enabled=True` → non-null residual |
| `test_compress_without_residual` | `residual_enabled=False` → null residual |

### decompress

| Test | Asserts |
|------|---------|
| `test_decompress_returns_fp32_array` | Result dtype is `float32` |
| `test_decompress_correct_dimension` | Result length matches config dimension |
| `test_decompress_deterministic` | Same input → identical output |
| `test_decompress_config_mismatch_raises` | Wrong config hash → `ConfigMismatchError` |
| `test_decompress_codebook_incompatible_raises` | Mismatched codebook → error |

### Round-trip

| Test | Asserts |
|------|---------|
| `test_compress_decompress_round_trip_recovers_shape` | Output has same shape as input |
| `test_round_trip_error_is_bounded` | MSE between original and recovered is below threshold |
| `test_round_trip_with_residual_has_lower_error` | Residual-enabled error < residual-disabled error |

### Batch operations

| Test | Asserts |
|------|---------|
| `test_compress_batch_count_matches` | N inputs → N outputs |
| `test_compress_batch_matches_individual` | Batch results identical to one-at-a-time |
| `test_decompress_batch_count_matches` | N inputs → N rows |
| `test_decompress_batch_matches_individual` | Batch results identical to one-at-a-time |

### build_codebook

| Test | Asserts |
|------|---------|
| `test_build_codebook_entry_count` | `2^bit_width` entries |
| `test_build_codebook_too_few_vectors_raises` | Insufficient training data → error |

### build_rotation

| Test | Asserts |
|------|---------|
| `test_build_rotation_is_orthogonal` | Result passes orthogonality check |
| `test_build_rotation_deterministic` | Same config → same matrix |

### Property-based (hypothesis)

| Test | Asserts |
|------|---------|
| `test_compress_decompress_determinism` | For any valid vector, double compress→decompress is identical |
| `test_batch_matches_individual` | Batch and loop produce identical results for any input set |

## See also

- [[classes/codec|Codec]]
- [[qa/unit-tests/test-codec-config|CodecConfig tests]]
- [[qa/unit-tests/test-codebook|Codebook tests]]
- [[qa/unit-tests/test-rotation-matrix|RotationMatrix tests]]
