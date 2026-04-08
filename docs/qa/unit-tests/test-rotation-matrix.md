---
title: "Unit Tests: RotationMatrix"
tags:
  - qa
  - unit-tests
  - codec
date-created: 2026-04-08
status: active
category: qa
---

# Unit Tests: RotationMatrix

> [!info] Class under test
> [[classes/rotation-matrix|RotationMatrix]] — deterministic orthogonal transform.

## Test file

```text
tests/codec/test_rotation_matrix.py
```

## Test cases

### Factory construction

| Test | Asserts |
|------|---------|
| `test_from_config_produces_correct_shape` | Matrix shape is `(dim, dim)` |
| `test_from_config_is_deterministic` | Same seed + dimension → identical matrix |
| `test_different_seeds_produce_different_matrices` | seed 42 ≠ seed 99 |
| `test_different_dimensions_produce_different_matrices` | dim 128 ≠ dim 768 |

### Orthogonality

| Test | Asserts |
|------|---------|
| `test_matrix_is_orthogonal` | `R @ R.T ≈ I` within tolerance |
| `test_verify_orthogonality_returns_true` | `verify_orthogonality()` passes |
| `test_corrupted_matrix_fails_orthogonality` | Manually perturbed matrix fails check |

### Apply and inverse

| Test | Asserts |
|------|---------|
| `test_apply_changes_vector` | `apply(v) != v` (for non-trivial v) |
| `test_apply_inverse_recovers_original` | `apply_inverse(apply(v)) ≈ v` |
| `test_apply_preserves_norm` | `‖apply(v)‖ ≈ ‖v‖` (orthogonal transform preserves norm) |
| `test_apply_dimension_mismatch_raises` | Wrong-length vector raises error |

### Property-based (hypothesis)

| Test | Asserts |
|------|---------|
| `test_round_trip_preserves_vector` | For any valid vector, `apply_inverse(apply(v)) ≈ v` |

## See also

- [[classes/rotation-matrix|RotationMatrix]]
- [[random-preconditioning-without-normalization-overhead]]
