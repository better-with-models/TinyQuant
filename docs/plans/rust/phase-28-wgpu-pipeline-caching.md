---
title: "Phase 28: wgpu Pipeline Caching, Residual Pass & Backend Preference"
tags:
  - plans
  - rust
  - phase-28
  - gpu
  - wgpu
  - wgsl
  - backend-preference
date-created: 2026-04-18
status: active
category: planning
---

# Phase 28: wgpu Pipeline Caching, Residual Pass & Backend Preference

> [!info] Goal
> Close two known gaps carried forward from Phase 27, and add two new
> caller-facing affordances, all inside `tinyquant-gpu-wgpu`:
>
> 1. **Pipeline caching** — `compress_batch` and `decompress_batch_into`
>    currently rebuild the WGSL compute pipelines (rotate, quantize,
>    dequantize) on every call.  Device shader compilation takes on the
>    order of milliseconds per call and will dominate the FR-GPU-004 ≤ 5 ms
>    throughput target on real hardware.  Add lazy-cached pipeline fields
>    to `WgpuBackend`, matching the pattern already used by `search_pipeline`
>    in Phase 27.5.
>
> 2. **Residual encode/decode pass** — `compress_batch` currently returns
>    `Err(ResidualNotSupported)` for any `PreparedCodec` with
>    `residual_enabled() == true`.  The WGSL shader (`residual_encode.wgsl`)
>    and pipeline builder (`build_residual_encode_pipeline`) were written in
>    Phase 27 but never wired.  This phase wires them and adds the
>    corresponding residual decode pass to `decompress_batch_into`.
>
> 3. **Explicit pipeline lifecycle** — expose `load_pipelines` /
>    `unload_pipelines` / `pipelines_loaded` on `WgpuBackend` so callers
>    can precompile all shaders on command (e.g., at server start) and
>    release GPU shader memory when idle.
>
> 4. **Backend preference & adapter enumeration** — expose a
>    `BackendPreference` enum and `WgpuBackend::new_with_preference` so
>    callers can request Vulkan, Metal, DX12, high-performance, or
>    low-power adapters explicitly.  Add `WgpuBackend::enumerate_adapters`
>    so callers can probe what is available before committing.  Wire the
>    appropriate platform backend features in `Cargo.toml` so all
>    supported backends are compiled in by default.
>
> No new crates, no changes to `tinyquant-core`.  All work is inside
> `tinyquant-gpu-wgpu`.

> [!note] Reference docs
> - [[design/rust/gpu-acceleration|GPU Acceleration Design]] — §Kernels, §Memory strategy
> - [[design/rust/phase-27-implementation-notes|Phase 27 Implementation Notes]] — §Pipeline Caching Deferred to Phase 28, §Residual encode/decode pass not implemented
> - [[requirements/gpu|GPU Requirements]] — FR-GPU-003, FR-GPU-004
> - [[plans/rust/phase-27-wgpu-wgsl-kernels|Phase 27]] — prerequisite; ships the crate skeleton and static pipeline helpers
> - [[plans/rust/phase-27.5-resident-corpus-search|Phase 27.5]] — prerequisite; FR-GPU-004 throughput baseline

## Prerequisites

- Phase 27.5 complete: `WgpuBackend::cosine_topk` functional and FR-GPU-004
  throughput measurement recorded.
- `WgpuContext`, `ComputeBackend`, `GpuPreparedState` all in place.
- `shaders/residual_encode.wgsl` and `src/pipelines/residual.rs`
  (`build_residual_encode_pipeline`) exist but are unused.

## Scope

All changes are within `tinyquant-gpu-wgpu`.  No other crate is modified.

Modified files:

| File | Change |
|---|---|
| `Cargo.toml` | Add `vulkan` and platform backend features to wgpu dependency |
| `src/backend.rs` | Add `CachedPipelines` field; lazy-build pipelines; wire residual; add `load_pipelines`, `unload_pipelines`, `pipelines_loaded`; add `new_with_preference`, `enumerate_adapters` |
| `src/context.rs` | Accept `BackendPreference` in `WgpuContext::new_with_preference`; add adapter scoring |
| `src/lib.rs` | Re-export `BackendPreference`, `AdapterCandidate`; remove dead-code attr |
| `src/error.rs` | Add `NoPreferredAdapter` variant |
| `shaders/residual_encode.wgsl` | Validate and fix if needed |
| `shaders/residual_decode.wgsl` | New shader for residual decode pass |
| `src/pipelines/residual.rs` | Add `build_residual_decode_pipeline`; remove dead-code attr |
| `tests/parity_compress.rs` | Add residual parity test and repeated-call regression guard |
| `tests/context_probe.rs` | Add `enumerate_adapters` smoke test; `load_pipelines` smoke test |

