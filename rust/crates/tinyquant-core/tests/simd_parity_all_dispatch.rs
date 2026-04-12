//! Dispatch-parity guards that do **not** require a forced
//! [`DispatchKind`]: NaN semantics and the Phase 14 LFS corpus quantize
//! parity check (Phase 20).
//!
//! The `simd_parity_under_every_dispatch` structural-guard test lives
//! in its own binary (`simd_parity_under_every_dispatch.rs`) because
//! `dispatch::force` panics if any other test in the same process has
//! already populated the `OnceLock` via `dispatch::current()`.
#![cfg(feature = "simd")]

mod common;

use tinyquant_core::codec::simd_api;

/// NaN inputs through every kernel must produce byte-identical output
/// between scalar and dispatched paths. This locks in the Phase 20 NaN
/// semantics: `quantize` returns index 0 for NaN; `cosine` returns 0.0
/// for NaN-containing input.
#[test]
fn simd_nan_semantics_match_scalar() {
    // NaN quantize -> matching index selection.
    let entries: Vec<f32> = (0_u8..16).map(|i| f32::from(i) * 0.125).collect();
    let nan_values = [f32::NAN, 0.5_f32, f32::NAN];
    let mut idx_scalar = vec![0_u8; nan_values.len()];
    let mut idx_simd = vec![0_u8; nan_values.len()];
    simd_api::scalar::quantize_into(&entries, &nan_values, &mut idx_scalar)
        .expect("scalar quantize (NaN input)");
    simd_api::quantize_into(&entries, &nan_values, &mut idx_simd)
        .expect("dispatched quantize (NaN input)");
    assert_eq!(
        idx_scalar, idx_simd,
        "quantize NaN: scalar vs dispatched must agree"
    );

    // NaN cosine -> 0.0 from both paths.
    let nan_vec = [f32::NAN, 1.0_f32, 0.0_f32];
    let normal_vec = [0.5_f32, 0.5_f32, 0.5_f32];
    let s_cos = simd_api::scalar::cosine(&nan_vec, &normal_vec);
    let d_cos = simd_api::cosine(&nan_vec, &normal_vec);
    assert_eq!(
        s_cos.to_bits(),
        d_cos.to_bits(),
        "cosine NaN: scalar vs dispatched bit-pattern"
    );
    assert_eq!(
        s_cos.to_bits(),
        0.0_f32.to_bits(),
        "scalar cosine must return 0.0 when an input contains NaN"
    );
}

/// Parity of the dispatch-routed quantize kernel against the scalar
/// reference on the Phase 14 `codebook_10k_d64` training corpus. This
/// guards against any future SIMD optimisation that accidentally
/// changes the bit-exact output on the real fixture data.
#[test]
fn parity_on_phase14_corpus_quantize() {
    let training = common::load_training_corpus();
    let entries: Vec<f32> = (0_u8..16).map(|i| f32::from(i) * 0.125 - 1.0).collect();
    let take = 1000_usize.min(training.len());
    let values = training.get(..take).expect("training corpus slice");
    let mut idx_scalar = vec![0_u8; values.len()];
    let mut idx_simd = vec![0_u8; values.len()];
    simd_api::scalar::quantize_into(&entries, values, &mut idx_scalar)
        .expect("scalar quantize on Phase 14 corpus");
    simd_api::quantize_into(&entries, values, &mut idx_simd)
        .expect("dispatched quantize on Phase 14 corpus");
    assert_eq!(
        idx_scalar, idx_simd,
        "quantize parity on Phase 14 corpus (first {take} values)"
    );
}
