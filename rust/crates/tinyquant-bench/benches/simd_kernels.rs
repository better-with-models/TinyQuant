//! SIMD kernel benchmarks (Phase 20).
//!
//! Measures quantize / dequantize / cosine / residual encode / residual
//! decode at several dimensions through both the runtime-dispatch
//! façade (`simd_api::*`) and the scalar reference (`simd_api::scalar::*`).
//! Because Phase 20 wires AVX2/NEON wrappers that currently delegate
//! to scalar, these numbers establish the framework baseline — once
//! real intrinsic paths land in Phase 22, the same benchmark file
//! measures their speedup without any test-plan rewrites.
//!
//! Dimension sweep: 64, 256, 768, 1536 (typical embedding widths).
//!
//! Run with: `cargo bench -p tinyquant-bench --features simd --bench simd_kernels`
//! Captured baselines live under `baselines/simd_kernels.json`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use tinyquant_core::codec::simd_api;

const DIMS: &[usize] = &[64, 256, 768, 1536];
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
    let mut group = c.benchmark_group("simd/quantize");
    for &dim in DIMS {
        let values = make_values(dim, 0.1);
        let mut indices = vec![0_u8; dim];
        group.throughput(Throughput::Elements(dim as u64));
        group.bench_with_input(BenchmarkId::new("dispatched_bw4", dim), &dim, |b, _| {
            b.iter(|| {
                simd_api::quantize_into(black_box(&entries), black_box(&values), &mut indices)
                    .expect("quantize_into");
            });
        });
        group.bench_with_input(BenchmarkId::new("scalar_bw4", dim), &dim, |b, _| {
            b.iter(|| {
                simd_api::scalar::quantize_into(
                    black_box(&entries),
                    black_box(&values),
                    &mut indices,
                )
                .expect("scalar quantize_into");
            });
        });
    }
    group.finish();
}

fn bench_dequantize(c: &mut Criterion) {
    let entries = make_entries();
    let mut group = c.benchmark_group("simd/dequantize");
    for &dim in DIMS {
        let indices: Vec<u8> = (0..dim)
            .map(|i| u8::try_from(i % ENTRIES_BW4).unwrap_or(0))
            .collect();
        let mut values = vec![0.0_f32; dim];
        group.throughput(Throughput::Elements(dim as u64));
        group.bench_with_input(BenchmarkId::new("dispatched_bw4", dim), &dim, |b, _| {
            b.iter(|| {
                simd_api::dequantize_into(black_box(&entries), black_box(&indices), &mut values)
                    .expect("dequantize_into");
            });
        });
        group.bench_with_input(BenchmarkId::new("scalar_bw4", dim), &dim, |b, _| {
            b.iter(|| {
                simd_api::scalar::dequantize_into(
                    black_box(&entries),
                    black_box(&indices),
                    &mut values,
                )
                .expect("scalar dequantize_into");
            });
        });
    }
    group.finish();
}

fn bench_cosine(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd/cosine");
    for &dim in DIMS {
        let a = make_values(dim, 0.1);
        let b_vec = make_values(dim, -0.2);
        group.throughput(Throughput::Elements(dim as u64));
        group.bench_with_input(BenchmarkId::new("dispatched", dim), &dim, |b, _| {
            b.iter(|| {
                let s = simd_api::cosine(black_box(&a), black_box(&b_vec));
                black_box(s);
            });
        });
        group.bench_with_input(BenchmarkId::new("scalar", dim), &dim, |b, _| {
            b.iter(|| {
                let s = simd_api::scalar::cosine(black_box(&a), black_box(&b_vec));
                black_box(s);
            });
        });
    }
    group.finish();
}

fn bench_residual_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd/residual_encode");
    for &dim in DIMS {
        let original = make_values(dim, 0.1);
        let reconstructed = make_values(dim, 0.0);
        let mut out = vec![0_u8; dim * 2];
        group.throughput(Throughput::Elements(dim as u64));
        group.bench_with_input(BenchmarkId::new("dispatched", dim), &dim, |b, _| {
            b.iter(|| {
                simd_api::compute_residual_into(
                    black_box(&original),
                    black_box(&reconstructed),
                    &mut out,
                );
            });
        });
        group.bench_with_input(BenchmarkId::new("scalar", dim), &dim, |b, _| {
            b.iter(|| {
                simd_api::scalar::compute_residual_into(
                    black_box(&original),
                    black_box(&reconstructed),
                    &mut out,
                );
            });
        });
    }
    group.finish();
}

fn bench_residual_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd/residual_decode");
    for &dim in DIMS {
        let original = make_values(dim, 0.1);
        let reconstructed = make_values(dim, 0.0);
        let mut residual = vec![0_u8; dim * 2];
        simd_api::scalar::compute_residual_into(&original, &reconstructed, &mut residual);
        group.throughput(Throughput::Elements(dim as u64));
        group.bench_with_input(BenchmarkId::new("dispatched", dim), &dim, |b, _| {
            b.iter_batched(
                || reconstructed.clone(),
                |mut acc| {
                    simd_api::apply_residual_into(&mut acc, black_box(&residual))
                        .expect("apply_residual_into");
                    black_box(acc);
                },
                criterion::BatchSize::SmallInput,
            );
        });
        group.bench_with_input(BenchmarkId::new("scalar", dim), &dim, |b, _| {
            b.iter_batched(
                || reconstructed.clone(),
                |mut acc| {
                    simd_api::scalar::apply_residual_into(&mut acc, black_box(&residual))
                        .expect("scalar apply_residual_into");
                    black_box(acc);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_quantize,
    bench_dequantize,
    bench_cosine,
    bench_residual_encode,
    bench_residual_decode,
);
criterion_main!(benches);
