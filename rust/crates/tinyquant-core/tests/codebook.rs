//! Integration tests for `tinyquant_core::codec::Codebook`.
//!
//! Covers:
//!
//! * Construction invariants — entry count, sort order, distinctness.
//! * `Codebook::train` byte parity against the frozen Python fixture
//!   for all three supported bit widths `{2, 4, 8}`.
//! * Round-trip behavior: nearest-entry quantize + exact-entry round
//!   trip + out-of-range rejection.
//! * Randomized (ChaCha-seeded) scan that every quantize index is in
//!   range, substituting for a proptest property.
//!
//! Fixtures are loaded at runtime via `fs::read` (matching
//! `rotation_fixture_parity.rs`) so the LFS blobs do not balloon the
//! integration-test binary.

use std::fs;
use std::path::PathBuf;

use bytemuck::cast_slice;
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

use tinyquant_core::codec::{Codebook, CodecConfig};
use tinyquant_core::errors::CodecError;

const FIXTURE_DIR_CODEBOOK: &str = "tests/fixtures/codebook";

fn fixture_bytes(rel: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURE_DIR_CODEBOOK)
        .join(rel);
    fs::read(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn load_training_fixture(rows: usize, cols: usize) -> Vec<f32> {
    let bytes = fixture_bytes(&format!("training_n{rows}_d{cols}.f32.bin"));
    assert_eq!(
        bytes.len(),
        rows * cols * 4,
        "training fixture has wrong byte length"
    );
    cast_slice::<u8, f32>(&bytes).to_vec()
}

fn load_expected_codebook(bit_width: u8) -> Vec<f32> {
    let bytes = fixture_bytes(&format!("expected_bw{bit_width}_seed42.f32.bin"));
    let num_entries = 1usize << bit_width;
    assert_eq!(
        bytes.len(),
        num_entries * 4,
        "expected codebook fixture has wrong byte length for bw={bit_width}"
    );
    cast_slice::<u8, f32>(&bytes).to_vec()
}

// --- construction invariants ---

#[test]
fn new_rejects_wrong_entry_count_for_bit_width_4() {
    let entries: Box<[f32]> = (0..10)
        .map(|i| i as f32)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let err = Codebook::new(entries, 4).expect_err("wrong count must fail");
    assert!(
        matches!(
            err,
            CodecError::CodebookEntryCount {
                expected: 16,
                got: 10,
                bit_width: 4
            }
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn new_rejects_unsorted_entries() {
    let mut e: Vec<f32> = (0..16).map(|i| i as f32).collect();
    e.swap(0, 1);
    let err = Codebook::new(e.into_boxed_slice(), 4).expect_err("unsorted must fail");
    assert_eq!(err, CodecError::CodebookNotSorted);
}

#[test]
fn new_rejects_duplicate_adjacent_entries() {
    let mut e: Vec<f32> = (0..16).map(|i| i as f32).collect();
    e[5] = e[4];
    let err = Codebook::new(e.into_boxed_slice(), 4).expect_err("dup must fail");
    assert!(
        matches!(err, CodecError::CodebookDuplicate { expected: 16, .. }),
        "unexpected error: {err:?}"
    );
}

#[test]
fn new_accepts_sorted_distinct_entries() {
    let e: Vec<f32> = (0..16).map(|i| i as f32).collect();
    let cb = Codebook::new(e.into_boxed_slice(), 4).expect("sorted distinct must succeed");
    assert_eq!(cb.num_entries(), 16);
    assert_eq!(cb.bit_width(), 4);
}

#[test]
fn new_accepts_bw2_and_bw8_bounds() {
    let bw2: Vec<f32> = vec![-1.0, 0.0, 0.5, 1.0];
    let cb2 = Codebook::new(bw2.into_boxed_slice(), 2).expect("bw=2 must succeed");
    assert_eq!(cb2.num_entries(), 4);

    let bw8: Vec<f32> = (0..256).map(|i| i as f32 * 0.01).collect();
    let cb8 = Codebook::new(bw8.into_boxed_slice(), 8).expect("bw=8 must succeed");
    assert_eq!(cb8.num_entries(), 256);
}

// --- training parity across all supported bit widths ---

fn assert_train_matches_fixture(bit_width: u8) {
    let training = load_training_fixture(10_000, 64);
    let config = CodecConfig::new(bit_width, 42, 64, false)
        .unwrap_or_else(|e| panic!("CodecConfig failed: {e:?}"));
    let cb = Codebook::train(&training, &config)
        .unwrap_or_else(|e| panic!("train failed for bw={bit_width}: {e:?}"));

    let expected = load_expected_codebook(bit_width);
    assert_eq!(
        cb.entries().len(),
        expected.len(),
        "entry count mismatch for bw={bit_width}"
    );
    for (i, (got, exp)) in cb.entries().iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            got.to_bits(),
            exp.to_bits(),
            "bw={bit_width} mismatch at entry {i}: got={got}, exp={exp}"
        );
    }
}

// These three tests compare Rust training output byte-for-byte against
// Python-generated fixtures. Like the dim=768 rotation test (see R19 in
// docs/design/rust/risks-and-mitigations.md), they are sensitive to the
// SIMD ISA active at runtime: AVX2 and AVX-512 kernels in pulp/faer
// produce different f64 bit patterns for the same input, so the fixture
// generated on one machine fails on another. Replaced by the orthogonality
// and round-trip checks below until RotationMatrix::build is made
// bit-reproducible across ISAs.
#[test]
fn train_matches_python_fixture_bw2_seed42_n10000_d64() {
    assert_train_matches_fixture(2);
}

#[test]
fn train_matches_python_fixture_bw4_seed42_n10000_d64() {
    assert_train_matches_fixture(4);
}

#[test]
fn train_matches_python_fixture_bw8_seed42_n10000_d64() {
    assert_train_matches_fixture(8);
}

// --- quantize / dequantize round-trip on a hand-crafted codebook ---

#[test]
fn quantize_then_dequantize_returns_nearest_entries() {
    // Entries are `(0..16) * 0.1` in f32 arithmetic, which is *not*
    // exactly `{0.0, 0.1, ..., 1.5}` because multiplying an integer by
    // `0.1_f32` accumulates rounding error. The test compares to the
    // actual `entries[k]` values rather than hand-typed f32 literals.
    let entries: Vec<f32> = (0..16).map(|i| i as f32 * 0.1).collect();
    let cb = Codebook::new(entries.clone().into_boxed_slice(), 4).unwrap();
    let values = vec![0.03_f32, 0.17, 0.85, 1.49];
    let mut idx = vec![0u8; 4];
    cb.quantize_into(&values, &mut idx).unwrap();
    // 0.03 → 0.0 (idx 0)
    // 0.17 → 0.2 (idx 2) — strictly closer to 0.2 than 0.1.
    // 0.85 → idx 9 — equidistant between entry 8 and 9, tie goes to
    //                the right neighbor.
    // 1.49 → idx 15 — strictly closer to 1.5 than to 1.4.
    assert_eq!(idx, vec![0, 2, 9, 15]);

    let mut out = vec![0.0f32; 4];
    cb.dequantize_into(&idx, &mut out).unwrap();
    assert_eq!(out, vec![entries[0], entries[2], entries[9], entries[15]]);
}

#[test]
fn quantize_exact_entry_values_produces_matching_indices() {
    let entries: Vec<f32> = (0..16).map(|i| i as f32).collect();
    let cb = Codebook::new(entries.into_boxed_slice(), 4).unwrap();
    let values: Vec<f32> = (0..16).map(|i| i as f32).collect();
    let mut idx = vec![0u8; 16];
    cb.quantize_into(&values, &mut idx).unwrap();
    assert_eq!(idx, (0..16u8).collect::<Vec<_>>());
}

#[test]
fn dequantize_rejects_index_ge_num_entries() {
    let entries: Vec<f32> = (0..16).map(|i| i as f32).collect();
    let cb = Codebook::new(entries.into_boxed_slice(), 4).unwrap();
    let idx = vec![0u8, 5, 20];
    let mut out = vec![0.0f32; 3];
    let err = cb.dequantize_into(&idx, &mut out).expect_err("must fail");
    assert_eq!(
        err,
        CodecError::IndexOutOfRange {
            index: 20,
            bound: 16,
        }
    );
}

// --- randomized index-in-range scan ---

/// Draw a uniform `f32` in `[-10, 10]` from a ChaCha stream. This uses
/// only the crate's existing `rand_chacha` dependency so no new crate
/// enters the test closure.
fn next_finite_f32(rng: &mut ChaCha20Rng) -> f32 {
    let bits = rng.next_u64();
    // Take the low 32 bits as a uniform u32 and map to [-10, 10].
    let u = (bits & 0xFFFF_FFFF) as u32;
    let unit = (u as f64) / (u32::MAX as f64); // [0, 1]
    (unit * 20.0 - 10.0) as f32
}

#[test]
fn quantize_indices_always_in_codebook_across_random_inputs() {
    let entries: Vec<f32> = (0..16).map(|i| i as f32).collect();
    let cb = Codebook::new(entries.into_boxed_slice(), 4).unwrap();

    // 256 mini-batches of up to 512 finite values each.
    let mut rng = ChaCha20Rng::seed_from_u64(1337);
    for _ in 0..256 {
        let len = 1 + (rng.next_u32() as usize % 512);
        let values: Vec<f32> = (0..len).map(|_| next_finite_f32(&mut rng)).collect();
        let mut idx = vec![0u8; len];
        cb.quantize_into(&values, &mut idx).unwrap();
        assert!(
            idx.iter().all(|&i| i < 16),
            "found out-of-range index in batch of len={len}"
        );
    }
}
