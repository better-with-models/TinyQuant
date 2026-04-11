use std::{fs, path::Path};
use tinyquant_core::codec::{Codebook, Codec, CodecConfig};

fn load_f32(rel: &str) -> Vec<f32> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    let bytes = fs::read(&p)
        .unwrap_or_else(|_| panic!("fixture missing: {}", p.display()));
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

// ── Round-trip ────────────────────────────────────────────────────────────────

#[test]
fn compress_then_decompress_recovers_vector_within_bound() {
    let training = load_f32("tests/fixtures/codebook/training_n10000_d64.f32.bin");
    let config = CodecConfig::new(4, 42, 64, true).unwrap();
    let codebook = Codebook::train(&training, &config).unwrap();
    let codec = Codec::new();

    let vector = &training[0..64];
    let cv = codec.compress(vector, &config, &codebook).unwrap();
    assert_eq!(cv.config_hash(), config.config_hash());
    assert_eq!(cv.bit_width(), 4);
    assert!(cv.has_residual());
    assert_eq!(cv.dimension(), 64);

    let recovered = codec.decompress(&cv, &config, &codebook).unwrap();
    assert_eq!(recovered.len(), 64);
    let mse: f32 = vector
        .iter()
        .zip(recovered.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f32>()
        / 64.0;
    assert!(mse < 1e-2, "mse={mse}");
}

// ── Determinism ───────────────────────────────────────────────────────────────

#[test]
fn compress_is_deterministic_across_calls() {
    let training = load_f32("tests/fixtures/codebook/training_n10000_d64.f32.bin");
    let config = CodecConfig::new(8, 42, 64, true).unwrap();
    let codebook = Codebook::train(&training, &config).unwrap();
    let codec = Codec::new();
    let v = &training[640..704];

    let a = codec.compress(v, &config, &codebook).unwrap();
    let b = codec.compress(v, &config, &codebook).unwrap();
    assert_eq!(a.indices(), b.indices());
    assert_eq!(a.residual(), b.residual());
}

// ── Error paths ───────────────────────────────────────────────────────────────

#[test]
fn compress_rejects_dimension_mismatch() {
    let training = load_f32("tests/fixtures/codebook/training_n10000_d64.f32.bin");
    let config = CodecConfig::new(4, 42, 64, true).unwrap();
    let codebook = Codebook::train(&training, &config).unwrap();
    let bad = vec![0.0_f32; 32];
    let err = Codec::new().compress(&bad, &config, &codebook).unwrap_err();
    assert!(
        matches!(
            err,
            tinyquant_core::errors::CodecError::DimensionMismatch {
                expected: 64,
                got: 32
            }
        ),
        "unexpected: {err:?}"
    );
}

#[test]
fn compress_rejects_codebook_bit_width_mismatch() {
    let training = load_f32("tests/fixtures/codebook/training_n10000_d64.f32.bin");
    let cfg4 = CodecConfig::new(4, 42, 64, true).unwrap();
    let cfg8 = CodecConfig::new(8, 42, 64, true).unwrap();
    let cb8 = Codebook::train(&training, &cfg8).unwrap();
    let v = &training[0..64];
    let err = Codec::new().compress(v, &cfg4, &cb8).unwrap_err();
    assert!(
        matches!(
            err,
            tinyquant_core::errors::CodecError::CodebookIncompatible {
                expected: 4,
                got: 8
            }
        ),
        "unexpected: {err:?}"
    );
}

#[test]
fn decompress_rejects_config_hash_mismatch() {
    let training = load_f32("tests/fixtures/codebook/training_n10000_d64.f32.bin");
    let cfg_a = CodecConfig::new(4, 42, 64, true).unwrap();
    let cfg_b = CodecConfig::new(4, 43, 64, true).unwrap(); // different seed
    let cb = Codebook::train(&training, &cfg_a).unwrap();
    let v = &training[0..64];
    let cv = Codec::new().compress(v, &cfg_a, &cb).unwrap();
    let err = Codec::new().decompress(&cv, &cfg_b, &cb).unwrap_err();
    assert!(
        matches!(
            err,
            tinyquant_core::errors::CodecError::ConfigMismatch { .. }
        ),
        "unexpected: {err:?}"
    );
}

// ── Batch equivalence ─────────────────────────────────────────────────────────

#[test]
fn compress_batch_matches_individual_calls() {
    let training = load_f32("tests/fixtures/codebook/training_n10000_d64.f32.bin");
    let config = CodecConfig::new(4, 42, 64, true).unwrap();
    let codebook = Codebook::train(&training, &config).unwrap();
    let codec = Codec::new();

    let rows = 16_usize;
    let cols = 64_usize;
    let batch = &training[0..rows * cols];

    let batch_cvs = codec
        .compress_batch(batch, rows, cols, &config, &codebook)
        .unwrap();
    for row in 0..rows {
        let individual = codec
            .compress(&batch[row * cols..(row + 1) * cols], &config, &codebook)
            .unwrap();
        assert_eq!(
            batch_cvs[row].indices(),
            individual.indices(),
            "indices mismatch row {row}"
        );
        assert_eq!(
            batch_cvs[row].residual(),
            individual.residual(),
            "residual mismatch row {row}"
        );
    }
}

#[test]
fn decompress_batch_matches_individual_calls() {
    let training = load_f32("tests/fixtures/codebook/training_n10000_d64.f32.bin");
    let config = CodecConfig::new(4, 42, 64, true).unwrap();
    let codebook = Codebook::train(&training, &config).unwrap();
    let codec = Codec::new();

    let rows = 8_usize;
    let cols = 64_usize;
    let batch = &training[0..rows * cols];
    let cvs = codec
        .compress_batch(batch, rows, cols, &config, &codebook)
        .unwrap();

    let mut out = vec![0.0_f32; rows * cols];
    codec
        .decompress_batch_into(&cvs, &config, &codebook, &mut out)
        .unwrap();

    for row in 0..rows {
        let per = codec.decompress(&cvs[row], &config, &codebook).unwrap();
        assert_eq!(
            &out[row * cols..(row + 1) * cols],
            per.as_slice(),
            "batch vs individual mismatch row {row}"
        );
    }
}
