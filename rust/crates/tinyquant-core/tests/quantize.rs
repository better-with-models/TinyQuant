//! Integration tests for the private scalar quantize/dequantize
//! kernels re-exposed via [`Codebook`].
//!
//! The `scalar_*` helpers are `pub(crate)` inside the crate, so these
//! tests cross-check the kernels *through* `Codebook::quantize_into` /
//! `Codebook::dequantize_into`. That keeps the public surface honest
//! and is enough for Phase 14: the Phase 20 SIMD path will compare
//! directly against the scalar kernels via an internal module.
//!
//! Byte parity is verified against Python-generated fixtures under
//! `tests/fixtures/quantize/` for all three supported bit widths.

use std::fs;
use std::path::PathBuf;

use bytemuck::cast_slice;

use tinyquant_core::codec::Codebook;

const FIXTURE_DIR_QUANTIZE: &str = "tests/fixtures/quantize";
const FIXTURE_DIR_CODEBOOK: &str = "tests/fixtures/codebook";

fn read_bytes(dir: &str, rel: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(dir)
        .join(rel);
    fs::read(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn load_values(count: usize) -> Vec<f32> {
    let bytes = read_bytes(FIXTURE_DIR_QUANTIZE, &format!("values_n{count}.f32.bin"));
    assert_eq!(bytes.len(), count * 4, "values fixture wrong size");
    cast_slice::<u8, f32>(&bytes).to_vec()
}

fn load_expected_indices(bit_width: u8) -> Vec<u8> {
    read_bytes(
        FIXTURE_DIR_QUANTIZE,
        &format!("expected_bw{bit_width}_seed42.u8.bin"),
    )
}

fn load_codebook(bit_width: u8) -> Codebook {
    let bytes = read_bytes(
        FIXTURE_DIR_CODEBOOK,
        &format!("expected_bw{bit_width}_seed42.f32.bin"),
    );
    let entries = cast_slice::<u8, f32>(&bytes).to_vec();
    Codebook::new(entries.into_boxed_slice(), bit_width)
        .unwrap_or_else(|e| panic!("codebook load failed for bw={bit_width}: {e:?}"))
}

fn assert_quantize_matches_fixture(bit_width: u8) {
    let values = load_values(10_000);
    let expected = load_expected_indices(bit_width);
    let cb = load_codebook(bit_width);

    let mut got = vec![0u8; values.len()];
    cb.quantize_into(&values, &mut got)
        .unwrap_or_else(|e| panic!("quantize_into failed for bw={bit_width}: {e:?}"));

    assert_eq!(
        got.len(),
        expected.len(),
        "index length mismatch for bw={bit_width}"
    );
    let mismatches = got
        .iter()
        .zip(expected.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        mismatches,
        0,
        "bw={bit_width}: {mismatches} of {} indices disagree with Python",
        got.len()
    );
}

#[test]
fn quantize_matches_python_fixture_bw2() {
    assert_quantize_matches_fixture(2);
}

#[test]
fn quantize_matches_python_fixture_bw4() {
    assert_quantize_matches_fixture(4);
}

#[test]
fn quantize_matches_python_fixture_bw8() {
    assert_quantize_matches_fixture(8);
}

#[test]
fn dequantize_round_trips_fixture_indices_bw4() {
    let bit_width = 4;
    let expected = load_expected_indices(bit_width);
    let cb = load_codebook(bit_width);

    let mut out = vec![0.0f32; expected.len()];
    cb.dequantize_into(&expected, &mut out).unwrap();

    // Every f32 in `out` must belong to the codebook entry set.
    let entries = cb.entries();
    for (i, v) in out.iter().enumerate() {
        assert!(
            entries.iter().any(|e| e.to_bits() == v.to_bits()),
            "dequantized value at {i} ({v}) is not in the codebook"
        );
    }
}
