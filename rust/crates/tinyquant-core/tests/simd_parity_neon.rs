//! SIMD parity test binary — forces [`DispatchKind::Neon`] (Phase 20).
#![cfg(all(feature = "simd", target_arch = "aarch64"))]

mod common;

use tinyquant_core::codec::dispatch::{force, DispatchKind};

#[test]
fn neon_parity() {
    force(DispatchKind::Neon);
    common::run_parity(DispatchKind::Neon);
}
