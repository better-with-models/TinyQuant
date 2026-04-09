---
title: Codebook
tags:
  - classes
  - codec
  - value-object
date-created: 2026-04-08
status: active
category: design
---

# Codebook

> [!info] Responsibility
> Immutable lookup table mapping quantized indices to representative FP32
> values. Trained from sample data and frozen before use.

## Location

```text
src/tinyquant_cpu/codec/codebook.py
```

## Category

**Value object** — `@dataclass(frozen=True)`

## Fields

| Field | Type | Invariant |
|-------|------|-----------|
| `entries` | `NDArray[np.float32]` | Shape is `(num_entries,)`; ordered, non-duplicated |
| `bit_width` | `int` | Matches the originating `CodecConfig.bit_width` |

## Properties

| Property | Return type | Description |
|----------|-------------|-------------|
| `num_entries` | `int` | `len(self.entries)` — must equal `2 ** bit_width` |

## Class methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `train` | `(cls, vectors: NDArray[np.float32], config: CodecConfig) -> Codebook` | Train a codebook from representative vectors using the config's bit width. Returns a frozen Codebook. |

## Instance methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `quantize` | `(self, values: NDArray[np.float32]) -> NDArray[np.uint8]` | Map continuous values to nearest codebook indices |
| `dequantize` | `(self, indices: NDArray[np.uint8]) -> NDArray[np.float32]` | Map indices back to representative FP32 values |

## Invariants

1. `len(entries) == 2 ** bit_width` — exact count, validated at construction
2. Entries are sorted in ascending order — enables efficient nearest-neighbor lookup
3. No duplicate entries — each index maps to a unique representative value
4. Immutability — the underlying array must not be modified after construction

## Relationships

| Direction | Class | Nature |
|-----------|-------|--------|
| Produced by | `Codebook.train()` (class method) | Trained from sample vectors |
| Consumed by | [[classes/codec\|Codec]] | Used in `compress` (quantize) and `decompress` (dequantize) |
| Consumed by | [[classes/corpus\|Corpus]] | Frozen on the corpus alongside `CodecConfig` |
| Parameterized by | [[classes/codec-config\|CodecConfig]] | `bit_width` determines entry count |

## Example

```python
config = CodecConfig(bit_width=4, seed=42, dimension=768)
codebook = Codebook.train(training_vectors, config)
assert codebook.num_entries == 16

indices = codebook.quantize(rotated_values)
reconstructed = codebook.dequantize(indices)
```

## Test file

```text
tests/codec/test_codebook.py
```

## See also

- [[classes/codec-config|CodecConfig]]
- [[classes/codec|Codec]]
- [[classes/compressed-vector|CompressedVector]]
