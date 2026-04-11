//! `simd_parity_under_every_dispatch` — structural guard referenced in
//! Phase 22 exit criteria (Phase 20).
//!
//! This binary forces [`DispatchKind::Scalar`] before any dispatch call
//! so the `OnceLock` is pre-populated with a deterministic value.
//! Parity under [`DispatchKind::Avx2`] and [`DispatchKind::Neon`] is
//! covered by `simd_parity_avx2.rs` and `simd_parity_neon.rs`
//! respectively — each integration test file becomes its own process
//! with its own fresh dispatch cache.
//!
//! The test is isolated in its own binary because `dispatch::force`
//! panics if the cache has already been populated by another test
//! within the same process.
#![cfg(feature = "simd")]

mod common;

use tinyquant_core::codec::dispatch::{force, DispatchKind};

#[test]
fn simd_parity_under_every_dispatch() {
    // Force scalar dispatch for this process. Subsequent calls to
    // `dispatch::current()` return `DispatchKind::Scalar`.
    force(DispatchKind::Scalar);
    common::assert_all_kernels_match_scalar();
}
