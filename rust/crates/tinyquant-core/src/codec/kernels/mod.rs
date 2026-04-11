//! Kernel implementations for quantize, dequantize, cosine, and residual
//! helpers (Phase 20).
//!
//! The `scalar` submodule is the canonical reference and is always
//! compiled. Under `feature = "simd"`, architecture-specific submodules
//! (`avx2`, `neon`) provide wrappers that delegate to scalar until full
//! intrinsic paths land. The `portable` submodule exists as a documented
//! fallback — on MSRV 1.81 `core::simd` is unavailable, so it also
//! delegates to scalar.

pub mod scalar;

#[cfg(feature = "simd")]
pub mod portable;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub mod avx2;

#[cfg(all(feature = "simd", target_arch = "aarch64"))]
pub mod neon;

#[cfg(all(feature = "simd", feature = "avx512", target_arch = "x86_64"))]
pub mod avx512;
