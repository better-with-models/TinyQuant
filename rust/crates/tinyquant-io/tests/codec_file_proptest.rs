//! Property-style round-trip tests for the Level-2 TQCV corpus file format.

use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tinyquant_core::codec::CompressedVector;
use tinyquant_io::codec_file::{CodecFileReader, CodecFileWriter};

fn make_cv(dim: usize, bit_width: u8, rng: &mut ChaCha20Rng) -> CompressedVector {
    let max_val = (1u16 << bit_width) - 1;
    let indices: Vec<u8> = (0..dim)
        .map(|_| (rng.next_u32() as u8) & max_val as u8)
        .collect();
    CompressedVector::new(
        indices.into_boxed_slice(),
        None,
        Arc::from("proptest"),
        dim as u32,
        bit_width,
    )
    .unwrap_or_else(|e| panic!("cv build: {e}"))
}

#[test]
fn codec_file_proptest_round_trip() {
    let mut rng = ChaCha20Rng::seed_from_u64(17);
    let bit_widths = [2u8, 4, 8];

    for case in 0..64u32 {
        let bit_width = bit_widths[(case % 3) as usize];
        let dim = (rng.next_u32() % 64 + 1) as usize;
        let count = (rng.next_u32() % 10 + 1) as usize;

        let mut original: Vec<Vec<u8>> = Vec::with_capacity(count);
        let tmpfile = NamedTempFile::new().unwrap_or_else(|e| panic!("tmpfile: {e}"));

        {
            let mut writer = CodecFileWriter::create(
                tmpfile.path(),
                "proptest",
                dim as u32,
                bit_width,
                false,
                &[],
            )
            .unwrap_or_else(|e| panic!("writer create: {e}"));

            for _ in 0..count {
                let cv = make_cv(dim, bit_width, &mut rng);
                original.push(cv.indices().to_vec());
                writer.append(&cv).unwrap_or_else(|e| panic!("append: {e}"));
            }
            writer
                .finalize()
                .unwrap_or_else(|e| panic!("finalize: {e}"));
        }

        // Read back via streaming reader
        {
            let file = std::fs::File::open(tmpfile.path()).unwrap_or_else(|e| panic!("open: {e}"));
            let mut reader =
                CodecFileReader::new(file).unwrap_or_else(|e| panic!("reader new: {e}"));

            assert_eq!(reader.header().vector_count, count as u64);
            assert_eq!(reader.header().dimension, dim as u32);
            assert_eq!(reader.header().bit_width, bit_width);

            for (i, expected) in original.iter().enumerate() {
                let cv = reader
                    .next_vector()
                    .unwrap_or_else(|e| panic!("next_vector[{i}]: {e}"))
                    .unwrap_or_else(|| panic!("no vector at [{i}]"));
                assert_eq!(cv.indices(), expected.as_slice(), "case={case} i={i}");
            }
            let no_more = reader
                .next_vector()
                .unwrap_or_else(|e| panic!("next_vector after end: {e}"));
            assert!(no_more.is_none());
        }
    }
}
