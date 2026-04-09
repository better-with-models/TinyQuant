---
title: CompressionPolicy
tags:
  - classes
  - corpus
  - value-object
  - enum
date-created: 2026-04-08
status: active
category: design
---

# CompressionPolicy

> [!info] Responsibility
> Enum defining the write-path behavior for a corpus. Determines how incoming
> vectors are stored: full codec compression, FP16 downcast, or FP32
> passthrough.

## Location

```text
src/tinyquant_cpu/corpus/compression_policy.py
```

## Category

**Value object (Enum)** — `enum.Enum` subclass. Immutable by nature.

## Members

| Member | Value | Behavior |
|--------|-------|----------|
| `COMPRESS` | `"compress"` | Full codec pipeline: rotation, quantization, residual correction |
| `PASSTHROUGH` | `"passthrough"` | Store FP32 vectors without any transformation |
| `FP16` | `"fp16"` | Downcast to FP16 without codec quantization |

## Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `requires_codec` | `(self) -> bool` | Returns `True` only for `COMPRESS` |
| `storage_dtype` | `(self) -> np.dtype` | Returns `np.float32` for passthrough, `np.float16` for fp16, `np.uint8` for compress |

## Invariants

1. A corpus's policy cannot change after the first vector is inserted
2. The policy governs all vectors in the corpus uniformly — no per-vector overrides

## Relationships

| Direction | Class | Nature |
|-----------|-------|--------|
| Consumed by | [[classes/corpus\|Corpus]] | Determines write-path behavior in `insert` and `insert_batch` |
| Referenced by | [[classes/corpus-events\|CorpusCreated]] | Recorded in the creation event |

## Example

```python
policy = CompressionPolicy.COMPRESS
assert policy.requires_codec() is True

policy = CompressionPolicy.PASSTHROUGH
assert policy.requires_codec() is False
assert policy.storage_dtype == np.float32
```

## Test file

```text
tests/corpus/test_compression_policy.py
```

## See also

- [[classes/corpus|Corpus]]
- [[per-collection-compression-policy]]
