//! error_conversion — `From<TinyQuantGpuError> for CodecError` mapping tests.
//!
//! Verifies that each `TinyQuantGpuError` variant routes to the correct
//! `CodecError` variant through the `From` impl in `backend_gpu_impl.rs`:
//!
//! - `NoAdapter` and `NoPreferredAdapter` → `CodecError::GpuUnavailable`
//! - all other variants → `CodecError::GpuError`
//!
//! `DeviceRequest` and `BufferMap` are excluded: they wrap
//! `wgpu::RequestDeviceError` and `wgpu::BufferAsyncError` respectively,
//! neither of which can be constructed outside wgpu's own internals.

use tinyquant_core::errors::CodecError;
use tinyquant_gpu_wgpu::TinyQuantGpuError;

// ---------------------------------------------------------------------------
// GpuUnavailable branch
// ---------------------------------------------------------------------------

#[test]
fn no_adapter_maps_to_gpu_unavailable() {
    let err = CodecError::from(TinyQuantGpuError::NoAdapter);
    assert!(
        matches!(err, CodecError::GpuUnavailable(_)),
        "NoAdapter must convert to GpuUnavailable, got: {err:?}"
    );
    assert!(
        err.to_string().contains("GPU unavailable"),
        "GpuUnavailable display must contain 'GPU unavailable', got: {err}"
    );
}

#[test]
fn no_preferred_adapter_maps_to_gpu_unavailable() {
    let err = CodecError::from(TinyQuantGpuError::NoPreferredAdapter);
    assert!(
        matches!(err, CodecError::GpuUnavailable(_)),
        "NoPreferredAdapter must convert to GpuUnavailable, got: {err:?}"
    );
}

/// Documents exhaustiveness of the "unavailable" branch.
///
/// Both unit variants that represent a missing adapter must produce
/// `GpuUnavailable` — verifying neither was accidentally remapped to
/// `GpuError` by a future change to the `match` arms.
#[test]
fn unavailable_variants_are_exhaustive() {
    let cases = [
        TinyQuantGpuError::NoAdapter,
        TinyQuantGpuError::NoPreferredAdapter,
    ];
    for variant in cases {
        let label = format!("{variant:?}");
        let err = CodecError::from(variant);
        assert!(
            matches!(err, CodecError::GpuUnavailable(_)),
            "{label} must convert to GpuUnavailable, got: {err:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// GpuError branch — unit variants
// ---------------------------------------------------------------------------

#[test]
fn unit_error_variants_map_to_gpu_error() {
    let cases = [
        TinyQuantGpuError::NotPrepared,
        TinyQuantGpuError::ResidualNotSupported,
        TinyQuantGpuError::CorpusNotPrepared,
        TinyQuantGpuError::InvalidTopK,
    ];
    for variant in cases {
        let label = format!("{variant:?}");
        let err = CodecError::from(variant);
        assert!(
            matches!(err, CodecError::GpuError(_)),
            "{label} must convert to GpuError, got: {err:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// GpuError branch — struct variants
// ---------------------------------------------------------------------------

#[test]
fn struct_error_variants_map_to_gpu_error() {
    let cases: Vec<TinyQuantGpuError> = vec![
        TinyQuantGpuError::DimensionMismatch {
            expected: 64,
            got: 128,
        },
        TinyQuantGpuError::BatchMismatch {
            detail: "row count mismatch".to_string(),
        },
    ];
    for variant in cases {
        let label = format!("{variant:?}");
        let err = CodecError::from(variant);
        assert!(
            matches!(err, CodecError::GpuError(_)),
            "{label} must convert to GpuError, got: {err:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// GpuError branch — Codec-wrapped error
// ---------------------------------------------------------------------------

#[test]
fn codec_wrapped_error_maps_to_gpu_error() {
    let inner = TinyQuantGpuError::Codec(CodecError::GpuError("inner".into()));
    let err = CodecError::from(inner);
    assert!(
        matches!(err, CodecError::GpuError(_)),
        "Codec-wrapped error must convert to GpuError, got: {err:?}"
    );
}
