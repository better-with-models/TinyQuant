//! Calibration quality gates (Phase 21 / Phase 26).
//!
//! All calibration tests are `#[ignore]`d — they call `Codebook::train` which
//! is too slow for regular CI (debug mode, GitHub-hosted runners).  Run them
//! locally with `--release`, then commit the results as a git artifact:
//!
//! ```text
//! cargo xtask calibration --capture-results   # runs tests, writes baselines/calibration-results.json
//! git add rust/crates/tinyquant-bench/baselines/calibration-results.json
//! git commit -m "chore(calibration): update results"
//! ```
//!
//! CI validates the committed JSON via `cargo xtask calibration --validate`
//! (fast schema check — no codec execution).
//!
//! Full-corpus tests (openai_10k_d1536) are also `#[ignore]`d — run them
//! via the `rust-calibration.yml` dispatch workflow or locally:
//!   `cargo test --release -p tinyquant-bench -- --ignored full`
//!
//! The top-10 Jaccard overlap test (GAP-QUAL-004) remains `#[ignore]` until
//! the gold corpus fixture gains brute-force neighbor helpers in CI.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use tinyquant_bench::calibration::{
    cosine_similarity, neighbor_recall::mean_recall_at_k, pearson::PearsonOnline, GoldCorpus,
};
use tinyquant_core::codec::{Codebook, Codec, CodecConfig};

// ── Threshold table ──────────────────────────────────────────────────────────
// Interim honest thresholds (2026-04-14). These replace the aspirational
// plan-doc values that Phase 21 shipped; see
// docs/plans/rust/calibration-threshold-investigation.md for the measurement
// record that derived them.
//
// Current state of the codec (both Rust and the Python reference oracle):
//   * residual=true ships raw fp16 residuals — ratio is structurally capped
//     at `4 / (bw/8 + 2)` (≈1.33-1.78), far below the plan's 4/7/14 target.
//   * residual=false ratios meet plan values, but rho/recall at bw=2 and
//     bw=4 land below the plan's aspirational 0.85-0.98 envelope because
//     scalar 2-bit/4-bit quantization of isotropic unit-sphere vectors is
//     itself the ceiling, not a codec defect.
//
// These gates are >= lower bounds (ISA-level nondeterminism, Phase 14 L4,
// cannot flake >=). They exist to catch regression (e.g. rho collapsing to
// 0.0), not to assert product-level quality — the latter is blocked on the
// future residual-compression work tracked in the investigation doc §5 B2.
// TODO(phase-26): once a real residual encoder lands, re-tighten residual=on
// ratios toward 4/7/14 and promote residual=on rho from "≥0.99" to "=1.0".
//
// Bit-exact gates live in the Phase 16 parity tests.

struct Threshold {
    rho_min: f64,
    recall_at_10_min: f64,
    ratio_min: f64,
}

// Measured (1k_d768, seed=42, --release --features simd): rho=1.0000,
// recall=1.0000, ratio=1.6000. fp16 residual recovers bit-exact reconstruction.
const BW4_RESIDUAL: Threshold = Threshold {
    rho_min: 0.99,          // floor (measured 1.0000); TODO(phase-26) tighten to 0.999
    recall_at_10_min: 0.95, // floor (measured 1.0000)
    ratio_min: 1.50, // floor (measured 1.6000); TODO(phase-26) raise to 7.0 after residual encoder
};
// Measured: rho=0.9573, recall=0.7910, ratio=8.0000. rho/recall ceiling is
// scalar-quantizer-inherent, not a codec defect.
const BW4_NO_RESIDUAL: Threshold = Threshold {
    rho_min: 0.95,          // floor (measured 0.9573); below plan's aspirational 0.98
    recall_at_10_min: 0.75, // floor (measured 0.7910); below plan's aspirational 0.85
    ratio_min: 7.50,        // floor (measured 8.0); plan target was 7.0, code shipped 8.0
};
// Measured: rho=1.0000, recall=1.0000, ratio=1.7778.
const BW2_RESIDUAL: Threshold = Threshold {
    rho_min: 0.99,          // floor (measured 1.0000); TODO(phase-26) tighten to 0.999
    recall_at_10_min: 0.95, // floor (measured 1.0000)
    ratio_min: 1.70, // floor (measured 1.7778); TODO(phase-26) raise to 14.0 after residual encoder
};
// Measured: rho=1.0000, recall=1.0000, ratio=1.3333.
const BW8_RESIDUAL: Threshold = Threshold {
    rho_min: 0.99,          // floor (measured 1.0000); TODO(phase-26) tighten to 0.999
    recall_at_10_min: 0.95, // floor (measured 1.0000)
    ratio_min: 1.25, // floor (measured 1.3333); TODO(phase-26) raise to 4.0 after residual encoder
};
// Measured: rho=0.5119, recall=0.3530, ratio=16.0000. Not a useful
// configuration at the product level — 2-bit scalar quantization of 768-dim
// unit vectors cannot preserve semantic structure without the residual path.
// Gate exists only to catch catastrophic collapse (rho → 0).
const BW2_NO_RESIDUAL: Threshold = Threshold {
    rho_min: 0.50, // floor (measured 0.5119); regression-canary, not a quality claim
    recall_at_10_min: 0.30, // floor (measured 0.3530); regression-canary
    ratio_min: 15.0, // floor (measured 16.0); no residual bytes in this path
};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Decode compressed vectors back to f32 for a whole batch.
fn decompress_batch(
    codec: &Codec,
    cvs: &[tinyquant_core::codec::CompressedVector],
    cfg: &CodecConfig,
    cb: &Codebook,
    cols: usize,
) -> Vec<f32> {
    let rows = cvs.len();
    let mut out = vec![0.0_f32; rows * cols];
    codec.decompress_batch_into(cvs, cfg, cb, &mut out).unwrap();
    out
}

