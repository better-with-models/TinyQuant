//! Welford-based online Pearson correlation coefficient (Phase 21).
//!
//! The two-pass formula `cov(x,y) / (std(x) * std(y))` suffers from
//! catastrophic cancellation when x and y are nearly identical or when
//! the magnitudes are large relative to the variance.  Welford's online
//! algorithm accumulates cross-products and variances in a single pass
//! without materialising full running sums.
//!
//! # Accuracy note
//!
//! For calibration workloads `n ≤ 10^6` samples, f64 accumulators are
//! sufficient — error is O(1 / sqrt(n)) from the cosine sampling variance,
//! not from numerical instability.

/// Online Pearson ρ calculator.
///
/// Feed pairs with [`Self::update`], then read ρ with [`Self::value`].
#[derive(Default)]
pub struct PearsonOnline {
    n: u64,
    mean_x: f64,
    mean_y: f64,
    m2_x: f64,
    m2_y: f64,
    c_xy: f64,
}

impl PearsonOnline {
    /// Create a new accumulator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one (x, y) pair using Welford's online algorithm.
    // dx_prev / dy_prev are paired mathematical notation (δx, δy); the
    // similar_names lint fires on them despite their distinct semantics.
    #[allow(clippy::similar_names)]
    pub fn update(&mut self, x: f64, y: f64) {
        self.n += 1;
        // n ≤ 10^6 in our calibration workloads; u64→f64 is exact up to 2^53.
        #[allow(clippy::cast_precision_loss)]
        let n = self.n as f64;
        let dx_prev = x - self.mean_x;
        let dy_prev = y - self.mean_y;
        self.mean_x += dx_prev / n;
        self.mean_y += dy_prev / n;
        let dx_curr = x - self.mean_x;
        let dy_curr = y - self.mean_y;
        self.m2_x += dx_prev * dx_curr;
        self.m2_y += dy_prev * dy_curr;
        self.c_xy += dx_prev * dy_curr;
    }

    /// Pearson ρ, or `0.0` if `n < 2` or variance is zero.
    #[must_use]
    pub fn value(&self) -> f64 {
        if self.n < 2 {
            return 0.0;
        }
        let denom = self.m2_x.sqrt() * self.m2_y.sqrt();
        if !denom.is_finite() || denom < f64::EPSILON {
            return 0.0;
        }
        (self.c_xy / denom).clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::PearsonOnline;

    #[test]
    fn perfect_positive_correlation() {
        let mut p = PearsonOnline::new();
        for i in 0_i32..10 {
            let v = f64::from(i);
            p.update(v, v);
        }
        assert!(
            (p.value() - 1.0).abs() < 1e-10,
            "expected ρ≈1.0, got {}",
            p.value()
        );
    }

    #[test]
    fn perfect_negative_correlation() {
        let mut p = PearsonOnline::new();
        for i in 0_i32..10 {
            let v = f64::from(i);
            p.update(v, -v);
        }
        assert!(
            (p.value() + 1.0).abs() < 1e-10,
            "expected ρ≈-1.0, got {}",
            p.value()
        );
    }

    #[test]
    fn zero_correlation() {
        let mut p = PearsonOnline::new();
        // x constant, y varies → ρ = 0.
        for i in 0_i32..10 {
            p.update(3.0, f64::from(i));
        }
        assert!(
            p.value().abs() < f64::EPSILON,
            "expected ρ≈0 for constant x, got {}",
            p.value()
        );
    }

    #[test]
    fn empty_and_singleton() {
        let p = PearsonOnline::new();
        assert!(p.value().abs() < f64::EPSILON);
        let mut p1 = PearsonOnline::new();
        p1.update(1.0, 2.0);
        assert!(p1.value().abs() < f64::EPSILON);
    }
}
