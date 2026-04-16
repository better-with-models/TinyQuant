//! Integration tests for the stateless `Codec` service.
//!
//! Exercises compress/decompress round-trips against the shared fixture
//! corpus shared with the Python reference implementation.

use std::{fs, path::Path};
use tinyquant_core::codec::{Codebook, Codec, CodecConfig};

fn fixture_vector(dim: usize, seed: u32) -> Vec<f32> {
    let mut s = seed;
    (0..dim)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            (s as f32) / u32::MAX as f32
        })
        .collect()
}

fn load_f32(rel: &str) -> Vec<f32> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    let bytes = fs::read(&p).unwrap_or_else(|_| panic!("fixture missing: {}", p.display()));
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

// ── Residual quality gate ─────────────────────────────────────────────────────

/// GAP-DECOMP-004: `residual_enabled=true` must produce lower MSE than
/// `residual_enabled=false` on the same input vectors for ≥ 95% of vectors.
///
/// Uses a dim=64 synthetic fixture; runs in < 100 ms.
#[test]
#[allow(clippy::cast_precision_loss)]
fn residual_on_has_lower_mse_than_residual_off() {
    let dim: usize = 64;
    let seed = 42_u64;
    let n_vectors = 100_usize;

    let config_on = CodecConfig::new(4, seed, dim as u32, true).unwrap();
    let config_off = CodecConfig::new(4, seed, dim as u32, false).unwrap();

    let training: Vec<f32> = (0..256 * dim)
        .map(|i| (i as f32 * 0.001).sin())
        .collect();

    let codebook_on = Codebook::train(&training, &config_on).unwrap();
    let codebook_off = Codebook::train(&training, &config_off).unwrap();

    let mut better_count = 0_usize;
    for i in 0..n_vectors {
        let v = fixture_vector(dim, i as u32);

        let cv_on = Codec::new()
            .compress(&v, &config_on, &codebook_on)
            .unwrap();
        let cv_off = Codec::new()
            .compress(&v, &config_off, &codebook_off)
            .unwrap();

        let mut out_on = vec![0f32; dim];
        let mut out_off = vec![0f32; dim];
        Codec::new()
            .decompress_into(&cv_on, &config_on, &codebook_on, &mut out_on)
            .unwrap();
        Codec::new()
            .decompress_into(&cv_off, &config_off, &codebook_off, &mut out_off)
            .unwrap();

        let mse_on: f32 = v
            .iter()
            .zip(&out_on)
            .map(|(o, r)| (o - r).powi(2))
            .sum::<f32>()
            / dim as f32;
        let mse_off: f32 = v
            .iter()
            .zip(&out_off)
            .map(|(o, r)| (o - r).powi(2))
            .sum::<f32>()
            / dim as f32;

        if mse_on < mse_off {
            better_count += 1;
        }
    }

    assert!(
        better_count >= n_vectors * 95 / 100,
        "residual_on must have lower MSE than residual_off for ≥ 95% of vectors; \
         got {better_count}/{n_vectors}"
    );
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
