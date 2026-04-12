//! SIMD parity test binary — forces [`DispatchKind::Avx2`] (Phase 20).
#![cfg(all(feature = "simd", target_arch = "x86_64"))]

mod common;

use tinyquant_core::codec::dispatch::{force, DispatchKind};

#[test]
fn avx2_parity() {
    if !is_x86_feature_detected!("avx2") || !is_x86_feature_detected!("fma") {
        eprintln!("skipping avx2_parity: CPU lacks avx2+fma");
        return;
    }
    force(DispatchKind::Avx2);
    common::run_parity(DispatchKind::Avx2);
}
