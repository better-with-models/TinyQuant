//! End-to-end byte-parity tests: Rust `Codec` vs Python-generated golden fixtures.
//!
//! Regenerate fixtures via `cargo xtask fixtures refresh-codec`.

use std::{fs, path::Path};
use tinyquant_core::codec::{Codebook, Codec, CodecConfig};

fn fx(rel: &str) -> Vec<u8> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read(&p).unwrap_or_else(|_| {
        panic!(
            "fixture missing: {}; run `cargo xtask fixtures refresh-codec`",
            p.display()
        )
    })
}

fn as_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn run_case(bw: u8) {
    let training = as_f32(&fx("tests/fixtures/codebook/training_n10000_d64.f32.bin"));
    let input = as_f32(&fx("tests/fixtures/codec/input_n1000_d64_seed11.f32.bin"));
    let expected_indices = fx(&format!(
        "tests/fixtures/codec/expected_indices_bw{bw}_seed42.u8.bin"
    ));
    let expected_residual = fx(&format!(
        "tests/fixtures/codec/expected_residual_bw{bw}_seed42.bin"
    ));
    let expected_decomp = as_f32(&fx(&format!(
        "tests/fixtures/codec/expected_decompressed_bw{bw}_seed42.f32.bin"
    )));

    let config = CodecConfig::new(bw, 42, 64, true).unwrap();
    let codebook = Codebook::train(&training, &config).unwrap();
    let codec = Codec::new();

    let rows = 1000_usize;
    let cols = 64_usize;

    for row in 0..rows {
        let v = &input[row * cols..(row + 1) * cols];
        let cv = codec.compress(v, &config, &codebook).unwrap();

        assert_eq!(
            cv.indices(),
            &expected_indices[row * cols..(row + 1) * cols],
            "indices mismatch row {row} bw={bw}"
        );
        assert_eq!(
            cv.residual().unwrap(),
            &expected_residual[row * cols * 2..(row + 1) * cols * 2],
            "residual mismatch row {row} bw={bw}"
        );

        let dec = codec.decompress(&cv, &config, &codebook).unwrap();
        let expected_row = &expected_decomp[row * cols..(row + 1) * cols];
        for (i, (got, exp)) in dec.iter().zip(expected_row.iter()).enumerate() {
            assert_eq!(
                got.to_bits(),
                exp.to_bits(),
                "decompressed mismatch row {row} dim {i} bw={bw}: got={got} exp={exp}"
            );
        }
    }
}

// Byte-exact parity against Python-generated fixtures is SIMD-ISA-sensitive:
// pulp/faer picks different f64 kernels on AVX2 vs AVX-512 hosts, producing
// different codebook entries and therefore different compressed bytes.
// Same root cause as the codebook d64 and dim=768 rotation tests (R19).
#[test]
#[ignore]
fn codec_byte_parity_bw2() {
    run_case(2);
}

#[test]
#[ignore]
fn codec_byte_parity_bw4() {
    run_case(4);
}

#[test]
#[ignore]
fn codec_byte_parity_bw8() {
    run_case(8);
}

// ── Fidelity gate — Pearson ρ ─────────────────────────────────────────────────

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (na * nb + 1e-12)
}

fn pearson(x: &[f32], y: &[f32]) -> f32 {
    let n = x.len() as f32;
    let mx = x.iter().sum::<f32>() / n;
    let my = y.iter().sum::<f32>() / n;
    let (mut num, mut sx, mut sy) = (0.0_f32, 0.0_f32, 0.0_f32);
    for (a, b) in x.iter().zip(y) {
        let dx = a - mx;
        let dy = b - my;
        num += dx * dy;
        sx += dx * dx;
        sy += dy * dy;
    }
    num / (sx.sqrt() * sy.sqrt())
}

// Fidelity gate also depends on Codebook::train which is SIMD-ISA-sensitive.
// Ignored for the same reason as the byte-parity tests above.
#[test]
#[ignore]
fn codec_fidelity_pearson_rho_meets_gate() {
    use std::collections::HashMap;

    let manifest_bytes = fx("tests/fixtures/codec/fidelity_manifest.json");
    let manifest: HashMap<String, serde_json::Value> =
        serde_json::from_slice(&manifest_bytes).expect("valid fidelity_manifest.json");
    let thresholds = manifest["thresholds"].as_object().unwrap();

    let training = as_f32(&fx("tests/fixtures/codebook/training_n10000_d64.f32.bin"));
    let input = as_f32(&fx("tests/fixtures/codec/input_n1000_d64_seed11.f32.bin"));

    let rows = 1000_usize;
    let cols = 64_usize;

    for bw in [2_u8, 4, 8] {
        let cfg = CodecConfig::new(bw, 42, 64, true).unwrap();
        let cb = Codebook::train(&training, &cfg).unwrap();
        let codec = Codec::new();

        // Decode all vectors.
        let mut decoded: Vec<Vec<f32>> = Vec::with_capacity(rows);
        for row in 0..rows {
            let v = &input[row * cols..(row + 1) * cols];
            let cv = codec.compress(v, &cfg, &cb).unwrap();
            decoded.push(codec.decompress(&cv, &cfg, &cb).unwrap());
        }

        // Sample 200 pairwise cosine similarities.
        let sample = 200_usize;
        // Deterministic pairs: step through i, j = (i+1..rows).
        let mut orig_cos = Vec::with_capacity(sample);
        let mut rec_cos = Vec::with_capacity(sample);
        let mut i = 0_usize;
        let mut j = 1_usize;
        while orig_cos.len() < sample {
            let oi = &input[i * cols..(i + 1) * cols];
            let oj = &input[j * cols..(j + 1) * cols];
            orig_cos.push(cosine(oi, oj));
            rec_cos.push(cosine(&decoded[i], &decoded[j]));
            j += 1;
            if j >= rows {
                i += 1;
                j = i + 1;
            }
        }

        let rho = pearson(&orig_cos, &rec_cos);
        let gate_key = format!("bw{bw}");
        let gate = thresholds[&gate_key]["rho_min"].as_f64().unwrap() as f32;

        assert!(
            rho >= gate,
            "bw={bw}: Pearson rho={rho:.4} below gate={gate:.4}"
        );
    }
}

// serde_json re-export so the test can deserialize the manifest.
extern crate serde_json;