---

## Part A — Pipeline caching (Steps 1–2)

### Step 1 — Failing tests (red)

Add a throughput regression guard to `tests/parity_compress.rs`:

```rust
/// Calling compress_batch twice on the same WgpuBackend must not
/// recompile pipelines — the second call must be faster than a threshold
/// that exceeds any plausible compile latency.
///
/// On a no-GPU (llvmpipe) runner this test is gated with `skip_if_no_adapter`.
/// On a real GPU runner it enforces FR-GPU-004's spirit: repeated calls must
/// not regress due to per-call shader compilation.
#[test]
fn compress_batch_repeated_calls_do_not_recompile() {
    let Some(mut backend) = skip_if_no_adapter() else {
        eprintln!("SKIP: no wgpu adapter");
        return;
    };

    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let training = vec![0f32; 256 * 64];
    let codebook = Codebook::train(&config, &training).unwrap();
    let mut prepared = PreparedCodec::new(config.clone(), codebook).unwrap();
    backend.prepare_for_device(&mut prepared).unwrap();

    let n = 512usize;
    let dim = 64usize;
    let input: Vec<f32> = (0..n * dim).map(|i| (i as f32 * 0.001).sin()).collect();

    // Warm-up call (pipeline build happens here).
    let _ = backend.compress_batch(&input, n, dim, &prepared).unwrap();

    // Second call — must not rebuild pipelines.  Measure it.
    let t0 = std::time::Instant::now();
    let _ = backend.compress_batch(&input, n, dim, &prepared).unwrap();
    let elapsed_ms = t0.elapsed().as_secs_f64() * 1e3;

    // 50 ms is a very conservative upper bound: pipeline compilation alone
    // typically costs 5–50 ms; if we're below 50 ms we did NOT recompile.
    // On a real GPU runner this should be < 1 ms; on llvmpipe < 20 ms.
    assert!(
        elapsed_ms < 50.0,
        "second compress_batch call took {elapsed_ms:.1} ms — likely recompiling pipelines"
    );
}
```

### Step 2 — Implement `CachedPipelines`

Add a struct and fields to `WgpuBackend` in `src/backend.rs`:

```rust
/// Lazily-cached compute pipelines for compress/decompress operations.
///
/// Each field is `None` until the first call that requires it, then stored
/// for all subsequent calls.  The `search_pipeline` field uses the same
/// pattern and was introduced in Phase 27.5.
struct CachedPipelines {
    rotate:      Option<wgpu::ComputePipeline>,
    quantize:    Option<wgpu::ComputePipeline>,
    dequantize:  Option<wgpu::ComputePipeline>,
    residual_enc: Option<wgpu::ComputePipeline>,
    residual_dec: Option<wgpu::ComputePipeline>,
}

impl CachedPipelines {
    fn new() -> Self {
        Self {
            rotate:       None,
            quantize:     None,
            dequantize:   None,
            residual_enc: None,
            residual_dec: None,
        }
    }
}
```

Update `WgpuBackend`:

```rust
pub struct WgpuBackend {
    ctx:             Option<WgpuContext>,
    corpus_state:    Option<GpuCorpusState>,
    search_pipeline: Option<wgpu::ComputePipeline>,
    pipelines:       CachedPipelines,               // ← new
}

impl WgpuBackend {
    pub async fn new() -> Result<Self, TinyQuantGpuError> {
        let ctx = WgpuContext::new().await?;
        Ok(Self {
            ctx: Some(ctx),
            corpus_state: None,
            search_pipeline: None,
            pipelines: CachedPipelines::new(),      // ← new
        })
    }

    pub fn unavailable() -> Self {
        Self {
            ctx: None,
            corpus_state: None,
            search_pipeline: None,
            pipelines: CachedPipelines::new(),      // ← new
        }
    }
}
```

In `compress_batch`, replace:

```rust
// TODO(phase-28): cache pipelines in WgpuBackend to avoid per-call shader
// compilation (order of milliseconds per call). ...
let rotate_pipeline   = build_rotate_pipeline(ctx);
let quantize_pipeline = build_quantize_pipeline(ctx);
```

with:

