use alloc::sync::Arc;
use tinyquant_core::codec::CompressedVector;
use tinyquant_core::errors::CodecError;

extern crate alloc;

fn hash() -> Arc<str> {
    Arc::from("deadbeef".repeat(8).as_str())
}

#[test]
fn new_accepts_matching_indices_length() {
    let indices = vec![0_u8; 64].into_boxed_slice();
    let cv = CompressedVector::new(indices, None, hash(), 64, 4).unwrap();
    assert_eq!(cv.dimension(), 64);
    assert_eq!(cv.bit_width(), 4);
    assert!(!cv.has_residual());
    assert_eq!(cv.indices().len(), 64);
    assert!(cv.residual().is_none());
}

#[test]
fn new_rejects_index_length_mismatch() {
    let indices = vec![0_u8; 32].into_boxed_slice();
    let err = CompressedVector::new(indices, None, hash(), 64, 4).unwrap_err();
    assert!(
        matches!(
            err,
            CodecError::DimensionMismatch {
                expected: 64,
                got: 32
            }
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn new_rejects_unsupported_bit_width() {
    let indices = vec![0_u8; 64].into_boxed_slice();
    let err = CompressedVector::new(indices, None, hash(), 64, 3).unwrap_err();
    assert!(
        matches!(err, CodecError::UnsupportedBitWidth { got: 3 }),
        "unexpected error: {err:?}"
    );
}

#[test]
fn new_rejects_residual_length_mismatch() {
    let indices = vec![0_u8; 64].into_boxed_slice();
    let residual = Some(vec![0_u8; 64].into_boxed_slice()); // should be 128
    let err = CompressedVector::new(indices, residual, hash(), 64, 4).unwrap_err();
    assert!(
        matches!(
            err,
            CodecError::LengthMismatch {
                left: 64,
                right: 128
            }
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn size_bytes_matches_python_formula() {
    // 64 dims × 4 bits = 256 bits = 32 bytes packed, + 128-byte residual
    let indices = vec![0_u8; 64].into_boxed_slice();
    let residual = Some(vec![0_u8; 128].into_boxed_slice());
    let cv = CompressedVector::new(indices, residual, hash(), 64, 4).unwrap();
    assert_eq!(cv.size_bytes(), 32 + 128);

    // 65 dims × 4 bits = 260 bits → ceil(260/8) = 33 bytes, no residual
    let idx2 = vec![0_u8; 65].into_boxed_slice();
    let cv2 = CompressedVector::new(idx2, None, hash(), 65, 4).unwrap();
    assert_eq!(cv2.size_bytes(), 33);
}

#[test]
fn has_residual_reflects_presence() {
    let cv_no_res =
        CompressedVector::new(vec![0_u8; 16].into_boxed_slice(), None, hash(), 16, 8).unwrap();
    assert!(!cv_no_res.has_residual());

    let cv_res = CompressedVector::new(
        vec![0_u8; 16].into_boxed_slice(),
        Some(vec![0_u8; 32].into_boxed_slice()),
        hash(),
        16,
        8,
    )
    .unwrap();
    assert!(cv_res.has_residual());
    assert_eq!(cv_res.residual().unwrap().len(), 32);
}
