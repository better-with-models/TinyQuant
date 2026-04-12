//! Deterministic ChaCha20-backed standard-normal f64 stream.
//!
//! This is the canonical Gaussian source for Rust-side rotation matrix
//! generation. It uses `ChaCha20` (via `rand_chacha::ChaCha20Rng`) for the
//! uniform stream and the polar Box-Muller transform to emit independent
//! `N(0, 1)` samples in `f64`.
//!
//! The exact recipe is specified in
//! `docs/design/rust/numerical-semantics.md` §R1 and matches the pseudo-
//! code in `docs/plans/rust/phase-13-rotation-numerics.md`. Any change
//! here invalidates every rotation fixture under
//! `tests/fixtures/rotation/`.

use libm::{cos, log, sin, sqrt};
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

/// A deterministic standard-normal `f64` generator seeded from `u64`.
///
/// Box-Muller turns two uniform samples into two normals; the second
/// normal is cached in `spare` and returned on the next call.
pub(crate) struct ChaChaGaussianStream {
    rng: ChaCha20Rng,
    spare: Option<f64>,
}

impl ChaChaGaussianStream {
    /// Seed a new stream using `ChaCha20Rng::seed_from_u64(seed)`.
    pub fn new(seed: u64) -> Self {
        Self {
            rng: ChaCha20Rng::seed_from_u64(seed),
            spare: None,
        }
    }

    /// Return the next standard-normal sample.
    pub fn next_f64(&mut self) -> f64 {
        if let Some(spare) = self.spare.take() {
            return spare;
        }
        // Polar (classical) Box-Muller: reject u1 == 0 to avoid ln(0).
        loop {
            let u1 = self.next_uniform();
            let u2 = self.next_uniform();
            if u1 <= 0.0 {
                continue;
            }
            let r = sqrt(-2.0 * log(u1));
            let theta = 2.0 * core::f64::consts::PI * u2;
            let z0 = r * cos(theta);
            let z1 = r * sin(theta);
            self.spare = Some(z1);
            return z0;
        }
    }

    /// Return a uniform `f64` in `[0, 1)` using 53 bits of mantissa.
    fn next_uniform(&mut self) -> f64 {
        let n = self.rng.next_u64();
        // The top 53 bits are exactly representable in an `f64` mantissa,
        // so the cast is lossless; scale into `[0, 1)` by `2^-53`.
        #[allow(clippy::cast_precision_loss)]
        let numerator = (n >> 11) as f64;
        // 2^53 is `9_007_199_254_740_992` — exact in `f64`.
        numerator * (1.0_f64 / 9_007_199_254_740_992.0_f64)
    }
}

#[cfg(test)]
mod tests {
    use super::ChaChaGaussianStream;

    // `libm` 0.2.16 uses SSE2 inline assembly (`sqrtsd`) for `f64::sqrt` on
    // x86_64. Miri's interpreter does not support inline assembly, so these
    // three tests — which all call `ChaChaGaussianStream::next_f64` and
    // therefore hit `libm::sqrt` via the Box-Muller transform — are skipped
    // under Miri. They run normally under `cargo test`.
    // See Phase 20 implementation notes §Miri / libm inline-asm gap.

    #[test]
    #[cfg_attr(miri, ignore)]
    fn same_seed_produces_identical_stream() {
        let mut a = ChaChaGaussianStream::new(42);
        let mut b = ChaChaGaussianStream::new(42);
        for _ in 0..64 {
            assert_eq!(a.next_f64().to_bits(), b.next_f64().to_bits());
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn different_seeds_diverge() {
        let mut a = ChaChaGaussianStream::new(42);
        let mut b = ChaChaGaussianStream::new(43);
        let mut diffs = 0;
        for _ in 0..64 {
            if a.next_f64().to_bits() != b.next_f64().to_bits() {
                diffs += 1;
            }
        }
        assert!(diffs > 0, "different seeds must not collide bit-for-bit");
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn samples_have_reasonable_spread() {
        let mut s = ChaChaGaussianStream::new(0);
        let mut sum = 0.0f64;
        let mut sum_sq = 0.0f64;
        let n = 2048;
        for _ in 0..n {
            let x = s.next_f64();
            sum += x;
            sum_sq += x * x;
        }
        let mean = sum / f64::from(n);
        let var = sum_sq / f64::from(n) - mean * mean;
        assert!(mean.abs() < 0.1, "sample mean too far from 0: {mean}");
        assert!(
            (var - 1.0).abs() < 0.2,
            "sample variance too far from 1: {var}"
        );
    }
}