```rust
// Lazy-build rotate and quantize pipelines (compiled once per WgpuBackend).
let rotate_pipeline = self.pipelines.rotate.get_or_insert_with(|| {
    build_rotate_pipeline(self.ctx.as_ref().unwrap())
});
let quantize_pipeline = self.pipelines.quantize.get_or_insert_with(|| {
    build_quantize_pipeline(self.ctx.as_ref().unwrap())
});
```

Apply the same pattern for `dequantize_pipeline` in `decompress_batch_into`.

> [!note] Borrow-checker note
> `get_or_insert_with` takes a `&mut self.pipelines.*` borrow. Because
> `ctx` is accessed inside the closure via `self.ctx`, split the borrows
> by extracting `ctx` before the closure: build pipeline using a local
> `ctx` ref captured before entering `compress_batch`'s body.  The
> existing code already does this (see the `let ctx = self.ctx.as_ref()...`
> guard at the top of each method); extend the pattern to pipeline init.

---

## Part B — Residual encode/decode pass (Steps 3–5)

### Step 3 — Failing tests (red)

Add to `tests/parity_compress.rs`:

```rust
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

    let config = CodecConfig::new(4, 42, 64, true).unwrap();  // residual_enabled = true
    let training = vec![0f32; 256 * 64];
    let codebook = Codebook::train(&config, &training).unwrap();
    let mut prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();
    backend.prepare_for_device(&mut prepared).unwrap();

    let n = 128usize;
    let dim = 64usize;
    let input: Vec<f32> = (0..n * dim)
        .map(|i| (i as f32 * 0.003).cos())
        .collect();

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
            .decompress_prepared_into(
                &cv,
                &prepared,
                &mut cpu_out[i * dim..(i + 1) * dim],
            )
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
```

### Step 4 — Write `shaders/residual_decode.wgsl` and add `build_residual_decode_pipeline`

#### `shaders/residual_decode.wgsl`

Mirrors the buffer layout of `residual_encode.wgsl`.  Each invocation
processes one `u32` pack (two packed fp16 residuals) and **adds** the decoded
residuals in-place to `values`.  After this pass:
`values[i] += f16_to_f32(residual_f16[i])`, which reconstructs
`dequantized + (original_rotated − dequantized) ≈ original_rotated` before
the inverse-rotate pass.

```wgsl
// shaders/residual_decode.wgsl
//
// Unpack fp16 pairs from `residual_u16` and add them in-place to `values`.
//
// Buffer layout mirrors residual_encode.wgsl:
//   residual_u16[k] encodes two fp16 residuals:
//     lo 16 bits  = residual_f16[2k]
//     hi 16 bits  = residual_f16[2k+1]
//
// After this pass: values[i] += f16_to_f32(residual_f16[i])
// Dispatch: ceil(arrayLength(residual_u16) / 256) workgroups.

@group(0) @binding(0) var<storage, read>       residual_u16: array<u32>;  // packed fp16 pairs
@group(0) @binding(1) var<storage, read_write> values:       array<f32>;  // dequantized values (modified in-place)

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // One invocation per u32 pack (= two fp16 residuals).
    let pair_idx = gid.x;
    if (pair_idx >= arrayLength(&residual_u16)) { return; }

    let packed   = residual_u16[pair_idx];
    let n_values = arrayLength(&values);

    let idx0 = pair_idx * 2u;
    let idx1 = pair_idx * 2u + 1u;

    if (idx0 < n_values) {
        values[idx0] += f16_to_f32(packed & 0xFFFFu);
    }
    if (idx1 < n_values) {
        values[idx1] += f16_to_f32((packed >> 16u) & 0xFFFFu);
    }
}

/// Convert a 16-bit IEEE 754 half-precision bit pattern to f32.
///
/// Handles all IEEE 754 cases: ±zero, subnormal f16, normal, ±inf, NaN.
fn f16_to_f32(h: u32) -> f32 {
    let sign     = (h >> 15u) & 0x1u;
    let h_exp    = (h >> 10u) & 0x1Fu;
    let mantissa = h & 0x3FFu;

    // ±zero
    if (h_exp == 0u && mantissa == 0u) {
        return bitcast<f32>(sign << 31u);
    }
    // inf or NaN
    if (h_exp == 31u) {
        return bitcast<f32>((sign << 31u) | 0x7F800000u | (mantissa << 13u));
    }
    // subnormal f16 (h_exp == 0, mantissa != 0):
    //   value = (-1)^sign × 2^(−14) × (mantissa / 1024)
    //   f32 biased exponent: -14 + 127 = 113; mantissa left-shifted by 13.
    if (h_exp == 0u) {
        return bitcast<f32>((sign << 31u) | (113u << 23u) | (mantissa << 13u));
    }
    // normal f16 → normal f32
    let f_exp  = h_exp + 127u - 15u;
    let f_bits = (sign << 31u) | (f_exp << 23u) | (mantissa << 13u);
    return bitcast<f32>(f_bits);
}
```

