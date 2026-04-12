//! Parallel batch correctness tests (Phase 21).
//!
//! Verifies that `compress_batch_with(Parallelism::Custom(...))` produces
//! byte-identical output to `compress_batch_with(Parallelism::Serial)`.

use std::path::Path;
use tinyquant_core::codec::{Codebook, Codec, CodecConfig, Parallelism};

fn load_training() -> Vec<f32> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/codebook/training_n10000_d64.f32.bin");
    let bytes = std::fs::read(&p)
        .unwrap_or_else(|e| panic!("training fixture missing at {}: {e}", p.display()));
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Sequential driver that mimics rayon's interface for determinism tests
/// without requiring the `rayon` feature on `tinyquant-core`.
fn sequential_driver(count: usize, body: &(dyn Fn(usize) + Sync + Send)) {
    (0..count).for_each(body);
}

#[test]
fn compress_batch_custom_matches_serial_bw4() {
    let training = load_training();
    let cfg = CodecConfig::new(4, 42, 64, true).unwrap();
    let cb = Codebook::train(&training, &cfg).unwrap();
    let codec = Codec::new();

    let rows = 32_usize;
    let cols = 64_usize;
    let batch = &training[..rows * cols];

    let serial = codec
        .compress_batch_with(batch, rows, cols, &cfg, &cb, Parallelism::Serial)
        .unwrap();
    let custom = codec
        .compress_batch_with(
            batch,
            rows,
            cols,
            &cfg,
            &cb,
            Parallelism::Custom(sequential_driver),
        )
        .unwrap();

    assert_eq!(serial.len(), custom.len());
    for (i, (a, b)) in serial.iter().zip(custom.iter()).enumerate() {
        assert_eq!(
            a.indices(),
            b.indices(),
            "indices mismatch at row {i} (bw=4)"
        );
        assert_eq!(
            a.residual(),
            b.residual(),
            "residual mismatch at row {i} (bw=4)"
        );
    }
}

#[test]
fn compress_batch_custom_matches_serial_bw2() {
    let training = load_training();
    let cfg = CodecConfig::new(2, 42, 64, true).unwrap();
    let cb = Codebook::train(&training, &cfg).unwrap();
    let codec = Codec::new();

    let rows = 16_usize;
    let cols = 64_usize;
    let batch = &training[..rows * cols];

    let serial = codec
        .compress_batch_with(batch, rows, cols, &cfg, &cb, Parallelism::Serial)
        .unwrap();
    let custom = codec
        .compress_batch_with(
            batch,
            rows,
            cols,
            &cfg,
            &cb,
            Parallelism::Custom(sequential_driver),
        )
        .unwrap();

    for (i, (a, b)) in serial.iter().zip(custom.iter()).enumerate() {
        assert_eq!(
            a.indices(),
            b.indices(),
            "indices mismatch at row {i} (bw=2)"
        );
    }
}

#[test]
fn compress_batch_custom_matches_serial_bw8_no_residual() {
    let training = load_training();
    let cfg = CodecConfig::new(8, 42, 64, false).unwrap();
    let cb = Codebook::train(&training, &cfg).unwrap();
    let codec = Codec::new();

    let rows = 8_usize;
    let cols = 64_usize;
    let batch = &training[..rows * cols];

    let serial = codec
        .compress_batch_with(batch, rows, cols, &cfg, &cb, Parallelism::Serial)
        .unwrap();
    let custom = codec
        .compress_batch_with(
            batch,
            rows,
            cols,
            &cfg,
            &cb,
            Parallelism::Custom(sequential_driver),
        )
        .unwrap();

    for (i, (a, b)) in serial.iter().zip(custom.iter()).enumerate() {
        assert_eq!(
            a.indices(),
            b.indices(),
            "indices mismatch at row {i} (bw=8, no residual)"
        );
        assert!(a.residual().is_none());
        assert!(b.residual().is_none());
    }
}

#[test]
fn compress_batch_error_on_dimension_mismatch_propagates() {
    let training = load_training();
    let cfg = CodecConfig::new(4, 42, 64, false).unwrap();
    let cb = Codebook::train(&training, &cfg).unwrap();
    let codec = Codec::new();

    // Pass wrong cols (32 instead of 64)
    let err = codec
        .compress_batch_with(&training[..32 * 32], 32, 32, &cfg, &cb, Parallelism::Serial)
        .unwrap_err();
    assert!(
        matches!(
            err,
            tinyquant_core::errors::CodecError::DimensionMismatch { .. }
        ),
        "expected DimensionMismatch, got {err:?}"
    );
}

#[test]
fn decompress_batch_matches_individual() {
    let training = load_training();
    let cfg = CodecConfig::new(4, 42, 64, true).unwrap();
    let cb = Codebook::train(&training, &cfg).unwrap();
    let codec = Codec::new();

    let rows = 16_usize;
    let cols = 64_usize;
    let batch = &training[..rows * cols];

    let cvs = codec
        .compress_batch(batch, rows, cols, &cfg, &cb)
        .unwrap();

    let mut out = vec![0.0_f32; rows * cols];
    codec
        .decompress_batch_into(&cvs, &cfg, &cb, &mut out)
        .unwrap();

    for row in 0..rows {
        let per = codec.decompress(&cvs[row], &cfg, &cb).unwrap();
        assert_eq!(
            &out[row * cols..(row + 1) * cols],
            per.as_slice(),
            "decompress_batch mismatch at row {row}"
        );
    }
}
