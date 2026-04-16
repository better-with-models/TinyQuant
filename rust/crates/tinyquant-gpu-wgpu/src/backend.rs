//! `WgpuBackend`: implements `ComputeBackend` using wgpu + WGSL.

use crate::{context::WgpuContext, error::TinyQuantGpuError, ComputeBackend};
use tinyquant_core::codec::{CompressedVector, PreparedCodec};

/// GPU compute backend backed by wgpu.
///
/// Construct with [`WgpuBackend::new`]. If no adapter is available,
/// `new()` returns `Err(TinyQuantGpuError::NoAdapter)` — the caller should
/// fall back to the CPU path.
pub struct WgpuBackend {
    /// `None` when constructed via `unavailable()` (no-adapter stub).
    ctx: Option<WgpuContext>,
}

impl WgpuBackend {
    /// Initialise the backend. Returns `Err(NoAdapter)` if no wgpu adapter is present.
    pub async fn new() -> Result<Self, TinyQuantGpuError> {
        let ctx = WgpuContext::new().await?;
        Ok(Self { ctx: Some(ctx) })
    }

    /// Construct a no-adapter stub for testing.
    ///
    /// Returns a `WgpuBackend` with no context — `is_available()` returns `false`
    /// and all GPU operations return `Err(NoAdapter)`.  Useful in unit and
    /// integration tests running on headless CI with no GPU present.
    #[doc(hidden)]
    pub fn unavailable() -> Self {
        Self { ctx: None }
    }

    /// Human-readable adapter name for diagnostics.
    ///
    /// Returns `None` when no adapter is present (stub mode).
    pub fn adapter_name(&self) -> Option<&str> {
        self.ctx.as_ref().map(|c| c.adapter_info.name.as_str())
    }

    /// Return `true` when rows ≥ [`crate::GPU_BATCH_THRESHOLD`].
    ///
    /// A pure function — does not check whether an adapter is present.
    pub fn should_use_gpu(rows: usize) -> bool {
        rows >= crate::GPU_BATCH_THRESHOLD
    }
}

impl ComputeBackend for WgpuBackend {
    fn name(&self) -> &'static str {
        "wgpu"
    }

    fn is_available(&self) -> bool {
        self.ctx.is_some()
    }

    fn compress_batch(
        &mut self,
        _input: &[f32],
        _rows: usize,
        _cols: usize,
        _prepared: &PreparedCodec,
    ) -> Result<Vec<CompressedVector>, TinyQuantGpuError> {
        // TODO: implemented in Part C (Steps 5-6)
        Err(TinyQuantGpuError::NoAdapter)
    }

    fn decompress_batch_into(
        &mut self,
        _compressed: &[CompressedVector],
        _prepared: &PreparedCodec,
        _out: &mut [f32],
    ) -> Result<(), TinyQuantGpuError> {
        // TODO: implemented in Part C (Steps 5-6)
        Err(TinyQuantGpuError::NoAdapter)
    }

    fn prepare_for_device(
        &mut self,
        _prepared: &mut PreparedCodec,
    ) -> Result<(), TinyQuantGpuError> {
        // TODO: implemented in Part C (Steps 5-6)
        Ok(())
    }
}
