---
title: "Phase 14: Codebook and Quantize Kernels"
tags:
  - plans
  - rust
  - phase-14
  - codebook
  - quantize
date-created: 2026-04-10
date-completed: 2026-04-10
status: complete
category: planning
---

> [!success] Status: complete (2026-04-10)
> Landed on the `phase-14-codebook-quantize` branch with byte parity
> across all three supported bit widths. Full execution notes in
> [[design/rust/phase-14-implementation-notes|Phase 14 Implementation
> Notes]]. Fixture sweep was expanded from `bw=4` only (in this
> original draft) to the full `{2, 4, 8}` set during execution.


# Phase 14: Codebook and Quantize Kernels

> [!info] Goal
> Implement `Codebook` (value object, `train`, `quantize_into`,
> `dequantize_into`) and the private `scalar_quantize` /
> `scalar_dequantize` helpers with byte-level parity against the
> Python reference.

> [!note] Reference docs
> - [[design/rust/type-mapping|Type Mapping]] §Codebook
> - [[design/rust/numerical-semantics|Numerical Semantics]] §Quantization, §Codebook training
> - [[design/rust/memory-layout|Memory Layout]] §Codebook layout

## Prerequisites

- Phase 13 complete (`CodecConfig`, rotation primitives in place).

## Deliverables

### Files to create

| File | Purpose |
|------|---------|
| `rust/crates/tinyquant-core/src/codec/codebook.rs` | `Codebook` |
| `rust/crates/tinyquant-core/src/codec/quantize.rs` | Private scalar helpers |
| `rust/crates/tinyquant-core/tests/codebook.rs` | Unit tests |
| `rust/crates/tinyquant-core/tests/quantize.rs` | Unit tests for helpers |
| `rust/crates/tinyquant-core/tests/fixtures/codebook/training_n10000_d64.f32.bin` | Training data |
| `rust/crates/tinyquant-core/tests/fixtures/codebook/expected_bw4_seed42.f32.bin` | Python-generated codebook |

## Steps (TDD order)

- [ ] **Step 1: Write failing `tests/codebook.rs` — construction invariants**

```rust
use tinyquant_core::codec::codebook::Codebook;
use tinyquant_core::errors::CodecError;

#[test]
fn new_rejects_wrong_entry_count_for_bit_width_4() {
    let entries = vec![0.0_f32; 10].into_boxed_slice();
    let err = Codebook::new(entries, 4).unwrap_err();
    assert!(matches!(
        err,
        CodecError::CodebookEntryCount { expected: 16, got: 10, bit_width: 4 }
    ));
}

#[test]
fn new_rejects_unsorted_entries() {
    let mut e = (0..16).map(|i| i as f32).collect::<Vec<_>>();
    e.swap(0, 1);
    let err = Codebook::new(e.into_boxed_slice(), 4).unwrap_err();
    assert!(matches!(err, CodecError::CodebookNotSorted));
}

#[test]
fn new_rejects_duplicate_adjacent_entries() {
    let mut e = (0..16).map(|i| i as f32).collect::<Vec<_>>();
    e[5] = e[4];
    let err = Codebook::new(e.into_boxed_slice(), 4).unwrap_err();
    assert!(matches!(err, CodecError::CodebookDuplicate { .. }));
}

#[test]
fn new_accepts_sorted_distinct_entries() {
    let e: Vec<f32> = (0..16).map(|i| i as f32).collect();
    let cb = Codebook::new(e.into_boxed_slice(), 4).unwrap();
    assert_eq!(cb.num_entries(), 16);
    assert_eq!(cb.bit_width(), 4);
}
```

- [ ] **Step 2: Run — expect failure** (module does not exist).

- [ ] **Step 3: Scaffold `codebook.rs`**

Add stub `Codebook::new`, `num_entries`, `bit_width`. Implement
validation invariants to make the first four tests pass.

- [ ] **Step 4: Run tests — expect pass on construction tests.**

- [ ] **Step 5: Write failing training parity test**

