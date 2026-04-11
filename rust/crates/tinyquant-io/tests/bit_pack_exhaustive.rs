//! Exhaustive round-trip tests for the bit-packing logic.
//!
//! For each supported bit width we verify:
//! - pack -> unpack returns the original indices
//! - known byte patterns match the LSB-first spec

use std::sync::Arc;
use tinyquant_core::codec::CompressedVector;
use tinyquant_io::compressed_vector::{from_bytes, to_bytes};

fn make_cv(indices: Vec<u8>, bit_width: u8) -> CompressedVector {
    let dim = indices.len() as u32;
    CompressedVector::new(
        indices.into_boxed_slice(),
        None,
        Arc::from("h"),
        dim,
        bit_width,
    )
    .unwrap()
}

#[test]
fn bw8_round_trip_all_values() {
    let indices: Vec<u8> = (0..=255u8).collect();
    let cv = make_cv(indices.clone(), 8);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), indices.as_slice());
}

#[test]
fn bw4_known_packing() {
    // indices [0x0, 0xF, 0x5, 0xA] -> byte0 = 0xF0, byte1 = 0xA5
    let indices = vec![0x0u8, 0xF, 0x5, 0xA];
    let cv = make_cv(indices.clone(), 4);
    let bytes = to_bytes(&cv);

    // header is 70 bytes, packed starts at offset 70
    // byte0 = (0x0 & 0xF) | ((0xF & 0xF) << 4) = 0xF0
    // byte1 = (0x5 & 0xF) | ((0xA & 0xF) << 4) = 0xA5
    assert_eq!(bytes[70], 0xF0, "bw=4 byte0 mismatch");
    assert_eq!(bytes[71], 0xA5, "bw=4 byte1 mismatch");

    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), indices.as_slice());
}

#[test]
fn bw2_known_packing() {
    // indices [0,1,2,3] -> byte0 = 0b11_10_01_00 = 0xE4
    let indices = vec![0u8, 1, 2, 3];
    let cv = make_cv(indices.clone(), 2);
    let bytes = to_bytes(&cv);

    assert_eq!(bytes[70], 0xE4, "bw=2 byte0 mismatch");

    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), indices.as_slice());
}

#[test]
fn bw4_all_256_values_round_trip() {
    // dim=256, all possible 4-bit values (0..=15) repeated
    let indices: Vec<u8> = (0..=15u8).cycle().take(256).collect();
    let cv = make_cv(indices.clone(), 4);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), indices.as_slice());
}

#[test]
fn bw2_all_4_values_round_trip() {
    // dim=400, all possible 2-bit values cycling
    let indices: Vec<u8> = (0..=3u8).cycle().take(400).collect();
    let cv = make_cv(indices.clone(), 2);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), indices.as_slice());
}

#[test]
fn bw4_odd_dim_round_trip() {
    // dim=17 (not a multiple of 2), bw=4
    let indices: Vec<u8> = (0..17u8).map(|i| i % 16).collect();
    let cv = make_cv(indices.clone(), 4);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), indices.as_slice());
}

#[test]
fn bw2_odd_dim_round_trip() {
    // dim=17 (not a multiple of 4), bw=2
    let indices: Vec<u8> = (0..17u8).map(|i| i % 4).collect();
    let cv = make_cv(indices.clone(), 2);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), indices.as_slice());
}

#[test]
fn bw4_dim1_round_trip() {
    let indices = vec![0x7u8];
    let cv = make_cv(indices.clone(), 4);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), indices.as_slice());
}

#[test]
fn bw2_dim1_round_trip() {
    let indices = vec![0x3u8];
    let cv = make_cv(indices.clone(), 2);
    let bytes = to_bytes(&cv);
    let cv2 = from_bytes(&bytes).unwrap();
    assert_eq!(cv2.indices(), indices.as_slice());
}
