//! Calibration quality gates (Phase 21).
//!
//! All tests are `#[ignore]` and run on the LFS-tracked fixtures.
//! They exercise the PR-speed fixture (`openai_1k_d768`) by default;
//! the main-only fixture (`openai_10k_d1536`) runs in the dedicated
//! `rust-ci/calibration` CI job via `--include-ignored`.
//!
//! Run locally:
//!   `cargo test -p tinyquant-bench -- --ignored`
//!
//! CI gate:
//!   `cargo test --workspace --all-features -- --ignored`

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use tinyquant_bench::calibration::{
    cosine_similarity, neighbor_recall::mean_recall_at_k, pearson::PearsonOnline, GoldCorpus,
};
use tinyquant_core::codec::{Codebook, Codec, CodecConfig};

// ── Threshold table ──────────────────────────────────────────────────────────
// Matches plan §Calibration thresholds.  These are >= lower bounds, not
// == — ISA-level nondeterminism (Phase 14 L4) cannot flake >=.
// Bit-exact gates live in the Phase 16 parity tests.

struct Threshold {
    rho_min: f64,
    recall_at_10_min: f64,
    ratio_min: f64,
}

const BW4_RESIDUAL: Threshold = Threshold {
    rho_min: 0.998,          // spec lower bound: bw=4 residual=on (§Calibration thresholds)
    recall_at_10_min: 0.95,  // spec lower bound: bw=4 residual=on
    ratio_min: 7.0,          // spec lower bound: bw=4 residual=on
};
const BW4_NO_RESIDUAL: Threshold = Threshold {
    rho_min: 0.98,           // spec lower bound: bw=4 residual=off (§Calibration thresholds)
    recall_at_10_min: 0.85,  // spec lower bound: bw=4 residual=off
    ratio_min: 8.0,          // spec lower bound: bw=4 residual=off
};
const BW2_RESIDUAL: Threshold = Threshold {
    rho_min: 0.95,           // spec lower bound: bw=2 residual=on (§Calibration thresholds)
    recall_at_10_min: 0.80,  // spec lower bound: bw=2 residual=on
    ratio_min: 14.0,         // spec lower bound: bw=2 residual=on
};
const BW8_RESIDUAL: Threshold = Threshold {
    rho_min: 0.999,          // spec lower bound: bw=8 residual=on (§Calibration thresholds)
    recall_at_10_min: 0.98,  // spec lower bound: bw=8 residual=on
    ratio_min: 4.0,          // spec lower bound: bw=8 residual=on
};
const BW2_NO_RESIDUAL: Threshold = Threshold {
    rho_min: 0.85,          // bw=2 no-residual: lower than residual path due to unrecovered quality loss
    recall_at_10_min: 0.60, // bw=2 no-residual: lower than residual path due to unrecovered quality loss
    ratio_min: 14.0,        // bw=2 no-residual: higher ratio than residual path (no residual bytes)
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
        .compress_batch(
            &corpus.vectors,
            corpus.rows,
            corpus.cols,
            &cfg,
            &cb,
        )
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
}

// ── PR-speed tests (openai_1k_d768) ─────────────────────────────────────────

#[test]
#[ignore = "calibration; run with --ignored or in rust-ci/calibration job"]
fn pr_speed_bw4_residual_on_meets_thresholds() {
    run_gate(GoldCorpus::load_openai_1k_d768(), 4, true, &BW4_RESIDUAL);
}

#[test]
#[ignore = "calibration; run with --ignored or in rust-ci/calibration job"]
fn pr_speed_bw4_residual_off_meets_thresholds() {
    run_gate(GoldCorpus::load_openai_1k_d768(), 4, false, &BW4_NO_RESIDUAL);
}

#[test]
#[ignore = "calibration; run with --ignored or in rust-ci/calibration job"]
fn pr_speed_bw2_residual_on_meets_thresholds() {
    run_gate(GoldCorpus::load_openai_1k_d768(), 2, true, &BW2_RESIDUAL);
}

#[test]
#[ignore = "calibration; run with --ignored or in rust-ci/calibration job"]
fn pr_speed_bw2_residual_off_meets_thresholds() {
    run_gate(GoldCorpus::load_openai_1k_d768(), 2, false, &BW2_NO_RESIDUAL);
}

#[test]
#[ignore = "calibration; run with --ignored or in rust-ci/calibration job"]
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
    run_gate(GoldCorpus::load_openai_10k_d1536(), 4, false, &BW4_NO_RESIDUAL);
}

#[test]
#[ignore = "calibration/full; run on main with: cargo test -- --ignored"]
fn full_bw2_residual_on_meets_thresholds() {
    run_gate(GoldCorpus::load_openai_10k_d1536(), 2, true, &BW2_RESIDUAL);
}

#[test]
#[ignore = "calibration/full; run on main with: cargo test -- --ignored"]
fn full_bw2_residual_off_meets_thresholds() {
    run_gate(GoldCorpus::load_openai_10k_d1536(), 2, false, &BW2_NO_RESIDUAL);
}

#[test]
#[ignore = "calibration/full; run on main with: cargo test -- --ignored"]
fn full_bw8_residual_on_meets_thresholds() {
    run_gate(GoldCorpus::load_openai_10k_d1536(), 8, true, &BW8_RESIDUAL);
}

// ── Passthrough identity (not ignore — always runs) ──────────────────────────

#[test]
fn passthrough_cosine_similarity_is_one() {
    // cosine_similarity with identical vectors must be 1.0.
    let v: Vec<f32> = (0..64).map(|i| (i as f32).sin()).collect();
    let c = cosine_similarity(&v, &v);
    assert!((c - 1.0).abs() < 1e-5, "expected 1.0, got {c}");
}
