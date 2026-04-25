//! Calibration helpers for score-fidelity and retrieval-recall gates (Phase 21).
//!
//! Shared between `tests/calibration.rs` (quality gates, all `#[ignore]`)
//! and `benches/batch_parallel.rs` (throughput scaling).
//!
//! # Fixture layout
//!
//! ```text
//! rust/crates/tinyquant-bench/fixtures/calibration/
//!   openai_10k_d1536.f32.bin   — 10 000 × 1536 f32 LE  (CI-main only)
//!   openai_1k_d768.f32.bin     — 1 000  × 768  f32 LE  (PR-speed)
//!   manifest.json              — SHA-256 per file
//! ```
//!
//! Fixtures are LFS-tracked.  CI jobs MUST set `with: {lfs: true}` on their
//! `actions/checkout` step (Phase 14 Lesson L2).

pub mod neighbor_recall;
pub mod pearson;

use std::path::{Path, PathBuf};

/// A loaded calibration corpus (raw f32 vectors, row-major).
pub struct GoldCorpus {
    /// Row-major f32 vectors; `vectors.len() == rows * cols`.
    pub vectors: Vec<f32>,
    /// Number of rows.
    pub rows: usize,
    /// Dimensionality.
    pub cols: usize,
}

impl GoldCorpus {
    /// Load `openai_10k_d1536.f32.bin` (full calibration fixture).
    ///
    /// # Panics
    ///
    /// Panics if the fixture file is missing or has wrong length.
    #[must_use]
    pub fn load_openai_10k_d1536() -> Self {
        Self::load_fixture("openai_10k_d1536.f32.bin", 10_000, 1536)
    }

    /// Load `openai_1k_d768.f32.bin` (PR-speed alternative, 10× smaller).
    ///
    /// # Panics
    ///
    /// Panics if the fixture file is missing or has wrong length.
    #[must_use]
    pub fn load_openai_1k_d768() -> Self {
        Self::load_fixture("openai_1k_d768.f32.bin", 1_000, 768)
    }

    fn load_fixture(name: &str, expected_rows: usize, expected_cols: usize) -> Self {
        let path = fixture_path(name);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| {
            panic!(
                "calibration fixture missing: {}\n  \
                 Regenerate with: python scripts/calibration/gen_openai_sample.py\n  \
                 Error: {e}",
                path.display()
            )
        });
        let expected_bytes = expected_rows * expected_cols * 4;
        assert_eq!(
            bytes.len(),
            expected_bytes,
            "fixture {name} has wrong length: got {} bytes, expected {expected_bytes}",
            bytes.len()
        );

        // Check manifest SHA-256 to detect fixture drift.
        let manifest = load_manifest();
        if let Some(expected_sha) = manifest.get(name) {
            let actual_sha = sha256_hex(&bytes);
            assert_eq!(
                &actual_sha, expected_sha,
                "fixture {name} SHA-256 mismatch — drift detected.\n\
                 Re-run: python scripts/calibration/gen_openai_sample.py\n\
                 or update manifest.json to match the current fixture."
            );
        }

        let vectors: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        Self {
            vectors,
            rows: expected_rows,
            cols: expected_cols,
        }
    }
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("calibration")
}

fn fixture_path(name: &str) -> PathBuf {
    fixture_dir().join(name)
}

fn load_manifest() -> std::collections::HashMap<String, String> {
    let path = fixture_dir().join("manifest.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!(
            "calibration: manifest.json not found at {}; \
             SHA-256 drift check skipped. \
             Re-run: python scripts/calibration/gen_openai_sample.py",
            path.display()
        );
        return std::collections::HashMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

/// Compute cosine similarity between two f32 slices of equal length.
///
/// Returns `0.0` if either vector has zero magnitude.
///
/// # Panics
///
/// Panics if `a.len() != b.len()`.
#[must_use]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "cosine_similarity: dimension mismatch");
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}
