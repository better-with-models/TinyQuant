//! Example binary that writes canonical end-to-end codec fixtures.
//!
//! Rust and Python use different RNG algorithms for the rotation matrix
//! (ChaCha20 vs PCG64), so byte-parity on the full codec pipeline cannot
//! be verified against Python-generated files.  Instead, fixtures are
//! generated from Rust itself to lock in deterministic behaviour and detect
//! future regressions.
//!
//! Usage:
//!
//! ```text
//! cargo run -p tinyquant-core --example dump_codec_fixture --features std -- \
//!     <input-seed> <codec-seed> <rows> <cols> \
//!     <training-f32-bin> <out-dir>
//! ```
//!
//! Outputs (inside `<out-dir>/`):
//!
//! * `input_n{rows}_d{cols}_seed{input_seed}.f32.bin`
//! * `expected_indices_bw{2,4,8}_seed{codec_seed}.u8.bin`
//! * `expected_residual_bw{2,4,8}_seed{codec_seed}.bin`
//! * `expected_decompressed_bw{2,4,8}_seed{codec_seed}.f32.bin`

use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;

use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use tinyquant_core::codec::{Codebook, Codec, CodecConfig};

const WORKER_STACK_BYTES: usize = 64 * 1024 * 1024;
const SUPPORTED_BIT_WIDTHS: &[u8] = &[2, 4, 8];

fn main() -> ExitCode {
    let handle = match thread::Builder::new()
        .name(String::from("dump_codec_fixture"))
        .stack_size(WORKER_STACK_BYTES)
        .spawn(run)
    {
        Ok(h) => h,
        Err(err) => {
            eprintln!("failed to spawn worker: {err}");
            return ExitCode::from(2);
        }
    };
    match handle.join() {
        Ok(code) => code,
        Err(_) => {
            eprintln!("worker panicked");
            ExitCode::from(2)
        }
    }
}

