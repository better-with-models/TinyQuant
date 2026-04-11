//! FP32 ↔ pgvector text-wire format.
//!
//! `pgvector` stores vectors as `"[x1,x2,...,xn]"` strings in SQL.  This
//! module converts between `&[f32]` and that representation.

use std::sync::Arc;

use crate::errors::BackendError;

/// Encode an `f32` slice as pgvector text format `"[x1,x2,...,xn]"`.
///
/// Returns `Err` if any value is `NaN` or infinite (`pgvector` rejects them).
///
/// # Errors
///
/// Returns `Err(BackendError::Adapter(_))` when any element is `NaN` or
/// infinite.
pub fn encode_vector(v: &[f32]) -> Result<String, BackendError> {
    for (i, &x) in v.iter().enumerate() {
        if !x.is_finite() {
            return Err(BackendError::Adapter(Arc::from(format!(
                "NaN or Inf in vector at index {i}"
            ))));
        }
    }
    let inner: Vec<String> = v.iter().map(|&x| format!("{x:.7e}")).collect();
    Ok(format!("[{}]", inner.join(",")))
}

/// Decode a pgvector text format string `"[x1,x2,...,xn]"` into `Vec<f32>`.
///
/// Used by the `live-db` feature path to parse results from `PostgreSQL`.
///
/// # Errors
///
/// Returns `Err(BackendError::Adapter(_))` when any token cannot be parsed
/// as an `f32`.
#[cfg_attr(not(feature = "live-db"), allow(dead_code))]
pub fn decode_vector(s: &str) -> Result<Vec<f32>, BackendError> {
    let trimmed = s.trim().trim_start_matches('[').trim_end_matches(']');
    if trimmed.is_empty() {
        return Ok(vec![]);
    }
    trimmed
        .split(',')
        .map(|t| {
            t.trim()
                .parse::<f32>()
                .map_err(|e| BackendError::Adapter(Arc::from(format!("parse error: {e}"))))
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::{decode_vector, encode_vector};

    #[test]
    fn round_trip_basic() {
        let v = [1.0_f32, 2.0, 3.0];
        let encoded = encode_vector(&v).expect("encode should succeed");
        let decoded = decode_vector(&encoded).expect("decode should succeed");
        for (a, b) in v.iter().zip(decoded.iter()) {
            assert!((a - b).abs() < 1e-5, "mismatch: {a} vs {b}");
        }
    }

    #[test]
    fn encode_rejects_nan() {
        let v = [1.0_f32, f32::NAN];
        assert!(encode_vector(&v).is_err());
    }

    #[test]
    fn encode_rejects_inf() {
        let v = [1.0_f32, f32::INFINITY];
        assert!(encode_vector(&v).is_err());
    }

    #[test]
    fn decode_empty_brackets() {
        let decoded = decode_vector("[]").expect("decode empty should succeed");
        assert!(decoded.is_empty());
    }
}
