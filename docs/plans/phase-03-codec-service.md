---
title: "Phase 3: Codec Service"
tags:
  - plans
  - phase-3
  - codec
  - service
date-created: 2026-04-08
status: complete
date-completed: 2026-04-08
category: planning
---

# Phase 3: Codec Service

> [!info] Goal
> Implement the `Codec` domain service and private `_quantize` helpers.
> This completes the codec bounded context — compress and decompress work
> end-to-end.

## Prerequisites

- [[plans/phase-02-codec-value-objects|Phase 2]] complete (all value objects pass tests)

## Deliverables

| File | Class/Module | Spec |
|------|-------------|------|
| `src/tinyquant_cpu/codec/_quantize.py` | Private helpers | [[classes/quantize-internal\|_quantize]] |
| `src/tinyquant_cpu/codec/codec.py` | `Codec` service + module functions | [[classes/codec\|Codec]] |
| `src/tinyquant_cpu/codec/__init__.py` | Add `Codec`, `compress`, `decompress` | — |
| `tests/codec/test_codec.py` | ~25 tests | [[qa/unit-tests/test-codec\|Test spec]] |
| `tests/conftest.py` | Add trained_codebook, sample_vector fixtures | — |

## Steps (TDD order)

### 1. Private quantization helpers

1. Write `_quantize.py` with `scalar_quantize`, `scalar_dequantize`
2. These are tested indirectly through Codec tests, but add a few focused edge-case tests if needed

### 2. Codec.compress (stage 1 only, no residuals)

1. Write `test_compress_returns_compressed_vector` — RED
2. Implement `Codec.compress()` pipeline: validate → rotate → quantize → wrap — GREEN
3. Write `test_compress_indices_in_valid_range` — RED → GREEN
4. Write `test_compress_config_hash_matches` — RED → GREEN
5. Write `test_compress_deterministic` — RED → GREEN
6. Write `test_compress_dimension_mismatch_raises` — RED → GREEN

### 3. Codec.decompress (stage 1 only)

1. Write `test_decompress_returns_fp32_array` — RED
2. Implement `Codec.decompress()`: validate → dequantize → inverse rotate — GREEN
3. Write `test_decompress_config_mismatch_raises` — RED → GREEN
4. Write `test_compress_decompress_round_trip_recovers_shape` — RED → GREEN

### 4. Residual correction

1. Implement `compute_residual` and `apply_residual` in `_quantize.py`
2. Write `test_compress_with_residual` — RED → GREEN
3. Write `test_round_trip_with_residual_has_lower_error` — RED → GREEN

### 5. Batch operations

1. Write `test_compress_batch_count_matches` — RED
2. Implement `compress_batch` / `decompress_batch` — GREEN
3. Write `test_batch_matches_individual` — RED → GREEN

### 6. Build helpers

1. Write `test_build_codebook_entry_count` — RED → GREEN (delegates to `Codebook.train`)
2. Write `test_build_rotation_is_orthogonal` — RED → GREEN (delegates to `RotationMatrix.from_config`)

### 7. Module-level convenience functions

1. Add `compress()` and `decompress()` as module-level functions
2. Update `codec/__init__.py` exports

### 8. Property-based tests

1. Write hypothesis test for determinism
2. Write hypothesis test for batch/individual equivalence

## Verification

```bash
ruff check . && ruff format --check .
mypy --strict .
pytest tests/codec/ -v --cov=tinyquant_cpu/codec --cov-fail-under=95
```

## Estimated scope

- ~2 source files created, 1 modified
- ~1 test file created, 1 fixture file updated
- ~25 tests
- ~300 lines of source + ~400 lines of tests

## See also

- [[roadmap|Implementation Roadmap]]
- [[plans/phase-02-codec-value-objects|Phase 2]]
- [[plans/phase-04-corpus-layer|Phase 4: Corpus Layer]]
