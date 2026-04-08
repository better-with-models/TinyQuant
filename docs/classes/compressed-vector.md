---
title: CompressedVector
tags:
  - classes
  - codec
  - value-object
date-created: 2026-04-08
status: active
category: design
---

# CompressedVector

> [!info] Responsibility
> Immutable output of a codec compression pass. Carries quantized indices,
> optional residual data, and a config hash linking it to the originating
> configuration.

## Location

```text
src/tinyquant/codec/compressed_vector.py
```

## Category

**Value object** — `@dataclass(frozen=True)`

## Fields

| Field | Type | Invariant |
|-------|------|-----------|
| `indices` | `NDArray[np.uint8]` | One index per dimension; each in `[0, 2^bit_width)` |
| `residual` | `bytes \| None` | Non-empty when `residual_enabled` was true; `None` otherwise |
| `config_hash` | `ConfigHash` | Fingerprint of the `CodecConfig` that produced this vector |
| `dimension` | `int` | `len(indices)` — stored explicitly for fast validation |
| `bit_width` | `int` | Quantization bit width; must be one of {2, 4, 8} |

## Properties

| Property | Return type | Description |
|----------|-------------|-------------|
| `has_residual` | `bool` | `self.residual is not None` |
| `size_bytes` | `int` | Approximate storage footprint of this compressed vector |

## Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `__post_init__` | `(self) -> None` | Validates `len(indices) == dimension`; validates index range if bit_width context is available |
| `to_bytes` | `(self) -> bytes` | Serialize to a compact binary representation |
| `from_bytes` | `(cls, data: bytes) -> CompressedVector` | Deserialize from binary representation |

## Invariants

1. `len(indices) == dimension` — validated at construction
2. Every index is in `[0, 2^bit_width)` — validated when bit_width context is available
3. `residual` is `None` if and only if residual correction was disabled
4. `config_hash` matches the `CodecConfig` used during compression
5. Immutability — fields cannot be modified after construction

## Relationships

| Direction | Class | Nature |
|-----------|-------|--------|
| Produced by | [[classes/codec\|Codec]] | Output of `compress()` |
| Consumed by | [[classes/codec\|Codec]] | Input to `decompress()` |
| Consumed by | [[classes/vector-entry\|VectorEntry]] | Stored as the `compressed` field |
| Traced to | [[classes/codec-config\|CodecConfig]] | Via `config_hash` |

## Serialization

The `to_bytes` / `from_bytes` pair provides a compact binary format for
persistent storage. The format includes:

1. A version byte (for future format evolution)
2. The `config_hash` (fixed-length)
3. The `dimension` (uint32)
4. Packed `indices` (bit-packed at the original bit width)
5. The `residual` length + data (if present)

> [!warning] Format stability
> The binary format is part of the public contract. Changes must be
> backwards-compatible or versioned.

## Example

```python
cv = CompressedVector(
    indices=np.array([3, 7, 1, 15], dtype=np.uint8),
    residual=None,
    config_hash="abc123",
    dimension=4,
    bit_width=4,
)
assert not cv.has_residual
raw = cv.to_bytes()
restored = CompressedVector.from_bytes(raw)
assert restored == cv
```

## Test file

```text
tests/codec/test_compressed_vector.py
```

## See also

- [[classes/codec|Codec]]
- [[classes/codec-config|CodecConfig]]
- [[classes/vector-entry|VectorEntry]]