> [!note] Fix for `residual_encode.wgsl` subnormal guard
> `residual_encode.wgsl` contains a known bug (marked `KNOWN LIMITATION`):
> `h_exp` is `u32` so `if (h_exp <= 0u)` is always false, causing subnormal
> f32 inputs to wrap instead of flush.  Fix `f32_to_f16` in that shader as
> part of this step by adding an explicit biased-exponent guard:
>
> ```wgsl
> // Replace the dead `if (h_exp <= 0u)` branch with an explicit underflow guard.
> // h_exp = exp - 127 + 15; underflows when exp < 112 (biased).
> if (exp < 113u) { return (bits >> 31u) << 15u; }  // flush subnormal to ±zero f16
> let h_exp = exp - 127u + 15u;
> // h_exp is now ≥ 1; the `if (h_exp <= 0u)` guard below is removed.
> ```
>
> Remove the now-dead `if (h_exp <= 0u) { return 0u; }` line and the
> KNOWN LIMITATION comment.

#### `src/pipelines/residual.rs` — add `build_residual_decode_pipeline`

```rust
/// Build the compute pipeline for the GPU residual decode pass.
///
/// Semantics: for each packed fp16 pair in `residual_u16`, unpack both
/// fp16 values and add them in-place to `values`.  After this pass
/// `values[i]` equals the original pre-quantisation value (to fp16
/// precision), ready for the inverse-rotate pass.
///
/// Buffer bindings (group 0):
///   0 — `residual_u16`: `storage, read`   — packed fp16 pairs
///   1 — `values`:       `storage, read_write` — dequantised values
pub fn build_residual_decode_pipeline(ctx: &WgpuContext) -> wgpu::ComputePipeline {
    ctx.build_compute_pipeline(
        "tinyquant/residual_decode",
        include_str!("../../shaders/residual_decode.wgsl"),
        "main",
    )
}
```

### Step 5 — Wire residual passes in `backend.rs`

**In `compress_batch`:** remove the `ResidualNotSupported` early-return guard
and add Pass 3 after the existing quantize pass:

```rust
// Pass 3: residual encode (only when residual_enabled).
if prepared.config().residual_enabled() {
    let residual_pipeline = self.pipelines.residual_enc.get_or_insert_with(|| {
        build_residual_encode_pipeline(ctx)
    });
    // Encode residual using the quantised indices and the original input.
    // Bind dims, codebook, input, indices, residual_indices output buffer.
    // Dispatch and copy residual_indices to readback buffer.
    // Append residual payload to each CompressedVector.
    // (Full bind-group and dispatch code follows the same pattern as Pass 2.)
    todo!("Phase 28: residual encode pass")
}
```

Replace the `todo!` with the full bind-group + dispatch + readback + payload
attachment implementation, following the same structure as the quantize pass.

**In `decompress_batch_into`:** add Pass 0 before the existing dequantize pass:

```rust
// Pass 0: residual decode (only when residual_enabled).
if prepared.config().residual_enabled() {
    let residual_dec_pipeline = self.pipelines.residual_dec.get_or_insert_with(|| {
        build_residual_decode_pipeline(ctx)
    });
    // Decode residual indices → residual values, then subtract from values_buf
    // before the inverse-rotate pass.
    todo!("Phase 28: residual decode pass")
}
```

Remove `#[allow(dead_code)]` from `build_residual_encode_pipeline` in
`src/pipelines/residual.rs`.

---

## Part C — Explicit pipeline lifecycle (Steps 6–7)

### Step 6 — Failing tests (red)

Add to `tests/context_probe.rs`:

