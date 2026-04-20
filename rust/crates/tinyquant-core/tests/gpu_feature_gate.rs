//! Integration tests for the `gpu-wgpu` feature gate of `tinyquant-core`.
//!
//! These tests verify: re-exports are accessible, routing is correct, error
//! variants exist and convert correctly, and the CPU fallback path is
//! byte-identical to `compress_batch_with`.
//!
//! Because `tinyquant-gpu-wgpu` already depends on `tinyquant-core`, it
//! cannot be added as an optional dependency or dev-dependency of
//! `tinyquant-core` without creating a cyclic Cargo dependency.  Therefore
//! these tests use a minimal in-test mock that implements `GpuComputeBackend`
//! so the routing and error conversion logic can be exercised without a live
//! wgpu adapter.  Parity against the real `WgpuBackend` is covered by the
//! `tinyquant-gpu-wgpu` integration tests in `parity_compress.rs`.

#![cfg(feature = "gpu-wgpu")]

use tinyquant_core::{
    codec::{Codebook, Codec, CodecConfig, Parallelism, PreparedCodec},
    errors::CodecError,
    GpuComputeBackend, GPU_BATCH_THRESHOLD,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a small `(config, codebook, prepared)` triple for testing.
///
/// Uses `bit_width=4`, `seed=42`, `dimension=64`, `residual_enabled=false`.
/// Training data: 256 sine-wave samples of dim 64 (enough distinct entries).
fn make_prepared() -> (CodecConfig, Codebook, PreparedCodec) {
    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let training: Vec<f32> = (0..256 * 64)
        .map(|i| (i as f32 * 0.001_f32).sin())
        .collect();
    let codebook = Codebook::train(&training, &config).unwrap();
    let prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();
    (config, codebook, prepared)
}

// ---------------------------------------------------------------------------
// Mock `GpuComputeBackend` that delegates to the CPU path.
//
// Tracks `prepare_for_device` call count for the idempotency test.
// ---------------------------------------------------------------------------

/// A test double that delegates `compress_batch` to `Codec::compress_batch`.
///
/// Used to verify the `GpuComputeBackend` trait surface and the routing
/// logic inside `compress_batch_gpu_with` without requiring a live wgpu
/// adapter.
struct MockCpuBackend {
    /// Number of times `prepare_for_device` was called.
    prepare_calls: usize,
    /// If `Some`, return this error from `compress_batch`.
    inject_error: Option<CodecError>,
}

impl MockCpuBackend {
    fn new() -> Self {
        Self {
            prepare_calls: 0,
            inject_error: None,
        }
    }

    fn with_error(err: CodecError) -> Self {
        Self {
            prepare_calls: 0,
            inject_error: Some(err),
        }
    }
}

impl GpuComputeBackend for MockCpuBackend {
    type Error = CodecError;

    fn prepare_for_device(
        &mut self,
        _prepared: &mut PreparedCodec,
    ) -> Result<(), Self::Error> {
        self.prepare_calls += 1;
        Ok(())
    }

    fn compress_batch(
        &mut self,
        input: &[f32],
        rows: usize,
        cols: usize,
        prepared: &PreparedCodec,
    ) -> Result<Vec<tinyquant_core::codec::CompressedVector>, Self::Error> {
        if let Some(ref e) = self.inject_error {
            return Err(e.clone());
        }
        // Delegate to the CPU path for bit-identical output.
        Codec::new().compress_batch_with(
            input,
            rows,
            cols,
            prepared.config(),
            prepared.codebook(),
            Parallelism::Serial,
        )
    }
}

// ---------------------------------------------------------------------------
// Test 1: GPU types accessible via `tinyquant_core`
// ---------------------------------------------------------------------------

/// Smoke test: the `GpuComputeBackend` trait and `GPU_BATCH_THRESHOLD`
/// constant are accessible from `tinyquant_core` when `gpu-wgpu` is enabled.
#[test]
fn gpu_types_accessible_via_tinyquant_core() {
    assert!(GPU_BATCH_THRESHOLD > 0);
    // Instantiate the trait bound to confirm the type is reachable.
    let _backend: &dyn GpuComputeBackend<Error = CodecError> = &MockCpuBackend::new();
}

// ---------------------------------------------------------------------------
// Test 2: GPU_BATCH_THRESHOLD value
// ---------------------------------------------------------------------------

/// `GPU_BATCH_THRESHOLD` must equal 512 — the value documented in the Phase
/// 28.5 plan and mirrored in `tinyquant_gpu_wgpu::GPU_BATCH_THRESHOLD`.
#[test]
fn gpu_batch_threshold_is_512() {
    assert_eq!(GPU_BATCH_THRESHOLD, 512);
}

// ---------------------------------------------------------------------------
// Test 3: CodecError::GpuUnavailable accessible and formattable
// ---------------------------------------------------------------------------

/// `CodecError::GpuUnavailable` is present when `gpu-wgpu` feature is on.
#[test]
fn codec_error_gpu_unavailable_variant_is_accessible() {
    let err = CodecError::GpuUnavailable("no adapter found".into());
    let msg = err.to_string();
    assert!(
        msg.contains("GPU unavailable"),
        "unexpected message: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Test 4: CodecError::GpuError accessible and formattable
// ---------------------------------------------------------------------------

/// `CodecError::GpuError` is present when `gpu-wgpu` feature is on.
#[test]
fn codec_error_gpu_error_variant_is_accessible() {
    let err = CodecError::GpuError("buffer map failed".into());
    let msg = err.to_string();
    assert!(msg.contains("GPU error"), "unexpected message: {msg}");
}

// ---------------------------------------------------------------------------
// Test 5: codec_error_from_gpu_unavailable_maps_correctly
// ---------------------------------------------------------------------------

/// Constructing `CodecError::GpuUnavailable` from a string produces the
/// expected display message.
#[test]
fn codec_error_gpu_unavailable_message_contains_detail() {
    let detail = "no wgpu adapter found";
    let err = CodecError::GpuUnavailable(detail.into());
    let msg = err.to_string();
    assert!(
        msg.contains(detail),
        "error message should contain the detail string; got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Test 6: compress_batch_gpu_with below threshold uses CPU path
// ---------------------------------------------------------------------------

/// When `rows < GPU_BATCH_THRESHOLD`, `compress_batch_gpu_with` must fall
/// through to the CPU path and produce byte-identical output to
/// `compress_batch_with`.
#[test]
fn compress_batch_gpu_with_below_threshold_uses_cpu_path() {
    let (config, codebook, mut prepared) = make_prepared();
    let rows = 1_usize; // well below 512
    let cols = 64_usize;
    let vectors: Vec<f32> = (0..rows * cols)
        .map(|i| (i as f32 * 0.001_f32).cos())
        .collect();

    let mut backend = MockCpuBackend::new();

    let cpu_result = Codec::new()
        .compress_batch_with(&vectors, rows, cols, &config, &codebook, Parallelism::Serial)
        .unwrap();

    let gpu_path_result = Codec::new()
        .compress_batch_gpu_with(
            &vectors,
            rows,
            cols,
            &mut prepared,
            &mut backend,
            Parallelism::Serial,
        )
        .unwrap();

    assert_eq!(cpu_result.len(), gpu_path_result.len());
    for (cpu_cv, gpu_cv) in cpu_result.iter().zip(gpu_path_result.iter()) {
        assert_eq!(
            cpu_cv.indices(),
            gpu_cv.indices(),
            "CPU and GPU-routed results must be byte-identical for below-threshold batch"
        );
        assert_eq!(cpu_cv.config_hash(), gpu_cv.config_hash());
    }

    // Backend's prepare_for_device must NOT have been called (CPU fallback).
    assert_eq!(
        backend.prepare_calls, 0,
        "prepare_for_device should not be called for below-threshold rows"
    );
}

// ---------------------------------------------------------------------------
// Test 7: compress_batch_gpu_with above threshold calls backend
// ---------------------------------------------------------------------------

/// When `rows >= GPU_BATCH_THRESHOLD`, `compress_batch_gpu_with` must call
/// `prepare_for_device` then `compress_batch` on the backend.
#[test]
fn compress_batch_gpu_with_above_threshold_calls_backend() {
    let (_config, _codebook, mut prepared) = make_prepared();
    let rows = GPU_BATCH_THRESHOLD; // exactly at threshold
    let cols = 64_usize;
    let vectors: Vec<f32> = (0..rows * cols)
        .map(|i| (i as f32 / 1000.0_f32).sin())
        .collect();

    let mut backend = MockCpuBackend::new();

    let result = Codec::new()
        .compress_batch_gpu_with(
            &vectors,
            rows,
            cols,
            &mut prepared,
            &mut backend,
            Parallelism::Serial,
        )
        .unwrap();

    assert_eq!(result.len(), rows, "must return one CompressedVector per row");
    assert_eq!(
        backend.prepare_calls, 1,
        "prepare_for_device must be called exactly once for above-threshold rows"
    );
}

// ---------------------------------------------------------------------------
// Test 8: prepare_for_device is idempotent via compress_batch_gpu_with
// ---------------------------------------------------------------------------

/// Calling `compress_batch_gpu_with` twice on the same `PreparedCodec` must
/// succeed (idempotency: `prepare_for_device` is safe to call multiple times).
#[test]
fn prepare_for_device_is_idempotent_via_codec() {
    let (_config, _codebook, mut prepared) = make_prepared();
    let rows = GPU_BATCH_THRESHOLD;
    let cols = 64_usize;
    let vectors: Vec<f32> = (0..rows * cols)
        .map(|i| (i as f32 * 0.002_f32).cos())
        .collect();

    let mut backend = MockCpuBackend::new();
    let codec = Codec::new();

    let result1 = codec
        .compress_batch_gpu_with(
            &vectors,
            rows,
            cols,
            &mut prepared,
            &mut backend,
            Parallelism::Serial,
        )
        .unwrap();

    let result2 = codec
        .compress_batch_gpu_with(
            &vectors,
            rows,
            cols,
            &mut prepared,
            &mut backend,
            Parallelism::Serial,
        )
        .unwrap();

    assert_eq!(result1.len(), result2.len());
    assert_eq!(
        backend.prepare_calls, 2,
        "prepare_for_device must be called once per compress_batch_gpu_with invocation"
    );
}

// ---------------------------------------------------------------------------
// Test 9: backend errors propagate as CodecError
// ---------------------------------------------------------------------------

/// When the backend returns `CodecError::GpuUnavailable`, it must propagate
/// unchanged through `compress_batch_gpu_with`.
#[test]
fn compress_batch_gpu_with_propagates_backend_error() {
    let (_config, _codebook, mut prepared) = make_prepared();
    let rows = GPU_BATCH_THRESHOLD;
    let cols = 64_usize;
    let vectors: Vec<f32> = vec![0.0_f32; rows * cols];

    let injected = CodecError::GpuUnavailable("no adapter".into());
    let mut backend = MockCpuBackend::with_error(injected.clone());

    let err = Codec::new()
        .compress_batch_gpu_with(
            &vectors,
            rows,
            cols,
            &mut prepared,
            &mut backend,
            Parallelism::Serial,
        )
        .unwrap_err();

    assert_eq!(
        err, injected,
        "backend error must propagate unchanged as CodecError"
    );
}

// ---------------------------------------------------------------------------
// Test 10: gpu module re-exports GpuComputeBackend and GPU_BATCH_THRESHOLD
// ---------------------------------------------------------------------------

/// `tinyquant_core::gpu::GpuComputeBackend` and
/// `tinyquant_core::gpu::GPU_BATCH_THRESHOLD` are accessible via the
/// namespaced `gpu` module.
#[test]
fn gpu_module_exports_trait_and_constant() {
    use tinyquant_core::gpu::GPU_BATCH_THRESHOLD as THRESH;
    assert_eq!(THRESH, 512);
}
