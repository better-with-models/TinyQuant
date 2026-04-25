//! context_probe — FR-GPU-002: adapter detection tests, shader compilation,
//! pipeline lifecycle, and backend preference/enumeration.

use tinyquant_gpu_wgpu::{BackendPreference, ComputeBackend, TinyQuantGpuError, WgpuBackend};

// FR-GPU-002: WgpuBackend::new() must not panic when no adapter is available.
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
                matches!(e, TinyQuantGpuError::NoAdapter),
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
        Ok(b) => assert!(
            b.is_available(),
            "backend with adapter must report is_available = true"
        ),
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
        ("rotate", include_str!("../shaders/rotate.wgsl")),
        ("quantize", include_str!("../shaders/quantize.wgsl")),
        ("dequantize", include_str!("../shaders/dequantize.wgsl")),
        (
            "residual_encode",
            include_str!("../shaders/residual_encode.wgsl"),
        ),
        (
            "residual_decode",
            include_str!("../shaders/residual_decode.wgsl"),
        ),
        ("cosine_topk", include_str!("../shaders/cosine_topk.wgsl")),
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

// ─── Pipeline lifecycle (Part C) ─────────────────────────────────────────────

/// load_pipelines must succeed on an adapter-present backend and set
/// pipelines_loaded() to true.
#[test]
fn load_pipelines_marks_loaded() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut backend = match rt.block_on(WgpuBackend::new()) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("SKIP: no wgpu adapter");
            return;
        }
    };
    assert!(!backend.pipelines_loaded(), "pipelines must start unloaded");
    backend
        .load_pipelines()
        .expect("load_pipelines must succeed");
    assert!(
        backend.pipelines_loaded(),
        "pipelines must be loaded after load_pipelines"
    );
}

/// unload_pipelines must release all cached pipelines.
#[test]
fn unload_pipelines_clears_loaded_state() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut backend = match rt.block_on(WgpuBackend::new()) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("SKIP: no wgpu adapter");
            return;
        }
    };
    backend.load_pipelines().unwrap();
    assert!(backend.pipelines_loaded());
    backend.unload_pipelines();
    assert!(
        !backend.pipelines_loaded(),
        "pipelines must be unloaded after unload_pipelines"
    );
}

/// load_pipelines must be idempotent.
#[test]
fn load_pipelines_is_idempotent() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut backend = match rt.block_on(WgpuBackend::new()) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("SKIP: no wgpu adapter");
            return;
        }
    };
    backend.load_pipelines().unwrap();
    backend
        .load_pipelines()
        .expect("second load_pipelines must succeed");
    assert!(backend.pipelines_loaded());
}

/// compress_batch after load_pipelines must not regress vs unloaded path.
#[test]
fn compress_after_explicit_load_succeeds() {
    use tinyquant_core::codec::{Codebook, CodecConfig, PreparedCodec};

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut backend = match rt.block_on(WgpuBackend::new()) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("SKIP: no wgpu adapter");
            return;
        }
    };

    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let training: Vec<f32> = (0..256 * 64).map(|i| (i as f32 * 0.001).sin()).collect();
    let codebook = Codebook::train(&training, &config).unwrap();
    let mut prepared = PreparedCodec::new(config.clone(), codebook).unwrap();
    backend.prepare_for_device(&mut prepared).unwrap();

    backend.load_pipelines().unwrap();

    let input: Vec<f32> = (0..64).map(|i| i as f32 * 0.01).collect();
    backend
        .compress_batch(&input, 1, 64, &prepared)
        .expect("compress_batch after load_pipelines must succeed");
}

// ─── Backend preference & adapter enumeration (Part D) ───────────────────────

/// enumerate_adapters(Auto) must return at least one candidate when any
/// wgpu-compatible adapter is present (physical or software).
#[test]
fn enumerate_adapters_returns_at_least_one_on_adapter_machine() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    if rt.block_on(WgpuBackend::new()).is_err() {
        eprintln!("SKIP: no wgpu adapter");
        return;
    }

    let candidates = WgpuBackend::enumerate_adapters(BackendPreference::Auto);
    assert!(
        !candidates.is_empty(),
        "enumerate_adapters must return ≥ 1 candidate when any adapter is present"
    );
}

/// new_with_preference(Auto) must produce a backend equivalent to new().
#[test]
fn new_with_preference_auto_equivalent_to_new() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = rt.block_on(WgpuBackend::new_with_preference(BackendPreference::Auto));
    match rt.block_on(WgpuBackend::new()) {
        Ok(_) => assert!(
            result.is_ok(),
            "new_with_preference(Auto) must succeed when new() succeeds"
        ),
        Err(_) => assert!(
            result.is_err(),
            "new_with_preference(Auto) must fail when new() fails"
        ),
    }
}

/// new_with_preference(Software) is advisory: skip rather than fail if no
/// software adapter is available in this environment.
#[test]
fn new_with_preference_software_available_on_ci() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    match rt.block_on(WgpuBackend::new_with_preference(
        BackendPreference::Software,
    )) {
        Ok(b) => assert!(b.is_available()),
        Err(TinyQuantGpuError::NoPreferredAdapter) | Err(TinyQuantGpuError::NoAdapter) => {
            eprintln!("SKIP: no software adapter available in this environment");
        }
        Err(e) => panic!("unexpected error: {e}"),
    }
}