```rust
#[test]
fn train_matches_python_fixture_bw4_seed42_n10000_d64() {
    let training_bytes = include_bytes!("fixtures/codebook/training_n10000_d64.f32.bin");
    let training: &[f32] = bytemuck::cast_slice(training_bytes);
    let config = CodecConfig::new(4, 42, 64, true).unwrap();

    let cb = Codebook::train(training, 10_000, 64, &config).unwrap();

    let expected_bytes = include_bytes!("fixtures/codebook/expected_bw4_seed42.f32.bin");
    let expected: &[f32] = bytemuck::cast_slice(expected_bytes);

    assert_eq!(cb.entries().len(), expected.len());
    for (got, exp) in cb.entries().iter().zip(expected.iter()) {
        assert_eq!(got.to_bits(), exp.to_bits(), "got={got}, exp={exp}");
    }
}
```

- [ ] **Step 6: Capture the fixtures from Python**

```bash
cargo xtask fixtures refresh-codebook --seed 42 --bit-width 4 --rows 10000 --cols 64
```

Produces the two fixture files.

- [ ] **Step 7: Implement `Codebook::train`**

Algorithm (matches Python exactly):

1. Promote input to f64.
2. Sort the flattened buffer with `f64::total_cmp`.
3. For each `q_k = k / (num_entries - 1)`, compute
   `h = q_k * (len - 1)`, `i = floor(h)`, `frac = h - i`, and
   `value = sorted[i] + frac * (sorted[i + 1] - sorted[i])` (clamp
   `i + 1` to `len - 1`).
4. Cast to f32.
5. Sort ascending (no-op if already sorted).
6. Assert distinct neighbors; return `InsufficientTrainingData`
   otherwise.

- [ ] **Step 8: Run training test — expect pass.**

- [ ] **Step 9: Write failing quantize round-trip test**

```rust
#[test]
fn quantize_then_dequantize_returns_nearest_entries() {
    let cb = Codebook::new(
        (0..16).map(|i| i as f32 * 0.1).collect::<Vec<_>>().into_boxed_slice(),
        4,
    ).unwrap();
    let values = vec![0.03, 0.17, 0.85, 1.49];
    let mut idx = vec![0u8; 4];
    cb.quantize_into(&values, &mut idx).unwrap();
    assert_eq!(idx, vec![0, 2, 8, 15]); // Nearest entries
    let mut out = vec![0.0f32; 4];
    cb.dequantize_into(&idx, &mut out).unwrap();
    assert_eq!(out, vec![0.0, 0.2, 0.8, 1.5]);
}

#[test]
fn quantize_exact_entry_values_produces_matching_indices() {
    let cb = Codebook::new(
        (0..16).map(|i| i as f32).collect::<Vec<_>>().into_boxed_slice(),
        4,
    ).unwrap();
    let mut idx = vec![0u8; 16];
    cb.quantize_into(cb.entries(), &mut idx).unwrap();
    assert_eq!(idx, (0..16).collect::<Vec<_>>());
}
```

- [ ] **Step 10: Run — expect failure.**

- [ ] **Step 11: Implement `quantize_into` and `dequantize_into`**

```rust
pub fn quantize_into(&self, values: &[f32], indices: &mut [u8]) -> Result<(), CodecError> {
    if values.len() != indices.len() {
        return Err(CodecError::LengthMismatch { left: values.len(), right: indices.len() });
    }
    let entries = &self.entries;
    let n = entries.len();
    for (i, &v) in values.iter().enumerate() {
        let pos = entries.binary_search_by(|e| e.partial_cmp(&v).unwrap_or(core::cmp::Ordering::Less));
        let idx = match pos {
            Ok(p) => p,
            Err(p) => p.min(n - 1),
        };
        let left = idx.saturating_sub(1);
        let right = idx.min(n - 1);
        let ld = (v - entries[left]).abs();
        let rd = (v - entries[right]).abs();
        // Python uses strict `<`, which favors the right neighbor on ties.
        let chosen = if ld < rd { left } else { right };
        indices[i] = chosen as u8;
    }
    Ok(())
}

pub fn dequantize_into(&self, indices: &[u8], values: &mut [f32]) -> Result<(), CodecError> {
    if indices.len() != values.len() {
        return Err(CodecError::LengthMismatch { left: indices.len(), right: values.len() });
    }
    let bound = self.entries.len() as u32;
    for (i, &idx) in indices.iter().enumerate() {
        if u32::from(idx) >= bound {
            return Err(CodecError::IndexOutOfRange { index: idx, bound });
        }
        values[i] = self.entries[idx as usize];
    }
    Ok(())
}
```

