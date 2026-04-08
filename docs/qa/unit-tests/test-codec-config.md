---
title: "Unit Tests: CodecConfig"
tags:
  - qa
  - unit-tests
  - codec
date-created: 2026-04-08
status: active
category: qa
---

# Unit Tests: CodecConfig

> [!info] Class under test
> [[classes/codec-config|CodecConfig]] — immutable configuration value object.

## Test file

```text
tests/codec/test_codec_config.py
```

## Test cases

### Construction and validation

| Test | Asserts |
|------|---------|
| `test_valid_config_creates_successfully` | 4-bit, seed 42, dim 768 constructs without error |
| `test_unsupported_bit_width_raises` | bit_width=3 raises `ValueError` |
| `test_negative_seed_raises` | seed=-1 raises `ValueError` |
| `test_zero_dimension_raises` | dimension=0 raises `ValueError` |
| `test_negative_dimension_raises` | dimension=-5 raises `ValueError` |
| `test_residual_enabled_defaults_true` | Omitting residual_enabled defaults to `True` |

### Immutability

| Test | Asserts |
|------|---------|
| `test_frozen_fields_reject_assignment` | Assigning to any field raises `FrozenInstanceError` |

### Properties

| Test | Asserts |
|------|---------|
| `test_num_codebook_entries_4bit` | `config.num_codebook_entries == 16` |
| `test_num_codebook_entries_2bit` | `config.num_codebook_entries == 4` |
| `test_num_codebook_entries_8bit` | `config.num_codebook_entries == 256` |
| `test_config_hash_is_deterministic` | Same fields produce same hash across calls |
| `test_config_hash_differs_for_different_fields` | Changing any field changes the hash |

### Equality and hashing

| Test | Asserts |
|------|---------|
| `test_equal_configs_are_equal` | Two configs with same fields are `==` |
| `test_equal_configs_have_same_hash` | Hash consistency for dict/set usage |
| `test_different_configs_are_not_equal` | Different bit_width → not `==` |

### Property-based (hypothesis)

| Test | Asserts |
|------|---------|
| `test_any_supported_config_is_valid` | All combinations of supported bit_widths, valid seeds, and positive dimensions construct successfully |

## See also

- [[classes/codec-config|CodecConfig]]
- [[qa/unit-tests/README|Unit Tests]]
