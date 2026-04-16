//! parity_compress — FR-GPU-003 and FR-GPU-006 parity and idempotency tests.
//!
//! These tests require a wgpu adapter (physical GPU or llvmpipe software renderer).
//! They skip automatically when no adapter is found via `skip_if_no_adapter()`.

use tinyquant_core::codec::{Codec, CodecConfig, Codebook, PreparedCodec};
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
