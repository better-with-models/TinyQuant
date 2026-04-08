---
title: "Unit Tests: Corpus Events"
tags:
  - qa
  - unit-tests
  - corpus
  - events
date-created: 2026-04-08
status: active
category: qa
---

# Unit Tests: Corpus Events

> [!info] Classes under test
> [[classes/corpus-events|Corpus Events]] — domain event dataclasses.

## Test file

```text
tests/corpus/test_events.py
```

## Test cases

### CorpusCreated

| Test | Asserts |
|------|---------|
| `test_corpus_created_fields` | All fields populated correctly |
| `test_corpus_created_frozen` | Assignment to fields raises error |
| `test_corpus_created_timestamp_is_utc` | Timestamp is timezone-aware UTC |

### VectorsInserted

| Test | Asserts |
|------|---------|
| `test_vectors_inserted_fields` | `vector_ids` is a tuple, `count` matches length |
| `test_vectors_inserted_frozen` | Immutable |
| `test_vectors_inserted_ids_are_tuple` | Uses `tuple`, not `list` (immutability guarantee) |

### CorpusDecompressed

| Test | Asserts |
|------|---------|
| `test_corpus_decompressed_fields` | `vector_count` and `corpus_id` populated |
| `test_corpus_decompressed_frozen` | Immutable |

### CompressionPolicyViolationDetected

| Test | Asserts |
|------|---------|
| `test_violation_detected_fields` | `violation_type`, `detail`, `corpus_id` populated |
| `test_violation_detected_frozen` | Immutable |

## See also

- [[classes/corpus-events|Corpus Events]]
- [[qa/unit-tests/test-corpus|Corpus tests]]
