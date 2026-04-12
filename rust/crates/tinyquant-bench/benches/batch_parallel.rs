//! Parallel batch compress scaling benchmark (Phase 21).
//!
//! Measures `compress_batch_with` throughput at thread counts
//! `[1, 2, 4, 8]` (capped at the host CPU count) against a
//! deterministic synthetic corpus.
//!
//! Run with:
//!   `cargo bench -p tinyquant-bench --bench batch_parallel`
//!
//! Each group is `batch_parallel/t{threads}/d{dim}_n{rows}`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::indexing_slicing
)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rayon::prelude::*;
use tinyquant_core::codec::{Codebook, Codec, CodecConfig, Parallelism};

const ROWS: usize = 256;
const THREAD_COUNTS: &[usize] = &[1, 2, 4, 8];
const DIMS: &[usize] = &[64, 256, 768, 1536];

fn make_corpus(seed: u64, rows: usize, cols: usize) -> Vec<f32> {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    (0..rows * cols)
        .map(|_| {
            let bits = rng.next_u32();
            (bits as f32 / u32::MAX as f32) * 2.0 - 1.0
        })
        .collect()
}

fn bench_batch_parallel(c: &mut Criterion) {
    let max_threads = rayon::current_num_threads().min(8);

    for &threads in THREAD_COUNTS {
        if threads > max_threads {
            continue;
        }

        let pool = if threads > 1 {
            Some(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .unwrap(),
            )
        } else {
            None
        };

        for &dim in DIMS {
            let training = make_corpus(42, 10_000.min(ROWS * 4), dim);
            let cfg = CodecConfig::new(4, 42, dim as u32, true).unwrap();
            let cb = Codebook::train(&training, &cfg).unwrap();
            let codec = Codec::new();
            let batch = make_corpus(99, ROWS, dim);

            let mut group = c.benchmark_group(format!("batch_parallel/d{dim}_n{ROWS}"));
            group.throughput(Throughput::Elements(ROWS as u64));

            if threads == 1 {
                group.bench_with_input(BenchmarkId::new("t", threads), &threads, |b, _| {
                    b.iter(|| {
                        codec
                            .compress_batch_with(&batch, ROWS, dim, &cfg, &cb, Parallelism::Serial)
                            .unwrap()
                    });
                });
            } else {
                let pool = pool.as_ref().unwrap();
                group.bench_with_input(BenchmarkId::new("t", threads), &threads, |b, _| {
                    pool.install(|| {
                        b.iter(|| {
                            codec
                                .compress_batch_with(
                                    &batch,
                                    ROWS,
                                    dim,
                                    &cfg,
                                    &cb,
                                    Parallelism::Custom(|count, body| {
                                        (0..count).into_par_iter().for_each(body);
                                    }),
                                )
                                .unwrap()
                        })
                    });
                });
            }

            group.finish();
        }
    }
}

criterion_group!(benches, bench_batch_parallel);
criterion_main!(benches);
