//! batch_threshold — FR-GPU-005: `should_use_gpu` threshold tests.
//!
//! Verifies that `WgpuBackend::should_use_gpu(rows)` returns `false` for
//! any `rows < GPU_BATCH_THRESHOLD` and `true` for `rows >= GPU_BATCH_THRESHOLD`.

// FR-GPU-005: should_use_gpu(rows) returns false when rows < BATCH_THRESHOLD.
use tinyquant_gpu_wgpu::{WgpuBackend, GPU_BATCH_THRESHOLD};

#[test]
fn should_use_gpu_returns_false_below_threshold() {
    let cases_false = [0usize, 1, GPU_BATCH_THRESHOLD - 1];
    let cases_true = [GPU_BATCH_THRESHOLD, GPU_BATCH_THRESHOLD + 1, 10_000];

    for rows in cases_false {
        assert!(
            !WgpuBackend::should_use_gpu(rows),
            "rows={rows} < BATCH_THRESHOLD={GPU_BATCH_THRESHOLD} must return false"
        );
    }
    for rows in cases_true {
        assert!(
            WgpuBackend::should_use_gpu(rows),
            "rows={rows} >= BATCH_THRESHOLD={GPU_BATCH_THRESHOLD} must return true"
        );
    }
}