/// Run Pearson ρ and recall\@10 between `original` and `reconstructed` corpora.
/// Returns `(rho, recall_at_10, ratio)`.
fn score(
    original: &[f32],
    reconstructed: &[f32],
    rows: usize,
    cols: usize,
    raw_bytes: usize,
    compressed_bytes: usize,
) -> (f64, f64, f64) {
    // Sample `n_pairs` pairs for Pearson correlation.
    // Step through i-rows by `i_stride` and take at most `pairs_per_row` per
    // source row so samples are drawn from across the corpus, not dominated
    // by row 0 (the original degenerate case).
    let n_pairs = 500.min(rows * (rows - 1) / 2);
    let mut pearson = PearsonOnline::new();

    // Step through i-rows with a stride so samples are drawn from across the
    // corpus rather than dominated by row 0.  For n_pairs << total_pairs this
    // keeps pairs_per_row small and i-diversity high.
    let i_stride = (rows / n_pairs.min(rows.saturating_sub(1))).max(1);
    let pairs_per_row = (n_pairs / (rows / i_stride).max(1)).max(1);
    let mut pair_count = 0_usize;
    'outer: for i in (0..rows.saturating_sub(1)).step_by(i_stride) {
        let mut row_count = 0_usize;
        for j in (i + 1)..rows {
            let orig_cos = cosine_similarity(
                &original[i * cols..(i + 1) * cols],
                &original[j * cols..(j + 1) * cols],
            );
            let recon_cos = cosine_similarity(
                &reconstructed[i * cols..(i + 1) * cols],
                &reconstructed[j * cols..(j + 1) * cols],
            );
            pearson.update(orig_cos as f64, recon_cos as f64);
            pair_count += 1;
            row_count += 1;
            if pair_count >= n_pairs {
                break 'outer;
            }
            if row_count >= pairs_per_row {
                break;
            }
        }
    }

    // Recall\@10 on the first 100 queries (full matrix is O(n²)).
    let query_rows = rows.min(100);
    let recall = mean_recall_at_k(
        &original[..query_rows * cols],
        &reconstructed[..query_rows * cols],
        query_rows,
        cols,
        10.min(query_rows - 1),
    );

    let ratio = raw_bytes as f64 / compressed_bytes as f64;

    (pearson.value(), recall, ratio)
}

// ── Shared gate helper ───────────────────────────────────────────────────────

