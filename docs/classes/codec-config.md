---
title: CodecConfig
tags:
  - classes
  - codec
  - value-object
date-created: 2026-04-08
status: active
category: design
---

# CodecConfig

> [!info] Responsibility
> Immutable configuration snapshot that fully determines codec behavior.
> Two configs with identical fields are interchangeable.

## Location

```text
src/tinyquant/codec/codec_config.py
```

## Category

**Value object** — `@dataclass(frozen=True)`

## Fields

| Field | Type | Default | Invariant |
|-------|------|---------|-----------|
| `bit_width` | `int` | — | Must be in `SUPPORTED_BIT_WIDTHS` (e.g. `{2, 4, 8}`) |
| `seed` | `int` | — | `>= 0`; controls rotation matrix generation |
| `dimension` | `int` | — | `> 0`; expected input vector dimensionality |
| `residual_enabled` | `bool` | `True` | Whether stage-2 residual correction is active |

## Properties

| Property | Return type | Description |
|----------|-------------|-------------|
| `num_codebook_entries` | `int` | `2 ** self.bit_width` — number of quantization levels |
| `config_hash` | `ConfigHash` | Deterministic hash of all fields; used for traceability |

## Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `__post_init__` | `(self) -> None` | Validates all invariants; raises `ValueError` on violation |
| `__eq__` | `(self, other: object) -> bool` | Structural equality (provided by frozen dataclass) |
| `__hash__` | `(self) -> int` | Hash based on all fields (provided by frozen dataclass) |

## Invariants

1. `bit_width` is one of the supported values — no arbitrary integers
2. `seed >= 0` — negative seeds are rejected at construction
3. `dimension > 0` — zero or negative dimensions are rejected
4. Immutability — fields cannot be modified after construction
5. `config_hash` is deterministic — same fields always produce the same hash

## Relationships

| Direction | Class | Nature |
|-----------|-------|--------|
| Consumed by | [[classes/codec\|Codec]] | Passed to `compress`, `decompress`, `build_codebook`, `build_rotation` |
| Consumed by | [[classes/corpus\|Corpus]] | Frozen on the corpus at creation time |
| Consumed by | [[classes/rotation-matrix\|RotationMatrix]] | `seed` and `dimension` determine the rotation |
| Consumed by | [[classes/codebook\|Codebook]] | `bit_width` determines entry count |
| Produces | [[classes/shared-types\|ConfigHash]] | Via `config_hash` property |

## Example

```python
config = CodecConfig(bit_width=4, seed=42, dimension=768)
assert config.num_codebook_entries == 16
assert config.config_hash  # deterministic string
```

## Test file

```text
tests/codec/test_codec_config.py
```

## See also

- [[classes/codec|Codec]]
- [[classes/compressed-vector|CompressedVector]]
- [[design/domain-layer/aggregates-and-entities|Aggregates and Entities]]
