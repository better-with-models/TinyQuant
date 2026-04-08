---
title: RotationMatrix
tags:
  - classes
  - codec
  - value-object
date-created: 2026-04-08
status: active
category: design
---

# RotationMatrix

> [!info] Responsibility
> Deterministically generated orthogonal matrix that preconditions vector
> coordinates before quantization, eliminating per-block normalization overhead.

## Location

```text
src/tinyquant/codec/rotation_matrix.py
```

## Category

**Value object** — `@dataclass(frozen=True)`

## Fields

| Field | Type | Invariant |
|-------|------|-----------|
| `matrix` | `NDArray[np.float64]` | Shape is `(dimension, dimension)`; orthogonal within FP tolerance |
| `seed` | `int` | The seed used to generate this matrix |
| `dimension` | `int` | Matrix row/column count; matches `CodecConfig.dimension` |

## Class methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `from_config` | `(cls, config: CodecConfig) -> RotationMatrix` | Factory that generates the matrix deterministically from `config.seed` and `config.dimension` |

## Instance methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `apply` | `(self, vector: NDArray[np.float32]) -> NDArray[np.float32]` | Rotate a vector: `R @ vector` |
| `apply_inverse` | `(self, rotated: NDArray[np.float32]) -> NDArray[np.float32]` | Inverse rotation: `R.T @ rotated` (valid because R is orthogonal) |
| `verify_orthogonality` | `(self, tol: float = 1e-6) -> bool` | Check that `R @ R.T ≈ I` within tolerance |

## Invariants

1. **Orthogonality:** `matrix @ matrix.T` equals the identity within floating-point tolerance
2. **Determinism:** `from_config` with the same seed and dimension always returns the same matrix
3. **Immutability:** the underlying NumPy array must not be modified after construction

## Relationships

| Direction | Class | Nature |
|-----------|-------|--------|
| Produced by | [[classes/codec\|Codec]] | Via `build_rotation(config)` |
| Consumed by | [[classes/codec\|Codec]] | Used in `compress` (apply) and `decompress` (apply_inverse) |
| Parameterized by | [[classes/codec-config\|CodecConfig]] | `seed` and `dimension` fields |

## Implementation notes

> [!tip] Generation algorithm
> The rotation matrix should be generated via QR decomposition of a random
> matrix seeded by `seed`. This produces a uniformly distributed orthogonal
> matrix (Haar measure) and is the approach used in the PolarQuant research
> lineage. See [[random-preconditioning-without-normalization-overhead]].

## Example

```python
config = CodecConfig(bit_width=4, seed=42, dimension=768)
rotation = RotationMatrix.from_config(config)
assert rotation.verify_orthogonality()

rotated = rotation.apply(vector)
recovered = rotation.apply_inverse(rotated)
np.testing.assert_allclose(vector, recovered, atol=1e-5)
```

## Test file

```text
tests/codec/test_rotation_matrix.py
```

## See also

- [[classes/codec-config|CodecConfig]]
- [[classes/codec|Codec]]
- [[random-preconditioning-without-normalization-overhead]]
