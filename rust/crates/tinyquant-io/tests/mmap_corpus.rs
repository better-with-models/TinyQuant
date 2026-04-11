//! mmap-based corpus file tests (require `--features mmap`).

#![cfg(feature = "mmap")]

use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::io::Write as _;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tinyquant_core::codec::CompressedVector;
use tinyquant_io::codec_file::{CodecFileReader, CodecFileWriter};
use tinyquant_io::mmap::CorpusFileReader;

fn make_cv(dim: usize, bit_width: u8, rng: &mut ChaCha20Rng) -> CompressedVector {
    let max_val = (1u16 << bit_width) - 1;
    let indices: Vec<u8> = (0..dim)
        .map(|_| (rng.next_u32() as u8) & max_val as u8)
        .collect();
    CompressedVector::new(
        indices.into_boxed_slice(),
        None,
        Arc::from("mmap-test"),
        dim as u32,
        bit_width,
    )
    .unwrap_or_else(|e| panic!("cv build: {e}"))
}

#[test]
fn corpus_file_write_then_read_round_trip() {
    let mut rng = ChaCha20Rng::seed_from_u64(99);
    let count = 100;
    let dim = 64;
    let bit_width = 4u8;

    let mut original_indices: Vec<Vec<u8>> = Vec::with_capacity(count);
    let tmpfile = NamedTempFile::new().unwrap_or_else(|e| panic!("tmpfile: {e}"));

    {
        let mut writer =
            CodecFileWriter::create(tmpfile.path(), "mmap-test", dim, bit_width, false, &[])
                .unwrap_or_else(|e| panic!("writer create: {e}"));

        for _ in 0..count {
            let cv = make_cv(dim as usize, bit_width, &mut rng);
            original_indices.push(cv.indices().to_vec());
            writer.append(&cv).unwrap_or_else(|e| panic!("append: {e}"));
        }
        writer
            .finalize()
            .unwrap_or_else(|e| panic!("finalize: {e}"));
    }

    let reader =
        CorpusFileReader::open(tmpfile.path()).unwrap_or_else(|e| panic!("mmap open: {e}"));
    assert_eq!(reader.header().vector_count, count as u64);
    assert_eq!(reader.header().dimension, dim);
    assert_eq!(reader.header().bit_width, bit_width);
    assert_eq!(reader.header().config_hash, "mmap-test");

    for (i, (view, expected)) in reader.iter().zip(original_indices.iter()).enumerate() {
        let view = view.unwrap_or_else(|e| panic!("iter[{i}] error: {e}"));
        let mut unpacked = vec![0u8; dim as usize];
        view.unpack_into(&mut unpacked)
            .unwrap_or_else(|e| panic!("unpack[{i}]: {e}"));
        assert_eq!(&unpacked, expected, "mismatch at [{i}]");
    }
}

#[test]
fn finalize_interrupted_before_magic_rewrite_is_rejected() {
    // Write a TQCX file (tentative) without finalizing, and verify open rejects it.
    let tmpfile = NamedTempFile::new().unwrap_or_else(|e| panic!("tmpfile: {e}"));
    let mut rng = ChaCha20Rng::seed_from_u64(1234);
    let cv = make_cv(16, 4, &mut rng);

    {
        let mut writer =
            CodecFileWriter::create(tmpfile.path(), "tentative-test", 16, 4, false, &[])
                .unwrap_or_else(|e| panic!("writer create: {e}"));
        writer.append(&cv).unwrap_or_else(|e| panic!("append: {e}"));
        // Do NOT call finalize — magic stays as TQCX
        drop(writer);
    }

    let result = CorpusFileReader::open(tmpfile.path());
    assert!(
        result.is_err(),
        "expected error opening tentative TQCX file, got Ok"
    );
}

#[test]
fn corpus_file_empty_file_is_rejected() {
    let tmpfile = NamedTempFile::new().unwrap_or_else(|e| panic!("tmpfile: {e}"));
    // Write zero bytes (empty file)
    {
        let _file = std::fs::File::create(tmpfile.path()).unwrap_or_else(|e| panic!("create: {e}"));
        // immediately dropped = empty file
    }
    let result = CorpusFileReader::open(tmpfile.path());
    match result {
        Err(tinyquant_io::errors::IoError::Truncated { needed: 24, got: 0 }) => {
            // expected
        }
        Err(e) => panic!("expected Truncated{{24, 0}}, got: {e:?}"),
        Ok(_) => panic!("expected error, got Ok"),
    }
}

