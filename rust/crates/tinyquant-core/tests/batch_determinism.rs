//! Determinism gate for the parallel batch path (Phase 21).
//!
//! This test is **not** `#[ignore]` — it runs on every PR and is a hard
//! phase-exit gate. It verifies that `compress_batch` produces byte-identical
//! output across:
//!
//! - 10 repeated runs (same seed, same thread count)
//! - A sweep over thread counts [1, 2, 4, N_CPU] via local rayon pools
//!
//! See `batch.rs` §Parallelism determinism contract for the invariants.

use std::path::Path;
use tinyquant_core::codec::{Codebook, Codec, CodecConfig, CompressedVector, Parallelism};

fn load_training() -> Vec<f32> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/codebook/training_n10000_d64.f32.bin");
    let bytes = std::fs::read(&p)
        .unwrap_or_else(|e| panic!("training fixture missing: {e}"));
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Compress 64 vectors using `n_threads` via a local rayon pool.
///
/// Uses `Parallelism::Custom` with rayon's `into_par_iter` running inside
/// a dedicated `ThreadPool` so the global pool is not polluted.
fn full_pipeline(training: &[f32], n_threads: usize) -> Vec<CompressedVector> {
    let cfg = CodecConfig::new(4, 42, 64, true).unwrap();
    let cb = Codebook::train(training, &cfg).unwrap();
    let codec = Codec::new();

    let rows = 64_usize;
    let cols = 64_usize;
    let batch = &training[..rows * cols];

    if n_threads == 1 {
        // Serial path — no rayon dependency here.
        codec
            .compress_batch_with(batch, rows, cols, &cfg, &cb, Parallelism::Serial)
            .unwrap()
    } else {
        // Build a local pool and drive via Custom.
        // We use rayon only in the test binary (dev dependency), not in tinyquant-core.
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(n_threads)
            .build()
            .expect("failed to build local rayon pool");

        pool.install(|| {
            let par = Parallelism::Custom(|count, body| {
                use rayon::prelude::*;
                (0..count).into_par_iter().for_each(body);
            });
            codec
                .compress_batch_with(batch, rows, cols, &cfg, &cb, par)
                .unwrap()
        })
    }
}

fn cvs_eq(a: &[CompressedVector], b: &[CompressedVector], label: &str) {
    assert_eq!(a.len(), b.len(), "{label}: length mismatch");
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(
            x.indices(),
            y.indices(),
            "{label}: indices mismatch at row {i}"
        );
        assert_eq!(
            x.residual(),
            y.residual(),
            "{label}: residual mismatch at row {i}"
        );
    }
}

#[test]
fn full_pipeline_deterministic_across_repeat_runs() {
    let training = load_training();
    let first = full_pipeline(&training, 1);
    for run in 1..10 {
        let again = full_pipeline(&training, 1);
        cvs_eq(&first, &again, &format!("repeat-run {run}"));
    }
}

#[test]
fn full_pipeline_deterministic_across_thread_counts() {
    let training = load_training();
    let reference = full_pipeline(&training, 1); // serial baseline

    let n_cpu = rayon::current_num_threads().max(2);
    for &t in &[2_usize, 4, n_cpu] {
        if t > n_cpu {
            continue; // skip if the machine has fewer cores
        }
        let out = full_pipeline(&training, t);
        cvs_eq(&reference, &out, &format!("t={t}"));
    }
}
