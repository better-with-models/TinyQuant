//! Top-k recall helper (Phase 21).
//!
//! Measures retrieval quality: given the true top-k neighbours by cosine
//! similarity in the original space, what fraction of them appear in the
//! top-k retrieved from the compressed space?
//!
//! This is the standard recall\@k metric used in approximate nearest-
//! neighbour benchmarks (e.g. ann-benchmarks.com).
//!
//! # Complexity
//!
//! `O(n^2 * d)` — suitable only for small calibration corpora (`n ≤ 10^4`).
//! Large-scale ANNS is out of scope for Phase 21.

/// Compute recall\@k for a corpus of original and reconstructed vectors.
///
/// For each query (every row), the true top-k neighbours are computed from
/// `original_rows`, and the retrieved top-k from `reconstructed_rows`.
/// Returns the mean recall\@k over all queries (range `[0.0, 1.0]`).
///
/// `original_rows` and `reconstructed_rows` must have the same layout:
/// `rows × cols` f32, row-major.
///
/// # Panics
///
/// Panics if the dimensions do not match or `k >= rows`.
#[must_use]
pub fn mean_recall_at_k(
    original_rows: &[f32],
    reconstructed_rows: &[f32],
    rows: usize,
    cols: usize,
    k: usize,
) -> f64 {
    assert_eq!(original_rows.len(), rows * cols, "orig dimension mismatch");
    assert_eq!(
        reconstructed_rows.len(),
        rows * cols,
        "recon dimension mismatch"
    );
    assert!(k < rows, "k must be less than rows");

    // Pre-compute all pairwise cosines for original and reconstructed.
    let orig_cos = pairwise_cosine(original_rows, rows, cols);
    let recon_cos = pairwise_cosine(reconstructed_rows, rows, cols);

    let mut total_recall = 0.0_f64;

    for query in 0..rows {
        let true_top_k = top_k_neighbours(&orig_cos, query, rows, k);
        let retr_top_k = top_k_neighbours(&recon_cos, query, rows, k);
        let hits = true_top_k.iter().filter(|n| retr_top_k.contains(n)).count();
        #[allow(clippy::cast_precision_loss)]
        let recall = hits as f64 / k as f64;
        total_recall += recall;
    }

    #[allow(clippy::cast_precision_loss)]
    let mean = total_recall / rows as f64;
    mean
}

/// Cosine similarity between row `a` and row `b`.
fn cosine(mat: &[f32], cols: usize, a: usize, b: usize) -> f32 {
    let row_a = &mat[a * cols..(a + 1) * cols];
    let row_b = &mat[b * cols..(b + 1) * cols];
    let dot: f32 = row_a.iter().zip(row_b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = row_a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = row_b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Compute the full n×n cosine matrix (upper-triangle mirrored).
fn pairwise_cosine(mat: &[f32], rows: usize, cols: usize) -> Vec<f32> {
    let mut out = vec![0.0_f32; rows * rows];
    for i in 0..rows {
        out[i * rows + i] = 1.0;
        for j in (i + 1)..rows {
            let c = cosine(mat, cols, i, j);
            out[i * rows + j] = c;
            out[j * rows + i] = c;
        }
    }
    out
}

/// Return the indices of the top-k neighbours of `query` (excluding self).
fn top_k_neighbours(cos_mat: &[f32], query: usize, rows: usize, k: usize) -> Vec<usize> {
    let mut scores: Vec<(usize, f32)> = (0..rows)
        .filter(|&j| j != query)
        .map(|j| (j, cos_mat[query * rows + j]))
        .collect();
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    scores.into_iter().take(k).map(|(j, _)| j).collect()
}

#[cfg(test)]
mod tests {
    use super::mean_recall_at_k;

    /// Identity: reconstructed == original → recall = 1.0.
    #[test]
    fn perfect_reconstruction_gives_recall_one() {
        let data: Vec<f32> = (0..4 * 8)
            .map(|i| {
                #[allow(clippy::cast_precision_loss)]
                let v = (i as f32).sin();
                v
            })
            .collect();
        let recall = mean_recall_at_k(&data, &data, 4, 8, 2);
        assert!(
            (recall - 1.0).abs() < 1e-6,
            "expected recall=1.0, got {recall}"
        );
    }
}
