//! CPU-only vector quantization codec — core types, codec, corpus, and
//! backend trait.
//!
//! This crate is `no_std` (requires `alloc` only). It contains no I/O,
//! no file-system access, and no platform-specific code.
#![no_std]
#![deny(
    warnings,
    missing_docs,
    unsafe_op_in_unsafe_fn,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cognitive_complexity
)]
#![allow(clippy::module_name_repetitions, clippy::must_use_candidate)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod backend;
pub mod codec;
pub mod corpus;
pub mod errors;
pub mod prelude;
pub mod types;

/// GPU acceleration trait and threshold constant.
///
/// # Why these are defined here (not re-exported from `tinyquant-gpu-wgpu`)
///
/// `tinyquant-gpu-wgpu` depends on `tinyquant-core`, so adding
/// `tinyquant-gpu-wgpu` as an optional dependency of `tinyquant-core` would
/// create a Cargo cyclic dependency (which Cargo rejects unconditionally).
/// Therefore `GpuComputeBackend` and `GPU_BATCH_THRESHOLD` are defined
/// directly in `tinyquant-core`. Concrete implementations (`WgpuBackend`)
/// live in `tinyquant-gpu-wgpu` and implement `GpuComputeBackend`.
pub mod gpu {
    pub use crate::codec::service::{GpuComputeBackend, GPU_BATCH_THRESHOLD};
}

// Flat re-exports for ergonomic use without the `gpu` module prefix.
/// The backend trait for GPU-accelerated compute operations.
pub use crate::codec::service::GpuComputeBackend;

/// Minimum batch size below which GPU offload is not attempted.
pub use crate::codec::service::GPU_BATCH_THRESHOLD;
