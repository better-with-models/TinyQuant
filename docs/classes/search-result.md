---
title: SearchResult
tags:
  - classes
  - backend
  - value-object
date-created: 2026-04-08
status: active
category: design
---

# SearchResult

> [!info] Responsibility
> Immutable value object representing a single ranked result from a search
> backend query.

## Location

```text
src/tinyquant/backend/protocol.py
```

(Co-located with [[classes/search-backend|SearchBackend]] since it is part of
the protocol contract.)

## Category

**Value object** — `@dataclass(frozen=True)`

## Fields

| Field | Type | Invariant |
|-------|------|-----------|
| `vector_id` | `str` | Non-empty; matches an ID previously ingested |
| `score` | `float` | Similarity score (higher is more similar) |

## Properties

| Property | Return type | Description |
|----------|-------------|-------------|
| *(none beyond fields)* | | Deliberately minimal |

## Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `__lt__` | `(self, other: SearchResult) -> bool` | Order by score descending (for sorting) |

## Invariants

1. `vector_id` is non-empty
2. `score` is a finite float (not NaN or inf)
3. Results are comparable by score for ranking

## Relationships

| Direction | Class | Nature |
|-----------|-------|--------|
| Returned by | [[classes/search-backend\|SearchBackend]] | `search()` returns a list of these |
| Returned by | [[classes/brute-force-backend\|BruteForceBackend]] | Concrete search results |

## Example

```python
result = SearchResult(vector_id="vec-042", score=0.95)
results = sorted([r1, r2, r3])  # ranked by score
```

## Test file

```text
tests/backend/test_brute_force.py  (tested via search result verification)
```

## See also

- [[classes/search-backend|SearchBackend]]
