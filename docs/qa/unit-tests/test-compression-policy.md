---
title: "Unit Tests: CompressionPolicy"
tags:
  - qa
  - unit-tests
  - corpus
date-created: 2026-04-08
status: active
category: qa
---

# Unit Tests: CompressionPolicy

> [!info] Class under test
> [[classes/compression-policy|CompressionPolicy]] — enum governing write-path
> behavior.

## Test file

```text
tests/corpus/test_compression_policy.py
```

## Test cases

| Test | Asserts |
|------|---------|
| `test_compress_requires_codec` | `COMPRESS.requires_codec() is True` |
| `test_passthrough_does_not_require_codec` | `PASSTHROUGH.requires_codec() is False` |
| `test_fp16_does_not_require_codec` | `FP16.requires_codec() is False` |
| `test_compress_storage_dtype` | `COMPRESS.storage_dtype == np.uint8` |
| `test_passthrough_storage_dtype` | `PASSTHROUGH.storage_dtype == np.float32` |
| `test_fp16_storage_dtype` | `FP16.storage_dtype == np.float16` |

## See also

- [[classes/compression-policy|CompressionPolicy]]
- [[qa/unit-tests/test-corpus|Corpus tests]]
