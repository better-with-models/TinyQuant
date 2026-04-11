//! Frozen-snapshot parity test for `RotationMatrix`.
//!
//! The fixture files are byte-for-byte `f64` dumps produced by
//! `cargo run --release -p tinyquant-core --example dump_rotation_fixture
//! --features std -- <seed> <dim> <out>`. A failure here means the
//! canonical rotation pipeline (ChaCha20 → Box-Muller → faer QR → sign
//! correction) has drifted, either because a dependency upgrade changed
//! the RNG / linear-algebra stream or because a local edit accidentally
//! perturbed the recipe. The remediation is either to revert the change
//! or to regenerate all fixtures in a single audited commit.

use std::fs;
use std::path::PathBuf;

use tinyquant_core::codec::RotationMatrix;

const FIXTURE_DIR: &str = "tests/fixtures/rotation";

/// Load an `f64` little-endian fixture and return it as a `Vec<f64>`.
fn load_fixture(name: &str, expected_dim: usize) -> Vec<f64> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURE_DIR)
        .join(name);
    let bytes =
        fs::read(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    assert_eq!(
        bytes.len(),
        expected_dim * expected_dim * 8,
        "fixture {name} wrong size"
    );
    let mut values = Vec::with_capacity(expected_dim * expected_dim);
    for chunk in bytes.chunks_exact(8) {
        values.push(f64::from_le_bytes(chunk.try_into().unwrap()));
    }
    values
}

#[test]
fn seed_42_dim_64_matches_frozen_snapshot_bit_for_bit() {
    let expected = load_fixture("seed_42_dim_64.f64.bin", 64);
    let rot = RotationMatrix::build(42, 64);
    let actual = rot.matrix();
    assert_eq!(actual.len(), expected.len());
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            a.to_bits(),
            e.to_bits(),
            "fixture mismatch at index {i}: {a} vs {e}"
        );
    }
}

#[test]
fn seed_42_dim_64_fixture_is_orthogonal_within_1e_12() {
    let expected = load_fixture("seed_42_dim_64.f64.bin", 64);
    // Quick orthogonality check: Σ_k M[i,k] M[j,k] == δ_ij.
    let dim = 64usize;
    for i in 0..dim {
        for j in 0..dim {
            let mut acc = 0.0f64;
            for k in 0..dim {
                acc += expected[i * dim + k] * expected[j * dim + k];
            }
            let target = if i == j { 1.0 } else { 0.0 };
            assert!(
                (acc - target).abs() < 1e-12,
                "loaded fixture not orthogonal at ({i},{j}): {acc}"
            );
        }
    }
}

#[test]
fn seed_42_dim_768_matches_frozen_snapshot_bit_for_bit() {
    let expected = load_fixture("seed_42_dim_768.f64.bin", 768);
    let rot = RotationMatrix::build(42, 768);
    let actual = rot.matrix();
    assert_eq!(actual.len(), expected.len());
    let mut mismatches = 0usize;
    for (a, e) in actual.iter().zip(expected.iter()) {
        if a.to_bits() != e.to_bits() {
            mismatches += 1;
        }
    }
    assert_eq!(
        mismatches, 0,
        "{mismatches} f64 words differ between build and fixture for (42, 768)"
    );
}
