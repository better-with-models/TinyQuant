---
title: Shared Types
tags:
  - classes
  - shared
  - types
date-created: 2026-04-08
status: active
category: design
---

# Shared Types

> [!info] Responsibility
> Central type aliases used across TinyQuant packages. Provides semantic
> naming for common types to improve readability and catch misuse through
> mypy.

## Location

```text
src/tinyquant_cpu/_types.py
```

## Category

**Private module** — shared type aliases. Not part of the public API but
imported by internal modules across packages.

## Type aliases

| Alias | Underlying type | Purpose |
|-------|----------------|---------|
| `VectorId` | `str` | Semantic alias for vector identity strings |
| `ConfigHash` | `str` | Deterministic hash identifying a `CodecConfig` |
| `CorpusId` | `str` | Semantic alias for corpus identity strings |
| `Vector` | `NDArray[np.float32]` | 1-D FP32 embedding array |
| `VectorBatch` | `NDArray[np.float32]` | 2-D FP32 array of shape `(n, dimension)` |

## Design rationale

- **Semantic clarity:** `VectorId` communicates intent better than bare `str`
- **Grep-friendly:** searching for `VectorId` finds all vector-ID usage sites
- **Refactoring target:** if `VectorId` later becomes a `NewType`, mypy will
  immediately flag all misuse
- **Cross-package:** aliases live in `_types.py` because they are used by
  codec, corpus, and backend packages

## NewType consideration

> [!tip] Future hardening
> These start as simple aliases (`VectorId = str`) for ergonomic adoption.
> If stronger guarantees are needed, they can be promoted to
> `NewType("VectorId", str)` which makes mypy reject bare `str` at typed
> boundaries without runtime cost.

## Relationships

| Consumed by | Usage |
|-------------|-------|
| [[classes/codec-config\|CodecConfig]] | `ConfigHash` for `config_hash` property |
| [[classes/compressed-vector\|CompressedVector]] | `ConfigHash`, `VectorId` references |
| [[classes/corpus\|Corpus]] | `CorpusId`, `VectorId` |
| [[classes/vector-entry\|VectorEntry]] | `VectorId` |
| [[classes/search-result\|SearchResult]] | `VectorId` |

## Example

```python
from tinyquant_cpu._types import VectorId, ConfigHash, Vector

def compress(vector: Vector, ...) -> CompressedVector: ...
def get(self, vector_id: VectorId) -> VectorEntry: ...
```

## Test file

No dedicated test file — validated transitively through all consumer tests
and mypy strict mode.

## See also

- [[design/architecture/type-safety|Type Safety]]
- [[design/architecture/namespace-and-module-structure|Namespace and Module Structure]]