```rust
/// load_pipelines must succeed on an adapter-present backend and set
/// pipelines_loaded() to true.
#[test]
fn load_pipelines_marks_loaded() {
    let Some(mut backend) = skip_if_no_adapter() else {
        eprintln!("SKIP: no wgpu adapter");
        return;
    };
    assert!(!backend.pipelines_loaded(), "pipelines must start unloaded");
    backend.load_pipelines().expect("load_pipelines must succeed");
    assert!(backend.pipelines_loaded(), "pipelines must be loaded after load_pipelines");
}

/// unload_pipelines must release all cached pipelines.
#[test]
fn unload_pipelines_clears_loaded_state() {
    let Some(mut backend) = skip_if_no_adapter() else {
        eprintln!("SKIP: no wgpu adapter");
        return;
    };
    backend.load_pipelines().unwrap();
    assert!(backend.pipelines_loaded());
    backend.unload_pipelines();
    assert!(!backend.pipelines_loaded(), "pipelines must be unloaded after unload_pipelines");
}

/// load_pipelines must be idempotent.
#[test]
fn load_pipelines_is_idempotent() {
    let Some(mut backend) = skip_if_no_adapter() else {
        eprintln!("SKIP: no wgpu adapter");
        return;
    };
    backend.load_pipelines().unwrap();
    backend.load_pipelines().expect("second load_pipelines must succeed");
    assert!(backend.pipelines_loaded());
}

/// compress_batch after load_pipelines must not regress vs unloaded path.
#[test]
fn compress_after_explicit_load_succeeds() {
    let Some(mut backend) = skip_if_no_adapter() else {
        eprintln!("SKIP: no wgpu adapter");
        return;
    };
    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let training = vec![0f32; 256 * 64];
    let codebook = Codebook::train(&config, &training).unwrap();
    let mut prepared = PreparedCodec::new(config.clone(), codebook).unwrap();
    backend.prepare_for_device(&mut prepared).unwrap();

    backend.load_pipelines().unwrap();

    let input: Vec<f32> = (0..64).map(|i| i as f32 * 0.01).collect();
    backend
        .compress_batch(&input, 1, 64, &prepared)
        .expect("compress_batch after load_pipelines must succeed");
}
```

### Step 7 — Implement `load_pipelines`, `unload_pipelines`, `pipelines_loaded`

Add to `WgpuBackend` in `src/backend.rs`:

```rust
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
    // Build each pipeline if not already cached.
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
```

---

## Part D — Backend preference & adapter enumeration (Steps 8–10)

### Step 8 — Failing tests (red)

Add to `tests/context_probe.rs`:

```rust
/// enumerate_adapters(Auto) must return at least one candidate when any
/// wgpu-compatible adapter is present (physical or software).
#[test]
fn enumerate_adapters_returns_at_least_one_on_adapter_machine() {
    // This test is meaningful only on machines where WgpuBackend::new succeeds.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let backend_result = rt.block_on(WgpuBackend::new());
    if backend_result.is_err() {
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
    // Either both succeed or both fail — the failure mode is the same.
    match rt.block_on(WgpuBackend::new()) {
        Ok(_) => assert!(result.is_ok(), "new_with_preference(Auto) must succeed when new() succeeds"),
        Err(_) => assert!(result.is_err(), "new_with_preference(Auto) must fail when new() fails"),
    }
}

/// new_with_preference(Software) must succeed on any CI runner (software
/// renderer is always present when wgpu is compiled in).
#[test]
fn new_with_preference_software_available_on_ci() {
    // Software/GL backends may not be available in all environments.
    // This test is advisory: skip rather than fail if no software adapter.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    match rt.block_on(WgpuBackend::new_with_preference(BackendPreference::Software)) {
        Ok(b) => assert!(b.is_available()),
        Err(TinyQuantGpuError::NoPreferredAdapter) => {
            eprintln!("SKIP: no software adapter available in this environment");
        }
        Err(e) => panic!("unexpected error: {e}"),
    }
}
```

### Step 9 — Add `BackendPreference` and `AdapterCandidate` types

Add `src/backend_preference.rs` (new file):

