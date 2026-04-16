//! Parity tests: GPU `cosine_topk` must agree with CPU `BruteForceBackend`.
//!
//! Tests skip automatically when no wgpu adapter is found (headless CI).
//! The Jaccard-overlap threshold of ≥ 0.95 matches the Phase 27.5
//! acceptance criterion.
//!
//! **Pre-normalisation:** the `cosine_topk` kernel computes dot products and
//! therefore requires unit-norm vectors.  Both the GPU corpus/query and the
//! CPU ingest entries are L2-normalised before use so that dot product ==
//! cosine similarity, giving both backends the same ranking.
//!
//! **Id convention:** GPU row indices (0-based integers rendered as strings)
//! match the numeric ids passed to `BruteForceBackend::ingest`, so the two
//! top-k sets are directly comparable.

use std::sync::Arc;

use tinyquant_bruteforce::BruteForceBackend;
use tinyquant_core::backend::SearchBackend;
use tinyquant_gpu_wgpu::WgpuBackend;

/// Returns `Some(WgpuBackend)` if an adapter is present, `None` otherwise.
fn try_gpu_backend() -> Option<WgpuBackend> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    rt.block_on(WgpuBackend::new()).ok()
}

/// In-place L2 normalisation.  No-op for zero vectors.
fn normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0_f32 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
}

/// GPU top-10 results must overlap with CPU top-10 by ≥ 95 % (Jaccard).
///
/// Corpus: 1 000 unit-norm FP32 vectors, dim=64.
/// 10 unit-norm query vectors drawn from outside the corpus.
#[test]
fn gpu_top10_overlaps_cpu_top10() {
    let Some(mut backend) = try_gpu_backend() else {
        eprintln!("SKIP gpu_top10_overlaps_cpu_top10: no wgpu adapter");
        return;
    };

    let dim = 64_usize;
    let n_corpus = 1_000_usize;
    let n_queries = 10_usize;

    // Build and normalise corpus rows.
    let mut corpus_fp32: Vec<f32> = (0..n_corpus * dim)
        .map(|i| (i as f32 * 0.001_f32).sin())
        .collect();
    for row in 0..n_corpus {
        normalize(&mut corpus_fp32[row * dim..(row + 1) * dim]);
    }

    // Upload normalised corpus to GPU.
    backend
        .prepare_corpus_for_device(&corpus_fp32, n_corpus, dim)
        .expect("prepare_corpus_for_device");

    // CPU reference — same normalised vectors, numeric string ids.
    let mut cpu = BruteForceBackend::new();
    let entries: Vec<(Arc<str>, Vec<f32>)> = (0..n_corpus)
        .map(|i| {
            let vec = corpus_fp32[i * dim..(i + 1) * dim].to_vec();
            let id: Arc<str> = Arc::from(i.to_string().as_str());
            (id, vec)
        })
        .collect();
    cpu.ingest(&entries).expect("cpu ingest");

    for q in 0..n_queries {
        // Build and normalise query.
        let mut query: Vec<f32> = (0..dim)
            .map(|j| ((q * dim + j) as f32 * 0.007_f32).cos())
            .collect();
        normalize(&mut query);

        let gpu_results = backend.cosine_topk(&query, 10).expect("cosine_topk");
        let cpu_results = cpu.search(&query, 10).expect("cpu search");

        let gpu_ids: std::collections::HashSet<Arc<str>> =
            gpu_results.into_iter().map(|r| r.vector_id).collect();
        let cpu_ids: std::collections::HashSet<Arc<str>> =
            cpu_results.into_iter().map(|r| r.vector_id).collect();

        let intersection = gpu_ids.intersection(&cpu_ids).count();
        let union = gpu_ids.union(&cpu_ids).count();
        let jaccard = if union == 0 { 1.0_f64 } else { intersection as f64 / union as f64 };

        assert!(
            jaccard >= 0.95,
            "query {q}: GPU vs CPU top-10 Jaccard = {jaccard:.3}, must be ≥ 0.95"
        );
    }
}

/// GPU `cosine_topk` returns exactly `top_k` results, or all rows when
/// `top_k > n_corpus`.
#[test]
fn gpu_cosine_topk_returns_correct_count() {
    let Some(mut backend) = try_gpu_backend() else {
        eprintln!("SKIP gpu_cosine_topk_returns_correct_count: no wgpu adapter");
        return;
    };

    let dim = 32_usize;
    let n_corpus = 20_usize;
    let mut corpus: Vec<f32> = (0..n_corpus * dim).map(|i| i as f32 * 0.01_f32).collect();
    // Normalise each row so the kernel produces meaningful scores.
    for row in 0..n_corpus {
        normalize(&mut corpus[row * dim..(row + 1) * dim]);
    }
    backend
        .prepare_corpus_for_device(&corpus, n_corpus, dim)
        .expect("prepare_corpus_for_device");

    let mut query: Vec<f32> = vec![1.0_f32; dim];
    normalize(&mut query);

    // top_k ≤ n_corpus — must return exactly top_k.
    let results = backend.cosine_topk(&query, 10).expect("cosine_topk k=10");
    assert_eq!(
        results.len(),
        10,
        "must return exactly 10 results from a {n_corpus}-row corpus"
    );

    // top_k > n_corpus — must return all rows.
    let results_all = backend.cosine_topk(&query, 50).expect("cosine_topk k=50");
    assert_eq!(
        results_all.len(),
        n_corpus,
        "top_k > n_corpus must return all {n_corpus} rows"
    );
}
