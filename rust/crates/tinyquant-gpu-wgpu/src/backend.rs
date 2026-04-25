//! `WgpuBackend`: implements `ComputeBackend` using wgpu + WGSL.
//!
//! # GPU pipeline overview
//!
//! ## compress_batch
//!
//! 1. Upload input vectors to GPU.
//! 2. Rotate: `rotated = R^T @ input`  (forward rotation shader).
//! 3. Quantize: `indices = argmin_k |rotated[i] - codebook[k]|`.
//! 4. (optional) Dequantize → residual encode: `residual = f16(rotated - reconstructed)`.
//! 5. Readback indices (and residual) → construct [`CompressedVector`] per row.
//!
//! ## decompress_batch_into
//!
//! 1. Upload indices to GPU.
//! 2. Dequantize: `values = codebook[indices]`.
//! 3. (optional) Residual decode: `values += f16_to_f32(residual)`.
//! 4. Inverse-rotate: `out = R @ values`.
//! 5. Readback result → write into `out` slice.

use crate::{
    backend_preference::{AdapterCandidate, BackendPreference},
    context::WgpuContext,
    error::TinyQuantGpuError,
    pipelines::{
        quantize::{build_dequantize_pipeline, build_quantize_pipeline},
        residual::{build_residual_decode_pipeline, build_residual_encode_pipeline},
        rotate::build_rotate_pipeline,
        search::build_cosine_topk_pipeline,
    },
    prepared::{GpuCorpusState, GpuPreparedState},
    ComputeBackend,
};
use std::sync::Arc;
use tinyquant_core::{
    backend::SearchResult,
    codec::{CompressedVector, PreparedCodec},
};
use wgpu::util::DeviceExt;

/// Lazily-cached compute pipelines for compress/decompress operations.
///
/// Each field is `None` until the first call that requires it, then stored
/// for all subsequent calls.
struct CachedPipelines {
    rotate: Option<wgpu::ComputePipeline>,
    quantize: Option<wgpu::ComputePipeline>,
    dequantize: Option<wgpu::ComputePipeline>,
    residual_enc: Option<wgpu::ComputePipeline>,
    residual_dec: Option<wgpu::ComputePipeline>,
}

impl CachedPipelines {
    fn new() -> Self {
        Self {
            rotate: None,
            quantize: None,
            dequantize: None,
            residual_enc: None,
            residual_dec: None,
        }
    }
}

/// GPU compute backend backed by wgpu.
///
/// Construct with [`WgpuBackend::new`]. If no adapter is available,
/// `new()` returns `Err(TinyQuantGpuError::NoAdapter)` — the caller should
/// fall back to the CPU path.
///
/// After construction, call [`prepare_corpus_for_device`](Self::prepare_corpus_for_device)
/// to upload a corpus for GPU nearest-neighbour search, then call
/// [`cosine_topk`](Self::cosine_topk) to score queries against it.
pub struct WgpuBackend {
    /// `None` when constructed via `unavailable()` (no-adapter stub).
    ctx: Option<WgpuContext>,
    /// GPU-resident corpus state; `None` until `prepare_corpus_for_device` is called.
    corpus_state: Option<GpuCorpusState>,
    /// Lazily-cached cosine top-k compute pipeline.
    search_pipeline: Option<wgpu::ComputePipeline>,
    /// Lazily-cached compress/decompress pipelines.
    pipelines: CachedPipelines,
}

impl WgpuBackend {
    /// Initialise the backend. Returns `Err(NoAdapter)` if no wgpu adapter is present.
    pub async fn new() -> Result<Self, TinyQuantGpuError> {
        let ctx = WgpuContext::new().await?;
        Ok(Self {
            ctx: Some(ctx),
            corpus_state: None,
            search_pipeline: None,
            pipelines: CachedPipelines::new(),
        })
    }

