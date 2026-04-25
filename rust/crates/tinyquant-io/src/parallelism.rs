//! Rayon-backed `Parallelism` driver (Phase 21).
//!
//! This module is compiled only when the `rayon` feature is enabled.
//! It provides [`rayon_parallelism`], which wraps rayon's parallel iterator
//! as a [`tinyquant_core::codec::Parallelism::Custom`] driver.
//!
//! The driver satisfies the Phase 21 determinism contract: each row `i` is
//! compressed independently of all other rows, so rayon's work-stealing
//! schedule does not affect the per-vector outputs.

use tinyquant_core::codec::Parallelism;

/// Return a [`Parallelism::Custom`] driver backed by the rayon global thread pool.
///
/// The driver calls `body(i)` for each `i in 0..count` using
/// `rayon::prelude::IntoParallelIterator::into_par_iter`. The order of
/// invocations is non-deterministic, but each call site writes to an
/// independent slot, so the final output is byte-identical to the serial path.
///
/// # Example
///
/// ```rust,ignore
/// use tinyquant_io::parallelism::rayon_parallelism;
/// let par = rayon_parallelism();
/// let cvs = codec.compress_batch_with(&vectors, rows, cols, &config, &codebook, par)?;
/// ```
#[must_use]
pub fn rayon_parallelism() -> Parallelism {
    Parallelism::Custom(|count, body| {
        use rayon::prelude::*;
        (0..count).into_par_iter().for_each(|i| body(i));
    })
}
