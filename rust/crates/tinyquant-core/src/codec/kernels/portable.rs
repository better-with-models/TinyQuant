//! Portable fallback kernel path.
//!
//! `core::simd` / `std::simd` was stabilized in Rust 1.82, which is
//! **above** the `TinyQuant` workspace MSRV of 1.81. Until the MSRV
//! rises, the portable path is defined as "scalar re-export" — it
//! exists so that downstream code can reference a
//! `kernels::portable::*` symbol without each call-site having to
//! replicate the `cfg(not(any(target_arch = "x86_64", target_arch =
//! "aarch64")))` pattern.
//!
//! When the MSRV eventually rises to 1.82+, this module will grow a
//! real `core::simd::f32x8`-based implementation.

#[allow(unused_imports)]
pub use super::scalar::{
    apply_residual_into, compute_residual_into, cosine, dequantize_into, quantize_into,
};
