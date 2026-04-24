//! Canonical orthogonal rotation matrix (`ChaCha` → faer QR → sign correction).
//!
//! Pipeline:
//!
//! 1. Fill a `dim * dim` row-major `f64` buffer from [`ChaChaGaussianStream`].
//! 2. Load the buffer into a faer `Mat<f64>` and compute `A = Q R` via
//!    [`faer::Mat::qr`].
//! 3. Apply the Haar-measure sign correction `Q[:, j] *= sign(R[j, j])`
//!    so that the resulting orthogonal matrix is uniquely determined by
//!    the RNG stream (mirrors the Python reference).
//! 4. Store the corrected `Q` in row-major order inside an `Arc<[f64]>`.
//!
//! Numerical contract:
//!
//! * `apply_into` promotes `f32` to `f64`, does the matmul in `f64`, and
//!   casts the result back to `f32`. This matches `NumPy`'s implicit
//!   promotion for `float64 @ float32` and is required for the round-trip
//!   parity target of `< 1e-5`.
//! * `apply_inverse_into` uses the stored matrix's transpose (valid
//!   because it's orthogonal), again with an `f64` accumulator.
//!
//! See `docs/design/rust/numerical-semantics.md` §R1 for the full recipe
//! and rationale.

use alloc::sync::Arc;
use alloc::vec;

use faer::{Mat, Parallelism};
use libm::fabs;

use crate::codec::codec_config::CodecConfig;
use crate::codec::gaussian::ChaChaGaussianStream;
use crate::errors::CodecError;

/// Deterministically generated orthogonal matrix for vector preconditioning.
///
/// The inner storage is `Arc<[f64]>` so `Clone` is O(1) — callers can
/// pass copies between tasks without reallocating the `dim * dim` buffer.
#[derive(Clone, Debug)]
pub struct RotationMatrix {
    matrix: Arc<[f64]>,
    seed: u64,
    dimension: u32,
}

impl RotationMatrix {
    /// Build a rotation matrix for the given [`CodecConfig`].
    #[inline]
    pub fn from_config(config: &CodecConfig) -> Self {
        Self::build(config.seed(), config.dimension())
    }

    /// Build a rotation matrix for the `(seed, dimension)` pair.
    ///
    /// # Panics
    ///
    /// Panics if `dimension == 0`. In the normal flow, dimensions reach
    /// this function only via a validated [`CodecConfig`], so this cannot
    /// be triggered by safe public APIs of the crate.
    #[allow(clippy::indexing_slicing)] // bounds are statically derived from `dim`
    pub fn build(seed: u64, dimension: u32) -> Self {
        assert!(
            dimension > 0,
            "RotationMatrix::build requires dimension > 0"
        );
        let dim = dimension as usize;

        // Step 1: fill the dim*dim row-major buffer from the ChaCha stream.
        let mut data = vec![0.0f64; dim * dim];
        let mut stream = ChaChaGaussianStream::new(seed);
        for slot in &mut data {
            *slot = stream.next_f64();
        }

        // Step 2: load into faer (column-major internal storage) and QR.
        // We pass a closure that reads from our row-major buffer.
        let a = Mat::<f64>::from_fn(dim, dim, |i, j| data[i * dim + j]);
        // Prevent parallel (rayon) Householder reduction so that reduction
        // order cannot vary across thread counts. SIMD divergence (R19) is
        // separately handled by the AVX2 feature cap in rust/.cargo/config.toml.
        //
        // FIXME(thread-safety): get/set_global_parallelism mutates a
        // process-global atomic. Today the rotation cache is a single
        // Mutex<Vec<…>> in rotation_cache.rs so all builds serialise and the
        // race is unreachable. Any future concurrent caller of
        // RotationMatrix::build (a sharded cache, or a non-cache call site)
        // can race with another thread's get/set, returning a wrong-mode QR.
        // Revisit if faer 0.20+ exposes a per-call parallelism API. Track:
        // github.com/better-with-models/TinyQuant/issues/<TBD>.
        let prev_par = faer::get_global_parallelism();
        faer::set_global_parallelism(Parallelism::None);
        let qr = a.qr();
        faer::set_global_parallelism(prev_par);
        let q = qr.compute_q();
        let r = qr.compute_r();

        // Step 3: Haar sign correction — multiply column j of Q by the
        // sign of R[j, j]. The convention `sign(0) = 1` matches numpy
        // when diag elements collide at exactly zero (rare for ChaCha).
        let mut row_major = vec![0.0f64; dim * dim];
        for j in 0..dim {
            let diag = r[(j, j)];
            let sign = if diag >= 0.0 { 1.0 } else { -1.0 };
            for i in 0..dim {
                row_major[i * dim + j] = q[(i, j)] * sign;
            }
        }

        Self {
            matrix: Arc::from(row_major.into_boxed_slice()),
            seed,
            dimension,
        }
    }

