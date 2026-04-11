//! SIMD kernel benchmarks (Phase 20).
//!
//! Measures quantize / dequantize / cosine / residual at dim = 1536
//! through the runtime-dispatch façade. Because Phase 20 wires
//! AVX2/NEON wrappers that currently delegate to scalar, these
//! numbers establish the framework baseline — once real intrinsic
//! paths land in Phase 22, the same benchmark file measures their
//! speedup without any test-plan rewrites.
//!
//! Run with: `cargo bench -p tinyquant-bench --features simd --bench simd_kernels`
//! Captured baselines live under `baselines/simd_kernels.json`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

use tinyquant_core::codec::simd_api;

const DIM: usize = 1536;
const ENTRIES_BW4: usize = 16;

fn make_entries() -> Vec<f32> {
    let mut v: Vec<f32> = (0..ENTRIES_BW4)
        .map(|i| (i as f32) * 0.125_f32 - 1.0_f32)
        .collect();
    // Already strictly ascending.
    v.sort_by(f32::total_cmp);
    v
}

fn make_values(dim: usize, offset: f32) -> Vec<f32> {
    (0..dim)
        .map(|i| ((i as f32).sin() * 0.7) + offset)
        .collect()
}

fn bench_quantize(c: &mut Criterion) {
    let entries = make_entries();
    let values = make_values(DIM, 0.1);
    let mut indices = vec![0_u8; DIM];
    let mut group = c.benchmark_group("simd/quantize");
    group.throughput(Throughput::Elements(DIM as u64));
    group.bench_function("dim_1536_bw4", |b| {
        b.iter(|| {
            simd_api::quantize_into(black_box(&entries), black_box(&values), &mut indices)
                .expect("quantize_into");
        });
    });
    group.finish();
}

fn bench_dequantize(c: &mut Criterion) {
    let entries = make_entries();
    let indices: Vec<u8> = (0..DIM)
        .map(|i| u8::try_from(i % ENTRIES_BW4).unwrap_or(0))
        .collect();
    let mut values = vec![0.0_f32; DIM];
    let mut group = c.benchmark_group("simd/dequantize");
    group.throughput(Throughput::Elements(DIM as u64));
    group.bench_function("dim_1536_bw4", |b| {
        b.iter(|| {
            simd_api::dequantize_into(black_box(&entries), black_box(&indices), &mut values)
                .expect("dequantize_into");
        });
    });
    group.finish();
}

fn bench_cosine(c: &mut Criterion) {
    let a = make_values(DIM, 0.1);
    let b_vec = make_values(DIM, -0.2);
    let mut group = c.benchmark_group("simd/cosine");
    group.throughput(Throughput::Elements(DIM as u64));
    group.bench_function("dim_1536", |b| {
        b.iter(|| {
            let s = simd_api::cosine(black_box(&a), black_box(&b_vec));
            black_box(s);
        });
    });
    group.finish();
}

fn bench_residual_encode(c: &mut Criterion) {
    let original = make_values(DIM, 0.1);
    let reconstructed = make_values(DIM, 0.0);
    let mut out = vec![0_u8; DIM * 2];
    let mut group = c.benchmark_group("simd/residual_encode");
    group.throughput(Throughput::Elements(DIM as u64));
    group.bench_function("dim_1536", |b| {
        b.iter(|| {
            simd_api::compute_residual_into(
                black_box(&original),
                black_box(&reconstructed),
                &mut out,
            );
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_quantize,
    bench_dequantize,
    bench_cosine,
    bench_residual_encode
);
criterion_main!(benches);
