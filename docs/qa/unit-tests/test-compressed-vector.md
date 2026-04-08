---
title: "Unit Tests: CompressedVector"
tags:
  - qa
  - unit-tests
  - codec
date-created: 2026-04-08
status: active
category: qa
---

# Unit Tests: CompressedVector

> [!info] Class under test
> [[classes/compressed-vector|CompressedVector]] — codec output value object.

## Test file

```text
tests/codec/test_compressed_vector.py
```

## Test cases

### Construction and validation

| Test | Asserts |
|------|---------|
| `test_valid_compressed_vector_creates` | Matching dimension and indices succeeds |
| `test_dimension_mismatch_raises` | `len(indices) != dimension` raises `ValueError` |
| `test_residual_none_when_disabled` | Residual field is `None` when correction was off |
| `test_residual_present_when_enabled` | Residual field is non-empty bytes |

### Properties

| Test | Asserts |
|------|---------|
| `test_has_residual_true` | `has_residual` returns `True` when residual is set |
| `test_has_residual_false` | `has_residual` returns `False` when residual is `None` |
| `test_size_bytes_positive` | `size_bytes` returns a positive integer |
| `test_size_bytes_larger_with_residual` | Residual increases the byte count |

### Serialization

| Test | Asserts |
|------|---------|
| `test_to_bytes_produces_bytes` | Returns `bytes` object |
| `test_from_bytes_round_trip` | `from_bytes(to_bytes(cv)) == cv` |
| `test_from_bytes_corrupted_data_raises` | Truncated or malformed bytes raise error |
| `test_serialization_includes_config_hash` | Config hash survives round trip |

### Immutability

| Test | Asserts |
|------|---------|
| `test_frozen_rejects_field_assignment` | Assigning to any field raises error |

## See also

- [[classes/compressed-vector|CompressedVector]]
- [[qa/unit-tests/test-codec|Codec tests]]
