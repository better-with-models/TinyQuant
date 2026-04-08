---
title: "Unit Tests: Corpus"
tags:
  - qa
  - unit-tests
  - corpus
date-created: 2026-04-08
status: active
category: qa
---

# Unit Tests: Corpus

> [!info] Class under test
> [[classes/corpus|Corpus]] — aggregate root for stored compressed vectors.

## Test file

```text
tests/corpus/test_corpus.py
```

## Test cases

### Construction

| Test | Asserts |
|------|---------|
| `test_create_empty_corpus` | Corpus created with zero vectors |
| `test_create_corpus_freezes_config` | `codec_config` is immutable after creation |
| `test_create_corpus_freezes_codebook` | `codebook` is immutable after creation |
| `test_create_corpus_raises_corpus_created_event` | `CorpusCreated` event with correct fields |

### insert

| Test | Asserts |
|------|---------|
| `test_insert_stores_vector` | `vector_count` increments; vector retrievable |
| `test_insert_compresses_with_compress_policy` | Under `COMPRESS`, a `CompressedVector` is created |
| `test_insert_stores_fp32_with_passthrough_policy` | Under `PASSTHROUGH`, original FP32 is stored |
| `test_insert_stores_fp16_with_fp16_policy` | Under `FP16`, vector is downcast |
| `test_insert_config_hash_matches` | Entry's config hash matches corpus config |
| `test_insert_dimension_mismatch_raises` | Wrong-dimension vector → `DimensionMismatchError` |
| `test_insert_duplicate_id_raises` | Same vector_id twice → `DuplicateVectorError` |
| `test_insert_sets_inserted_at` | Entry has a valid UTC timestamp |
| `test_insert_with_metadata` | Per-vector metadata is preserved |

### insert_batch

| Test | Asserts |
|------|---------|
| `test_insert_batch_stores_all` | N vectors → N entries |
| `test_insert_batch_is_atomic` | If one fails, none are stored |
| `test_insert_batch_raises_vectors_inserted_event` | Event lists all inserted IDs |
| `test_insert_batch_empty_is_noop` | Empty dict → no error, no event |

### Policy enforcement

| Test | Asserts |
|------|---------|
| `test_policy_immutable_after_first_insert` | Changing policy after insert → error |
| `test_policy_changeable_when_empty` | Policy can change on empty corpus (if API supports it) |
| `test_cross_config_vector_rejected` | Vector with different config_hash → `ConfigMismatchError` |

### get and contains

| Test | Asserts |
|------|---------|
| `test_get_returns_correct_entry` | Correct vector_id and compressed data |
| `test_get_missing_raises_key_error` | Unknown ID → `KeyError` |
| `test_contains_true_for_stored` | Inserted ID → `True` |
| `test_contains_false_for_missing` | Unknown ID → `False` |

### decompress and decompress_all

| Test | Asserts |
|------|---------|
| `test_decompress_single_returns_fp32` | Correct dimension, float32 dtype |
| `test_decompress_all_returns_all_vectors` | All IDs present in result dict |
| `test_decompress_all_raises_event` | `CorpusDecompressed` event with correct count |
| `test_decompress_missing_raises_key_error` | Unknown ID → `KeyError` |

### remove

| Test | Asserts |
|------|---------|
| `test_remove_decrements_count` | `vector_count` decreases |
| `test_remove_missing_raises_key_error` | Unknown ID → `KeyError` |
| `test_remove_makes_id_unavailable` | `contains` returns `False` after removal |

### Properties

| Test | Asserts |
|------|---------|
| `test_vector_count_reflects_state` | Matches number of inserts minus removes |
| `test_is_empty_on_new_corpus` | `True` initially |
| `test_vector_ids_returns_all_ids` | `frozenset` of all stored IDs |

## See also

- [[classes/corpus|Corpus]]
- [[qa/unit-tests/test-compression-policy|CompressionPolicy tests]]
- [[qa/unit-tests/test-events|Event tests]]