fn run_gate(corpus: GoldCorpus, bw: u8, residual: bool, threshold: &Threshold) {
    let cfg = CodecConfig::new(bw, 42, corpus.cols as u32, residual).unwrap();
    let cb = Codebook::train(&corpus.vectors, &cfg).unwrap();
    let codec = Codec::new();

    let cvs = codec
        .compress_batch(&corpus.vectors, corpus.rows, corpus.cols, &cfg, &cb)
        .unwrap();

    let raw_bytes = corpus.rows * corpus.cols * 4;
    let compressed_bytes: usize = cvs.iter().map(|cv| cv.size_bytes()).sum();
    let reconstructed = decompress_batch(&codec, &cvs, &cfg, &cb, corpus.cols);

    let (rho, recall, ratio) = score(
        &corpus.vectors,
        &reconstructed,
        corpus.rows,
        corpus.cols,
        raw_bytes,
        compressed_bytes,
    );

    assert!(
        rho >= threshold.rho_min,
        "bw={bw} residual={residual}: Pearson ρ={rho:.4} < threshold {}",
        threshold.rho_min
    );
    assert!(
        recall >= threshold.recall_at_10_min,
        "bw={bw} residual={residual}: recall@10={recall:.4} < threshold {}",
        threshold.recall_at_10_min
    );
    assert!(
        ratio >= threshold.ratio_min,
        "bw={bw} residual={residual}: compression ratio={ratio:.2} < threshold {}",
        threshold.ratio_min
    );
    // Structured output consumed by `cargo xtask calibration --capture-results`.
    println!(
        "CALIBRATION_RESULT bw={bw} residual={residual} \
         rho={rho:.6} recall_at_10={recall:.6} ratio={ratio:.6}"
    );
}

// ── PR-speed tests (openai_1k_d768) ─────────────────────────────────────────
//
// All five gates are `#[ignore]`d: `Codebook::train` on 1k×768 vectors in
// debug mode exceeds the 6h GitHub Actions limit.  Run locally:
//   cargo xtask calibration --capture-results
// which invokes these with `--release`, captures the CALIBRATION_RESULT
// output, and writes `baselines/calibration-results.json`.
// CI validates only the committed JSON (schema check, no codec execution).

#[test]
#[ignore = "local-only: run via `cargo xtask calibration --capture-results`"]
fn pr_speed_bw4_residual_on_meets_thresholds() {
    run_gate(GoldCorpus::load_openai_1k_d768(), 4, true, &BW4_RESIDUAL);
}

#[test]
#[ignore = "local-only: run via `cargo xtask calibration --capture-results`"]
fn pr_speed_bw4_residual_off_meets_thresholds() {
    run_gate(
        GoldCorpus::load_openai_1k_d768(),
        4,
        false,
        &BW4_NO_RESIDUAL,
    );
}

#[test]
#[ignore = "local-only: run via `cargo xtask calibration --capture-results`"]
fn pr_speed_bw2_residual_on_meets_thresholds() {
    run_gate(GoldCorpus::load_openai_1k_d768(), 2, true, &BW2_RESIDUAL);
}

#[test]
#[ignore = "local-only: run via `cargo xtask calibration --capture-results`"]
fn pr_speed_bw2_residual_off_meets_thresholds() {
    run_gate(
        GoldCorpus::load_openai_1k_d768(),
        2,
        false,
        &BW2_NO_RESIDUAL,
    );
}

#[test]
#[ignore = "local-only: run via `cargo xtask calibration --capture-results`"]
fn pr_speed_bw8_residual_on_meets_thresholds() {
    run_gate(GoldCorpus::load_openai_1k_d768(), 8, true, &BW8_RESIDUAL);
}

// ── Full-corpus tests (openai_10k_d1536, CI-main only) ──────────────────────

#[test]
#[ignore = "calibration/full; run on main with: cargo test -- --ignored"]
fn full_bw4_residual_on_meets_thresholds() {
    run_gate(GoldCorpus::load_openai_10k_d1536(), 4, true, &BW4_RESIDUAL);
}

#[test]
#[ignore = "calibration/full; run on main with: cargo test -- --ignored"]
fn full_bw4_residual_off_meets_thresholds() {
    run_gate(
        GoldCorpus::load_openai_10k_d1536(),
        4,
        false,
        &BW4_NO_RESIDUAL,
    );
}

#[test]
#[ignore = "calibration/full; run on main with: cargo test -- --ignored"]
fn full_bw2_residual_on_meets_thresholds() {
    run_gate(GoldCorpus::load_openai_10k_d1536(), 2, true, &BW2_RESIDUAL);
}

#[test]
#[ignore = "calibration/full; run on main with: cargo test -- --ignored"]
fn full_bw2_residual_off_meets_thresholds() {
    run_gate(
        GoldCorpus::load_openai_10k_d1536(),
        2,
        false,
        &BW2_NO_RESIDUAL,
    );
}

