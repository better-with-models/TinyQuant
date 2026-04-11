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

## Rust port

> [!success] Rust parity landed 2026-04-10 (Phase 14)
> `tinyquant_core::codec::Codebook` is the byte-compatible Rust
> counterpart, living at
> `rust/crates/tinyquant-core/src/codec/codebook.rs`. Field-by-field
> mapping:

| Python field | Rust equivalent |
|--------------|-----------------|
| `entries: NDArray[np.float32]` | `entries: Arc<[f32]>` |
| `bit_width: int` | `bit_width: u8` |
| `num_entries` (property) | `Codebook::num_entries(&self) -> u32` |
| `Codebook.train(vectors, config)` | `Codebook::train(&[f32], &CodecConfig) -> Result<Codebook, CodecError>` |
| `codebook.quantize(values)` | `Codebook::quantize(&[f32]) -> Result<Vec<u8>, CodecError>` (alloc) and `Codebook::quantize_into(&[f32], &mut [u8])` (borrow) |
| `codebook.dequantize(indices)` | `Codebook::dequantize(&[u8]) -> Result<Vec<f32>, CodecError>` and `Codebook::dequantize_into(&[u8], &mut [f32])` |
| `ValueError("wrong count")` | `CodecError::CodebookEntryCount { expected, got, bit_width }` |
| `ValueError("insufficient training data")` | `CodecError::InsufficientTrainingData { expected }` |
| `ValueError("index out of range")` | `CodecError::IndexOutOfRange { index, bound }` |

The Rust version is `no_std` (`alloc` only), panic-free under the
crate's `clippy::pedantic + nursery + unwrap_used + indexing_slicing`
profile, and compares equal (`PartialEq`) by hashing the f32 bit
patterns rather than allocation identity. Training uses `libm::floor`
so the quantile arithmetic builds on bare-metal targets like
`thumbv7em-none-eabihf`. Construction validates entry count, strict
ascending order, and distinctness in a single non-indexing pass; all
three invariants are re-checked when `train` delegates to `new`.

Private scalar kernels
(`tinyquant_core::codec::quantize::scalar_quantize`,
`scalar_dequantize`) are the reference baseline Phase 20's SIMD
implementation will benchmark against. Ties in `quantize_into` favor
the right neighbor because the Python reference uses strict `<` on
`left_dist < right_dist`; this is preserved byte-for-byte.

See [[design/rust/phase-14-implementation-notes|Phase 14
Implementation Notes]] for the execution log and
[[design/rust/type-mapping#codebook|Type Mapping §Codebook]] for the
design-level type signature.

## See also

- [[classes/codec-config|CodecConfig]]
- [[classes/codec|Codec]]
- [[classes/compressed-vector|CompressedVector]]
- [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]]
