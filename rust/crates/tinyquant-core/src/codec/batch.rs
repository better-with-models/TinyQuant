//! Parallel batch compress/decompress internals (Phase 21).
//!
//! # Parallelism determinism contract
//!
//! `compress_batch_parallel` MUST produce byte-identical `CompressedVector`s
//! regardless of which `Parallelism` driver is passed:
//!
//! - `Parallelism::Serial`
//! - `Parallelism::Custom(rayon_driver)` with any thread count
//!
//! This is upheld by three design decisions:
//!
//! 1. **No shared reduction.** The parallel path writes into
//!    `PartialInit<CompressedVector>` at **independent indices**. There is no
//!    cross-worker reduction step.
//! 2. **Per-vector purity.** Each vector's compression is a pure,
//!    order-independent function of `(vector, config, codebook)`. All sums
//!    inside `Codec::compress` run inside a single worker and therefore always
//!    run in the same order.
//! 3. **Read-only shared state.** The codebook, config, and rotation matrix
//!    are passed as shared references; no worker mutates them.
//!
//! # SAFETY annotation for `AtomicPtr`
//!
//! `AtomicPtr<CompressedVector>` stores the base pointer of the `PartialInit`
//! allocation. Workers load it with `Relaxed` ordering (the address is
//! constant) and write at non-overlapping offsets, so there is no data race on
//! the element slots. The `AtomicPtr` is captured by the `for_each_row`
//! closure and is used only until `for_each_row` returns (before `into_vec`).
//!
//! `has_error` uses `Relaxed` ordering for the fast-path load: a stale
//! read only delays error detection by at most one row; the authoritative
//! result is always the first error stored under the `Mutex`.

use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use crate::{
    codec::{
        batch_error::PartialInit, codebook::Codebook, codec_config::CodecConfig,
        compressed_vector::CompressedVector, parallelism::Parallelism, service::Codec,
    },
    errors::CodecError,
};
use alloc::vec::Vec;

/// Parallel batch compress.
///
/// When `parallelism` is `Serial` this is equivalent to the serial loop in
/// `service.rs`. When `Custom(driver)` is passed, workers write into
/// `PartialInit<CompressedVector>` at independent indices via `AtomicPtr<CompressedVector>`.
///
/// On any per-vector error the first error is returned and all successfully
/// initialized `CompressedVector`s are dropped by `PartialInit::drop`.
///
/// # Errors
///
/// Returns the first `CodecError` produced by any worker.
#[allow(unsafe_code)]
pub(super) fn compress_batch_parallel(
    vectors: &[f32],
    rows: usize,
    cols: usize,
    config: &CodecConfig,
    codebook: &Codebook,
    parallelism: Parallelism,
) -> Result<Vec<CompressedVector>, CodecError> {
    debug_assert_eq!(
        vectors.len(),
        rows * cols,
        "compress_batch_parallel: vectors.len() ({}) != rows ({}) * cols ({})",
        vectors.len(),
        rows,
        cols
    );
    let mut partial: PartialInit<CompressedVector> = PartialInit::new(rows);

    // `AtomicPtr<CompressedVector>` is `Send + Sync`, so the closure below can
    // capture it safely. Workers load the base pointer with `Relaxed` ordering
    // (the allocation address never changes) and write at non-overlapping
    // offsets, so there is no data race on the element slots themselves.
    let out_atomic = AtomicPtr::new(partial.as_mut_ptr().cast::<CompressedVector>());

    let flags: &[AtomicBool] = partial.flags();

    let first_error = std::sync::Mutex::new(None::<CodecError>);
    // `has_error` is an atomic flag for the fast-path check so workers do not
    // need to acquire `first_error`'s mutex on every row when no error has occurred.
    let has_error = AtomicBool::new(false);

    parallelism.for_each_row(rows, |i| {
        // Fast-path: if another worker already failed, skip this slot.
        if has_error.load(Ordering::Relaxed) {
            return;
        }
        // Safety: `i < rows` (enforced by `for_each_row`); `i*cols..(i+1)*cols`
        // is in-bounds because `vectors.len() == rows * cols` was checked by
        // the caller.
        #[allow(clippy::indexing_slicing)]
        let slice = &vectors[i * cols..(i + 1) * cols];

        match Codec::new().compress(slice, config, codebook) {
            Ok(cv) => {
                // SAFETY: worker `i` writes exclusively to slot `i`. `Relaxed`
                // is sufficient for the pointer load because the allocation
                // address is fixed; the actual element write at slot `i` is
                // non-overlapping with all other slots, so no data race occurs.
                let base = out_atomic.load(Ordering::Relaxed);
                unsafe { base.add(i).write(cv) };
                // `Release` store pairs with `Acquire` in PartialInit::drop /
                // into_vec, ensuring the write above is visible.
                // flags is indexed 0..rows and i < rows (enforced by for_each_row).
                if let Some(flag) = flags.get(i) {
                    flag.store(true, Ordering::Release);
                }
            }
            Err(e) => {
                // Announce the error on the atomic flag first so other workers
                // can short-circuit without touching the mutex.
                has_error.store(true, Ordering::Relaxed);
                let mut slot = first_error
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if slot.is_none() {
                    *slot = Some(e);
                }
            }
        }
    });

    match first_error
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
    {
        Some(e) => {
            // `partial` drops here; `PartialInit::drop` cleans up initialized slots.
            drop(partial);
            Err(e)
        }
        None => {
            // Every slot was written successfully.
            // SAFETY: all rows completed without error, so all slots are initialized.
            Ok(unsafe { partial.into_vec() })
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::compress_batch_parallel;
    use crate::codec::{codebook::Codebook, codec_config::CodecConfig, parallelism::Parallelism};

    fn make_training() -> alloc::vec::Vec<f32> {
        // Small synthetic corpus: 32 vectors × 8 dims, deterministic.
        use rand_chacha::rand_core::{RngCore, SeedableRng};
        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(99);
        (0..32 * 8)
            .map(|_| {
                let bits = rng.next_u32();
                #[allow(clippy::cast_precision_loss)]
                let v = (bits as f32 / u32::MAX as f32) * 2.0 - 1.0;
                v
            })
            .collect()
    }

    #[test]
    fn serial_and_custom_match_byte_for_byte() {
        let training = make_training();
        let cfg = CodecConfig::new(4, 42, 8, true).unwrap();
        let cb = Codebook::train(&training, &cfg).unwrap();

        // Use 16 vectors from the same training corpus.
        let rows = 16_usize;
        let cols = 8_usize;
        let batch = &training[..rows * cols];

        let serial =
            compress_batch_parallel(batch, rows, cols, &cfg, &cb, Parallelism::Serial).unwrap();
        let custom = compress_batch_parallel(
            batch,
            rows,
            cols,
            &cfg,
            &cb,
            Parallelism::Custom(|count, body| (0..count).for_each(body)),
        )
        .unwrap();

        assert_eq!(serial.len(), custom.len());
        for (a, b) in serial.iter().zip(custom.iter()) {
            assert_eq!(a.indices(), b.indices(), "indices mismatch");
            assert_eq!(a.residual(), b.residual(), "residual mismatch");
        }
    }
}
