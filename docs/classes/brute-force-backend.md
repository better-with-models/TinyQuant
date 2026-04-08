---
title: BruteForceBackend
tags:
  - classes
  - backend
  - implementation
date-created: 2026-04-08
status: active
category: design
---

# BruteForceBackend

> [!info] Responsibility
> Reference implementation of the [[classes/search-backend|SearchBackend]]
> protocol using exhaustive cosine similarity comparison. Intended for testing,
> calibration, and small-corpus use cases — not production-scale search.

## Location

```text
src/tinyquant/backend/brute_force.py
```

## Category

**Concrete implementation** of the `SearchBackend` protocol.

## Constructor

| Parameter | Type | Description |
|-----------|------|-------------|
| *(none)* | — | Initializes with an empty vector store |

## Fields

| Field | Type | Mutability | Description |
|-------|------|-----------|-------------|
| `_vectors` | `dict[str, NDArray[np.float32]]` | Internal | In-memory vector store |

## Methods

### `search`

```python
def search(
    self,
    query: NDArray[np.float32],
    top_k: int,
) -> list[SearchResult]:
```

**Purpose:** Compute cosine similarity between `query` and all stored vectors,
return the top-k results.

**Complexity:** O(n * d) where n is stored vector count and d is dimension.

---

### `ingest`

```python
def ingest(
    self,
    vectors: dict[str, NDArray[np.float32]],
) -> None:
```

**Purpose:** Add or replace vectors in the in-memory store.

---

### `remove`

```python
def remove(
    self,
    vector_ids: Sequence[str],
) -> None:
```

**Purpose:** Remove vectors by ID.

---

### `clear`

```python
def clear(self) -> None:
```

**Purpose:** Remove all stored vectors. Convenience method for test teardown.

## Properties

| Property | Return type | Description |
|----------|-------------|-------------|
| `count` | `int` | Number of stored vectors |

## Invariants

1. Only stores FP32 vectors (enforced by type signature)
2. `search` results are ranked by cosine similarity, descending
3. Duplicate IDs on `ingest` overwrite the previous vector

## Relationships

| Direction | Class | Nature |
|-----------|-------|--------|
| Implements | [[classes/search-backend\|SearchBackend]] | Satisfies the protocol structurally |
| Returns | [[classes/search-result\|SearchResult]] | Ranked results from `search` |
| Used by | Calibration tests | Score fidelity and rank preservation verification |

## Example

```python
backend = BruteForceBackend()
backend.ingest(decompressed_vectors)
results = backend.search(query_vector, top_k=10)
assert len(results) == 10
assert results[0].score >= results[1].score
```

## Test file

```text
tests/backend/test_brute_force.py
```

## See also

- [[classes/search-backend|SearchBackend]]
- [[classes/search-result|SearchResult]]
- [[classes/pgvector-adapter|PgvectorAdapter]]
