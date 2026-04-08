---
title: "Unit Tests: Codebook"
tags:
  - qa
  - unit-tests
  - codec
date-created: 2026-04-08
status: active
category: qa
---

# Unit Tests: Codebook

> [!info] Class under test
> [[classes/codebook|Codebook]] — quantization lookup table.

## Test file

```text
tests/codec/test_codebook.py
```

## Test cases

### Training

| Test | Asserts |
|------|---------|
| `test_train_produces_correct_entry_count_4bit` | `num_entries == 16` |
| `test_train_produces_correct_entry_count_2bit` | `num_entries == 4` |
| `test_train_entries_are_sorted` | Entries in ascending order |
| `test_train_entries_have_no_duplicates` | All entries distinct |
| `test_train_with_too_few_vectors_raises` | Insufficient training data → `ValueError` |
| `test_train_is_deterministic` | Same training data + config → identical codebook |

### Quantization

| Test | Asserts |
|------|---------|
| `test_quantize_returns_valid_indices` | All indices in `[0, num_entries)` |
| `test_quantize_maps_to_nearest_entry` | Each value maps to its closest codebook entry |
| `test_quantize_exact_entry_value` | A value equal to a codebook entry maps to that entry's index |
| `test_quantize_output_dtype` | Returns `uint8` array |

### Dequantization

| Test | Asserts |
|------|---------|
| `test_dequantize_maps_indices_to_entries` | Index `i` → `entries[i]` |
| `test_dequantize_output_dtype` | Returns `float32` array |
| `test_dequantize_invalid_index_raises` | Index >= `num_entries` raises error |

### Immutability

| Test | Asserts |
|------|---------|
| `test_frozen_rejects_entry_modification` | Modifying `entries` array raises or is prevented |

## See also

- [[classes/codebook|Codebook]]
- [[qa/unit-tests/test-codec|Codec tests]]
