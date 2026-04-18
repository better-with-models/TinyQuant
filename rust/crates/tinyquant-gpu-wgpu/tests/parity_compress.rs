//! parity_compress — FR-GPU-003 and FR-GPU-006 parity and idempotency tests.
//!
//! These tests require a wgpu adapter (physical GPU or llvmpipe software renderer).
//! They skip automatically when no adapter is found via `skip_if_no_adapter()`.

use tinyquant_core::codec::{Codebook, Codec, CodecConfig, PreparedCodec};
use tinyquant_gpu_wgpu::{ComputeBackend, WgpuBackend};

fn skip_if_no_adapter() -> Option<WgpuBackend> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(WgpuBackend::new()).ok()
}

/// FR-GPU-003: GPU and CPU decompress output agrees to within 1e-3.
#[test]
fn gpu_decompress_matches_cpu_within_tolerance() {
    let Some(mut backend) = skip_if_no_adapter() else {
        eprintln!("SKIP: no wgpu adapter");
        return;
    };

    let config = CodecConfig::new(4, 42, 768, false).unwrap();
    let training: Vec<f32> = (0..256 * 768).map(|i| (i as f32 * 0.001).sin()).collect();
    let codebook = Codebook::train(&training, &config).unwrap();
    let mut prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();

    backend.prepare_for_device(&mut prepared).unwrap();

    let n_vectors = 512;
    let dim = 768;
    let input: Vec<f32> = (0..n_vectors * dim)
        .map(|i| (i as f32 * 0.001).cos())
        .collect();

    // CPU path
    let mut cpu_out = vec![0f32; n_vectors * dim];
    for i in 0..n_vectors {
        let v = &input[i * dim..(i + 1) * dim];
        let cv = Codec::new().compress_prepared(v, &prepared).unwrap();
        let mut row = vec![0f32; dim];
        Codec::new()
            .decompress_prepared_into(&cv, &prepared, &mut row)
            .unwrap();
        cpu_out[i * dim..(i + 1) * dim].copy_from_slice(&row);
    }

    // GPU path
    let compressed = backend
        .compress_batch(&input, n_vectors, dim, &prepared)
        .unwrap();
    let mut gpu_out = vec![0f32; n_vectors * dim];
    backend
        .decompress_batch_into(&compressed, &prepared, &mut gpu_out)
        .unwrap();

    let max_delta = cpu_out
        .iter()
        .zip(&gpu_out)
        .map(|(c, g)| (c - g).abs())
        .fold(0f32, f32::max);

    assert!(
        max_delta <= 1e-3,
        "max element-wise delta GPU vs CPU = {max_delta:.6e}, must be ≤ 1e-3"
    );
}

/// FR-GPU-006: prepare_for_device is idempotent.
#[test]
fn prepare_for_device_is_idempotent() {
    let Some(mut backend) = skip_if_no_adapter() else {
        eprintln!("SKIP: no wgpu adapter");
        return;
    };
    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let training_data: Vec<f32> = (0..256 * 64).map(|i| i as f32 * 0.001).collect();
    let codebook = Codebook::train(&training_data, &config).unwrap();
    let mut prepared = PreparedCodec::new(config, codebook).unwrap();

    backend.prepare_for_device(&mut prepared).unwrap();
    backend.prepare_for_device(&mut prepared).unwrap();
    assert!(prepared.has_gpu_state());
}

/// Calling compress_batch twice on the same WgpuBackend must not recompile
/// pipelines — the second call must complete well below any plausible
/// compile latency (50 ms threshold).
///
/// On a no-GPU (llvmpipe) runner this test is gated with `skip_if_no_adapter`.
#[test]
fn compress_batch_repeated_calls_do_not_recompile() {
    let Some(mut backend) = skip_if_no_adapter() else {
        eprintln!("SKIP: no wgpu adapter");
        return;
    };

    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let training: Vec<f32> = (0..256 * 64).map(|i| (i as f32 * 0.001).sin()).collect();
    let codebook = Codebook::train(&training, &config).unwrap();
    let mut prepared = PreparedCodec::new(config.clone(), codebook).unwrap();
    backend.prepare_for_device(&mut prepared).unwrap();

    let n = 512usize;
    let dim = 64usize;
    let input: Vec<f32> = (0..n * dim).map(|i| (i as f32 * 0.001).sin()).collect();

    // Warm-up call (pipeline build happens here).
    let _ = backend.compress_batch(&input, n, dim, &prepared).unwrap();

    // Second call — must not rebuild pipelines.
    let t0 = std::time::Instant::now();
    let _ = backend.compress_batch(&input, n, dim, &prepared).unwrap();
    let elapsed_ms = t0.elapsed().as_secs_f64() * 1e3;

    // 50 ms is a very conservative upper bound: pipeline compilation alone
    // typically costs 5–50 ms; if we're below 50 ms we did NOT recompile.
    assert!(
        elapsed_ms < 50.0,
        "second compress_batch call took {elapsed_ms:.1} ms — likely recompiling pipelines"
    );
}

/// compress_batch must succeed for residual_enabled=true configs and
/// produce output that round-trips within FR-GPU-003 tolerance (≤ 1e-3).
///
/// Previously this returned Err(ResidualNotSupported); after Phase 28 it
/// must produce a CompressedVector with residual payload and decompress
/// back to within tolerance.
#[test]
fn compress_decompress_residual_enabled_matches_cpu() {
    let Some(mut backend) = skip_if_no_adapter() else {
        eprintln!("SKIP: no wgpu adapter");
        return;
    };

    let config = CodecConfig::new(4, 42, 64, true).unwrap(); // residual_enabled = true
    let training: Vec<f32> = (0..256 * 64).map(|i| (i as f32 * 0.001).sin()).collect();
    let codebook = Codebook::train(&training, &config).unwrap();
    let mut prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();
    backend.prepare_for_device(&mut prepared).unwrap();

    let n = 128usize;
    let dim = 64usize;
    let input: Vec<f32> = (0..n * dim).map(|i| (i as f32 * 0.003).cos()).collect();

    // GPU path — must not return ResidualNotSupported.
    let compressed = backend
        .compress_batch(&input, n, dim, &prepared)
        .expect("compress_batch must succeed for residual_enabled=true");

    let mut gpu_out = vec![0f32; n * dim];
    backend
        .decompress_batch_into(&compressed, &prepared, &mut gpu_out)
        .expect("decompress_batch_into must succeed for residual_enabled=true");

    // CPU reference.
    let mut cpu_out = vec![0f32; n * dim];
    for i in 0..n {
        let v = &input[i * dim..(i + 1) * dim];
        let cv = Codec::new().compress_prepared(v, &prepared).unwrap();
        Codec::new()
            .decompress_prepared_into(&cv, &prepared, &mut cpu_out[i * dim..(i + 1) * dim])
            .unwrap();
    }

    let max_delta = cpu_out
        .iter()
        .zip(&gpu_out)
        .map(|(c, g)| (c - g).abs())
        .fold(0f32, f32::max);

    assert!(
        max_delta <= 1e-3,
        "residual round-trip max delta GPU vs CPU = {max_delta:.6e}, must be ≤ 1e-3"
    );
}