- [ ] **Step 12: Private scalar helpers in `quantize.rs`**

Provide `pub(crate) fn scalar_quantize(...)` and
`pub(crate) fn scalar_dequantize(...)` that call the same logic
independently of `Codebook`. These are what the SIMD path (phase 20)
will benchmark against as the scalar reference.

Unit tests in `tests/quantize.rs` assert that
`scalar_quantize(v, entries)` equals `Codebook::quantize_into` on
equivalent inputs.

- [ ] **Step 13: Byte-level parity against Python for `quantize`**

Fixture: 10 000 random f32 values produced by Python's
`np.random.default_rng(7).standard_normal(10000).astype(np.float32)`
and the same codebook loaded from the fixture file above. Python
writes the expected `u8` indices to a `.bin` fixture. The Rust test
reads both and asserts byte equality.

```rust
#[test]
fn quantize_matches_python_fixture_bw4_seed42() {
    let values: &[f32] = bytemuck::cast_slice(include_bytes!("fixtures/quantize/values_n10000.f32.bin"));
    let expected: &[u8] = include_bytes!("fixtures/quantize/expected_bw4_seed42.u8.bin");
    let cb_entries: &[f32] = bytemuck::cast_slice(include_bytes!("fixtures/codebook/expected_bw4_seed42.f32.bin"));
    let cb = Codebook::new(cb_entries.to_vec().into_boxed_slice(), 4).unwrap();
    let mut out = vec![0u8; values.len()];
    cb.quantize_into(values, &mut out).unwrap();
    assert_eq!(out, expected);
}
```

- [ ] **Step 14: Capture the expected fixtures.**

```bash
cargo xtask fixtures refresh-quantize --seed 7 --count 10000 --bit-width 4
```

- [ ] **Step 15: Run tests — expect pass.**

- [ ] **Step 16: Proptest — quantize indices are in range**

```rust
proptest! {
    #[test]
    fn quantize_indices_always_in_codebook(
        values in prop::collection::vec(any::<f32>().prop_filter("finite", |x| x.is_finite()), 1..1024),
    ) {
        let entries = (0..16).map(|i| i as f32).collect::<Vec<_>>().into_boxed_slice();
        let cb = Codebook::new(entries, 4).unwrap();
        let mut idx = vec![0u8; values.len()];
        cb.quantize_into(&values, &mut idx).unwrap();
        prop_assert!(idx.iter().all(|&i| i < 16));
    }
}
```

- [ ] **Step 17: Dequantize out-of-range index rejection**

```rust
#[test]
fn dequantize_rejects_index_ge_num_entries() {
    let cb = Codebook::new((0..16).map(|i| i as f32).collect::<Vec<_>>().into_boxed_slice(), 4).unwrap();
    let idx = vec![0u8, 5, 20];
    let mut out = vec![0.0f32; 3];
    let err = cb.dequantize_into(&idx, &mut out).unwrap_err();
    assert!(matches!(err, CodecError::IndexOutOfRange { index: 20, bound: 16 }));
}
```

- [ ] **Step 18: Run full workspace test suite, clippy, fmt.**

- [ ] **Step 19: Commit**

```bash
git add rust/crates/tinyquant-core
git commit -m "feat(tinyquant-core): add Codebook and scalar quantize kernels"
```

## Acceptance criteria

- Codebook construction enforces Python-parity invariants.
- `Codebook::train` produces byte-identical entries for the fixture
  `(seed=42, bit_width=4, n=10000, d=64)`.
- `quantize_into` matches Python indices byte-for-byte on the fixture.
- `dequantize_into` round-trips exact codebook values.
- Proptest confirms indices always land in `[0, 2^bit_width)`.
- `no_std` build still succeeds.
- Clippy + fmt clean.

## Out of scope

- Bit packing for on-disk format (phase 16).
- Residual correction (phase 15).
- SIMD versions of quantize (phase 20).

## See also

- [[plans/rust/phase-13-rotation-numerics|Phase 13]]
- [[plans/rust/phase-15-codec-service-residual|Phase 15]]
- [[design/rust/type-mapping|Type Mapping]]
- [[design/rust/numerical-semantics|Numerical Semantics]]
