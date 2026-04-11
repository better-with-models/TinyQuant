//! Zero-copy view tests.

use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::sync::Arc;
use tinyquant_core::codec::CompressedVector;
use tinyquant_io::compressed_vector::{from_bytes, to_bytes};
use tinyquant_io::zero_copy::view::CompressedVectorView;

/// Build a simple CompressedVector for testing (bit_width=4, no residual).
fn make_cv(dim: usize, rng: &mut ChaCha20Rng) -> CompressedVector {
    let max_val = (1u8 << 4) - 1;
    let indices: Vec<u8> = (0..dim).map(|_| (rng.next_u32() as u8) & max_val).collect();
    CompressedVector::new(
        indices.into_boxed_slice(),
        None,
        Arc::from("test-hash"),
        dim as u32,
        4,
    )
    .unwrap_or_else(|e| panic!("failed to create CompressedVector: {e}"))
}

#[test]
fn view_parses_serialized_compressed_vector_without_alloc() {
    let mut rng = ChaCha20Rng::seed_from_u64(42);
    let cv = make_cv(64, &mut rng);

    let bytes = to_bytes(&cv);
    let (view, tail) =
        CompressedVectorView::parse(&bytes).unwrap_or_else(|e| panic!("parse failed: {e}"));

    assert!(
        tail.is_empty(),
        "expected empty tail, got {} bytes",
        tail.len()
    );
    assert_eq!(view.format_version, 0x01);
    assert_eq!(view.config_hash, "test-hash");
    assert_eq!(view.dimension, 64);
    assert_eq!(view.bit_width, 4);
    assert!(view.residual.is_none());
}

#[test]
fn view_parse_agrees_with_from_bytes() {
    let mut rng = ChaCha20Rng::seed_from_u64(7);

    for _ in 0..256 {
        let dim = (rng.next_u32() % 32 + 1) as usize; // 1..=32
        let bit_width = [2u8, 4, 8][rng.next_u32() as usize % 3];
        let max_val = (1u16 << bit_width) - 1;
        let indices: Vec<u8> = (0..dim)
            .map(|_| (rng.next_u32() as u8) & max_val as u8)
            .collect();

        let cv = CompressedVector::new(
            indices.into_boxed_slice(),
            None,
            Arc::from("round-trip"),
            dim as u32,
            bit_width,
        )
        .unwrap_or_else(|e| panic!("cv build failed: {e}"));

        let bytes = to_bytes(&cv);

        // from_bytes
        let decoded = from_bytes(&bytes).unwrap_or_else(|e| panic!("from_bytes failed: {e}"));

        // view parse
        let (view, tail) = CompressedVectorView::parse(&bytes)
            .unwrap_or_else(|e| panic!("view parse failed: {e}"));
        assert!(tail.is_empty());
        assert_eq!(view.dimension, decoded.dimension());
        assert_eq!(view.bit_width, decoded.bit_width());
        assert_eq!(view.config_hash, decoded.config_hash().as_ref());

        // Unpack and compare indices
        let mut unpacked = vec![0u8; dim];
        view.unpack_into(&mut unpacked)
            .unwrap_or_else(|e| panic!("unpack_into failed: {e}"));
        assert_eq!(unpacked, decoded.indices());
    }
}
