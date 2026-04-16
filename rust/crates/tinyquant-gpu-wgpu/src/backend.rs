//! `WgpuBackend`: implements `ComputeBackend` using wgpu + WGSL.
//!
//! # GPU pipeline overview
//!
//! ## compress_batch
//!
//! 1. Upload input vectors to GPU.
//! 2. Rotate: `rotated = R @ input`  (forward rotation shader).
//! 3. Quantize: `indices = argmin_k |rotated[i] - codebook[k]|`  (quantize shader).
//! 4. Readback `indices` → construct [`CompressedVector`] per row.
//!
//! ## decompress_batch_into
//!
//! 1. Upload indices to GPU.
//! 2. Dequantize: `values = codebook[indices]` (dequantize shader).
//! 3. Inverse-rotate: `out = R^T @ values` (rotate shader with transposed matrix).
//! 4. Readback result → write into `out` slice.

use crate::{
    context::WgpuContext,
    error::TinyQuantGpuError,
    pipelines::{
        quantize::{build_dequantize_pipeline, build_quantize_pipeline},
        rotate::build_rotate_pipeline,
    },
    prepared::GpuPreparedState,
    ComputeBackend,
};
use std::sync::Arc;
use tinyquant_core::codec::{CompressedVector, PreparedCodec};
use wgpu::util::DeviceExt;

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

    /// No-adapter stub — `is_available()` returns `false`.
    ///
    /// `pub` rather than `#[cfg(test)]` so that integration test crates outside
    /// this crate (e.g. future `tinyquant-gpu-tests`) can construct the no-adapter
    /// path without requiring test-feature gates on the whole dependency graph.
    /// `#[doc(hidden)]` keeps it out of the public rustdoc surface — callers
    /// should always prefer `WgpuBackend::new().await` and inspect `is_available()`.
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
        input: &[f32],
        rows: usize,
        cols: usize,
        prepared: &PreparedCodec,
    ) -> Result<Vec<CompressedVector>, TinyQuantGpuError> {
        let ctx = self.ctx.as_ref().ok_or(TinyQuantGpuError::NoAdapter)?;

        // Validate codec dimension matches the supplied column count.
        let codec_dim = prepared.config().dimension() as usize;
        if cols != codec_dim {
            return Err(TinyQuantGpuError::DimensionMismatch {
                expected: codec_dim,
                got: cols,
            });
        }

        // GPU backend does not implement the residual stage yet (Phase 28).
        if prepared.config().residual_enabled() {
            return Err(TinyQuantGpuError::ResidualNotSupported);
        }

        // Validate input length.
        let expected = rows * cols;
        if input.len() != expected {
            return Err(TinyQuantGpuError::DimensionMismatch {
                expected,
                got: input.len(),
            });
        }

        // Retrieve device-resident GPU state.
        let gpu_state = prepared
            .gpu_state()
            .and_then(|s| s.downcast_ref::<GpuPreparedState>())
            .ok_or(TinyQuantGpuError::NotPrepared)?;

        let device = &ctx.device;
        let queue = &ctx.queue;

        // -----------------------------------------------------------------
        // Build pipelines.
        // TODO(phase-28): cache pipelines in WgpuBackend to avoid per-call shader
        // compilation (order of milliseconds per call). Required before the
        // FR-GPU-004 ≤ 5 ms throughput gate can be satisfied on real hardware.
        // -----------------------------------------------------------------
        let rotate_pipeline = build_rotate_pipeline(ctx);
        let quantize_pipeline = build_quantize_pipeline(ctx);

        // -----------------------------------------------------------------
        // Upload input to GPU.
        // -----------------------------------------------------------------
        let input_bytes = bytemuck::cast_slice::<f32, u8>(input);
        let input_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tinyquant/compress/input"),
            contents: input_bytes,
            usage: wgpu::BufferUsages::STORAGE,
        });

        // -----------------------------------------------------------------
        // Allocate intermediate buffer: rotated (rows × cols f32).
        // -----------------------------------------------------------------
        let rotated_size = (rows * cols * std::mem::size_of::<f32>()) as u64;
        let rotated_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tinyquant/compress/rotated"),
            size: rotated_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // -----------------------------------------------------------------
        // Allocate output buffer: indices (rows × cols u32).
        // -----------------------------------------------------------------
        let indices_size = (rows * cols * std::mem::size_of::<u32>()) as u64;
        let indices_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tinyquant/compress/indices"),
            size: indices_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // -----------------------------------------------------------------
        // Allocate readback staging buffer.
        // -----------------------------------------------------------------
        let readback_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tinyquant/compress/readback"),
            size: indices_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // -----------------------------------------------------------------
        // Uniform buffers for shader dims.
        // -----------------------------------------------------------------
        let rotate_dims: [u32; 2] = [rows as u32, cols as u32];
        let rotate_dims_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tinyquant/compress/rotate_dims"),
            contents: bytemuck::cast_slice::<u32, u8>(&rotate_dims),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let n_entries = gpu_state.n_entries as u32;
        let quant_dims: [u32; 2] = [(rows * cols) as u32, n_entries];
        let quant_dims_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tinyquant/compress/quant_dims"),
            contents: bytemuck::cast_slice::<u32, u8>(&quant_dims),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // -----------------------------------------------------------------
        // Build bind groups.
        // -----------------------------------------------------------------
        let rotate_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tinyquant/compress/rotate_bg"),
            layout: &rotate_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: rotate_dims_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    // Forward compress binds R^T so the shader computes
                    // `input @ R^T` = CPU's `R @ input` (row-vector convention).
                    resource: gpu_state.rotation_t_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: input_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: rotated_buf.as_entire_binding(),
                },
            ],
        });

        let quantize_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tinyquant/compress/quantize_bg"),
            layout: &quantize_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: quant_dims_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gpu_state.codebook_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: rotated_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: indices_buf.as_entire_binding(),
                },
            ],
        });

        // -----------------------------------------------------------------
        // Encode and submit GPU commands.
        // -----------------------------------------------------------------
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("tinyquant/compress"),
            });

        // Pass 1: rotate.
        {
            let mut pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("tinyquant/compress/rotate"),
                    timestamp_writes: None,
                });
            pass.set_pipeline(&rotate_pipeline);
            pass.set_bind_group(0, &rotate_bind_group, &[]);
            // workgroup_size(16, 16): dispatch ceil(cols/16) × ceil(rows/16)
            let wx = (cols as u32).div_ceil(16);
            let wy = (rows as u32).div_ceil(16);
            pass.dispatch_workgroups(wx, wy, 1);
        }

        // Pass 2: quantize.
        {
            let mut pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("tinyquant/compress/quantize"),
                    timestamp_writes: None,
                });
            pass.set_pipeline(&quantize_pipeline);
            pass.set_bind_group(0, &quantize_bind_group, &[]);
            // workgroup_size(256)
            let total = (rows * cols) as u32;
            let wx = total.div_ceil(256);
            pass.dispatch_workgroups(wx, 1, 1);
        }

        // Copy indices to readback buffer.
        encoder.copy_buffer_to_buffer(&indices_buf, 0, &readback_buf, 0, indices_size);

        queue.submit(std::iter::once(encoder.finish()));

        // -----------------------------------------------------------------
        // Readback: map and read indices.
        // -----------------------------------------------------------------
        let (tx, rx) = std::sync::mpsc::channel();
        let readback_slice = readback_buf.slice(..);
        readback_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv().map_err(|_| wgpu::BufferAsyncError)??;

        let mapped = readback_slice.get_mapped_range();
        // indices are u32 on GPU, one per element (rows × cols).
        let u32_indices: &[u32] = bytemuck::cast_slice::<u8, u32>(&mapped);

        // -----------------------------------------------------------------
        // Construct Vec<CompressedVector>.
        // -----------------------------------------------------------------
        let config = prepared.config();
        let config_hash = config.config_hash().clone();
        let bit_width = config.bit_width();
        let dim = cols as u32;

        let mut result = Vec::with_capacity(rows);
        for row in 0..rows {
            let row_indices = &u32_indices[row * cols..(row + 1) * cols];
            let byte_indices: Vec<u8> = row_indices
                .iter()
                .map(|&v| {
                    // Safe for bit_width ∈ {2, 4, 8} (max index ≤ 255).
                    // If bit_width is ever widened beyond 8 this cast silently
                    // truncates — the assertion catches that in debug builds.
                    debug_assert!(
                        v <= u8::MAX as u32,
                        "quantized index {v} exceeds u8::MAX for bit_width {bit_width}"
                    );
                    v as u8
                })
                .collect();
            // Phase 28 TODO: add residual_encode pass for residual_enabled=true configs.
            let cv = CompressedVector::new(
                byte_indices.into_boxed_slice(),
                None,
                config_hash.clone(),
                dim,
                bit_width,
            )
            .map_err(TinyQuantGpuError::Codec)?;
            result.push(cv);
        }

        drop(mapped);
        readback_buf.unmap();

        Ok(result)
    }

    fn decompress_batch_into(
        &mut self,
        compressed: &[CompressedVector],
        prepared: &PreparedCodec,
        out: &mut [f32],
    ) -> Result<(), TinyQuantGpuError> {
        let ctx = self.ctx.as_ref().ok_or(TinyQuantGpuError::NoAdapter)?;

        let rows = compressed.len();
        if rows == 0 {
            return Ok(());
        }
        let cols = compressed[0].dimension() as usize;

        // Validate every vector in the batch against the PreparedCodec.
        let expected_config_hash = prepared.config().config_hash();
        let expected_bit_width = prepared.config().bit_width();
        let expected_dim = prepared.config().dimension();
        for (i, cv) in compressed.iter().enumerate() {
            if cv.dimension() != expected_dim {
                return Err(TinyQuantGpuError::BatchMismatch {
                    detail: format!(
                        "vector[{i}] dimension {} ≠ codec dimension {}",
                        cv.dimension(),
                        expected_dim
                    ),
                });
            }
            if cv.config_hash() != expected_config_hash {
                return Err(TinyQuantGpuError::BatchMismatch {
                    detail: format!("vector[{i}] config_hash mismatch"),
                });
            }
            if cv.bit_width() != expected_bit_width {
                return Err(TinyQuantGpuError::BatchMismatch {
                    detail: format!(
                        "vector[{i}] bit_width {} ≠ codec bit_width {}",
                        cv.bit_width(),
                        expected_bit_width
                    ),
                });
            }
        }

        let expected = rows * cols;
        if out.len() != expected {
            return Err(TinyQuantGpuError::DimensionMismatch {
                expected,
                got: out.len(),
            });
        }

        // Retrieve device-resident GPU state.
        let gpu_state = prepared
            .gpu_state()
            .and_then(|s| s.downcast_ref::<GpuPreparedState>())
            .ok_or(TinyQuantGpuError::NotPrepared)?;

        let device = &ctx.device;
        let queue = &ctx.queue;

        // -----------------------------------------------------------------
        // Flatten indices to u32 for upload.
        // -----------------------------------------------------------------
        let mut flat_indices: Vec<u32> = Vec::with_capacity(rows * cols);
        for cv in compressed {
            for &b in cv.indices() {
                flat_indices.push(u32::from(b));
            }
        }

        // -----------------------------------------------------------------
        // Build pipelines.
        // TODO(phase-28): cache pipelines in WgpuBackend to avoid per-call shader
        // compilation (order of milliseconds per call). Required before the
        // FR-GPU-004 ≤ 5 ms throughput gate can be satisfied on real hardware.
        // -----------------------------------------------------------------
        let dequantize_pipeline = build_dequantize_pipeline(ctx);
        let rotate_pipeline = build_rotate_pipeline(ctx);

        // -----------------------------------------------------------------
        // Upload indices to GPU.
        // -----------------------------------------------------------------
        let indices_bytes = bytemuck::cast_slice::<u32, u8>(&flat_indices);
        let indices_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tinyquant/decompress/indices"),
            contents: indices_bytes,
            usage: wgpu::BufferUsages::STORAGE,
        });

        // -----------------------------------------------------------------
        // Allocate intermediate buffer: dequantized values (rows × cols f32).
        // -----------------------------------------------------------------
        let values_size = (rows * cols * std::mem::size_of::<f32>()) as u64;
        let values_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tinyquant/decompress/values"),
            size: values_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // -----------------------------------------------------------------
        // Allocate output buffer: inverse-rotated (rows × cols f32).
        // -----------------------------------------------------------------
        let output_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tinyquant/decompress/output"),
            size: values_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // -----------------------------------------------------------------
        // Allocate readback staging buffer.
        // -----------------------------------------------------------------
        let readback_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tinyquant/decompress/readback"),
            size: values_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // -----------------------------------------------------------------
        // Uniform buffer for rotate dims.
        // -----------------------------------------------------------------
        let rotate_dims: [u32; 2] = [rows as u32, cols as u32];
        let rotate_dims_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tinyquant/decompress/rotate_dims"),
            contents: bytemuck::cast_slice::<u32, u8>(&rotate_dims),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // -----------------------------------------------------------------
        // Build bind groups.
        // -----------------------------------------------------------------
        let dequant_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tinyquant/decompress/dequant_bg"),
            layout: &dequantize_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: gpu_state.codebook_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: indices_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: values_buf.as_entire_binding(),
                },
            ],
        });

        // Inverse-rotate binds R (`rotation_buf`) so the shader computes
        // `values @ R` = CPU's `R^T @ values` (R is orthogonal: R^{-1} = R^T).
        let rotate_inv_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tinyquant/decompress/rotate_inv_bg"),
            layout: &rotate_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: rotate_dims_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gpu_state.rotation_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: values_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buf.as_entire_binding(),
                },
            ],
        });

        // -----------------------------------------------------------------
        // Encode and submit GPU commands.
        // -----------------------------------------------------------------
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("tinyquant/decompress"),
            });

        // Pass 1: dequantize.
        {
            let mut pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("tinyquant/decompress/dequantize"),
                    timestamp_writes: None,
                });
            pass.set_pipeline(&dequantize_pipeline);
            pass.set_bind_group(0, &dequant_bind_group, &[]);
            let total = (rows * cols) as u32;
            let wx = total.div_ceil(256);
            pass.dispatch_workgroups(wx, 1, 1);
        }

        // Pass 2: inverse-rotate.
        {
            let mut pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("tinyquant/decompress/rotate_inv"),
                    timestamp_writes: None,
                });
            pass.set_pipeline(&rotate_pipeline);
            pass.set_bind_group(0, &rotate_inv_bind_group, &[]);
            let wx = (cols as u32).div_ceil(16);
            let wy = (rows as u32).div_ceil(16);
            pass.dispatch_workgroups(wx, wy, 1);
        }

        // Copy output to readback buffer.
        encoder.copy_buffer_to_buffer(&output_buf, 0, &readback_buf, 0, values_size);

        queue.submit(std::iter::once(encoder.finish()));

        // -----------------------------------------------------------------
        // Readback.
        // -----------------------------------------------------------------
        let (tx, rx) = std::sync::mpsc::channel();
        let readback_slice = readback_buf.slice(..);
        readback_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv().map_err(|_| wgpu::BufferAsyncError)??;

        let mapped = readback_slice.get_mapped_range();
        let result_f32: &[f32] = bytemuck::cast_slice::<u8, f32>(&mapped);
        out.copy_from_slice(result_f32);

        drop(mapped);
        readback_buf.unmap();

        Ok(())
    }

    fn prepare_for_device(
        &mut self,
        prepared: &mut PreparedCodec,
    ) -> Result<(), TinyQuantGpuError> {
        let ctx = self.ctx.as_ref().ok_or(TinyQuantGpuError::NoAdapter)?;

        // Idempotent: skip if GPU state is already attached.
        if prepared.has_gpu_state() {
            return Ok(());
        }

        let device = &ctx.device;
        let dim = prepared.config().dimension() as usize;

        // -----------------------------------------------------------------
        // Build rotation matrix as f32 (from f64 source).
        // -----------------------------------------------------------------
        // The WGSL rotate shader computes:
        //   output[row, col] = sum_k input[row, k] * rotation[k, col]
        // which is `output = input_batch @ rotation_matrix` (row-vector convention).
        //
        // The CPU path computes `R @ v` (column-vector convention) where `R`
        // is the row-major matrix from RotationMatrix::matrix().
        //
        // To make the GPU match the CPU:
        //   - Forward (compress): bind R^T so `v @ R^T = R @ v` ✓
        //   - Inverse (decompress): bind R so `v @ R = R^T @ v` ✓
        //     (valid because R is orthogonal: R^{-1} = R^T)
        let rot_f64: &[f64] = prepared.rotation().matrix();
        let rot_f32: Vec<f32> = rot_f64.iter().map(|&v| v as f32).collect();

        // Compute R^T (transposed rotation matrix, for forward compress pass).
        let mut rot_t_f32: Vec<f32> = vec![0.0f32; dim * dim];
        for i in 0..dim {
            for j in 0..dim {
                rot_t_f32[j * dim + i] = rot_f32[i * dim + j];
            }
        }

        // -----------------------------------------------------------------
        // Upload rotation and transposed rotation to GPU buffers.
        //
        // rotation_buf   = R    → bound by decompress (inverse rotate pass)
        // rotation_t_buf = R^T  → bound by compress   (forward rotate pass)
        // -----------------------------------------------------------------
        let rotation_buf = Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("tinyquant/rotation"),      // R for inverse pass
                contents: bytemuck::cast_slice::<f32, u8>(&rot_f32),
                usage: wgpu::BufferUsages::STORAGE,     // read-only shader input
            },
        ));

        let rotation_t_buf = Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("tinyquant/rotation_T"),    // R^T for forward pass
                contents: bytemuck::cast_slice::<f32, u8>(&rot_t_f32),
                usage: wgpu::BufferUsages::STORAGE,     // read-only shader input
            },
        ));

        // -----------------------------------------------------------------
        // Upload codebook entries to GPU buffer.
        // -----------------------------------------------------------------
        let entries: &[f32] = prepared.codebook().entries();
        let n_entries = entries.len();
        let codebook_buf = Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("tinyquant/codebook"),
                contents: bytemuck::cast_slice::<f32, u8>(entries),
                usage: wgpu::BufferUsages::STORAGE,     // read-only shader input
            },
        ));

        // -----------------------------------------------------------------
        // Store GPU state.
        // -----------------------------------------------------------------
        let gpu_state = GpuPreparedState {
            rotation_buf,
            rotation_t_buf,
            codebook_buf,
            dim,
            n_entries,
        };
        prepared.set_gpu_state(Box::new(gpu_state));

        Ok(())
    }
}

