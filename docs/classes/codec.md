---
title: Codec
tags:
  - classes
  - codec
  - domain-service
date-created: 2026-04-08
status: active
category: design
---

# Codec

> [!info] Responsibility
> Stateless domain service that performs vector compression and decompression.
> Owns the full pipeline: rotation, quantization, residual correction, and
> their inverses.

## Location

```text
src/tinyquant_cpu/codec/codec.py
```

## Category

**Domain service** — stateless class with no mutable instance state.
All behavior is determined by the `CodecConfig` and `Codebook` passed to
each method.

## Constructor

| Parameter | Type | Description |
|-----------|------|-------------|
| *(none)* | — | Codec is stateless; instantiation requires no arguments |

## Methods

### `compress`

```python
def compress(
    self,
    vector: NDArray[np.float32],
    config: CodecConfig,
    codebook: Codebook,
) -> CompressedVector:
```

**Purpose:** Compress an FP32 vector into a low-bit representation.

**Pipeline:**
1. Validate `len(vector) == config.dimension`
2. Apply rotation: `rotated = rotation.apply(vector)`
3. Quantize: `indices = codebook.quantize(rotated)`
4. If `config.residual_enabled`: compute and encode residual
5. Return `CompressedVector(indices, residual, config.config_hash, config.dimension, config.bit_width)`

**Preconditions:**
- `vector` has `config.dimension` elements
- `codebook.bit_width == config.bit_width`

**Postconditions:**
- Result indices all in `[0, 2^config.bit_width)`
- Result `config_hash` matches `config.config_hash`
- Deterministic: same inputs always produce identical output

**Raises:** `DimensionMismatchError`, `CodebookIncompatibleError`

---

### `decompress`

```python
def decompress(
    self,
    compressed: CompressedVector,
    config: CodecConfig,
    codebook: Codebook,
) -> NDArray[np.float32]:
```

**Purpose:** Reconstruct an FP32 vector from a compressed representation.

**Pipeline:**
1. Validate `compressed.config_hash == config.config_hash`
2. Dequantize: `values = codebook.dequantize(compressed.indices)`
3. If residual present: apply residual correction
4. Apply inverse rotation: `result = rotation.apply_inverse(values)`
5. Return FP32 vector

**Preconditions:**
- `compressed.config_hash` matches `config.config_hash`
- `codebook.bit_width == config.bit_width`

**Postconditions:**
- Result is an FP32 array of `config.dimension` elements
- Deterministic: same inputs always produce identical output

**Raises:** `ConfigMismatchError`, `CodebookIncompatibleError`

---

### `compress_batch`

```python
def compress_batch(
    self,
    vectors: NDArray[np.float32],
    config: CodecConfig,
    codebook: Codebook,
) -> list[CompressedVector]:
```

**Purpose:** Compress multiple vectors efficiently.

**Preconditions:** `vectors.shape == (n, config.dimension)` for some `n > 0`

**Postconditions:** Returns `n` CompressedVectors, each identical to what
`compress` would produce for the corresponding row.

---

### `decompress_batch`

```python
def decompress_batch(
    self,
    compressed: Sequence[CompressedVector],
    config: CodecConfig,
    codebook: Codebook,
) -> NDArray[np.float32]:
```

**Purpose:** Decompress multiple vectors efficiently.

**Postconditions:** Returns array of shape `(n, config.dimension)`, each row
identical to what `decompress` would produce for the corresponding input.

---

### `build_codebook`

```python
def build_codebook(
    self,
    training_vectors: NDArray[np.float32],
    config: CodecConfig,
) -> Codebook:
```

**Purpose:** Train a codebook from representative vectors.

**Postconditions:** Returned codebook has exactly `2^config.bit_width` entries.

**Raises:** `ValueError` if training set is too small.

---

### `build_rotation`

```python
def build_rotation(
    self,
    config: CodecConfig,
) -> RotationMatrix:
```

**Purpose:** Generate a rotation matrix from the config's seed and dimension.

**Postconditions:** Returned matrix is orthogonal and deterministic.

## Invariants

1. **Stateless:** no instance fields, no side effects, no hidden caches
2. **Deterministic:** identical inputs produce identical outputs
3. **Config-safe:** config hash mismatches are caught before producing output

## Relationships

| Direction | Class | Nature |
|-----------|-------|--------|
| Uses | [[classes/codec-config\|CodecConfig]] | Configuration for all operations |
| Uses | [[classes/codebook\|Codebook]] | Quantization/dequantization lookup |
| Uses | [[classes/rotation-matrix\|RotationMatrix]] | Preconditioning transform |
| Produces | [[classes/compressed-vector\|CompressedVector]] | Output of compression |
| Delegates to | [[classes/quantize-internal\|_quantize]] | Low-level quantization math |
| Called by | [[classes/corpus\|Corpus]] | For write-path compression and read-path decompression |

## Module-level convenience functions

The module also exposes top-level functions that delegate to a `Codec()`
instance for ergonomic one-shot use:

```python
def compress(vector, config, codebook) -> CompressedVector: ...
def decompress(compressed, config, codebook) -> NDArray[np.float32]: ...
```

## Test file

```text
tests/codec/test_codec.py
```

## See also

- [[classes/codec-config|CodecConfig]]
- [[classes/codebook|Codebook]]
- [[classes/rotation-matrix|RotationMatrix]]
- [[classes/compressed-vector|CompressedVector]]
- [[classes/quantize-internal|_quantize (internal)]]
