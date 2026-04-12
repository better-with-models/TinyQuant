//! Cosine similarity kernel used by `BruteForceBackend`.
//!
//! The scalar reference implementation below is the parity baseline.
//! When the crate is built with `feature = "simd"`, the wrapper routes
//! through `tinyquant_core::codec::simd_api::cosine`, which consults
//! the runtime ISA dispatch cache. SIMD and scalar paths must produce
//! byte-identical output (Phase 20 determinism contract).

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

    #[cfg(feature = "simd")]
    {
        crate::similarity_simd::cosine_dispatched(a, b)
    }
    #[cfg(not(feature = "simd"))]
    {
        scalar_cosine(a, b)
    }
}

/// Scalar reference kernel kept public-in-module so the SIMD-off build
/// and the unit tests below can call it without going through a cfg
/// thicket.
#[cfg_attr(feature = "simd", allow(dead_code))]
fn scalar_cosine(a: &[f32], b: &[f32]) -> f32 {
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