#[test]
fn corpus_file_bad_magic_is_rejected() {
    let tmpfile = NamedTempFile::new().unwrap_or_else(|e| panic!("tmpfile: {e}"));
    {
        let mut file =
            std::fs::File::create(tmpfile.path()).unwrap_or_else(|e| panic!("create: {e}"));
        // Write 24 bytes with wrong magic
        let mut header = [0u8; 24];
        header[0] = b'X';
        header[1] = b'X';
        header[2] = b'X';
        header[3] = b'X';
        header[4] = 0x01; // version
                          // dimension non-zero to avoid that error first
        header[16] = 1; // dimension = 1
        header[20] = 4; // bit_width = 4
        file.write_all(&header)
            .unwrap_or_else(|e| panic!("write: {e}"));
    }
    let result = CorpusFileReader::open(tmpfile.path());
    assert!(
        matches!(result, Err(tinyquant_io::errors::IoError::BadMagic { .. })),
        "expected BadMagic, got: {result:?}"
    );
}

#[test]
fn corpus_file_multi_reader_open() {
    // Two readers of the same file should both succeed.
    let tmpfile = NamedTempFile::new().unwrap_or_else(|e| panic!("tmpfile: {e}"));
    let mut rng = ChaCha20Rng::seed_from_u64(55);

    {
        let mut writer = CodecFileWriter::create(tmpfile.path(), "multi-reader", 32, 4, false, &[])
            .unwrap_or_else(|e| panic!("writer: {e}"));
        for _ in 0..5 {
            let cv = make_cv(32, 4, &mut rng);
            writer.append(&cv).unwrap_or_else(|e| panic!("append: {e}"));
        }
        writer
            .finalize()
            .unwrap_or_else(|e| panic!("finalize: {e}"));
    }

    let r1 = CorpusFileReader::open(tmpfile.path()).unwrap_or_else(|e| panic!("reader1: {e}"));
    let r2 = CorpusFileReader::open(tmpfile.path()).unwrap_or_else(|e| panic!("reader2: {e}"));
    assert_eq!(r1.header().vector_count, 5);
    assert_eq!(r2.header().vector_count, 5);
    assert_eq!(r1.iter().count(), 5);
    assert_eq!(r2.iter().count(), 5);
}

#[test]
fn corpus_file_nonzero_reserved_is_rejected() {
    let tmpfile = NamedTempFile::new().unwrap_or_else(|e| panic!("tmpfile: {e}"));

    // Write a well-formed TQCV header but with non-zero reserved bytes.
    let mut rng = ChaCha20Rng::seed_from_u64(7);
    {
        let mut writer =
            CodecFileWriter::create(tmpfile.path(), "reserved-test", 16, 4, false, &[])
                .unwrap_or_else(|e| panic!("writer: {e}"));
        let cv = make_cv(16, 4, &mut rng);
        writer.append(&cv).unwrap_or_else(|e| panic!("append: {e}"));
        writer
            .finalize()
            .unwrap_or_else(|e| panic!("finalize: {e}"));
    }

    // Corrupt the reserved bytes (offset 5).
    {
        use std::io::{Seek, SeekFrom, Write};
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(tmpfile.path())
            .unwrap_or_else(|e| panic!("open: {e}"));
        file.seek(SeekFrom::Start(5))
            .unwrap_or_else(|e| panic!("seek: {e}"));
        file.write_all(&[0x01])
            .unwrap_or_else(|e| panic!("write: {e}"));
    }

    let result = CorpusFileReader::open(tmpfile.path());
    assert!(
        matches!(result, Err(tinyquant_io::errors::IoError::InvalidHeader)),
        "expected InvalidHeader, got: {result:?}"
    );
}

