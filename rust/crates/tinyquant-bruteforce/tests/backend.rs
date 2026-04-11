//! Behaviour tests for `BruteForceBackend`.
//!
//! Every test row from the plan's behaviour matrix is covered here.

use std::sync::Arc;

use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use tinyquant_bruteforce::BruteForceBackend;
use tinyquant_core::backend::SearchBackend;
use tinyquant_core::errors::BackendError;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_id(s: &str) -> Arc<str> {
    Arc::from(s)
}

fn unit_vec(dim: usize, hot: usize) -> Vec<f32> {
    let mut v = vec![0.0_f32; dim];
    v[hot] = 1.0;
    v
}

fn random_vec(rng: &mut ChaCha20Rng, dim: usize) -> Vec<f32> {
    use rand::Rng;
    (0..dim).map(|_| rng.gen_range(-1.0_f32..1.0_f32)).collect()
}

// ---------------------------------------------------------------------------
// 1. ingest_then_search_returns_top_k_descending
// ---------------------------------------------------------------------------

#[test]
fn ingest_then_search_returns_top_k_descending() {
    let mut b = BruteForceBackend::new();
    // Three orthogonal unit vectors in dim=3
    b.ingest(&[
        (make_id("a"), unit_vec(3, 0)), // [1,0,0]
        (make_id("b"), unit_vec(3, 1)), // [0,1,0]
        (make_id("c"), unit_vec(3, 2)), // [0,0,1]
    ])
    .unwrap();

    // Query closest to "a"
    let query = [0.9_f32, 0.1, 0.0];
    let results = b.search(&query, 2).unwrap();
    assert_eq!(results.len(), 2);
    // First result must be "a" (highest cosine to query)
    assert_eq!(results[0].vector_id.as_ref(), "a");
    // Descending order
    assert!(results[0].score >= results[1].score);
}

// ---------------------------------------------------------------------------
// 2. identical_query_scores_near_one
// ---------------------------------------------------------------------------

#[test]
fn identical_query_scores_near_one() {
    let mut b = BruteForceBackend::new();
    let vec = vec![1.0_f32, 0.0, 0.0];
    b.ingest(&[(make_id("x"), vec.clone())]).unwrap();
    let results = b.search(&vec, 1).unwrap();
    assert_eq!(results.len(), 1);
    assert!(
        (results[0].score - 1.0).abs() < 1e-5,
        "expected score ~1.0, got {}",
        results[0].score
    );
}

// ---------------------------------------------------------------------------
// 3. orthogonal_query_scores_near_zero
// ---------------------------------------------------------------------------

#[test]
fn orthogonal_query_scores_near_zero() {
    let mut b = BruteForceBackend::new();
    let stored = vec![1.0_f32, 0.0];
    let query = [0.0_f32, 1.0];
    b.ingest(&[(make_id("x"), stored)]).unwrap();
    let results = b.search(&query, 1).unwrap();
    assert_eq!(results.len(), 1);
    assert!(
        results[0].score.abs() < 1e-5,
        "expected score ~0.0, got {}",
        results[0].score
    );
}

// ---------------------------------------------------------------------------
// 4. empty_backend_returns_empty_list
// ---------------------------------------------------------------------------

#[test]
fn empty_backend_returns_empty_list() {
    let b = BruteForceBackend::new();
    let query = [1.0_f32, 0.0];
    let results = b.search(&query, 10).unwrap();
    assert!(results.is_empty());
}

// ---------------------------------------------------------------------------
// 5. remove_ids_unknown_is_silent
// ---------------------------------------------------------------------------

#[test]
fn remove_ids_unknown_is_silent() {
    let mut b = BruteForceBackend::new();
    // Remove from empty backend — no panic, no error
    let res = b.remove(&[make_id("ghost")]);
    assert!(res.is_ok());

    // Also from non-empty backend
    b.ingest(&[(make_id("real"), vec![1.0_f32, 0.0])]).unwrap();
    let res = b.remove(&[make_id("ghost")]);
    assert!(res.is_ok());
    assert_eq!(b.len(), 1);
}

// ---------------------------------------------------------------------------
// 6. remove_ids_empty_list_is_noop
// ---------------------------------------------------------------------------

#[test]
fn remove_ids_empty_list_is_noop() {
    let mut b = BruteForceBackend::new();
    b.ingest(&[(make_id("a"), vec![1.0_f32, 0.0])]).unwrap();
    b.remove(&[]).unwrap();
    assert_eq!(b.len(), 1);
}

