---
title: "Phase 6: Serialization"
tags:
  - plans
  - phase-6
  - serialization
date-created: 2026-04-08
status: pending
category: planning
---

# Phase 6: Serialization

> [!info] Goal
> Implement `CompressedVector.to_bytes()` and `from_bytes()` with a versioned
> binary format, plus integration tests for serialization round trips.

## Prerequisites

- [[plans/phase-05-backend-layer|Phase 5]] complete (all three contexts functional)

## Deliverables

| File | What | Spec |
|------|------|------|
| `src/tinyquant/codec/compressed_vector.py` | `to_bytes`, `from_bytes` (replace stubs) | [[classes/compressed-vector\|CompressedVector]] |
| `tests/codec/test_compressed_vector.py` | Add serialization tests (~4 new) | [[qa/unit-tests/test-compressed-vector\|Test spec]] |
| `tests/integration/test_serialization.py` | ~7 integration tests | [[qa/integration-plan/README\|Integration Plan EB-01]] |

## Steps (TDD order)

### 1. Define binary format

Format (documented in class spec):
1. Version byte (`0x01`)
2. Config hash (32 bytes, UTF-8 padded)
3. Dimension (uint32, little-endian)
4. Bit width (uint8)
5. Packed indices (bit-packed at `bit_width` bits per index)
6. Residual flag (uint8: 0 or 1)
7. If residual present: residual length (uint32) + residual bytes

### 2. to_bytes

1. Write `test_to_bytes_produces_bytes` — RED
2. Implement `to_bytes()` using `struct` module — GREEN
3. Write `test_serialization_includes_config_hash` — RED → GREEN

### 3. from_bytes

1. Write `test_from_bytes_round_trip` — RED
2. Implement `from_bytes()` — GREEN
3. Write `test_from_bytes_corrupted_data_raises` — RED → GREEN

### 4. Integration tests

1. Write `test_serialize_deserialize_round_trip` (full pipeline: compress → serialize → deserialize → decompress) — RED → GREEN
2. Write `test_deserialization_rejects_wrong_version` — RED → GREEN
3. Write `test_deserialization_rejects_truncated_data` — RED → GREEN
4. Write `test_large_dimension_serialization` (dim=3072) — RED → GREEN
5. Write `test_residual_survives_serialization` — RED → GREEN

## Verification

```bash
ruff check . && ruff format --check .
mypy --strict .
pytest tests/codec/test_compressed_vector.py tests/integration/test_serialization.py -v
pytest  # full regression
```

## Estimated scope

- ~1 source file modified
- ~1 test file created, 1 modified
- ~11 tests (4 unit + 7 integration)
- ~150 lines of source + ~200 lines of tests

## See also

- [[roadmap|Implementation Roadmap]]
- [[plans/phase-05-backend-layer|Phase 5]]
- [[plans/phase-07-architecture-e2e-tests|Phase 7: Architecture & E2E Tests]]