    /// Create a backend using the given adapter preference.
    ///
    /// Equivalent to [`new`](Self::new) when `pref` is [`BackendPreference::Auto`].
    ///
    /// # Errors
    ///
    /// - [`TinyQuantGpuError::NoAdapter`] — `Auto`/`HighPerformance`/`LowPower`
    ///   preference and no adapter found.
    /// - [`TinyQuantGpuError::NoPreferredAdapter`] — a specific backend was
    ///   requested (Vulkan, Metal, DX12, Software) and no adapter for it exists.
    pub async fn new_with_preference(pref: BackendPreference) -> Result<Self, TinyQuantGpuError> {
        let ctx = WgpuContext::new_with_preference(pref).await?;
        Ok(Self {
            ctx: Some(ctx),
            corpus_state: None,
            search_pipeline: None,
            pipelines: CachedPipelines::new(),
        })
    }

    /// No-adapter stub — `is_available()` returns `false`.
    ///
    /// `#[doc(hidden)]` keeps it out of the public rustdoc surface — callers
    /// should always prefer `WgpuBackend::new().await` and inspect `is_available()`.
    #[doc(hidden)]
    pub fn unavailable() -> Self {
        Self {
            ctx: None,
            corpus_state: None,
            search_pipeline: None,
            pipelines: CachedPipelines::new(),
        }
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

    /// Enumerate all adapters that match `pref`, in selection order.
    ///
    /// Returns an empty `Vec` when no adapter matches.  Does not open a device.
    pub fn enumerate_adapters(pref: BackendPreference) -> Vec<AdapterCandidate> {
        WgpuContext::enumerate_adapters(pref)
    }

    /// Pre-compile all compute pipelines and store them in `self.pipelines`.
    ///
    /// Normally pipelines are compiled lazily on the first call that needs them.
    /// Call this method upfront — for example at server start — to eliminate
    /// first-call shader-compilation latency.  Safe to call multiple times;
    /// subsequent calls are no-ops if all pipelines are already cached.
    ///
    /// # Errors
    ///
    /// - [`TinyQuantGpuError::NoAdapter`] — called on a no-adapter stub.
    pub fn load_pipelines(&mut self) -> Result<(), TinyQuantGpuError> {
        let ctx = self.ctx.as_ref().ok_or(TinyQuantGpuError::NoAdapter)?;
        if self.pipelines.rotate.is_none() {
            self.pipelines.rotate = Some(build_rotate_pipeline(ctx));
        }
        if self.pipelines.quantize.is_none() {
            self.pipelines.quantize = Some(build_quantize_pipeline(ctx));
        }
        if self.pipelines.dequantize.is_none() {
            self.pipelines.dequantize = Some(build_dequantize_pipeline(ctx));
        }
        if self.pipelines.residual_enc.is_none() {
            self.pipelines.residual_enc = Some(build_residual_encode_pipeline(ctx));
        }
        if self.pipelines.residual_dec.is_none() {
            self.pipelines.residual_dec = Some(build_residual_decode_pipeline(ctx));
        }
        if self.search_pipeline.is_none() {
            self.search_pipeline = Some(build_cosine_topk_pipeline(ctx));
        }
        Ok(())
    }

    /// Drop all cached pipelines, freeing GPU shader memory.
    ///
    /// After this call [`pipelines_loaded`](Self::pipelines_loaded) returns
    /// `false`.  Pipelines are rebuilt lazily on the next operation that needs
    /// them.  The `WgpuContext` (device + queue) is not affected.
    pub fn unload_pipelines(&mut self) {
        self.pipelines = CachedPipelines::new();
        self.search_pipeline = None;
    }

    /// Returns `true` when all compute pipelines (rotate, quantize, dequantize,
    /// residual encode/decode, and cosine top-k search) are compiled and cached.
    ///
    /// Returns `false` on a no-adapter stub.
    pub fn pipelines_loaded(&self) -> bool {
        self.ctx.is_some()
            && self.pipelines.rotate.is_some()
            && self.pipelines.quantize.is_some()
            && self.pipelines.dequantize.is_some()
            && self.pipelines.residual_enc.is_some()
            && self.pipelines.residual_dec.is_some()
            && self.search_pipeline.is_some()
    }

    /// Upload a decompressed FP32 corpus to device-resident memory.
    ///
    /// Must be called before [`cosine_topk`](Self::cosine_topk).
    ///
    /// # Errors
    ///
    /// - [`TinyQuantGpuError::NoAdapter`] — called on a no-adapter stub.
    /// - [`TinyQuantGpuError::DimensionMismatch`] — `corpus.len() ≠ n_rows * cols`.
    pub fn prepare_corpus_for_device(
        &mut self,
        corpus: &[f32],
        n_rows: usize,
        cols: usize,
    ) -> Result<(), TinyQuantGpuError> {
        let ctx = self.ctx.as_ref().ok_or(TinyQuantGpuError::NoAdapter)?;

        let expected = n_rows * cols;
        if corpus.len() != expected {
            return Err(TinyQuantGpuError::DimensionMismatch {
                expected,
                got: corpus.len(),
            });
        }

        let corpus_buf = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tinyquant/corpus"),
                contents: bytemuck::cast_slice::<f32, u8>(corpus),
                usage: wgpu::BufferUsages::STORAGE,
            });

