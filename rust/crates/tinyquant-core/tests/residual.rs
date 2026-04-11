use tinyquant_core::codec::residual::{apply_residual_into, compute_residual};

#[test]
fn compute_and_apply_residual_round_trip_is_bounded() {
    let original = [0.12_f32, -0.34, 0.56, -0.78];
    let reconstructed = [0.10_f32, -0.30, 0.60, -0.80];
    let residual = compute_residual(&original, &reconstructed);
    assert_eq!(residual.len(), original.len() * 2); // 2 bytes per element (f16)

    let mut out = reconstructed;
    apply_residual_into(&mut out, &residual).unwrap();
    for (r, o) in out.iter().zip(original.iter()) {
        assert!((r - o).abs() < 1e-2, "r={r} o={o}");
    }
}

#[test]
fn apply_residual_rejects_mismatched_length() {
    let mut values = [0.0_f32; 4];
    let residual = [0_u8; 6]; // should be 8 (4 * 2)
    let err = apply_residual_into(&mut values, &residual).unwrap_err();
    assert!(matches!(
        err,
        tinyquant_core::errors::CodecError::LengthMismatch { left: 6, right: 8 }
    ));
}
