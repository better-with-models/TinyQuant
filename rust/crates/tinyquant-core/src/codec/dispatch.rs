//! Runtime ISA detection and dispatch cache (Phase 20).
//!
//! On first call, [`current`] probes the host CPU via
//! `is_x86_feature_detected!` (`x86_64`) or assumes NEON availability
//! (aarch64), caches the resulting [`DispatchKind`] in a [`OnceLock`],
//! and returns it. Subsequent calls hit the cache.
//!
//! This module is only compiled under `feature = "simd"` — that
//! feature also enables `feature = "std"`, which is required for
//! [`OnceLock`] and the `is_x86_feature_detected!` macro.

#[cfg(target_arch = "x86_64")]
use std::is_x86_feature_detected;
use std::sync::OnceLock;

/// Which kernel implementation the dispatcher currently favors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DispatchKind {
    /// Scalar reference kernels — always available, canonical source
    /// of truth for parity tests.
    Scalar,
    /// AVX2+FMA kernels (`x86_64` only). Selected when both features are
    /// reported as available by `is_x86_feature_detected!`.
    #[cfg(target_arch = "x86_64")]
    Avx2,
    /// NEON kernels (aarch64 only). NEON is mandatory on ARMv8, so on
    /// aarch64 the dispatcher unconditionally selects this variant.
    #[cfg(target_arch = "aarch64")]
    Neon,
}

static DISPATCH: OnceLock<DispatchKind> = OnceLock::new();

/// Return the cached [`DispatchKind`], detecting it on first call.
///
/// The returned value is stable for the lifetime of the process.
#[must_use]
pub fn current() -> DispatchKind {
    *DISPATCH.get_or_init(detect)
}

/// Override the dispatch cache before first use.
///
/// **Test helper only.** Each Rust integration test binary runs in
/// its own process, so forcing a specific kernel path is a reliable
/// way to exercise parity across implementations without mutating
/// global state for the rest of the application.
///
/// # Panics
///
/// Panics if the cache has already been populated by a previous call
/// to [`current`] or [`force`]. Tests that rely on [`force`] must
/// call it **before** any other code touches the dispatcher.
#[doc(hidden)]
#[allow(clippy::panic)]
pub fn force(kind: DispatchKind) {
    assert!(
        DISPATCH.set(kind).is_ok(),
        "dispatch cache already populated; call force() before current()"
    );
}

fn detect() -> DispatchKind {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            return DispatchKind::Avx2;
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        // NEON is mandatory on ARMv8 aarch64, no runtime probe needed.
        return DispatchKind::Neon;
    }
    #[allow(unreachable_code)]
    DispatchKind::Scalar
}
