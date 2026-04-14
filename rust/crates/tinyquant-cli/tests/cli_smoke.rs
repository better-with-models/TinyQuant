//! End-to-end smoke tests for the `tinyquant` CLI binary.
//!
//! These tests drive the released binary exactly as an operator would:
//! they shell out via `assert_cmd::Command::cargo_bin("tinyquant")` and
//! assert on exit status, stdout, and on-disk artefacts.
//!
//! The three functions mirror the contract in
//! `docs/plans/rust/phase-22-pyo3-cabi-release.md` §Step 12:
//!
//! 1. `info_subcommand_prints_version` — sanity-check the binary and
//!    its build-info surface.
//! 2. `codec_train_writes_codebook_file` — the simplest artefact-
//!    producing command; confirms `tinyquant-io` wiring + argument
//!    parsing + file output.
//! 3. `codec_compress_decompress_round_trip_via_cli` — the full
//!    pipeline: train → compress → decompress, with MSE < 1e-2.

use std::fs;
use std::io::Write;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Write `rows * cols` deterministic pseudo-random `f32`s as raw
/// little-endian bytes (the `.f32.bin` format the CLI consumes with
/// `--format f32`).
fn write_random_f32_bin(path: &Path, rows: usize, cols: usize) {
    // Seed is arbitrary but fixed for determinism across re-runs.
    let mut rng = ChaCha8Rng::seed_from_u64(0x5c04e_u64);
    let mut file = fs::File::create(path).expect("create training fixture");
    for _ in 0..(rows * cols) {
        // Values in [-1.0, 1.0) — a mild normal-ish distribution is
        // fine for codebook training; the goal is coverage, not
        // statistical fidelity.
        let v: f32 = rng.gen_range(-1.0_f32..1.0_f32);
        file.write_all(&v.to_le_bytes()).expect("write fixture");
    }
    file.flush().expect("flush fixture");
}

#[test]
fn info_subcommand_prints_version() {
    Command::cargo_bin("tinyquant")
        .expect("cargo_bin should find the compiled binary")
        .arg("info")
        .assert()
        .success()
        .stdout(predicate::str::contains("tinyquant"))
        .stdout(predicate::str::contains("version"));
}

#[test]
fn codec_train_writes_codebook_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let training = tmp.path().join("training.f32.bin");
    write_random_f32_bin(&training, 1000, 64);
    let codebook = tmp.path().join("codebook.bin");

    Command::cargo_bin("tinyquant")
        .expect("cargo_bin")
        .args([
            "codec",
            "train",
            "--input",
            training.to_str().unwrap(),
            "--rows",
            "1000",
            "--cols",
            "64",
            "--bit-width",
            "4",
            "--seed",
            "42",
            "--output",
            codebook.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        codebook.exists(),
        "codebook file was not written to {}",
        codebook.display()
    );
    let meta = fs::metadata(&codebook).expect("stat codebook");
    assert!(meta.len() > 0, "codebook file is empty");
}

#[test]
fn codec_compress_decompress_round_trip_via_cli() {
    let tmp = tempfile::tempdir().expect("tempdir");

    // 1. Training fixture.
    let training = tmp.path().join("training.f32.bin");
    write_random_f32_bin(&training, 2000, 64);

    // 2. Train.
    let codebook = tmp.path().join("codebook.bin");
    let config_json = tmp.path().join("config.json");
    Command::cargo_bin("tinyquant")
        .expect("cargo_bin")
        .args([
            "codec",
            "train",
            "--input",
            training.to_str().unwrap(),
            "--rows",
            "2000",
            "--cols",
            "64",
            "--bit-width",
            "4",
            "--seed",
            "42",
            "--output",
            codebook.to_str().unwrap(),
            "--config-out",
            config_json.to_str().unwrap(),
        ])
        .assert()
        .success();

    // 3. Compress — reuse the same fixture as the input corpus.
    let corpus = tmp.path().join("corpus.tqcv");
    Command::cargo_bin("tinyquant")
        .expect("cargo_bin")
        .args([
            "codec",
            "compress",
            "--input",
            training.to_str().unwrap(),
            "--rows",
            "2000",
            "--cols",
            "64",
            "--config-json",
            config_json.to_str().unwrap(),
            "--codebook",
            codebook.to_str().unwrap(),
            "--output",
            corpus.to_str().unwrap(),
        ])
        .assert()
        .success();

    // 4. Decompress.
    let recovered = tmp.path().join("recovered.f32.bin");
    Command::cargo_bin("tinyquant")
        .expect("cargo_bin")
        .args([
            "codec",
            "decompress",
            "--input",
            corpus.to_str().unwrap(),
            "--config-json",
            config_json.to_str().unwrap(),
            "--codebook",
            codebook.to_str().unwrap(),
            "--output",
            recovered.to_str().unwrap(),
        ])
        .assert()
        .success();

    // 5. MSE check.
    let original_bytes = fs::read(&training).expect("read training");
    let recovered_bytes = fs::read(&recovered).expect("read recovered");
    assert_eq!(
        original_bytes.len(),
        recovered_bytes.len(),
        "recovered file has wrong length"
    );

    let orig: &[f32] = bytemuck::cast_slice(&original_bytes);
    let rec: &[f32] = bytemuck::cast_slice(&recovered_bytes);
    assert_eq!(orig.len(), rec.len());
    let n = orig.len() as f64;
    let sse: f64 = orig
        .iter()
        .zip(rec.iter())
        .map(|(a, b)| {
            let d = f64::from(*a) - f64::from(*b);
            d * d
        })
        .sum();
    let mse = sse / n;
    assert!(
        mse < 1e-2,
        "round-trip MSE too high: {mse:.6} (threshold 1e-2)"
    );
}