        self.corpus_state = Some(GpuCorpusState {
            corpus_buf,
            n_rows,
            cols,
        });
        Ok(())
    }

    /// Score `query` against the GPU-resident corpus and return the top-k results
    /// sorted by descending cosine similarity.
    ///
    /// # Errors
    ///
    /// - [`TinyQuantGpuError::NoAdapter`] — called on a no-adapter stub.
    /// - [`TinyQuantGpuError::CorpusNotPrepared`] — corpus not uploaded yet.
    /// - [`TinyQuantGpuError::DimensionMismatch`] — `query.len() ≠ corpus cols`.
    /// - [`TinyQuantGpuError::BufferMap`] — GPU readback failed.
    /// - [`TinyQuantGpuError::InvalidTopK`] — `top_k == 0`.
    pub fn cosine_topk(
        &mut self,
        query: &[f32],
        top_k: usize,
    ) -> Result<Vec<SearchResult>, TinyQuantGpuError> {
        if top_k == 0 {
            return Err(TinyQuantGpuError::InvalidTopK);
        }
        if self.ctx.is_none() {
            return Err(TinyQuantGpuError::NoAdapter);
        }
        let (n_rows, cols) = {
            let corpus = self
                .corpus_state
                .as_ref()
                .ok_or(TinyQuantGpuError::CorpusNotPrepared)?;
            (corpus.n_rows, corpus.cols)
        };
        if query.len() != cols {
            return Err(TinyQuantGpuError::DimensionMismatch {
                expected: cols,
                got: query.len(),
            });
        }
        let effective_k = top_k.min(n_rows);
        if effective_k == 0 {
            return Ok(Vec::new());
        }

        if self.search_pipeline.is_none() {
            let p = build_cosine_topk_pipeline(self.ctx.as_ref().unwrap());
            self.search_pipeline = Some(p);
        }
        let pipeline = self.search_pipeline.as_ref().unwrap();
        let ctx = self.ctx.as_ref().unwrap();
        let corpus = self.corpus_state.as_ref().unwrap();
        let device = &ctx.device;
        let queue = &ctx.queue;

        let query_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tinyquant/search/query"),
            contents: bytemuck::cast_slice::<f32, u8>(query),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let scores_size = (n_rows * std::mem::size_of::<f32>()) as u64;
        let scores_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tinyquant/search/scores"),
            size: scores_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buf = create_readback_buf(device, "tinyquant/search/readback", scores_size);
        let dims: [u32; 4] = [n_rows as u32, cols as u32, top_k as u32, 0u32];
        let dims_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tinyquant/search/dims"),
            contents: bytemuck::cast_slice::<u32, u8>(&dims),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tinyquant/search/bg"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: dims_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: corpus.corpus_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: query_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: scores_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("tinyquant/search"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("tinyquant/search/cosine_topk"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((n_rows as u32).div_ceil(256), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&scores_buf, 0, &readback_buf, 0, scores_size);
        queue.submit(std::iter::once(encoder.finish()));

        poll_map_read(device, &readback_buf)?;
        let mapped = readback_buf.slice(..).get_mapped_range();
        let raw_scores: &[f32] = bytemuck::cast_slice::<u8, f32>(&mapped);
        let mut results: Vec<SearchResult> = raw_scores
            .iter()
            .copied()
            .enumerate()
            .map(|(row, score)| SearchResult {
                vector_id: Arc::from(row.to_string()),
                score,
            })
            .collect();
        results.sort();
        results.truncate(effective_k);
        drop(mapped);
        readback_buf.unmap();

        Ok(results)
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

        let codec_dim = prepared.config().dimension() as usize;
        if cols != codec_dim {
            return Err(TinyQuantGpuError::DimensionMismatch {
                expected: codec_dim,
                got: cols,
            });
        }
        let expected = rows * cols;
        if input.len() != expected {
            return Err(TinyQuantGpuError::DimensionMismatch {
                expected,
                got: input.len(),
            });
        }

        let residual_enabled = prepared.config().residual_enabled();

        // GPU residual path requires even dimension so fp16 pairs align per row.
        if residual_enabled && !cols.is_multiple_of(2) {
            return Err(TinyQuantGpuError::DimensionMismatch {
                expected: cols + 1,
                got: cols,
            });
        }

        let gpu_state = prepared
            .gpu_state()
            .and_then(|s| s.downcast_ref::<GpuPreparedState>())
            .ok_or(TinyQuantGpuError::NotPrepared)?;

        // -----------------------------------------------------------------
        // Lazily build required pipelines.
        // Borrows `self.pipelines.*` (mutable) and `self.ctx` (shared) as
        // disjoint fields — Rust 2021 NLL allows this split borrow.
        // -----------------------------------------------------------------
        if self.pipelines.rotate.is_none() {
            self.pipelines.rotate = Some(build_rotate_pipeline(ctx));
        }
        if self.pipelines.quantize.is_none() {
            self.pipelines.quantize = Some(build_quantize_pipeline(ctx));
        }
        if residual_enabled {
            if self.pipelines.dequantize.is_none() {
                self.pipelines.dequantize = Some(build_dequantize_pipeline(ctx));
            }
            if self.pipelines.residual_enc.is_none() {
                self.pipelines.residual_enc = Some(build_residual_encode_pipeline(ctx));
            }
        }

        let rotate_pipeline = self.pipelines.rotate.as_ref().unwrap();
        let quantize_pipeline = self.pipelines.quantize.as_ref().unwrap();
        let device = &ctx.device;
        let queue = &ctx.queue;

        // -----------------------------------------------------------------
        // Allocate GPU buffers.
        // -----------------------------------------------------------------
        let input_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tinyquant/compress/input"),
            contents: bytemuck::cast_slice::<f32, u8>(input),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let rotated_size = (rows * cols * std::mem::size_of::<f32>()) as u64;
        let rotated_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tinyquant/compress/rotated"),
            size: rotated_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let indices_size = (rows * cols * std::mem::size_of::<u32>()) as u64;
        let indices_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tinyquant/compress/indices"),
            size: indices_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buf = create_readback_buf(device, "tinyquant/compress/readback", indices_size);

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
        let rotate_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tinyquant/compress/rotate_bg"),
            layout: &rotate_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: rotate_dims_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
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

        let quantize_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        // Residual-path buffers + bind groups (only when residual_enabled).
        // Using a single Option<ResidualCompressBuffers> avoids placeholder logic.
        // -----------------------------------------------------------------
        struct ResidualCompressBuffers {
            #[allow(dead_code)]
            reconstructed_buf: wgpu::Buffer,
            residual_u16_buf: wgpu::Buffer,
            residual_readback_buf: wgpu::Buffer,
            n_pairs: usize,
            residual_size: u64,
            dequant_for_residual_bg: wgpu::BindGroup,
            residual_enc_bg: wgpu::BindGroup,
        }

        let residual_bufs: Option<ResidualCompressBuffers> = if residual_enabled {
            let dequantize_pipeline = self.pipelines.dequantize.as_ref().unwrap();
            let residual_enc_pipeline = self.pipelines.residual_enc.as_ref().unwrap();

            let reconstructed_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("tinyquant/compress/reconstructed"),
                size: rotated_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            let n_pairs = rows * cols / 2; // cols is even (checked above)
            let residual_size = (n_pairs * std::mem::size_of::<u32>()) as u64;
            let residual_u16_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("tinyquant/compress/residual_u16"),
                size: residual_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let residual_readback_buf = create_readback_buf(
                device,
                "tinyquant/compress/residual_readback",
                residual_size,
            );

            let dequant_for_residual_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tinyquant/compress/dequant_for_residual_bg"),
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
                        resource: reconstructed_buf.as_entire_binding(),
                    },
                ],
            });

            let residual_enc_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tinyquant/compress/residual_enc_bg"),
                layout: &residual_enc_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: rotated_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: reconstructed_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: residual_u16_buf.as_entire_binding(),
                    },
                ],
            });

            Some(ResidualCompressBuffers {
                reconstructed_buf,
                residual_u16_buf,
                residual_readback_buf,
                n_pairs,
                residual_size,
                dequant_for_residual_bg,
                residual_enc_bg,
            })
        } else {
            None
        };

        // -----------------------------------------------------------------
        // Encode and submit GPU commands.
        // -----------------------------------------------------------------
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("tinyquant/compress"),
        });

        // Pass 1: rotate (input → rotated).
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("tinyquant/compress/rotate"),
                timestamp_writes: None,
            });
            pass.set_pipeline(rotate_pipeline);
            pass.set_bind_group(0, &rotate_bg, &[]);
            pass.dispatch_workgroups((cols as u32).div_ceil(16), (rows as u32).div_ceil(16), 1);
        }

        // Pass 2: quantize (rotated → indices).
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("tinyquant/compress/quantize"),
                timestamp_writes: None,
            });
            pass.set_pipeline(quantize_pipeline);
            pass.set_bind_group(0, &quantize_bg, &[]);
            pass.dispatch_workgroups(((rows * cols) as u32).div_ceil(256), 1, 1);
        }

        // Passes 3–4: dequantize for residual + residual encode (only when residual_enabled).
        if let Some(rb) = &residual_bufs {
            let dequantize_pipeline = self.pipelines.dequantize.as_ref().unwrap();
            let residual_enc_pipeline = self.pipelines.residual_enc.as_ref().unwrap();

            // Pass 3: dequantize (indices → reconstructed).
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("tinyquant/compress/dequantize_for_residual"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(dequantize_pipeline);
                pass.set_bind_group(0, &rb.dequant_for_residual_bg, &[]);
                pass.dispatch_workgroups(((rows * cols) as u32).div_ceil(256), 1, 1);
            }

            // Pass 4: residual encode (rotated - reconstructed → residual_u16).
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("tinyquant/compress/residual_encode"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(residual_enc_pipeline);
                pass.set_bind_group(0, &rb.residual_enc_bg, &[]);
                pass.dispatch_workgroups((rb.n_pairs as u32).div_ceil(256), 1, 1);
            }

            encoder.copy_buffer_to_buffer(
                &rb.residual_u16_buf,
                0,
                &rb.residual_readback_buf,
                0,
                rb.residual_size,
            );
        }

        // Copy indices to readback buffer.
        encoder.copy_buffer_to_buffer(&indices_buf, 0, &readback_buf, 0, indices_size);
        queue.submit(std::iter::once(encoder.finish()));

        // -----------------------------------------------------------------
        // Readback: indices.
        // -----------------------------------------------------------------
        poll_map_read(device, &readback_buf)?;
        let mapped = readback_buf.slice(..).get_mapped_range();
        let u32_indices: &[u32] = bytemuck::cast_slice::<u8, u32>(&mapped);

        // Readback: residual bytes (when residual_enabled).
        let residual_bytes_all: Option<Vec<u8>> = if let Some(rb) = &residual_bufs {
            poll_map_read(device, &rb.residual_readback_buf)?;
            let res_mapped = rb.residual_readback_buf.slice(..).get_mapped_range();
            let bytes: Vec<u8> = res_mapped.to_vec();
            drop(res_mapped);
            rb.residual_readback_buf.unmap();
            Some(bytes)
        } else {
            None
        };

        // -----------------------------------------------------------------
        // Construct Vec<CompressedVector>.
        // residual bytes per row = cols * 2  (one f16 LE per element).
        // -----------------------------------------------------------------
        let config = prepared.config();
        let config_hash = config.config_hash().clone();
        let bit_width = config.bit_width();
        let dim = cols as u32;
        let residual_row_bytes = cols * 2;

        let mut result = Vec::with_capacity(rows);
        for row in 0..rows {
            let row_indices = &u32_indices[row * cols..(row + 1) * cols];
            let byte_indices: Vec<u8> = row_indices
                .iter()
                .map(|&v| {
                    debug_assert!(
                        v <= u8::MAX as u32,
                        "quantized index {v} exceeds u8::MAX for bit_width {bit_width}"
                    );
                    v as u8
                })
                .collect();

            let residual: Option<Box<[u8]>> = residual_bytes_all.as_ref().map(|all| {
                let start = row * residual_row_bytes;
                all[start..start + residual_row_bytes]
                    .to_vec()
                    .into_boxed_slice()
            });

            let cv = CompressedVector::new(
                byte_indices.into_boxed_slice(),
                residual,
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

        let gpu_state = prepared
            .gpu_state()
            .and_then(|s| s.downcast_ref::<GpuPreparedState>())
            .ok_or(TinyQuantGpuError::NotPrepared)?;

        let residual_enabled = prepared.config().residual_enabled();

        // Lazily build required pipelines.
        if self.pipelines.dequantize.is_none() {
            self.pipelines.dequantize = Some(build_dequantize_pipeline(ctx));
        }
        if self.pipelines.rotate.is_none() {
            self.pipelines.rotate = Some(build_rotate_pipeline(ctx));
        }
        if residual_enabled && self.pipelines.residual_dec.is_none() {
            self.pipelines.residual_dec = Some(build_residual_decode_pipeline(ctx));
        }

        let dequantize_pipeline = self.pipelines.dequantize.as_ref().unwrap();
        let rotate_pipeline = self.pipelines.rotate.as_ref().unwrap();
        let device = &ctx.device;
        let queue = &ctx.queue;

        // Flatten indices to u32.
        let mut flat_indices: Vec<u32> = Vec::with_capacity(rows * cols);
        for cv in compressed {
            for &b in cv.indices() {
                flat_indices.push(u32::from(b));
            }
        }

        let indices_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tinyquant/decompress/indices"),
            contents: bytemuck::cast_slice::<u32, u8>(&flat_indices),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let values_size = (rows * cols * std::mem::size_of::<f32>()) as u64;
        let values_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tinyquant/decompress/values"),
            size: values_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let output_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tinyquant/decompress/output"),
            size: values_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buf =
            create_readback_buf(device, "tinyquant/decompress/readback", values_size);

        let rotate_dims: [u32; 2] = [rows as u32, cols as u32];
        let rotate_dims_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tinyquant/decompress/rotate_dims"),
            contents: bytemuck::cast_slice::<u32, u8>(&rotate_dims),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let dequant_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
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

        let rotate_inv_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
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

        // Residual decode resources (only when residual_enabled).
        struct ResidualDecodeBuffers {
            _residual_buf: wgpu::Buffer, // kept alive for the GPU command lifetime
            residual_dec_bg: wgpu::BindGroup,
            n_pairs: u32,
        }

        let residual_dec_bufs: Option<ResidualDecodeBuffers> = if residual_enabled {
            let residual_dec_pipeline = self.pipelines.residual_dec.as_ref().unwrap();

            let mut flat_residual: Vec<u8> = Vec::with_capacity(rows * cols * 2);
            for cv in compressed {
                if let Some(res) = cv.residual() {
                    flat_residual.extend_from_slice(res);
                }
            }

            let residual_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tinyquant/decompress/residual_u16"),
                contents: &flat_residual,
                usage: wgpu::BufferUsages::STORAGE,
            });

            let residual_dec_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tinyquant/decompress/residual_dec_bg"),
                layout: &residual_dec_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: residual_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: values_buf.as_entire_binding(),
                    },
                ],
            });

            let n_pairs = (flat_residual.len() / (2 * std::mem::size_of::<u16>())) as u32;
            Some(ResidualDecodeBuffers {
                _residual_buf: residual_buf,
                residual_dec_bg,
                n_pairs,
            })
        } else {
            None
        };

        // -----------------------------------------------------------------
        // Encode and submit GPU commands.
        // -----------------------------------------------------------------
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("tinyquant/decompress"),
        });

        // Pass 1: dequantize (indices → values).
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("tinyquant/decompress/dequantize"),
                timestamp_writes: None,
            });
            pass.set_pipeline(dequantize_pipeline);
            pass.set_bind_group(0, &dequant_bg, &[]);
            pass.dispatch_workgroups(((rows * cols) as u32).div_ceil(256), 1, 1);
        }

        // Pass 2 (optional): residual decode (values += f16_to_f32(residual)).
        if let Some(rdb) = &residual_dec_bufs {
            let residual_dec_pipeline = self.pipelines.residual_dec.as_ref().unwrap();
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("tinyquant/decompress/residual_decode"),
                timestamp_writes: None,
            });
            pass.set_pipeline(residual_dec_pipeline);
            pass.set_bind_group(0, &rdb.residual_dec_bg, &[]);
            pass.dispatch_workgroups(rdb.n_pairs.div_ceil(256), 1, 1);
        }

        // Pass 3: inverse-rotate (values → output).
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("tinyquant/decompress/rotate_inv"),
                timestamp_writes: None,
            });
            pass.set_pipeline(rotate_pipeline);
            pass.set_bind_group(0, &rotate_inv_bg, &[]);
            pass.dispatch_workgroups((cols as u32).div_ceil(16), (rows as u32).div_ceil(16), 1);
        }

        encoder.copy_buffer_to_buffer(&output_buf, 0, &readback_buf, 0, values_size);
        queue.submit(std::iter::once(encoder.finish()));

        poll_map_read(device, &readback_buf)?;
        let mapped = readback_buf.slice(..).get_mapped_range();
        out.copy_from_slice(bytemuck::cast_slice::<u8, f32>(&mapped));
        drop(mapped);
        readback_buf.unmap();

        Ok(())
    }

    fn prepare_for_device(
        &mut self,
        prepared: &mut PreparedCodec,
    ) -> Result<(), TinyQuantGpuError> {
        let ctx = self.ctx.as_ref().ok_or(TinyQuantGpuError::NoAdapter)?;

        if prepared.has_gpu_state() {
            return Ok(());
        }

        let device = &ctx.device;
        let dim = prepared.config().dimension() as usize;

        let rot_f64: &[f64] = prepared.rotation().matrix();
        let rot_f32: Vec<f32> = rot_f64.iter().map(|&v| v as f32).collect();

        let mut rot_t_f32: Vec<f32> = vec![0.0f32; dim * dim];
        for i in 0..dim {
            for j in 0..dim {
                rot_t_f32[j * dim + i] = rot_f32[i * dim + j];
            }
        }

        let rotation_buf = Arc::new(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tinyquant/rotation"),
                contents: bytemuck::cast_slice::<f32, u8>(&rot_f32),
                usage: wgpu::BufferUsages::STORAGE,
            }),
        );
        let rotation_t_buf = Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("tinyquant/rotation_T"),
                contents: bytemuck::cast_slice::<f32, u8>(&rot_t_f32),
                usage: wgpu::BufferUsages::STORAGE,
            },
        ));

        let entries: &[f32] = prepared.codebook().entries();
        let n_entries = entries.len();
        let codebook_buf = Arc::new(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tinyquant/codebook"),
                contents: bytemuck::cast_slice::<f32, u8>(entries),
                usage: wgpu::BufferUsages::STORAGE,
            }),
        );

        prepared.set_gpu_state(Box::new(GpuPreparedState {
            rotation_buf,
            rotation_t_buf,
            codebook_buf,
            dim,
            n_entries,
        }));

        Ok(())
    }
}

/// Allocate a CPU-mappable staging buffer for GPU readback.
fn create_readback_buf(device: &wgpu::Device, label: &str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    })
}

/// Map `buf` for reading and block until the GPU signals completion.
fn poll_map_read(device: &wgpu::Device, buf: &wgpu::Buffer) -> Result<(), TinyQuantGpuError> {
    let (tx, rx) = std::sync::mpsc::channel::<Result<(), wgpu::BufferAsyncError>>();
    buf.slice(..).map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    Ok(rx.recv().map_err(|_| wgpu::BufferAsyncError)??)
}
