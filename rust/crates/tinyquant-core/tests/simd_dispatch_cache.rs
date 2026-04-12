//! Dispatch cache smoke tests (Phase 20).
//!
//! Verifies:
//!
//! * `dispatch::current()` returns a stable value across repeated
//!   calls in the same process.
//! * `dispatch::force()` panics when called after the cache has
//!   already been populated.

#![cfg(feature = "simd")]

use tinyquant_core::codec::dispatch::{current, force, DispatchKind};

#[test]
fn current_is_stable_across_calls() {
    let first = current();
    let second = current();
    let third = current();
    assert_eq!(first, second);
    assert_eq!(second, third);
}

#[test]
#[should_panic(expected = "dispatch cache already populated")]
fn force_panics_after_current() {
    // Populate the cache via detection, then try to override — this
    // should trip the assertion inside `force`.
    let _ = current();
    force(DispatchKind::Scalar);
}
