//! Generate the golden corpus fixture files for Phase 17 tests.
//!
//! Generates:
//! - `tests/fixtures/codec_file/golden_100.tqcv` — 100 vectors, dim=64, bw=4
//! - `tests/fixtures/codec_file/golden_100_indices.bin` — raw u8 index bytes (100×64)
//!
//! Usage:
//! ```
//! cargo run -p tinyquant-io --example gen_corpus_fixture
//! ```
//!
//! Indices are generated deterministically from `ChaCha20Rng::seed_from_u64(17)`.

use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use tinyquant_core::codec::CompressedVector;
use tinyquant_io::codec_file::CodecFileWriter;

const COUNT: usize = 100;
const DIM: u32 = 64;
const BIT_WIDTH: u8 = 4;
const CONFIG_HASH: &str = "phase17-golden";
const SEED: u64 = 17;

fn main() {
    let fixture_dir = find_fixture_dir();
    let tqcv_path = fixture_dir.join("golden_100.tqcv");
    let idx_path = fixture_dir.join("golden_100_indices.bin");

    let mut rng = ChaCha20Rng::seed_from_u64(SEED);
    let max_val = (1u16 << BIT_WIDTH) - 1;

    let mut all_indices: Vec<u8> = Vec::with_capacity(COUNT * DIM as usize);
    let mut cvs: Vec<CompressedVector> = Vec::with_capacity(COUNT);

    for _ in 0..COUNT {
        let indices: Vec<u8> = (0..DIM as usize)
            .map(|_| (rng.next_u32() as u8) & max_val as u8)
            .collect();
        all_indices.extend_from_slice(&indices);
        let cv = CompressedVector::new(
            indices.into_boxed_slice(),
            None,
            Arc::from(CONFIG_HASH),
            DIM,
            BIT_WIDTH,
        )
        .unwrap_or_else(|e| {
            eprintln!("error building CompressedVector: {e}");
            std::process::exit(1);
        });
        cvs.push(cv);
    }

    // Write TQCV file
    let mut writer = CodecFileWriter::create(&tqcv_path, CONFIG_HASH, DIM, BIT_WIDTH, false, &[])
        .unwrap_or_else(|e| {
            eprintln!("error creating {}: {e}", tqcv_path.display());
            std::process::exit(1);
        });
    for cv in &cvs {
        writer.append(cv).unwrap_or_else(|e| {
            eprintln!("error appending: {e}");
            std::process::exit(1);
        });
    }
    writer.finalize().unwrap_or_else(|e| {
        eprintln!("error finalizing: {e}");
        std::process::exit(1);
    });
    println!("wrote {}", tqcv_path.display());

    // Write raw indices binary
    let mut idx_file = std::fs::File::create(&idx_path).unwrap_or_else(|e| {
        eprintln!("error creating {}: {e}", idx_path.display());
        std::process::exit(1);
    });
    idx_file.write_all(&all_indices).unwrap_or_else(|e| {
        eprintln!("error writing indices: {e}");
        std::process::exit(1);
    });
    println!("wrote {}", idx_path.display());
}

fn find_fixture_dir() -> PathBuf {
    // When run as `cargo run`, cwd is the crate root.
    // The fixture directory is relative to the crate root.
    let candidate = PathBuf::from("tests/fixtures/codec_file");
    if candidate.is_dir() {
        return candidate;
    }
    // Fallback: look relative to the workspace root.
    let candidate2 = PathBuf::from("crates/tinyquant-io/tests/fixtures/codec_file");
    if candidate2.is_dir() {
        return candidate2;
    }
    eprintln!(
        "cannot find fixture directory; expected \
         'tests/fixtures/codec_file' relative to the crate root, or \
         'crates/tinyquant-io/tests/fixtures/codec_file' relative to the workspace root"
    );
    std::process::exit(1);
}