```rust
//! `BackendPreference`: caller-facing backend and adapter selection hint.

/// Controls which wgpu backend and physical adapter `WgpuBackend` selects.
///
/// Passed to [`WgpuBackend::new_with_preference`] and
/// [`WgpuBackend::enumerate_adapters`].
///
/// # Platform notes
///
/// Not all variants are available on all platforms:
/// - [`Metal`](Self::Metal) is only available on macOS/iOS.
/// - [`Dx12`](Self::Dx12) is only available on Windows.
/// - [`Vulkan`](Self::Vulkan) requires a Vulkan-capable driver (Linux,
///   Windows, Android; optional on macOS via MoltenVK).
///
/// When the preferred backend has no adapter,
/// [`new_with_preference`](WgpuBackend::new_with_preference) returns
/// [`TinyQuantGpuError::NoPreferredAdapter`] rather than silently falling
/// through to another backend.  Use [`Auto`](Self::Auto) for transparent
/// fallback behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendPreference {
    /// Select the highest-performance available adapter automatically.
    ///
    /// Tries primary backends (Metal → DX12 → Vulkan → WebGPU) in
    /// platform-native preference order.  Falls back to secondary backends
    /// (GL) if no primary adapter is found.  This is the behaviour of
    /// [`WgpuBackend::new`].
    #[default]
    Auto,
    /// Prefer a Vulkan adapter.  Returns
    /// [`NoPreferredAdapter`](TinyQuantGpuError::NoPreferredAdapter) if no
    /// Vulkan driver is available.
    Vulkan,
    /// Prefer a Metal adapter.  Returns
    /// [`NoPreferredAdapter`](TinyQuantGpuError::NoPreferredAdapter) on
    /// non-Apple platforms.
    Metal,
    /// Prefer a DirectX 12 adapter.  Returns
    /// [`NoPreferredAdapter`](TinyQuantGpuError::NoPreferredAdapter) on
    /// non-Windows platforms.
    Dx12,
    /// Prefer the discrete GPU.  Falls back to integrated GPU, then
    /// virtual GPU, then software.  Uses
    /// [`wgpu::PowerPreference::HighPerformance`] within the primary
    /// backend set.
    HighPerformance,
    /// Prefer the integrated or battery-saving GPU.  Uses
    /// [`wgpu::PowerPreference::LowPower`] within the primary backend set.
    LowPower,
    /// Force a software (CPU-emulated) renderer.  Never selects a
    /// physical GPU.  Available wherever wgpu compiles the GL backend
    /// (e.g., Mesa llvmpipe on Linux, ANGLE on Windows).  Useful for
    /// headless testing.
    Software,
}

impl BackendPreference {
    /// Returns the `wgpu::Backends` bitset to use when creating the wgpu
    /// instance and enumerating adapters for this preference.
    pub(crate) fn to_backends(self) -> wgpu::Backends {
        match self {
            Self::Auto | Self::HighPerformance | Self::LowPower => wgpu::Backends::PRIMARY,
            Self::Vulkan => wgpu::Backends::VULKAN,
            Self::Metal  => wgpu::Backends::METAL,
            Self::Dx12   => wgpu::Backends::DX12,
            Self::Software => wgpu::Backends::GL,
        }
    }

    /// Returns the `wgpu::PowerPreference` for this preference.
    pub(crate) fn to_power_preference(self) -> wgpu::PowerPreference {
        match self {
            Self::LowPower => wgpu::PowerPreference::LowPower,
            _ => wgpu::PowerPreference::HighPerformance,
        }
    }
}

/// A discovered wgpu adapter candidate returned by
/// [`WgpuBackend::enumerate_adapters`].
///
/// Does not hold a live device handle — creating this struct does not
/// consume driver resources.
#[derive(Debug, Clone)]
pub struct AdapterCandidate {
    /// Driver-provided adapter name (e.g. `"NVIDIA GeForce RTX 3060"`).
    pub name: String,
    /// The wgpu backend this adapter belongs to.
    pub backend: wgpu::Backend,
    /// Device type: discrete GPU, integrated GPU, virtual GPU, or CPU
    /// (software renderer).
    pub device_type: wgpu::DeviceType,
    /// PCI vendor ID (0 for software adapters).
    pub vendor: u32,
    /// PCI device ID (0 for software adapters).
    pub device: u32,
}
```

Re-export from `src/lib.rs`:

```rust
pub use crate::backend_preference::{AdapterCandidate, BackendPreference};
```

Add `NoPreferredAdapter` to `src/error.rs`:

```rust
/// No adapter matched the requested [`BackendPreference`].
///
/// The requested backend (Vulkan, Metal, DX12, or Software) produced no
/// adapters on this machine.  Use [`BackendPreference::Auto`] for
/// transparent fallback.
#[error("no adapter found for the requested backend preference")]
NoPreferredAdapter,
```

### Step 10 — Implement `enumerate_adapters` and `new_with_preference`

In `src/context.rs`, update `WgpuContext::new` signature and add
`new_with_preference`:

