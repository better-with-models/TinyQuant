---
title: Type Safety
tags:
  - design
  - architecture
  - typing
  - mypy
date-created: 2026-04-08
status: active
category: design
---

# Type Safety

> [!info] Policy
> All TinyQuant code is checked by mypy in strict mode. Type annotations are
> not optional documentation — they are enforced contracts.

## Mypy configuration

```toml
# pyproject.toml (excerpt)

[tool.mypy]
python_version = "3.12"
strict = true
warn_return_any = true
warn_unused_configs = true
warn_unreachable = true
disallow_any_generics = true
disallow_untyped_defs = true
disallow_incomplete_defs = true
check_untyped_defs = true
no_implicit_optional = true
no_implicit_reexport = true
```

### What strict mode enforces

| Flag | Effect |
|------|--------|
| `disallow_untyped_defs` | Every function must have full type annotations |
| `disallow_any_generics` | Generic types must be parameterized (`list[int]`, not `list`) |
| `no_implicit_optional` | `None` must be explicitly included in union types |
| `warn_return_any` | Functions that return `Any` are flagged |
| `warn_unreachable` | Dead code after type narrowing is flagged |
| `no_implicit_reexport` | Only explicitly re-exported symbols are visible from `__init__.py` |

## Typing strategy by layer

### Codec layer

- All function signatures fully annotated including return types
- NumPy arrays typed with `NDArray[np.float32]` or `NDArray[np.int8]`
- Value objects use `@dataclass(frozen=True)` with typed fields
- No `Any` anywhere in the codec layer

### Corpus layer

- Aggregate methods fully annotated
- Metadata typed as `dict[str, Any]` — the one permitted `Any` usage,
  because metadata is consumer-defined
- Event payloads typed with `TypedDict` or frozen dataclasses

### Backend layer

- `Protocol` classes define typed method signatures
- Adapter implementations must satisfy the protocol — mypy verifies this
  structurally
- Return types use domain-specific types, not raw dicts

## Protocol-based contracts

```python
from typing import Protocol

class SearchBackend(Protocol):
    """Contract for external search system integration."""

    def search(
        self,
        query: NDArray[np.float32],
        top_k: int,
    ) -> list[SearchResult]:
        """Return ranked results for a query vector."""
        ...

    def ingest(
        self,
        vectors: dict[str, NDArray[np.float32]],
    ) -> None:
        """Accept decompressed vectors for indexing."""
        ...
```

Mypy verifies that any class passed as a `SearchBackend` satisfies this
contract structurally — no inheritance required.

## Type suppression rules

> [!warning] Every suppression must be justified
> See [[architecture/linting-and-tooling|Linting and Tooling]] for the
> suppression comment policy.

Permitted `# type: ignore` cases:

| Case | Example |
|------|---------|
| NumPy overload gaps in stubs | `# type: ignore[arg-type] — numpy stubs lack this overload` |
| Third-party library with no stubs | `# type: ignore[import-untyped] — no stubs for library X` |
| Dynamic plugin loading | Must be isolated in a factory module with an explanatory comment |

Not permitted:

- `# type: ignore` without a bracket code and explanation
- Suppressing `disallow_untyped_defs` for convenience
- Using `cast()` to hide a real type error

## Benefits for TinyQuant specifically

| Benefit | How typing delivers it |
|---------|----------------------|
| **Config hash safety** | `config_hash: str` on CompressedVector prevents accidental int/bytes confusion |
| **Dimension enforcement** | Type-compatible but semantically wrong dimensions are caught by runtime validation, but typed APIs make the contract visible |
| **Protocol compliance** | mypy verifies backend adapters satisfy SearchBackend without manual testing |
| **Refactoring confidence** | Renaming a field or changing a return type immediately shows all affected call sites |
| **IDE support** | Full type information enables autocompletion, hover docs, and navigation |

## See also

- [[architecture/linting-and-tooling|Linting and Tooling]]
- [[architecture/solid-principles|SOLID Principles]]
- [[architecture/docstring-policy|Docstring Policy]]
- [[domain-layer/aggregates-and-entities|Aggregates and Entities]]
