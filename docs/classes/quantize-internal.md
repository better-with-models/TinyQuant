---
title: _quantize (internal)
tags:
  - classes
  - codec
  - internal
date-created: 2026-04-08
status: active
category: design
---

# _quantize (internal)

> [!info] Responsibility
> Private module containing low-level quantization helper functions.
> Not part of the public API. May be restructured without notice.

## Location

```text
src/tinyquant/codec/_quantize.py
```

## Category

**Private module** — underscore prefix signals internal-only usage.

## Planned functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `scalar_quantize` | `(values: NDArray[np.float32], entries: NDArray[np.float32]) -> NDArray[np.uint8]` | Map each value to the nearest codebook entry index |
| `scalar_dequantize` | `(indices: NDArray[np.uint8], entries: NDArray[np.float32]) -> NDArray[np.float32]` | Map indices back to codebook representative values |
| `compute_residual` | `(original: NDArray[np.float32], reconstructed: NDArray[np.float32]) -> bytes` | Encode the difference between original and stage-1 reconstruction using lightweight projection |
| `apply_residual` | `(reconstructed: NDArray[np.float32], residual: bytes) -> NDArray[np.float32]` | Apply residual correction to improve reconstruction fidelity |

## Design constraints

- All functions are **pure** — no side effects, no hidden state
- NumPy operations only — no external dependencies beyond numpy
- Each function should have CC ≤ 7
- Functions are tested indirectly through [[classes/codec\|Codec]] tests, but
  may also have focused unit tests for edge cases

## Why this is private

These functions are implementation details of the codec pipeline. The
public API is `Codec.compress` and `Codec.decompress`. If the internal
quantization strategy changes (e.g. switching from nearest-neighbor to
a learned assignment), consumers should not be affected.

## Relationships

| Direction | Class | Nature |
|-----------|-------|--------|
| Called by | [[classes/codec\|Codec]] | `compress` and `decompress` delegate here |
| Uses | [[classes/codebook\|Codebook]] | Entries array is passed as a parameter |

## Test file

```text
tests/codec/test_codec.py  (indirectly)
```

## See also

- [[classes/codec|Codec]]
- [[classes/codebook|Codebook]]
- [[two-stage-quantization-and-residual-correction]]
