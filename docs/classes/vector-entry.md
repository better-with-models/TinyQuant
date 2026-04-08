---
title: VectorEntry
tags:
  - classes
  - corpus
  - entity
date-created: 2026-04-08
status: active
category: design
---

# VectorEntry

> [!info] Responsibility
> Identity-bearing record of a single compressed vector within a corpus.
> Cannot exist outside its parent [[classes/corpus|Corpus]].

## Location

```text
src/tinyquant/corpus/vector_entry.py
```

## Category

**Entity** — has identity (`vector_id`) and lifecycle within the aggregate.

## Constructor

| Parameter | Type | Description |
|-----------|------|-------------|
| `vector_id` | `str` | Unique identity within the corpus |
| `compressed` | `CompressedVector` | The stored compressed representation |
| `inserted_at` | `datetime` | UTC timestamp of insertion |
| `metadata` | `dict[str, Any] \| None` | Optional per-vector metadata |

## Fields

| Field | Type | Mutability | Invariant |
|-------|------|-----------|-----------|
| `vector_id` | `str` | Immutable | Non-empty; unique within the corpus |
| `compressed` | `CompressedVector` | Immutable | Matches the corpus's `config_hash` |
| `inserted_at` | `datetime` | Immutable | UTC; set at insertion time |
| `metadata` | `dict[str, Any] \| None` | Mutable | Consumer-defined |

## Properties

| Property | Return type | Description |
|----------|-------------|-------------|
| `config_hash` | `ConfigHash` | Delegates to `self.compressed.config_hash` |
| `dimension` | `int` | Delegates to `self.compressed.dimension` |
| `has_residual` | `bool` | Delegates to `self.compressed.has_residual` |

## Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `__eq__` | `(self, other: object) -> bool` | Identity equality by `vector_id` |
| `__hash__` | `(self) -> int` | Hash by `vector_id` |

## Invariants

1. `vector_id` is non-empty and unique within its containing corpus
2. `compressed.config_hash` matches the parent corpus's config hash
3. `inserted_at` is timezone-aware UTC
4. Entity equality is by `vector_id`, not by field comparison

## Relationships

| Direction | Class | Nature |
|-----------|-------|--------|
| Owned by | [[classes/corpus\|Corpus]] | Cannot exist outside the aggregate |
| Contains | [[classes/compressed-vector\|CompressedVector]] | The stored representation |

## Example

```python
entry = VectorEntry(
    vector_id="vec-001",
    compressed=compressed_vector,
    inserted_at=datetime.now(UTC),
    metadata={"source": "batch-20260408"},
)
assert entry.config_hash == compressed_vector.config_hash
```

## Test file

```text
tests/corpus/test_vector_entry.py
```

## See also

- [[classes/corpus|Corpus]]
- [[classes/compressed-vector|CompressedVector]]
