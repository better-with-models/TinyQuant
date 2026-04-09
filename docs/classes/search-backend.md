---
title: SearchBackend
tags:
  - classes
  - backend
  - protocol
date-created: 2026-04-08
status: active
category: design
---

# SearchBackend

> [!info] Responsibility
> Protocol defining the contract that external search systems must satisfy
> to integrate with TinyQuant. Backends always receive FP32 vectors — never
> compressed representations.

## Location

```text
src/tinyquant_cpu/backend/protocol.py
```

## Category

**Protocol** — `typing.Protocol` class. No base class inheritance required;
mypy verifies structural conformance.

## Methods

### `search`

```python
def search(
    self,
    query: NDArray[np.float32],
    top_k: int,
) -> list[SearchResult]:
```

**Purpose:** Find the `top_k` most similar vectors to `query`.

**Preconditions:**
- `query` is a 1-D FP32 array matching the dimension of ingested vectors
- `top_k > 0`

**Postconditions:**
- Returns up to `top_k` results ranked by descending similarity
- Results contain `vector_id` and `score`

---

### `ingest`

```python
def ingest(
    self,
    vectors: dict[str, NDArray[np.float32]],
) -> None:
```

**Purpose:** Accept decompressed FP32 vectors for indexing.

**Preconditions:**
- All vectors have the same dimension
- Keys are `vector_id` strings

**Postconditions:**
- Vectors are available for subsequent `search` calls

---

### `remove`

```python
def remove(
    self,
    vector_ids: Sequence[str],
) -> None:
```

**Purpose:** Remove vectors from the backend's index.

## Invariants

1. **FP32 only:** the protocol never exposes compressed representations
2. **Structural typing:** any class with matching method signatures satisfies the protocol — no inheritance needed

## Relationships

| Direction | Class | Nature |
|-----------|-------|--------|
| Implemented by | [[classes/brute-force-backend\|BruteForceBackend]] | Reference implementation |
| Implemented by | [[classes/pgvector-adapter\|PgvectorAdapter]] | Wire-format adapter |
| Consumed by | Integration code | Receives decompressed vectors from [[classes/corpus\|Corpus]] |
| Returns | [[classes/search-result\|SearchResult]] | Ranked results |

## Test file

```text
tests/backend/test_brute_force.py  (via reference implementation)
```

## See also

- [[classes/search-result|SearchResult]]
- [[classes/brute-force-backend|BruteForceBackend]]
- [[storage-codec-vs-search-backend-separation]]
