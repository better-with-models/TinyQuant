//! Audit that header constants are self-consistent and the header layout
//! is exactly the documented 70 bytes.

use tinyquant_io::compressed_vector::HEADER_SIZE;

#[test]
fn header_size_is_seventy() {
    assert_eq!(HEADER_SIZE, 70, "header must be exactly 70 bytes");
}

#[test]
fn header_constants_add_up() {
    // 1 (version) + 64 (config_hash) + 4 (dimension u32 LE) + 1 (bit_width) = 70
    let computed = 1usize + 64 + 4 + 1;
    assert_eq!(computed, HEADER_SIZE);
}

#[test]
fn encode_produces_correct_offset_content() {
    use std::sync::Arc;
    use tinyquant_core::codec::CompressedVector;
    use tinyquant_io::compressed_vector::to_bytes;

    let indices = vec![0u8; 4];
    let cv = CompressedVector::new(
        indices.into_boxed_slice(),
        None,
        Arc::from("testhash"),
        4,
        8,
    )
    .unwrap();

    let bytes = to_bytes(&cv);

    // version byte
    assert_eq!(bytes[0], 0x01, "version byte must be 0x01");

    // config hash field at offset 1..65 — first 8 bytes are "testhash"
    let hash_bytes = &bytes[1..65];
    let trimmed: Vec<u8> = hash_bytes.iter().copied().filter(|&b| b != 0).collect();
    assert_eq!(trimmed, b"testhash");

    // dimension at offset 65..69
    let dim = u32::from_le_bytes([bytes[65], bytes[66], bytes[67], bytes[68]]);
    assert_eq!(dim, 4);

    // bit_width at offset 69
    assert_eq!(bytes[69], 8);
}
