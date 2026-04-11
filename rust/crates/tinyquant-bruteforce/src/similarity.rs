//! Scalar cosine similarity kernel.
//!
//! This is the reference implementation used by `BruteForceBackend`.
//! A SIMD-accelerated replacement ships in Phase 20 behind the `simd`
//! feature flag; for now the feature flag is accepted but has no effect.

/// Compute the cosine similarity between two vectors of the same length.
///
/// Returns `0.0` when either vector is the zero vector (denominator == 0).
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    #[cfg_attr(debug_assertions, allow(clippy::panic))]
    {
        debug_assert!(!a.iter().any(|x| x.is_nan()), "NaN in query; backend bug");
        debug_assert!(
            !b.iter().any(|x| x.is_nan()),
            "NaN in stored vector; backend bug"
        );
    }
    debug_assert_eq!(a.len(), b.len());
    let mut dot = 0.0_f32;
    let mut na = 0.0_f32;
    let mut nb = 0.0_f32;
    for (&x, &y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = (na * nb).sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

#[cfg(test)]
mod tests {
    use super::cosine_similarity;

    #[test]
    fn identical_unit_vectors() {
        let v = [1.0_f32, 0.0, 0.0];
        let s = cosine_similarity(&v, &v);
        assert!((s - 1.0).abs() < 1e-6, "expected ~1.0, got {s}");
    }

    #[test]
    fn orthogonal_vectors() {
        let a = [1.0_f32, 0.0];
        let b = [0.0_f32, 1.0];
        let s = cosine_similarity(&a, &b);
        assert!(s.abs() < 1e-6, "expected ~0.0, got {s}");
    }

    #[test]
    fn zero_vector_returns_zero() {
        let a = [0.0_f32, 0.0];
        let b = [1.0_f32, 0.0];
        let s = cosine_similarity(&a, &b);
        assert!(s.abs() < f32::EPSILON, "expected 0.0, got {s}");
    }
}
