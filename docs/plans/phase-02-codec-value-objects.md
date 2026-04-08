---
title: "Phase 2: Codec Value Objects"
tags:
  - plans
  - phase-2
  - codec
  - value-objects
date-created: 2026-04-08
status: pending
category: planning
---

# Phase 2: Codec Value Objects

> [!info] Goal
> Implement the four codec value objects and the shared type aliases. All
> are frozen dataclasses with full type annotations, docstrings, and tests.

## Prerequisites

- [[plans/phase-01-scaffolding|Phase 1]] complete (project installs, tooling passes)

## Deliverables

| File | Class | Spec |
|------|-------|------|
| `src/tinyquant/_types.py` | Type aliases | [[classes/shared-types\|Shared Types]] |
| `src/tinyquant/codec/codec_config.py` | `CodecConfig` | [[classes/codec-config\|CodecConfig]] |
| `src/tinyquant/codec/rotation_matrix.py` | `RotationMatrix` | [[classes/rotation-matrix\|RotationMatrix]] |
| `src/tinyquant/codec/codebook.py` | `Codebook` | [[classes/codebook\|Codebook]] |
| `src/tinyquant/codec/compressed_vector.py` | `CompressedVector` | [[classes/compressed-vector\|CompressedVector]] |
| `src/tinyquant/codec/__init__.py` | Public re-exports | [[design/architecture/namespace-and-module-structure\|Namespace Structure]] |
| `tests/codec/test_codec_config.py` | ~15 tests | [[qa/unit-tests/test-codec-config\|Test spec]] |
| `tests/codec/test_rotation_matrix.py` | ~12 tests | [[qa/unit-tests/test-rotation-matrix\|Test spec]] |
| `tests/codec/test_codebook.py` | ~14 tests | [[qa/unit-tests/test-codebook\|Test spec]] |
| `tests/codec/test_compressed_vector.py` | ~12 tests | [[qa/unit-tests/test-compressed-vector\|Test spec]] |
| `tests/conftest.py` | Shared fixtures | CodecConfig fixtures, sample vectors |

## Steps (TDD order)

### 1. Shared types

1. Populate `_types.py` with `VectorId`, `ConfigHash`, `CorpusId`, `Vector`, `VectorBatch`
2. No tests needed (validated transitively)

### 2. CodecConfig

1. Write `test_valid_config_creates_successfully` — RED
2. Implement `CodecConfig` frozen dataclass with `__post_init__` validation — GREEN
3. Write remaining validation tests (unsupported bit_width, negative seed, etc.) — RED → GREEN
4. Write `config_hash` and `num_codebook_entries` property tests — RED → GREEN
5. Write equality/hash tests — RED → GREEN
6. Write hypothesis property-based test — RED → GREEN
7. Refactor: extract `SUPPORTED_BIT_WIDTHS` constant

### 3. RotationMatrix

1. Write `test_from_config_produces_correct_shape` — RED
2. Implement `RotationMatrix.from_config()` using QR decomposition of seeded random matrix — GREEN
3. Write orthogonality tests — RED → GREEN
4. Write `apply` / `apply_inverse` tests — RED → GREEN
5. Write determinism and hypothesis round-trip tests — RED → GREEN

### 4. Codebook

1. Write `test_train_produces_correct_entry_count_4bit` — RED
2. Implement `Codebook.train()` using uniform quantile estimation — GREEN
3. Write `quantize` / `dequantize` tests — RED → GREEN
4. Write sorting, uniqueness, and immutability tests — RED → GREEN

### 5. CompressedVector

1. Write `test_valid_compressed_vector_creates` — RED
2. Implement `CompressedVector` frozen dataclass — GREEN
3. Write `has_residual`, `size_bytes` property tests — RED → GREEN
4. Write dimension mismatch and immutability tests — RED → GREEN
5. Stub `to_bytes` / `from_bytes` (raise `NotImplementedError`) — implemented in Phase 6

### 6. Update `__init__.py`

1. Add all public symbols to `codec/__init__.py` with `__all__`
2. Run `ruff check .` and `mypy .` — must pass

## Verification

```bash
ruff check . && ruff format --check .
mypy --strict .
pytest tests/codec/ -v --cov=tinyquant/codec --cov-fail-under=95
```

## Estimated scope

- ~6 source files modified/created
- ~4 test files created
- ~53 tests
- ~500 lines of source + ~600 lines of tests

## See also

- [[roadmap|Implementation Roadmap]]
- [[plans/phase-01-scaffolding|Phase 1]]
- [[plans/phase-03-codec-service|Phase 3: Codec Service]]
