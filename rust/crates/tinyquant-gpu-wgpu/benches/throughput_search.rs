//! FR-GPU-004 throughput evidence: GPU `cosine_topk` for 10 000 vectors
//! dim=1536 must complete in ≤ 5 ms median on RTX 3060-class hardware.
//!
//! Run with:
//!   cargo bench -p tinyquant-gpu-wgpu --bench throughput_search
//!
//! On headless CI (no adapter) the benchmark function exits early and logs a
//! skip message.  The bench does not fail — it simply produces no samples.

use criterion::{criterion_group, criterion_main, Criterion};
use tinyquant_gpu_wgpu::WgpuBackend;

fn bench_cosine_topk(c: &mut Criterion) {
    // Construct backend synchronously via a one-shot runtime.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    let mut backend = match rt.block_on(WgpuBackend::new()) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("SKIP throughput_search bench: no wgpu adapter");
            return;
        }
    };

    let dim = 1_536_usize;
    let n_corpus = 10_000_usize;

    // Corpus: n_corpus × dim FP32 values, values in [−1, 1].
    let corpus: Vec<f32> = (0..n_corpus * dim)
        .map(|i| (i as f32 * 0.001_f32).sin())
        .collect();

    // Pre-normalise each corpus row so the dot product equals cosine similarity,
    // matching the documented pre-normalisation contract for the search kernel.
    let mut corpus = corpus;
    for row in 0..n_corpus {
        let start = row * dim;
        let end = start + dim;
        let norm: f32 = corpus[start..end].iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0_f32 {
            corpus[start..end].iter_mut().for_each(|x| *x /= norm);
        }
    }

    backend
        .prepare_corpus_for_device(&corpus, n_corpus, dim)
        .expect("prepare_corpus_for_device");

    // Unit-norm query vector (pre-normalised as required by the kernel).
    let scale = 1.0_f32 / (dim as f32).sqrt();
    let query: Vec<f32> = vec![scale; dim];

    c.bench_function("cosine_topk_10k_dim1536", |b| {
        b.iter(|| {
            backend.cosine_topk(&query, 10).expect("cosine_topk");
        });
    });
}

criterion_group!(benches, bench_cosine_topk);
criterion_main!(benches);
