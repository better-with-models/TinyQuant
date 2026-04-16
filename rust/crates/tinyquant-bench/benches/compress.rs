//! Compress benchmark: `compress_prepared` (warm rotation) vs cold per-call path.
//!
//! Phase 26 deliverable — validates that the rotation warm-hit budget (≤ 40 ns
//! amortised overhead per vector) is met and surfaces the speedup from caching
//! the `RotationMatrix` across calls.
//!
//! Run with:
//!   `cargo bench -p tinyquant-bench --bench compress`

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::indexing_slicing
)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tinyquant_core::codec::{Codebook, Codec, CodecConfig, PreparedCodec};

const ROWS: usize = 1024;
const DIM: usize = 768;

fn bench_corpus(seed: u64, rows: usize, dim: usize) -> Vec<f32> {
    use rand_chacha::rand_core::{RngCore, SeedableRng};
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    (0..rows * dim)
        .map(|_| {
            let bits = rng.next_u32();
            (bits as f32 / u32::MAX as f32) * 2.0 - 1.0
        })
        .collect()
}

fn bench_codebook(config: &CodecConfig) -> Codebook {
    let training = bench_corpus(42, 2048, config.dimension() as usize);
    Codebook::train(&training, config).expect("bench codebook must train")
}

/// Phase 26: compare warm-rotation `compress_prepared` vs cold per-call compress.
fn bench_compress_prepared(c: &mut Criterion) {
    let config = CodecConfig::new(4, 42, DIM as u32, false).unwrap();
    let codebook = bench_codebook(&config);
    let prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();
    let vectors = bench_corpus(99, ROWS, DIM);

    c.bench_function("compress_prepared_1024x768", |b| {
        b.iter(|| {
            for row in 0..ROWS {
                let v = &vectors[row * DIM..(row + 1) * DIM];
                let _ = black_box(Codec::new().compress_prepared(black_box(v), &prepared));
            }
        });
    });

    c.bench_function("compress_cold_1024x768", |b| {
        b.iter(|| {
            for row in 0..ROWS {
                let v = &vectors[row * DIM..(row + 1) * DIM];
                let _ = black_box(Codec::new().compress(black_box(v), &config, &codebook));
            }
        });
    });
}

criterion_group!(benches, bench_compress_prepared);
criterion_main!(benches);