fn run() -> ExitCode {
    let mut args = env::args().skip(1);

    let input_seed: u64 = match args.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return usage("missing or invalid <input-seed>"),
    };
    let codec_seed: u64 = match args.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return usage("missing or invalid <codec-seed>"),
    };
    let rows: usize = match args.next().and_then(|s| s.parse().ok()) {
        Some(v) if v > 0 => v,
        _ => return usage("missing or invalid <rows>"),
    };
    let cols: u32 = match args.next().and_then(|s| s.parse().ok()) {
        Some(v) if v > 0 => v,
        _ => return usage("missing or invalid <cols>"),
    };
    let training_path: PathBuf = match args.next() {
        Some(s) => PathBuf::from(s),
        None => return usage("missing <training-f32-bin>"),
    };
    let out_dir: PathBuf = match args.next() {
        Some(s) => PathBuf::from(s),
        None => return usage("missing <out-dir>"),
    };

    if let Err(err) = fs::create_dir_all(&out_dir) {
        eprintln!("failed to create {}: {err}", out_dir.display());
        return ExitCode::from(2);
    }

    // Load training corpus.
    let training_bytes = match fs::read(&training_path) {
        Ok(b) => b,
        Err(err) => {
            eprintln!(
                "failed to read training corpus {}: {err}",
                training_path.display()
            );
            return ExitCode::from(2);
        }
    };
    let training: Vec<f32> = training_bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    // Generate input corpus with ChaCha20 (Box-Muller for Gaussian).
    let input: Vec<f32> = generate_normal_f32(input_seed, rows * cols as usize);

    // Write input corpus.
    let input_name = format!("input_n{rows}_d{cols}_seed{input_seed}.f32.bin");
    if let Err(e) = write_f32_bin(&out_dir.join(&input_name), &input) {
        eprintln!("failed to write {input_name}: {e}");
        return ExitCode::from(2);
    }
    println!("wrote {input_name}");

    let codec = Codec::new();

    for &bw in SUPPORTED_BIT_WIDTHS {
        let config = match CodecConfig::new(bw, codec_seed, cols, true) {
            Ok(c) => c,
            Err(err) => {
                eprintln!("CodecConfig::new failed bw={bw}: {err}");
                return ExitCode::from(2);
            }
        };
        let codebook = match Codebook::train(&training, &config) {
            Ok(cb) => cb,
            Err(err) => {
                eprintln!("Codebook::train failed bw={bw}: {err}");
                return ExitCode::from(2);
            }
        };

        let mut all_indices: Vec<u8> = Vec::with_capacity(rows * cols as usize);
        let mut all_residual: Vec<u8> = Vec::with_capacity(rows * cols as usize * 2);
        let mut all_decomp: Vec<f32> = Vec::with_capacity(rows * cols as usize);

        for row in 0..rows {
            let start = row * cols as usize;
            let v = &input[start..start + cols as usize];
            let cv = match codec.compress(v, &config, &codebook) {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("compress failed row={row} bw={bw}: {err}");
                    return ExitCode::from(2);
                }
            };
            all_indices.extend_from_slice(cv.indices());
            all_residual.extend_from_slice(cv.residual().unwrap_or(&[]));
            let dec = match codec.decompress(&cv, &config, &codebook) {
                Ok(d) => d,
                Err(err) => {
                    eprintln!("decompress failed row={row} bw={bw}: {err}");
                    return ExitCode::from(2);
                }
            };
            all_decomp.extend_from_slice(&dec);
        }

        let idx_name = format!("expected_indices_bw{bw}_seed{codec_seed}.u8.bin");
        if let Err(e) = fs::write(out_dir.join(&idx_name), &all_indices) {
            eprintln!("failed to write {idx_name}: {e}");
            return ExitCode::from(2);
        }
        println!("wrote {idx_name}");

        let res_name = format!("expected_residual_bw{bw}_seed{codec_seed}.bin");
        if let Err(e) = fs::write(out_dir.join(&res_name), &all_residual) {
            eprintln!("failed to write {res_name}: {e}");
            return ExitCode::from(2);
        }
        println!("wrote {res_name}");

        let dec_name = format!("expected_decompressed_bw{bw}_seed{codec_seed}.f32.bin");
        if let Err(e) = write_f32_bin(&out_dir.join(&dec_name), &all_decomp) {
            eprintln!("failed to write {dec_name}: {e}");
            return ExitCode::from(2);
        }
        println!("wrote {dec_name}");
    }

    ExitCode::SUCCESS
}

/// Generate `count` i.i.d. N(0,1) f32 values from a ChaCha20 seed using
/// Box-Muller transform (same algorithm as `RotationMatrix::build`).
fn generate_normal_f32(seed: u64, count: usize) -> Vec<f32> {
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let mut out = Vec::with_capacity(count);
    while out.len() < count {
        let u1 = next_f64(&mut rng);
        let u2 = next_f64(&mut rng);
        let r = (-2.0_f64 * u1.ln()).sqrt();
        let theta = std::f64::consts::TAU * u2;
        out.push((r * theta.cos()) as f32);
        if out.len() < count {
            out.push((r * theta.sin()) as f32);
        }
    }
    out
}

fn next_f64(rng: &mut ChaCha20Rng) -> f64 {
    // Uniform (0, 1) via 53-bit mantissa.
    let bits = (rng.next_u64() >> 11) | 1; // ensure non-zero
    (bits as f64) * (1.0_f64 / (1u64 << 53) as f64)
}

fn write_f32_bin(path: &Path, values: &[f32]) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    for v in values {
        writer.write_all(&v.to_le_bytes())?;
    }
    writer.flush()
}

fn usage(msg: &str) -> ExitCode {
    eprintln!("error: {msg}");
    eprintln!(
        "usage: dump_codec_fixture <input-seed> <codec-seed> <rows> <cols> \
         <training-f32-bin> <out-dir>"
    );
    ExitCode::from(1)
}