// ---------------------------------------------------------------------------
// 7. search_with_fewer_vectors_than_top_k_returns_all
// ---------------------------------------------------------------------------

#[test]
fn search_with_fewer_vectors_than_top_k_returns_all() {
    let mut b = BruteForceBackend::new();
    b.ingest(&[
        (make_id("a"), vec![1.0_f32, 0.0]),
        (make_id("b"), vec![0.0_f32, 1.0]),
    ])
    .unwrap();
    let results = b.search(&[1.0_f32, 0.0], 10).unwrap();
    assert_eq!(results.len(), 2);
}

// ---------------------------------------------------------------------------
// 8. search_with_top_k_zero_returns_invalid_top_k
// ---------------------------------------------------------------------------

#[test]
fn search_with_top_k_zero_returns_invalid_top_k() {
    let b = BruteForceBackend::new();
    let err = b.search(&[1.0_f32, 0.0], 0).unwrap_err();
    assert!(matches!(err, BackendError::InvalidTopK));
}

// ---------------------------------------------------------------------------
// 9. search_with_wrong_query_dim_returns_adapter_error
// ---------------------------------------------------------------------------

#[test]
fn search_with_wrong_query_dim_returns_adapter_error() {
    let mut b = BruteForceBackend::new();
    b.ingest(&[(make_id("x"), vec![1.0_f32, 0.0, 0.0])])
        .unwrap(); // dim=3
    let err = b.search(&[1.0_f32, 0.0], 1).unwrap_err(); // query dim=2
    assert!(
        matches!(err, BackendError::Adapter(_)),
        "expected Adapter, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 10. ingest_with_mismatched_dim_returns_adapter_error
// ---------------------------------------------------------------------------

#[test]
fn ingest_with_mismatched_dim_returns_adapter_error() {
    let mut b = BruteForceBackend::new();
    b.ingest(&[(make_id("x"), vec![1.0_f32, 0.0, 0.0])])
        .unwrap(); // dim=3
    let err = b
        .ingest(&[(make_id("y"), vec![1.0_f32, 0.0])]) // dim=2
        .unwrap_err();
    assert!(
        matches!(err, BackendError::Adapter(_)),
        "expected Adapter, got {err:?}"
    );
    // Original vector must still be present
    assert_eq!(b.len(), 1);
}

// ---------------------------------------------------------------------------
// 11. ingest_overwrites_existing_id_silently
// ---------------------------------------------------------------------------

#[test]
fn ingest_overwrites_existing_id_silently() {
    let mut b = BruteForceBackend::new();
    b.ingest(&[(make_id("x"), vec![1.0_f32, 0.0])]).unwrap();
    // Overwrite with a different vector, same id
    b.ingest(&[(make_id("x"), vec![0.0_f32, 1.0])]).unwrap();
    assert_eq!(b.len(), 1);
    // The new vector should be [0, 1]; query [0, 1] should score ~1.0
    let results = b.search(&[0.0_f32, 1.0], 1).unwrap();
    assert!(
        (results[0].score - 1.0).abs() < 1e-5,
        "expected overwritten vector to score ~1.0, got {}",
        results[0].score
    );
}

// ---------------------------------------------------------------------------
// 12. golden_fixture_top_10_ordering — ChaCha20Rng(seed=42)
// ---------------------------------------------------------------------------

#[test]
fn golden_fixture_top_10_ordering() {
    // Generate 100 vectors of dim=32 with seed=42
    let mut rng = ChaCha20Rng::seed_from_u64(42);
    let dim = 32_usize;
    let n = 100_usize;

    let vectors: Vec<(Arc<str>, Vec<f32>)> = (0..n)
        .map(|i| {
            (
                Arc::from(format!("v{i:03}").as_str()),
                random_vec(&mut rng, dim),
            )
        })
        .collect();

    let mut backend = BruteForceBackend::new();
    backend
        .ingest(
            &vectors
                .iter()
                .map(|(id, v)| (Arc::clone(id), v.clone()))
                .collect::<Vec<_>>(),
        )
        .unwrap();

    // Run 5 queries with the same rng (now advanced past the 100 vectors)
    for q in 0..5_usize {
        let query = random_vec(&mut rng, dim);
        let results = backend.search(&query, 10).unwrap();
        assert_eq!(results.len(), 10, "query {q}: expected 10 results");
        // Verify descending order
        for window in results.windows(2) {
            assert!(
                window[0].score >= window[1].score,
                "query {q}: results not sorted descending at window boundary: {} vs {}",
                window[0].score,
                window[1].score
            );
        }
    }
}
