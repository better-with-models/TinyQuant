---
title: "Unit Tests: BruteForceBackend"
tags:
  - qa
  - unit-tests
  - backend
date-created: 2026-04-08
status: active
category: qa
---

# Unit Tests: BruteForceBackend

> [!info] Class under test
> [[classes/brute-force-backend|BruteForceBackend]] — reference search
> implementation.

## Test file

```text
tests/backend/test_brute_force.py
```

## Test cases

### ingest

| Test | Asserts |
|------|---------|
| `test_ingest_stores_vectors` | `count` increases after ingestion |
| `test_ingest_duplicate_id_overwrites` | Second ingest with same ID replaces vector |
| `test_ingest_empty_dict_is_noop` | No error, count unchanged |

### search

| Test | Asserts |
|------|---------|
| `test_search_returns_top_k_results` | Exactly `top_k` results when enough vectors exist |
| `test_search_results_ranked_by_similarity` | `results[0].score >= results[1].score >= ...` |
| `test_search_with_fewer_vectors_than_top_k` | Returns all available vectors |
| `test_search_returns_search_result_objects` | Each result is a `SearchResult` |
| `test_search_identical_vector_has_score_near_one` | Cosine similarity ≈ 1.0 for exact match |
| `test_search_orthogonal_vector_has_score_near_zero` | Cosine similarity ≈ 0.0 |
| `test_search_empty_backend_returns_empty` | No vectors ingested → empty result list |

### remove

| Test | Asserts |
|------|---------|
| `test_remove_decreases_count` | Count goes down |
| `test_removed_vector_not_in_search_results` | Removed ID absent from subsequent searches |

### clear

| Test | Asserts |
|------|---------|
| `test_clear_empties_backend` | `count == 0` after clear |

## See also

- [[classes/brute-force-backend|BruteForceBackend]]
- [[classes/search-backend|SearchBackend]]
- [[classes/search-result|SearchResult]]
