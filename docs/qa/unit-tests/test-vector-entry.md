---
title: "Unit Tests: VectorEntry"
tags:
  - qa
  - unit-tests
  - corpus
date-created: 2026-04-08
status: active
category: qa
---

# Unit Tests: VectorEntry

> [!info] Class under test
> [[classes/vector-entry|VectorEntry]] — identity-bearing vector entity.

## Test file

```text
tests/corpus/test_vector_entry.py
```

## Test cases

| Test | Asserts |
|------|---------|
| `test_construction_with_valid_fields` | All fields accessible after creation |
| `test_config_hash_delegates_to_compressed` | `entry.config_hash == entry.compressed.config_hash` |
| `test_dimension_delegates_to_compressed` | `entry.dimension == entry.compressed.dimension` |
| `test_has_residual_delegates_to_compressed` | `entry.has_residual == entry.compressed.has_residual` |
| `test_equality_by_vector_id` | Same `vector_id` → equal, regardless of other fields |
| `test_inequality_by_vector_id` | Different `vector_id` → not equal |
| `test_hash_by_vector_id` | Same `vector_id` → same hash for set/dict use |
| `test_metadata_is_mutable` | Metadata dict can be updated after construction |

## See also

- [[classes/vector-entry|VectorEntry]]
- [[qa/unit-tests/test-corpus|Corpus tests]]
