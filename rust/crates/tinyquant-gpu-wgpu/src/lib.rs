//! `tinyquant-gpu-wgpu`: GPU-accelerated batch compression via wgpu + WGSL.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use tinyquant_gpu_wgpu::{WgpuBackend, ComputeBackend, GPU_BATCH_THRESHOLD};
//!
//! # async fn example() {
//! match WgpuBackend::new().await {
//!     Ok(mut backend) => {
//!         println!("GPU backend ready: {:?}", backend.adapter_name());
//!     }
//!     Err(e) => {
//!         eprintln!("no GPU adapter ({e}); falling back to CPU");
//!     }
//! }
//! # }
//! ```

use tinyquant_core::codec::{CompressedVector, PreparedCodec};

/// [`WgpuBackend`]: the primary GPU backend type.
pub mod backend;
/// Backend and adapter selection hints for [`WgpuBackend::new_with_preference`].
pub mod backend_preference;
/// Adapter initialisation and pipeline construction.
pub mod context;
/// Error types for the wgpu GPU backend.
pub mod error;
/// Compute pipeline builders for rotate, quantize, and residual shaders.
pub mod pipelines;
/// GPU-resident state attachment for [`tinyquant_core::codec::PreparedCodec`].
pub mod prepared;

/// The wgpu GPU backend implementation.
pub use backend::WgpuBackend;
/// Caller-facing backend and adapter selection hint.
pub use backend_preference::{AdapterCandidate, BackendPreference};
/// Errors emitted by the wgpu backend.
pub use error::TinyQuantGpuError;

/// Minimum batch size below which GPU offload is not attempted.
///
/// Below this threshold the host↔device transfer overhead exceeds the
/// compute savings.  The value is provisional; revise after Phase 27 benchmarks.
pub const GPU_BATCH_THRESHOLD: usize = 512;

/// Trait that every TinyQuant GPU backend must satisfy.
pub trait ComputeBackend {
    /// Human-readable backend name for logging and diagnostics.
    fn name(&self) -> &'static str;

    /// True if the backend has a usable compute adapter available.
    fn is_available(&self) -> bool;

    /// Compress `rows` FP32 vectors of dimension `cols` on the GPU.
    fn compress_batch(
        &mut self,
        input: &[f32],
        rows: usize,
        cols: usize,
        prepared: &PreparedCodec,
    ) -> Result<Vec<CompressedVector>, TinyQuantGpuError>;

    /// Decompress a batch of `CompressedVector`s into `out`.
    fn decompress_batch_into(
        &mut self,
        compressed: &[CompressedVector],
        prepared: &PreparedCodec,
        out: &mut [f32],
    ) -> Result<(), TinyQuantGpuError>;

    /// Upload `PreparedCodec` buffers to device memory. Idempotent.
    fn prepare_for_device(&mut self, prepared: &mut PreparedCodec)
        -> Result<(), TinyQuantGpuError>;
}
