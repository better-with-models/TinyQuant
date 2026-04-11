//! SIMD parity test binary — forces [`DispatchKind::Scalar`] (Phase 20).
#![cfg(feature = "simd")]

mod common;

use tinyquant_core::codec::dispatch::{force, DispatchKind};

#[test]
fn scalar_parity() {
    force(DispatchKind::Scalar);
    common::run_parity(DispatchKind::Scalar);
}