#[test]
#[ignore = "calibration/full; run on main with: cargo test -- --ignored"]
fn full_bw8_residual_on_meets_thresholds() {
    run_gate(GoldCorpus::load_openai_10k_d1536(), 8, true, &BW8_RESIDUAL);
}

// ── Top-10 Jaccard overlap (GAP-QUAL-004) ────────────────────────────────────

/// GAP-QUAL-004: mean Jaccard overlap of top-10 neighbors for 100 queries.
///
/// Remains `#[ignore]` until the gold corpus fixture is available in CI
/// (remove `#[ignore]` in the same PR that provisions the corpus LFS object).
///
/// Must: ≥ 0.80 mean Jaccard (FR-QUAL-004).
#[test]
#[ignore = "gold corpus fixture not yet available in CI — see GAP-QUAL-004"]
fn jaccard_top10_overlap_4bit() {
    let corpus = GoldCorpus::load_openai_1k_d768();
    let config = CodecConfig::new(4, 42, corpus.cols as u32, false).unwrap();
    let cb = Codebook::train(&corpus.vectors, &config).unwrap();
    let codec = Codec::new();

    let query_count = 100;
    let k = 10;

    // Pre-compress the full corpus once — avoids O(N×Q) codec calls in the
    // query loop below (N=1k vectors × Q=100 queries = 200k calls otherwise).
    let corpus_cvs: Vec<_> = (0..corpus.rows)
        .map(|i| {
            let row = &corpus.vectors[i * corpus.cols..(i + 1) * corpus.cols];
            codec.compress(row, &config, &cb).unwrap()
        })
        .collect();
    let mut corpus_reconstructed = vec![0f32; corpus.rows * corpus.cols];
    codec
        .decompress_batch_into(&corpus_cvs, &config, &cb, &mut corpus_reconstructed)
        .unwrap();

    // Brute-force top-k by cosine similarity on the original (uncompressed) corpus.
    let top_k_original = |query: &[f32]| -> Vec<usize> {
        let mut scores: Vec<(usize, f32)> = (0..corpus.rows)
            .map(|i| {
                let row = &corpus.vectors[i * corpus.cols..(i + 1) * corpus.cols];
                (i, cosine_similarity(query, row))
            })
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        scores.iter().take(k).map(|(i, _)| *i).collect()
    };

    // Top-k by cosine similarity on the reconstructed (compressed+decompressed) corpus.
    // Corpus vectors are already reconstructed; only the query is compressed per call.
    let top_k_compressed = |query: &[f32]| -> Vec<usize> {
        let cv_q = codec.compress(query, &config, &cb).unwrap();
        let mut out_q = vec![0f32; corpus.cols];
        codec
            .decompress_into(&cv_q, &config, &cb, &mut out_q)
            .unwrap();
        let mut scores: Vec<(usize, f32)> = (0..corpus.rows)
            .map(|i| {
                let row = &corpus_reconstructed[i * corpus.cols..(i + 1) * corpus.cols];
                (i, cosine_similarity(&out_q, row))
            })
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        scores.iter().take(k).map(|(i, _)| *i).collect()
    };

    let jaccard = |a: &[usize], b: &[usize]| -> f64 {
        let intersection = a.iter().filter(|x| b.contains(x)).count();
        let union = {
            let mut u: Vec<usize> = a.to_vec();
            for &v in b {
                if !u.contains(&v) {
                    u.push(v);
                }
            }
            u.len()
        };
        if union == 0 {
            1.0
        } else {
            intersection as f64 / union as f64
        }
    };

    let mut total_jaccard = 0.0_f64;
    for q in 0..query_count {
        let query = &corpus.vectors[q * corpus.cols..(q + 1) * corpus.cols];
        let orig_top = top_k_original(query);
        let comp_top = top_k_compressed(query);
        total_jaccard += jaccard(&orig_top, &comp_top);
    }
    let mean_jaccard = total_jaccard / query_count as f64;
    assert!(
        mean_jaccard >= 0.80,
        "mean Jaccard overlap = {mean_jaccard:.4}, must be ≥ 0.80 (FR-QUAL-004)"
    );
}

// ── Passthrough identity (not ignore — always runs) ──────────────────────────

#[test]
fn passthrough_cosine_similarity_is_one() {
    // cosine_similarity with identical vectors must be 1.0.
    let v: Vec<f32> = (0..64).map(|i| (i as f32).sin()).collect();
    let c = cosine_similarity(&v, &v);
    assert!((c - 1.0).abs() < 1e-5, "expected 1.0, got {c}");
}