```rust
impl WgpuContext {
    /// Initialise wgpu with `BackendPreference::Auto`.
    pub async fn new() -> Result<Self, TinyQuantGpuError> {
        Self::new_with_preference(BackendPreference::Auto).await
    }

    /// Initialise wgpu using the given backend preference.
    ///
    /// Returns `Err(NoPreferredAdapter)` when the preference specifies a
    /// concrete backend (Vulkan, Metal, DX12, Software) and no adapter for
    /// that backend is present.
    pub async fn new_with_preference(
        pref: BackendPreference,
    ) -> Result<Self, TinyQuantGpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: pref.to_backends(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: pref.to_power_preference(),
                compatible_surface: None,
                force_fallback_adapter: matches!(pref, BackendPreference::Software),
            })
            .await
            .ok_or(match pref {
                BackendPreference::Auto
                | BackendPreference::HighPerformance
                | BackendPreference::LowPower => TinyQuantGpuError::NoAdapter,
                _ => TinyQuantGpuError::NoPreferredAdapter,
            })?;

        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .map_err(TinyQuantGpuError::DeviceRequest)?;

        Ok(Self { device, queue, adapter_info })
    }

    /// Synchronously enumerate all adapters for the given preference,
    /// in selection order (best match first).
    ///
    /// Does not create a `Device` — safe to call before committing to a
    /// particular adapter.
    pub fn enumerate_adapters(pref: BackendPreference) -> Vec<AdapterCandidate> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: pref.to_backends(),
            ..Default::default()
        });
        let mut candidates: Vec<AdapterCandidate> = instance
            .enumerate_adapters(pref.to_backends())
            .map(|a| {
                let info = a.get_info();
                AdapterCandidate {
                    name:        info.name,
                    backend:     info.backend,
                    device_type: info.device_type,
                    vendor:      info.vendor,
                    device:      info.device,
                }
            })
            .collect();

        // Sort: discrete GPU > integrated > virtual > CPU (software).
        candidates.sort_by_key(|c| device_type_rank(c.device_type));
        candidates
    }
}

/// Lower rank = higher preference.
fn device_type_rank(dt: wgpu::DeviceType) -> u8 {
    match dt {
        wgpu::DeviceType::DiscreteGpu   => 0,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::VirtualGpu    => 2,
        wgpu::DeviceType::Cpu           => 3,
        wgpu::DeviceType::Other         => 4,
    }
}
```

Add to `WgpuBackend` in `src/backend.rs`:

```rust
/// Create a backend using the given adapter preference.
///
/// Equivalent to [`new`](Self::new) when `pref` is
/// [`BackendPreference::Auto`].
///
/// # Errors
///
/// - [`TinyQuantGpuError::NoAdapter`] — `Auto`/`HighPerformance`/`LowPower`
///   preference and no adapter found.
/// - [`TinyQuantGpuError::NoPreferredAdapter`] — a specific backend was
///   requested (Vulkan, Metal, DX12, Software) and no adapter for it exists.
pub async fn new_with_preference(
    pref: BackendPreference,
) -> Result<Self, TinyQuantGpuError> {
    let ctx = WgpuContext::new_with_preference(pref).await?;
    Ok(Self {
        ctx: Some(ctx),
        corpus_state: None,
        search_pipeline: None,
        pipelines: CachedPipelines::new(),
    })
}

/// Enumerate all adapters that match `pref`, in selection order.
///
/// The first entry is the adapter that
/// [`new_with_preference(pref)`](Self::new_with_preference) would select.
/// Returns an empty `Vec` when no adapter matches.
///
/// This is a synchronous probe — it does not open a device and does not
/// consume any GPU resources.
pub fn enumerate_adapters(pref: BackendPreference) -> Vec<AdapterCandidate> {
    WgpuContext::enumerate_adapters(pref)
}
```

### Step 10b — Cargo.toml: explicit backend features