#[test]
fn corpus_file_zero_dimension_is_rejected() {
    let tmpfile = NamedTempFile::new().unwrap_or_else(|e| panic!("tmpfile: {e}"));
    {
        let mut file =
            std::fs::File::create(tmpfile.path()).unwrap_or_else(|e| panic!("create: {e}"));
        let mut header = [0u8; 32]; // extra for alignment padding
                                    // TQCV magic
        header[0] = b'T';
        header[1] = b'Q';
        header[2] = b'C';
        header[3] = b'V';
        header[4] = 0x01; // version
                          // reserved 5..8 = 0
                          // vector_count 8..16 = 0
                          // dimension 16..20 = 0 (invalid)
        header[20] = 4; // bit_width = 4
                        // residual_flag 21 = 0
                        // config_hash_len 22..24 = 0
        file.write_all(&header)
            .unwrap_or_else(|e| panic!("write: {e}"));
    }
    let result = CorpusFileReader::open(tmpfile.path());
    assert!(
        matches!(result, Err(tinyquant_io::errors::IoError::InvalidHeader)),
        "expected InvalidHeader for zero dimension, got: {result:?}"
    );
}

#[test]
fn corpus_file_invalid_bit_width_is_rejected() {
    let tmpfile = NamedTempFile::new().unwrap_or_else(|e| panic!("tmpfile: {e}"));
    {
        let mut file =
            std::fs::File::create(tmpfile.path()).unwrap_or_else(|e| panic!("create: {e}"));
        let mut header = [0u8; 32];
        header[0] = b'T';
        header[1] = b'Q';
        header[2] = b'C';
        header[3] = b'V';
        header[4] = 0x01;
        header[16] = 64; // dimension = 64
        header[20] = 3; // bit_width = 3 (invalid)
        file.write_all(&header)
            .unwrap_or_else(|e| panic!("write: {e}"));
    }
    let result = CorpusFileReader::open(tmpfile.path());
    assert!(
        matches!(
            result,
            Err(tinyquant_io::errors::IoError::InvalidBitWidth { got: 3 })
        ),
        "expected InvalidBitWidth{{3}}, got: {result:?}"
    );
}

#[test]
fn streaming_reader_round_trip() {
    // Also verify the non-mmap CodecFileReader produces the same results.
    let mut rng = ChaCha20Rng::seed_from_u64(888);
    let count = 20;
    let dim = 32u32;
    let bit_width = 8u8;

    let mut original: Vec<Vec<u8>> = Vec::with_capacity(count);
    let tmpfile = NamedTempFile::new().unwrap_or_else(|e| panic!("tmpfile: {e}"));

    {
        let mut writer =
            CodecFileWriter::create(tmpfile.path(), "streaming", dim, bit_width, false, &[])
                .unwrap_or_else(|e| panic!("writer: {e}"));
        for _ in 0..count {
            let cv = make_cv(dim as usize, bit_width, &mut rng);
            original.push(cv.indices().to_vec());
            writer.append(&cv).unwrap_or_else(|e| panic!("append: {e}"));
        }
        writer
            .finalize()
            .unwrap_or_else(|e| panic!("finalize: {e}"));
    }

    // mmap reader
    let mmap_reader =
        CorpusFileReader::open(tmpfile.path()).unwrap_or_else(|e| panic!("mmap: {e}"));
    let mmap_results: Vec<Vec<u8>> = mmap_reader
        .iter()
        .map(|v| {
            let v = v.unwrap_or_else(|e| panic!("iter: {e}"));
            let mut out = vec![0u8; dim as usize];
            v.unpack_into(&mut out)
                .unwrap_or_else(|e| panic!("unpack: {e}"));
            out
        })
        .collect();

    // streaming reader
    let file = std::fs::File::open(tmpfile.path()).unwrap_or_else(|e| panic!("open: {e}"));
    let mut stream_reader =
        CodecFileReader::new(file).unwrap_or_else(|e| panic!("stream reader: {e}"));
    let stream_results: Vec<Vec<u8>> = (0..count)
        .map(|i| {
            stream_reader
                .next_vector()
                .unwrap_or_else(|e| panic!("next_vector[{i}]: {e}"))
                .unwrap_or_else(|| panic!("none at [{i}]"))
                .indices()
                .to_vec()
        })
        .collect();

    assert_eq!(mmap_results, original);
    assert_eq!(stream_results, original);
}
