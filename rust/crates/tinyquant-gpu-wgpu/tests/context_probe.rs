//! context_probe — FR-GPU-002: adapter detection tests.
//!
//! Verifies that `WgpuBackend::new()` does not panic when no adapter is
//! available (e.g. headless CI), and that `is_available()` correctly reflects
//! whether an adapter was found.

// FR-GPU-002: WgpuBackend::new() must not panic when no adapter is available.
use tinyquant_gpu_wgpu::{ComputeBackend, WgpuBackend};

#[test]
fn wgpu_backend_new_does_not_panic() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = rt.block_on(WgpuBackend::new());
    match &result {
        Ok(backend) => {
            println!("adapter found: {:?}", backend.adapter_name());
        }
        Err(e) => {
            println!("no adapter (expected on CI): {e}");
            assert!(
                matches!(e, tinyquant_gpu_wgpu::TinyQuantGpuError::NoAdapter),
                "expected NoAdapter on headless CI, got: {e}"
            );
        }
    }
}

#[test]
fn wgpu_backend_is_available_reflects_adapter_state() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let backend = rt.block_on(WgpuBackend::new());
    match backend {
        Ok(b) => assert!(b.is_available(), "backend with adapter must report is_available = true"),
        Err(_) => {
            let stub = WgpuBackend::unavailable();
            assert!(!stub.is_available());
        }
    }
}

/// Layer 2: validate all WGSL shaders compile without a physical device.
///
/// Uses naga's WGSL front-end and validator. No GPU adapter required.
#[test]
fn all_shaders_compile_without_device() {
    let shaders = [
        ("rotate",          include_str!("../shaders/rotate.wgsl")),
        ("quantize",        include_str!("../shaders/quantize.wgsl")),
        ("dequantize",      include_str!("../shaders/dequantize.wgsl")),
        ("residual_encode", include_str!("../shaders/residual_encode.wgsl")),
        ("cosine_topk",     include_str!("../shaders/cosine_topk.wgsl")),
    ];
    for (name, src) in shaders {
        let module = naga::front::wgsl::parse_str(src)
            .unwrap_or_else(|e| panic!("shader '{name}' failed WGSL parse: {e}"));
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        );
        validator
            .validate(&module)
            .unwrap_or_else(|e| panic!("shader '{name}' failed validation: {e}"));
    }
}
