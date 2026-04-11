---
title: "Unit Tests: Codebook"
tags:
  - qa
  - unit-tests
  - codec
date-created: 2026-04-08
status: active
category: qa
---

# Unit Tests: Codebook

> [!info] Class under test
> [[classes/codebook|Codebook]] — quantization lookup table.

## Test file

```text
tests/codec/test_codebook.py
```

## Test cases

### Training

| Test | Asserts |
|------|---------|
| `test_train_produces_correct_entry_count_4bit` | `num_entries == 16` |
| `test_train_produces_correct_entry_count_2bit` | `num_entries == 4` |
| `test_train_entries_are_sorted` | Entries in ascending order |
| `test_train_entries_have_no_duplicates` | All entries distinct |
| `test_train_with_too_few_vectors_raises` | Insufficient training data → `ValueError` |
| `test_train_is_deterministic` | Same training data + config → identical codebook |

### Quantization

| Test | Asserts |
|------|---------|
| `test_quantize_returns_valid_indices` | All indices in `[0, num_entries)` |
| `test_quantize_maps_to_nearest_entry` | Each value maps to its closest codebook entry |
| `test_quantize_exact_entry_value` | A value equal to a codebook entry maps to that entry's index |
| `test_quantize_output_dtype` | Returns `uint8` array |

### Dequantization

| Test | Asserts |
|------|---------|
| `test_dequantize_maps_indices_to_entries` | Index `i` → `entries[i]` |
| `test_dequantize_output_dtype` | Returns `float32` array |
| `test_dequantize_invalid_index_raises` | Index >= `num_entries` raises error |

### Immutability

| Test | Asserts |
|------|---------|
| `test_frozen_rejects_entry_modification` | Modifying `entries` array raises or is prevented |

## Rust integration tests

> [!info] Rust parity gate
> The Rust port in `tinyquant_core::codec::Codebook` has its own
> integration tests under
> `rust/crates/tinyquant-core/tests/codebook.rs` and
> `tests/quantize.rs`. They do not replace the Python unit tests
> listed above; they ride alongside as a byte-parity gate that locks
> Rust behavior to the Python reference.

| Rust test | Asserts |
|-----------|---------|
| `new_rejects_wrong_entry_count_for_bit_width_4` | Entry count mismatch → `CodecError::CodebookEntryCount` |
| `new_rejects_unsorted_entries` | Non-ascending entries → `CodecError::CodebookNotSorted` |
| `new_rejects_duplicate_adjacent_entries` | Equal neighbors → `CodecError::CodebookDuplicate` |
| `new_accepts_sorted_distinct_entries` | Well-formed 16-entry codebook round-trips construction |
| `new_accepts_bw2_and_bw8_bounds` | Bit-width sweep smoke test (4 entries and 256 entries) |
| `train_matches_python_fixture_bw2_seed42_n10000_d64` | Python byte parity on `bw=2` — 4 f32 entries |
| `train_matches_python_fixture_bw4_seed42_n10000_d64` | Python byte parity on `bw=4` — 16 f32 entries |
| `train_matches_python_fixture_bw8_seed42_n10000_d64` | Python byte parity on `bw=8` — 256 f32 entries |
| `quantize_then_dequantize_returns_nearest_entries` | Nearest-neighbor round trip on a handcrafted codebook |
| `quantize_exact_entry_values_produces_matching_indices` | Identity at exact entry values |
| `dequantize_rejects_index_ge_num_entries` | Out-of-range index → `CodecError::IndexOutOfRange` |
| `quantize_indices_always_in_codebook_across_random_inputs` | Deterministic `rand_chacha` scan; 256 batches × ≤512 values, every index in `[0, 16)`. Substitutes for a proptest property that Phase 14 could not land on MSRV 1.81 |
| `quantize_matches_python_fixture_bw2` | Python byte parity on 10 000 quantize indices at `bw=2` |
| `quantize_matches_python_fixture_bw4` | Python byte parity on 10 000 quantize indices at `bw=4` |
| `quantize_matches_python_fixture_bw8` | Python byte parity on 10 000 quantize indices at `bw=8` |
| `dequantize_round_trips_fixture_indices_bw4` | Every dequantized f32 belongs to the codebook entry set |

Fixtures are LFS-tracked under
`rust/crates/tinyquant-core/tests/fixtures/codebook/` and
`.../tests/fixtures/quantize/`. Refresh via
`cargo xtask fixtures refresh-all` from `rust/`; the subcommand
refuses to run the quantize pass if the matching codebook fixture is
missing, so Python and Rust cannot fall out of sync silently.

See [[design/rust/phase-14-implementation-notes|Phase 14
Implementation Notes]] for the execution log.

## See also

- [[classes/codebook|Codebook]]
- [[qa/unit-tests/test-codec|Codec tests]]
- [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]]
- [[design/rust/testing-strategy|Rust Testing Strategy]]
