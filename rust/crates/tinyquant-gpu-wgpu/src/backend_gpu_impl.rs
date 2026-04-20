//! Implements `tinyquant_core::GpuComputeBackend` for [`WgpuBackend`].
//!
//! Bridges the existing [`ComputeBackend`] interface (which returns
//! [`TinyQuantGpuError`]) to the [`GpuComputeBackend`] interface by wiring
//! `type Error = TinyQuantGpuError` and providing
//! `From<TinyQuantGpuError> for CodecError` so
//! `Codec::compress_batch_gpu_with` can map errors uniformly via `.into()`.

use tinyquant_core::{
    codec::{CompressedVector, PreparedCodec},
    errors::CodecError,
    GpuComputeBackend,
};

use crate::{backend::WgpuBackend, ComputeBackend, TinyQuantGpuError};

// ---------------------------------------------------------------------------
// Error conversion
// ---------------------------------------------------------------------------

impl From<TinyQuantGpuError> for CodecError {
    fn from(e: TinyQuantGpuError) -> Self {
        match &e {
            TinyQuantGpuError::NoAdapter | TinyQuantGpuError::NoPreferredAdapter => {
                CodecError::GpuUnavailable(e.to_string().into())
            }
            _ => CodecError::GpuError(e.to_string().into()),
        }
    }
}

// ---------------------------------------------------------------------------
// GpuComputeBackend impl
// ---------------------------------------------------------------------------

impl GpuComputeBackend for WgpuBackend {
    type Error = TinyQuantGpuError;

    fn prepare_for_device(
        &mut self,
        prepared: &mut PreparedCodec,
    ) -> Result<(), Self::Error> {
        ComputeBackend::prepare_for_device(self, prepared)
    }

    fn compress_batch(
        &mut self,
        input: &[f32],
        rows: usize,
        cols: usize,
        prepared: &PreparedCodec,
    ) -> Result<Vec<CompressedVector>, Self::Error> {
        ComputeBackend::compress_batch(self, input, rows, cols, prepared)
    }
}
