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

pub mod backend;
pub mod context;
pub mod error;
pub mod pipelines;
pub mod prepared;

pub use backend::WgpuBackend;
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
    fn prepare_for_device(
        &mut self,
        prepared: &mut PreparedCodec,
    ) -> Result<(), TinyQuantGpuError>;
}
