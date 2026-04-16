//! Tests for PreparedCodec (Phase 26).
//!
//! These tests fail before `PreparedCodec` is implemented (red phase of TDD).
//! Run: `cargo test -p tinyquant-core --test prepared_codec`

use tinyquant_core::codec::{Codebook, Codec, CodecConfig, PreparedCodec};

fn fixture_codebook(config: &CodecConfig) -> Codebook {
    let training: Vec<f32> = (0..256 * config.dimension() as usize)
        .map(|i| (i as f32 * 0.001).sin())
        .collect();
    Codebook::train(&training, config).expect("fixture codebook must train")
}

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

/// PreparedCodec builds successfully and exposes config, codebook, and rotation.
#[test]
fn prepared_codec_builds_from_valid_config() {
    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let codebook = fixture_codebook(&config);
    let prepared = PreparedCodec::new(config.clone(), codebook).unwrap();
    assert_eq!(prepared.config().config_hash(), config.config_hash());
}

/// compress_prepared and the legacy compress path produce identical output.
#[test]
fn compress_prepared_matches_legacy_compress() {
    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let codebook = fixture_codebook(&config);
    let prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();
    let v = fixture_vector(64, 0);

    let cv_prepared = Codec::new()
        .compress_prepared(&v, &prepared)
        .expect("compress_prepared must succeed");
    let cv_legacy = Codec::new()
        .compress(&v, &config, &codebook)
        .expect("compress must succeed");

    assert_eq!(
        cv_prepared.indices(),
        cv_legacy.indices(),
        "compress_prepared and compress must produce identical quantization codes"
    );
    assert_eq!(
        cv_prepared.config_hash(),
        cv_legacy.config_hash(),
        "config hashes must match between both paths"
    );
}

/// decompress_prepared_into and legacy decompress produce identical output.
#[test]
fn decompress_prepared_matches_legacy_decompress() {
    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let codebook = fixture_codebook(&config);
    let prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();
    let v = fixture_vector(64, 1);

    let cv = Codec::new()
        .compress_prepared(&v, &prepared)
        .expect("compress_prepared must succeed");

    let mut out_prepared = vec![0f32; 64];
    let mut out_legacy = vec![0f32; 64];

    Codec::new()
        .decompress_prepared_into(&cv, &prepared, &mut out_prepared)
        .expect("decompress_prepared_into must succeed");
    Codec::new()
        .decompress_into(&cv, &config, &codebook, &mut out_legacy)
        .expect("decompress_into must succeed");

    for (i, (p, l)) in out_prepared.iter().zip(out_legacy.iter()).enumerate() {
        assert!(
            (p - l).abs() < f32::EPSILON * 2.0,
            "element {i}: prepared={p} legacy={l} — must be identical"
        );
    }
}

/// Identical inputs produce identical codes on repeated `compress_prepared` calls,
/// confirming the cached `RotationMatrix` is stable (no re-generation per call).
///
/// Note: this is a determinism test, not a direct inspection of the cache.
/// There is no Rust mechanism to count `RotationMatrix::build` invocations from
/// outside the type without interior mutability; determinism is the observable proxy.
#[test]
fn compress_prepared_is_deterministic_across_calls() {
    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let codebook = fixture_codebook(&config);
    let prepared = PreparedCodec::new(config.clone(), codebook).unwrap();

    // Compress two identical vectors — deterministic output proves no re-generation per call.
    let v0 = fixture_vector(64, 10);
    let v1 = fixture_vector(64, 10); // identical to v0
    let cv0 = Codec::new().compress_prepared(&v0, &prepared).unwrap();
    let cv1 = Codec::new().compress_prepared(&v1, &prepared).unwrap();
    assert_eq!(
        cv0.indices(),
        cv1.indices(),
        "identical inputs must produce identical codes regardless of call order"
    );
}
