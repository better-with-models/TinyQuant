//! `PartialInit<T>` — safe partial-initialization guard for parallel batch writes.
//!
//! The parallel batch path writes into a `Vec<MaybeUninit<CompressedVector>>`
//! at non-overlapping indices using a `SyncPtr`. If a worker returns an error
//! we must drop every slot that was successfully initialized before returning
//! the error, without double-dropping the uninitialized slots.
//!
//! `PartialInit<T>` tracks which slots have been initialized via a parallel
//! `Vec<AtomicBool>` (one flag per slot, settable from concurrent workers).
//! Its `Drop` impl walks the bitmap and calls `assume_init_drop` only on
//! flagged slots.
//!
//! The data buffer is stored as `ManuallyDrop<Vec<MaybeUninit<T>>>` so that
//! `into_vec` can repurpose the allocation as a `Vec<T>` without a copy.

use alloc::vec::Vec;
use core::{
    mem::{ManuallyDrop, MaybeUninit},
    sync::atomic::{AtomicBool, Ordering},
};

/// Parallel-safe partial-initialization guard.
///
/// # Safety invariants
///
/// - Each slot index must be written at most once (the parallel batch path
///   upholds this because worker `i` writes exclusively to slot `i`).
/// - `mark_init(i)` must be called **after** writing the value at slot `i`.
///   The `Release` store pairs with the `Acquire` loads in `Drop` and `into_vec`.
/// - `into_vec` must be called only when every slot is initialized.
pub struct PartialInit<T> {
    // ManuallyDrop so we can move the buffer into the returned Vec<T> in
    // `into_vec` without triggering E0509 ("cannot move out of type that
    // implements Drop"). The Drop impl drops the allocation explicitly.
    data: ManuallyDrop<Vec<MaybeUninit<T>>>,
    flags: Vec<AtomicBool>,
}

impl<T> PartialInit<T> {
    /// Allocate capacity for `len` slots, all marked uninitialized.
    pub fn new(len: usize) -> Self {
        let mut data = Vec::with_capacity(len);
        // SAFETY: MaybeUninit<T> requires no initialization.
        unsafe { data.set_len(len) };

        let mut flags = Vec::with_capacity(len);
        for _ in 0..len {
            flags.push(AtomicBool::new(false));
        }
        Self {
            data: ManuallyDrop::new(data),
            flags,
        }
    }

    /// Raw mutable pointer to the start of the data buffer.
    ///
    /// Workers compute `ptr.add(i)` to reach slot `i`.
    pub fn as_mut_ptr(&mut self) -> *mut MaybeUninit<T> {
        self.data.as_mut_ptr()
    }

    /// Shared reference to the flags slice so callers can set individual flags
    /// from a concurrent closure without capturing all of `self`.
    pub fn flags(&self) -> &[AtomicBool] {
        &self.flags
    }

    /// Convert to `Vec<T>`, repurposing the underlying allocation.
    ///
    /// # Safety
    ///
    /// Every slot `0..len` must have been written and `mark_init`-ed.
    /// Typically called after `for_each_row` returns without error.
    pub unsafe fn into_vec(mut self) -> Vec<T> {
        debug_assert!(
            self.flags.iter().all(|f| f.load(Ordering::Acquire)),
            "into_vec called with uninitialized slots"
        );
        let len = self.data.len();
        let cap = self.data.capacity();
        // Cast MaybeUninit<T>* → T*; safe because all slots are initialized.
        let ptr = self.data.as_mut_ptr().cast::<T>();
        // Wrap in ManuallyDrop so that self's Drop impl does not run
        // (which would call drop_in_place on data, freeing the buffer we
        // are about to repurpose).
        let mut md = ManuallyDrop::new(self);
        // Drop flags explicitly so their heap allocation is freed.
        // Safety: `AtomicBool: !Drop` so `drop_in_place` cannot panic.
        unsafe { core::ptr::drop_in_place(&mut md.flags) };
        // The data buffer is now owned by the returned Vec<T>.
        // md itself is never dropped (ManuallyDrop prevents it).
        // SAFETY: all slots are initialized; allocation is valid.
        unsafe { Vec::from_raw_parts(ptr, len, cap) }
    }
}

