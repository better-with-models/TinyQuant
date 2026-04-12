//! Shared helpers for the SIMD parity test binaries (Phase 20).
//!
//! Cargo does **not** compile files inside `tests/common/` as
//! standalone integration test binaries, so this `mod.rs` is included
//! from each sibling test file with `mod common;` and reused for
//! deterministic input generation and the per-kernel parity run.

#![cfg(feature = "simd")]
#![allow(dead_code)]

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use tinyquant_core::codec::dispatch::DispatchKind;
use tinyquant_core::codec::simd_api;
use tinyquant_core::codec::simd_api::scalar;

const N: usize = 10_000;
const ENTRIES_BW4: usize = 16;

pub fn make_entries(num: usize, seed: u64) -> Vec<f32> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut v: Vec<f32> = (0..num).map(|_| rng.gen_range(-10.0_f32..10.0)).collect();
    v.sort_by(f32::total_cmp);
    for i in 1..v.len() {
        if v[i] <= v[i - 1] {
            v[i] = v[i - 1] + 1e-3;
        }
    }
    v
}

pub fn make_values(len: usize, seed: u64) -> Vec<f32> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..len).map(|_| rng.gen_range(-12.0_f32..12.0)).collect()
}

/// Load the Phase 14 training corpus (`codebook_10k_d64`) from the LFS
/// fixture. Panics with a helpful message if the fixture is missing.
pub fn load_training_corpus() -> Vec<f32> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/codebook/training_n10000_d64.f32.bin");
    let bytes = std::fs::read(&path).expect(
        "Phase 14 training fixture must exist; run `cargo xtask fixtures refresh-codebook`",
    );
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Parity guard used by the `simd_parity_under_every_dispatch` binary.
///
/// Assumes the caller has already installed the desired [`DispatchKind`]
/// via `dispatch::force` before invoking this helper — the guard itself
/// does not mutate the dispatch cache.
pub fn assert_all_kernels_match_scalar() {
    // Delegate to the existing parity suite. `run_parity` asserts the
    // current dispatch kind matches its argument, so pass whatever the
    // cache reports now.
    let kind = tinyquant_core::codec::dispatch::current();
    run_parity(kind);
}

pub fn run_parity(kind: DispatchKind) {
    assert_eq!(tinyquant_core::codec::dispatch::current(), kind);

    // Quantize parity.
    let entries = make_entries(ENTRIES_BW4, 1234);
    let values = make_values(N, 5678);
    let mut scalar_idx = vec![0_u8; N];
    let mut simd_idx = vec![0_u8; N];
    scalar::quantize_into(&entries, &values, &mut scalar_idx).expect("scalar quantize");
    simd_api::quantize_into(&entries, &values, &mut simd_idx).expect("simd quantize");
    assert_eq!(scalar_idx, simd_idx, "quantize parity failed for {kind:?}");

    // Dequantize parity.
    let mut scalar_dq = vec![0.0_f32; N];
    let mut simd_dq = vec![0.0_f32; N];
    scalar::dequantize_into(&entries, &scalar_idx, &mut scalar_dq).expect("scalar dequantize");
    simd_api::dequantize_into(&entries, &scalar_idx, &mut simd_dq).expect("simd dequantize");
    let scalar_bits: Vec<u32> = scalar_dq.iter().map(|f| f.to_bits()).collect();
    let simd_bits: Vec<u32> = simd_dq.iter().map(|f| f.to_bits()).collect();
    assert_eq!(
        scalar_bits, simd_bits,
        "dequantize parity failed for {kind:?}"
    );

    // Cosine parity (batch of 64-d vectors).
    let mut rng = StdRng::seed_from_u64(4242);
    for _ in 0..256_u32 {
        let a: Vec<f32> = (0..64).map(|_| rng.gen_range(-1.0_f32..1.0)).collect();
        let b: Vec<f32> = (0..64).map(|_| rng.gen_range(-1.0_f32..1.0)).collect();
        let s = scalar::cosine(&a, &b);
        let d = simd_api::cosine(&a, &b);
        assert_eq!(
            s.to_bits(),
            d.to_bits(),
            "cosine parity failed for {kind:?}"
        );
    }

    // Residual encode parity.
    let original = make_values(1024, 9001);
    let reconstructed = make_values(1024, 9002);
    let mut scalar_res = vec![0_u8; 1024 * 2];
    let mut simd_res = vec![0_u8; 1024 * 2];
    scalar::compute_residual_into(&original, &reconstructed, &mut scalar_res);
    simd_api::compute_residual_into(&original, &reconstructed, &mut simd_res);
    assert_eq!(
        scalar_res, simd_res,
        "residual encode parity failed for {kind:?}"
    );

    // Residual decode parity.
    let mut scalar_acc = reconstructed.clone();
    let mut simd_acc = reconstructed.clone();
    scalar::apply_residual_into(&mut scalar_acc, &scalar_res).expect("scalar apply");
    simd_api::apply_residual_into(&mut simd_acc, &simd_res).expect("simd apply");
    let scalar_bits: Vec<u32> = scalar_acc.iter().map(|f| f.to_bits()).collect();
    let simd_bits: Vec<u32> = simd_acc.iter().map(|f| f.to_bits()).collect();
    assert_eq!(
        scalar_bits, simd_bits,
        "residual decode parity failed for {kind:?}"
    );
}