    /// Borrow the row-major `dim * dim` matrix buffer.
    #[inline]
    pub fn matrix(&self) -> &[f64] {
        &self.matrix
    }

    /// The row/column count of the square matrix.
    #[inline]
    pub const fn dimension(&self) -> u32 {
        self.dimension
    }

    /// The seed that generated this matrix.
    #[inline]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// Rotate `input` into `output`: `output = matrix @ input`.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::LengthMismatch`] if either slice length
    /// differs from [`Self::dimension`].
    pub fn apply_into(&self, input: &[f32], output: &mut [f32]) -> Result<(), CodecError> {
        let dim = self.dimension as usize;
        if input.len() != dim || output.len() != dim {
            return Err(CodecError::LengthMismatch {
                left: input.len(),
                right: output.len(),
            });
        }
        for (row, out_slot) in self.matrix.chunks_exact(dim).zip(output.iter_mut()) {
            let acc: f64 = row
                .iter()
                .zip(input.iter())
                .map(|(m, x)| m * f64::from(*x))
                .sum();
            #[allow(clippy::cast_possible_truncation)]
            {
                *out_slot = acc as f32;
            }
        }
        Ok(())
    }

    /// Apply the inverse rotation: `output = matrix^T @ input`.
    ///
    /// Valid because the matrix is orthogonal.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::LengthMismatch`] if either slice length
    /// differs from [`Self::dimension`].
    pub fn apply_inverse_into(&self, input: &[f32], output: &mut [f32]) -> Result<(), CodecError> {
        let dim = self.dimension as usize;
        if input.len() != dim || output.len() != dim {
            return Err(CodecError::LengthMismatch {
                left: input.len(),
                right: output.len(),
            });
        }
        // Accumulate `output[j] = Σ_i matrix[i, j] * input[i]` in `f64`
        // to preserve precision, then cast to `f32` once per output.
        let mut scratch = alloc::vec![0.0f64; dim];
        for (row, x) in self.matrix.chunks_exact(dim).zip(input.iter()) {
            let xf = f64::from(*x);
            for (scratch_slot, m) in scratch.iter_mut().zip(row.iter()) {
                *scratch_slot += m * xf;
            }
        }
        for (out_slot, value) in output.iter_mut().zip(scratch.iter()) {
            #[allow(clippy::cast_possible_truncation)]
            {
                *out_slot = *value as f32;
            }
        }
        Ok(())
    }

    /// Return `true` if `matrix @ matrix^T` equals the identity within
    /// `tol` on every entry.
    ///
    /// This is `O(n^3)`; reserve for tests and low-frequency sanity checks.
    pub fn verify_orthogonality(&self, tol: f64) -> bool {
        let dim = self.dimension as usize;
        for (i, row_i) in self.matrix.chunks_exact(dim).enumerate() {
            for (j, row_j) in self.matrix.chunks_exact(dim).enumerate() {
                let acc: f64 = row_i.iter().zip(row_j.iter()).map(|(a, b)| a * b).sum();
                let expected = if i == j { 1.0 } else { 0.0 };
                if fabs(acc - expected) > tol {
                    return false;
                }
            }
        }
        true
    }
}