impl<T> Drop for PartialInit<T> {
    fn drop(&mut self) {
        for i in 0..self.data.len() {
            // Bounds-checked: i < self.data.len() by the loop.
            let initialized = self
                .flags
                .get(i)
                .is_some_and(|f| f.load(Ordering::Acquire));
            if initialized {
                // SAFETY: flag true means the caller wrote a valid T at slot i.
                // assume_init_drop calls T::drop without freeing the slot's memory.
                unsafe { self.data.get_unchecked_mut(i).assume_init_drop() };
            }
        }
        // Free the data allocation.  At this point every initialized T has
        // been dropped above; MaybeUninit<T> has no Drop impl, so no
        // double-free occurs when the Vec<MaybeUninit<T>>'s allocator is released.
        // SAFETY: self.data is valid; we're explicitly managing cleanup.
        unsafe { ManuallyDrop::drop(&mut self.data) };
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::items_after_statements
)]
mod tests {
    use super::PartialInit;
    use crate::codec::compressed_vector::CompressedVector;
    use alloc::sync::Arc;

    fn mark(pi: &PartialInit<impl Sized>, i: usize) {
        // Flags are indexed 0..len, so get() always succeeds for valid i.
        if let Some(flag) = pi.flags().get(i) {
            flag.store(true, core::sync::atomic::Ordering::Release);
        }
    }

    #[test]
    fn partial_init_all_slots_round_trips() {
        let mut pi: PartialInit<u32> = PartialInit::new(4);
        let base = pi.as_mut_ptr();
        for i in 0..4_usize {
            #[allow(clippy::cast_possible_truncation)]
            let val = i as u32 * 10;
            unsafe { base.add(i).cast::<u32>().write(val) };
            mark(&pi, i);
        }
        let v = unsafe { pi.into_vec() };
        assert_eq!(v, alloc::vec![0, 10, 20, 30]);
    }

    #[test]
    fn partial_init_drops_initialized_on_partial_error() {
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicUsize, Ordering as Ord};

        struct Dropper(Arc<AtomicUsize>);
        impl Drop for Dropper {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ord::SeqCst);
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let mut pi: PartialInit<Dropper> = PartialInit::new(3);
        let base = pi.as_mut_ptr();
        // Initialize slots 0 and 2; leave slot 1 uninitialized.
        unsafe {
            base.add(0)
                .cast::<Dropper>()
                .write(Dropper(Arc::clone(&counter)));
        }
        mark(&pi, 0);
        unsafe {
            base.add(2)
                .cast::<Dropper>()
                .write(Dropper(Arc::clone(&counter)));
        }
        mark(&pi, 2);
        drop(pi);
        // Only 2 Droppers should have been dropped (slots 0 and 2).
        assert_eq!(counter.load(Ord::SeqCst), 2);
    }

    #[test]
    fn partial_init_empty() {
        let mut pi: PartialInit<u32> = PartialInit::new(0);
        let _ = pi.as_mut_ptr(); // ensure it's usable
        let v = unsafe { pi.into_vec() };
        assert_eq!(v.len(), 0);
    }

    #[test]
    fn partial_init_into_vec_with_compressed_vector() {
        // Build two real CompressedVector values (dimension=2, bit_width=4, no residual).
        let hash: Arc<str> = Arc::from("test-hash");
        let cv0 = CompressedVector::new(
            alloc::vec![1_u8, 2_u8].into_boxed_slice(),
            None,
            Arc::clone(&hash),
            2,
            4,
        )
        .unwrap();
        let cv1 = CompressedVector::new(
            alloc::vec![3_u8, 4_u8].into_boxed_slice(),
            None,
            Arc::clone(&hash),
            2,
            4,
        )
        .unwrap();

        let mut pi: PartialInit<CompressedVector> = PartialInit::new(2);
        let base = pi.as_mut_ptr();
        unsafe { base.add(0).cast::<CompressedVector>().write(cv0.clone()) };
        mark(&pi, 0);
        unsafe { base.add(1).cast::<CompressedVector>().write(cv1.clone()) };
        mark(&pi, 1);

        let v = unsafe { pi.into_vec() };
        assert_eq!(v.len(), 2);
        assert_eq!(v.first().unwrap().indices(), cv0.indices());
        assert_eq!(v.get(1).unwrap().indices(), cv1.indices());
    }
}