Update `Cargo.toml` to opt in to the Vulkan backend (not always included in
wgpu's defaults) and make all compiled-in backends visible:

```toml
[dependencies]
tinyquant-core = { workspace = true }
wgpu           = { version = "22", features = ["wgsl", "vulkan"] }
tokio          = { version = "1", features = ["rt"] }
thiserror      = { workspace = true }
bytemuck       = { workspace = true }
```

> [!note] wgpu 22 default backends
> wgpu 22 compiles in `metal`, `dx12`, and `webgpu` by default on the
> appropriate platforms.  The `vulkan` feature is not always in the default
> set, so we add it explicitly.  The `BackendPreference::to_backends()`
> helper maps preferences to `wgpu::Backends` bitsets at runtime, so the
> actual adapter in use reflects what's installed on the host machine — the
> feature flags control only what is compiled in, not what is selected.

---

## Steps summary

| Step | Gap / TODO closed | File | Est. effort |
|---|---|---|---|
| 1–2 | TODO(phase-28) pipeline caching in `backend.rs` | `backend.rs`, `parity_compress.rs` | 2–3 h |
| 3–5 | TODO(phase-28) residual pass in `backend.rs`; FR-GPU-003 residual parity | `backend.rs`, `pipelines/residual.rs`, two WGSL shaders, `parity_compress.rs` | 4–6 h |
| 6–7 | — | `backend.rs` (`load_pipelines`, `unload_pipelines`, `pipelines_loaded`), `context_probe.rs` | 1–2 h |
| 8–10 | — | `src/backend_preference.rs` (new), `src/context.rs`, `src/backend.rs`, `src/lib.rs`, `src/error.rs`, `Cargo.toml`, `context_probe.rs` | 2–3 h |

---

## Acceptance criteria

- [ ] `cargo test -p tinyquant-gpu-wgpu --test parity_compress` — all tests
      pass, including `compress_decompress_residual_enabled_matches_cpu` and
      `compress_batch_repeated_calls_do_not_recompile`.
- [ ] `compress_batch_repeated_calls_do_not_recompile`: second call < 50 ms
      on any adapter (including llvmpipe).
- [ ] `compress_decompress_residual_enabled_matches_cpu`: max delta ≤ 1e-3
      (FR-GPU-003 tolerance).
- [ ] No `TODO(phase-28)` markers remain in `backend.rs`.
- [ ] No `#[allow(dead_code)]` on `build_residual_encode_pipeline`.
- [ ] `cargo test -p tinyquant-gpu-wgpu --test context_probe` — all tests
      pass, including `load_pipelines_marks_loaded`,
      `unload_pipelines_clears_loaded_state`,
      `load_pipelines_is_idempotent`,
      `enumerate_adapters_returns_at_least_one_on_adapter_machine`,
      `new_with_preference_auto_equivalent_to_new`.
- [ ] `WgpuBackend::enumerate_adapters(BackendPreference::Auto)` returns ≥ 1
      entry on a runner with any wgpu adapter.
- [ ] `WgpuBackend::new_with_preference(BackendPreference::Auto)` succeeds
      wherever `WgpuBackend::new()` succeeds.
- [ ] `BackendPreference` and `AdapterCandidate` visible in `rustdoc` for
      the crate root.
- [ ] `cargo clippy --no-deps -p tinyquant-gpu-wgpu -- -D warnings` clean.
- [ ] `cargo test --workspace` still passes (no regressions in other crates).
- [ ] `tinyquant-core` no-GPU CI check still passes.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Borrow-checker conflict between `&mut self.pipelines` and `self.ctx` in `get_or_insert_with` | Medium | Low | Extract a local `ctx` ref before the closure; the existing code already uses this pattern for `search_pipeline` |
| Residual encode/decode convention mismatch (sign, index layout) vs CPU path | Medium | Medium | Red test (`compress_decompress_residual_enabled_matches_cpu`) catches this before green; compare against `Codec::compress_prepared` CPU reference |
| `residual_decode.wgsl` buffer layout inconsistency with `residual_encode.wgsl` | Low | Low | Follow the shared uniform-dims + storage-read/write pattern used by all other shaders |
| Pipeline cache adds memory pressure on large-dim models | Low | Low | Six cached pipelines × one `ComputePipeline` each is negligible compared to rotation and codebook buffer sizes |
| `BackendPreference::Vulkan` unavailable on macOS without MoltenVK | Medium | Low | `new_with_preference` returns `NoPreferredAdapter`; caller should use `Auto` or `Metal` on Apple platforms |
| `wgpu::Instance::enumerate_adapters` API changed in wgpu 22 vs 0.18 | Low | Low | Verify against wgpu 22 changelog; the signature changed from `enumerate_adapters(Backends)` to `enumerate_adapters()` in some versions — adapt accordingly |
| `force_fallback_adapter: true` for `Software` may not be supported in all wgpu 22 builds | Low | Low | Fall back to filtering candidates by `DeviceType::Cpu` in `enumerate_adapters` if the flag is unavailable |

## See also

- [[design/rust/gpu-acceleration|GPU Acceleration Design]] — §Kernels
- [[design/rust/phase-27-implementation-notes|Phase 27 Implementation Notes]] — §Pipeline Caching Deferred to Phase 28, §Residual encode/decode pass not implemented
- [[requirements/gpu|GPU Requirements]] — FR-GPU-003, FR-GPU-004
- [[plans/rust/phase-27-wgpu-wgsl-kernels|Phase 27]] — crate skeleton and static pipeline helpers
- [[plans/rust/phase-27.5-resident-corpus-search|Phase 27.5]] — FR-GPU-004 throughput baseline
- [[plans/rust/phase-29-cuda-backend|Phase 29]] — optional CUDA specialist path
