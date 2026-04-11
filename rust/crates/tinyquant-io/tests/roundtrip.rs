//! Round-trip serialization tests: to_bytes -> from_bytes must produce
//! an equivalent CompressedVector.

use std::sync::Arc;
use tinyquant_core::codec::CompressedVector;
use tinyquant_io::compressed_vector::{from_bytes, to_bytes};

fn make_cv_with_residual(
    indices: Vec<u8>,
    residual: Option<Vec<u8>>,
    config_hash: &str,
    bit_width: u8,
) -> CompressedVector {
    let dim = indices.len() as u32;
    CompressedVector::new(
        indices.into_boxed_slice(),
        residual.map(Vec::into_boxed_slice),
        Arc::from(config_hash),
        dim,
        bit_width,
    )
    .unwrap()
}

#[test]
fn bw4_dim768_no_residual() {
    let indices: Vec<u8> = (0..768u32).map(|i| (i % 16) as u8).collect();
    let cv = make_cv_with_residual(indices, None, "hash-bw4-d768", 4);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), cv.indices());
    assert_eq!(cv2.dimension(), 768);
    assert_eq!(cv2.bit_width(), 4);
    assert!(!cv2.has_residual());
    assert_eq!(cv2.config_hash().as_ref(), "hash-bw4-d768");
}

#[test]
fn bw4_dim768_with_residual() {
    let indices: Vec<u8> = (0..768u32).map(|i| (i % 16) as u8).collect();
    let residual: Vec<u8> = (0..768 * 2).map(|i| (i % 256) as u8).collect();
    let cv = make_cv_with_residual(indices, Some(residual), "hash-bw4-d768-r", 4);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), cv.indices());
    assert_eq!(cv2.residual(), cv.residual());
    assert!(cv2.has_residual());
}

#[test]
fn bw2_dim768_no_residual() {
    let indices: Vec<u8> = (0..768u32).map(|i| (i % 4) as u8).collect();
    let cv = make_cv_with_residual(indices, None, "hash-bw2", 2);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), cv.indices());
    assert_eq!(cv2.bit_width(), 2);
}

#[test]
fn bw8_dim768_no_residual() {
    let indices: Vec<u8> = (0..768u32).map(|i| (i % 256) as u8).collect();
    let cv = make_cv_with_residual(indices, None, "hash-bw8", 8);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), cv.indices());
    assert_eq!(cv2.bit_width(), 8);
}

#[test]
fn bw8_dim1536_with_residual() {
    let indices: Vec<u8> = (0..1536u32).map(|i| (i % 256) as u8).collect();
    let residual: Vec<u8> = (0..1536 * 2).map(|i| (i % 256) as u8).collect();
    let cv = make_cv_with_residual(indices, Some(residual), "hash-bw8-d1536", 8);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), cv.indices());
    assert_eq!(cv2.residual(), cv.residual());
    assert_eq!(cv2.dimension(), 1536);
}

#[test]
fn bw4_dim1_no_residual() {
    let indices = vec![0x7u8];
    let cv = make_cv_with_residual(indices, None, "hash-dim1", 4);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), cv.indices());
    assert_eq!(cv2.dimension(), 1);
}

#[test]
fn bw2_dim17_with_residual() {
    let indices: Vec<u8> = (0..17u8).map(|i| i % 4).collect();
    let residual: Vec<u8> = vec![0u8; 34];
    let cv = make_cv_with_residual(indices, Some(residual), "hash-bw2-d17", 2);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), cv.indices());
    assert_eq!(cv2.residual(), cv.residual());
}

#[test]
fn empty_config_hash_round_trips() {
    let indices = vec![0u8; 16];
    let cv = make_cv_with_residual(indices, None, "", 4);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.config_hash().as_ref(), "");
}

#[test]
fn long_config_hash_truncated_to_64_bytes() {
    // A 128-char hash gets truncated to 64 bytes in the header
    let long_hash: String = "a".repeat(128);
    let indices = vec![0u8; 4];
    let cv = make_cv_with_residual(indices, None, &long_hash, 4);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    // Only first 64 bytes survive
    assert_eq!(cv2.config_hash().as_ref(), &long_hash[..64]);
}

#[test]
fn serialized_length_is_correct() {
    // bw=4, dim=768, no residual:
    //   70 (header) + ceil(768*4/8) (packed) + 1 (flag) = 70 + 384 + 1 = 455
    let indices: Vec<u8> = vec![0u8; 768];
    let cv = make_cv_with_residual(indices, None, "h", 4);
    let bytes = to_bytes(&cv);
    assert_eq!(bytes.len(), 70 + 384 + 1);
}

#[test]
fn serialized_length_with_residual() {
    // bw=4, dim=768, residual=1536 bytes:
    //   70 (header) + 384 (packed) + 1 (flag) + 4 (len) + 1536 (residual) = 1995
    let indices: Vec<u8> = vec![0u8; 768];
    let residual: Vec<u8> = vec![0u8; 1536];
    let cv = make_cv_with_residual(indices, Some(residual), "h", 4);
    let bytes = to_bytes(&cv);
    assert_eq!(bytes.len(), 70 + 384 + 1 + 4 + 1536);
}
