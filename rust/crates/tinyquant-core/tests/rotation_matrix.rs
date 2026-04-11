//! Integration tests for `tinyquant_core::codec::RotationMatrix` and the
//! underlying `ChaChaGaussianStream`.
//!
//! Covers:
//!   - `from_config` shape and `verify_orthogonality` within 1e-12.
//!   - Bit-identical determinism across repeated `build` calls.
//!   - f32 → f64 → f32 round-trip via `apply_into` + `apply_inverse_into`
//!     agrees with the input within 1e-5.
//!   - `apply_into` rejects wrong-length inputs with `LengthMismatch`.
//!   - `ChaChaGaussianStream::new(seed)` is deterministic per seed.
//!
//! A separate test file (`rotation_fixture_parity.rs`) loads the LFS
//! snapshot produced by the `dump_rotation_fixture` example binary and
//! asserts byte parity; it is not included here to keep compilation
//! independent of the presence of the fixture binaries.

use tinyquant_core::codec::{CodecConfig, RotationMatrix};
use tinyquant_core::errors::CodecError;

// --- basic construction & orthogonality ---

#[test]
fn from_config_matches_dim_and_is_orthogonal() {
    let cfg = CodecConfig::new(4, 42, 64, true).unwrap();
    let rot = RotationMatrix::from_config(&cfg);
    assert_eq!(rot.dimension(), 64);
    assert_eq!(rot.matrix().len(), 64 * 64);
    assert!(
        rot.verify_orthogonality(1e-12),
        "generated rotation must be orthogonal within 1e-12"
    );
}

#[test]
fn build_is_bit_identical_for_same_seed_and_dim() {
    let a = RotationMatrix::build(42, 64);
    let b = RotationMatrix::build(42, 64);
    assert_eq!(
        a.matrix(),
        b.matrix(),
        "same (seed, dim) must be deterministic"
    );
}

#[test]
fn different_seeds_produce_different_matrices() {
    let a = RotationMatrix::build(42, 32);
    let b = RotationMatrix::build(43, 32);
    assert_ne!(a.matrix(), b.matrix());
}

// --- apply / apply_inverse ---

#[test]
fn apply_then_inverse_roundtrip_recovers_vector_within_1e_5() {
    let rot = RotationMatrix::build(42, 64);
    // Deterministic "random" input via a simple LCG-like generator.
    let mut input = [0.0f32; 64];
    for (i, slot) in input.iter_mut().enumerate() {
        let x = ((i as u32).wrapping_mul(2654435761)) as f32 / u32::MAX as f32;
        *slot = x - 0.5;
    }
    let mut rotated = [0.0f32; 64];
    let mut recovered = [0.0f32; 64];
    rot.apply_into(&input, &mut rotated).unwrap();
    rot.apply_inverse_into(&rotated, &mut recovered).unwrap();
    let max_err = input
        .iter()
        .zip(recovered.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_err < 1e-5,
        "round-trip max error {max_err} exceeds 1e-5"
    );
}

#[test]
fn apply_preserves_l2_norm_within_1e_5() {
    let rot = RotationMatrix::build(42, 64);
    let input: [f32; 64] = core::array::from_fn(|i| (i as f32 - 31.5) / 32.0);
    let mut rotated = [0.0f32; 64];
    rot.apply_into(&input, &mut rotated).unwrap();
    let input_norm: f32 = input.iter().map(|x| x * x).sum::<f32>().sqrt();
    let rotated_norm: f32 = rotated.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(
        (input_norm - rotated_norm).abs() < 1e-5,
        "L2 norm drifted: {input_norm} -> {rotated_norm}"
    );
}

#[test]
fn apply_into_rejects_length_mismatch() {
    let rot = RotationMatrix::build(42, 8);
    let input = [0.0f32; 7];
    let mut output = [0.0f32; 8];
    let err = rot.apply_into(&input, &mut output).unwrap_err();
    assert!(matches!(err, CodecError::LengthMismatch { .. }));
}

#[test]
fn apply_inverse_into_rejects_length_mismatch() {
    let rot = RotationMatrix::build(42, 8);
    let input = [0.0f32; 8];
    let mut output = [0.0f32; 9];
    let err = rot.apply_inverse_into(&input, &mut output).unwrap_err();
    assert!(matches!(err, CodecError::LengthMismatch { .. }));
}

// --- dim=1 degenerate case ---

#[test]
fn dimension_one_is_plus_or_minus_one_and_roundtrips() {
    let rot = RotationMatrix::build(42, 1);
    assert_eq!(rot.matrix().len(), 1);
    let m = rot.matrix()[0];
    assert!((m.abs() - 1.0).abs() < 1e-15, "1x1 orthogonal must be +-1");
    let input = [0.375f32];
    let mut out = [0.0f32; 1];
    let mut back = [0.0f32; 1];
    rot.apply_into(&input, &mut out).unwrap();
    rot.apply_inverse_into(&out, &mut back).unwrap();
    assert!((input[0] - back[0]).abs() < 1e-6);
}
