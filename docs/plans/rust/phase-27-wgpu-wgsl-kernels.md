---
title: "Phase 27: wgpu + WGSL Kernels"
tags:
  - plans
  - rust
  - phase-27
  - gpu
  - wgpu
  - wgsl
  - phase-27
date-created: 2026-04-15
status: draft
category: planning
---

# Phase 27: wgpu + WGSL Kernels

> [!info] Goal
> Introduce `tinyquant-gpu-wgpu` as a new workspace crate that implements
> GPU-accelerated batch compression, decompression, and the groundwork for
> corpus search scoring. All production code in `tinyquant-core` and
> companion crates is unchanged — the GPU path is strictly additive.
>
> Deliverables in this phase:
> - `WgpuContext`: adapter detection with graceful `NoAdapter` fallback
> - `ComputeBackend` trait (WGSL crate owns this)
> - Four WGSL compute shaders: `rotate`, `quantize`, `dequantize`,
>   `residual_encode`
> - `WgpuBackend::compress_batch` and `decompress_batch_into` (search
>   scoring deferred to Phase 27.5)
> - CI Layer 2 (compile + shader validation on GitHub-hosted runners)
> - CI Layer 3 stub (runtime smoke on self-hosted GPU runner, advisory)
> - `GAP-GPU-001` closed by adding musl isolation check (prerequisite
>   already covered in Phase 26; confirmed here)

> [!note] Reference docs
> - [[design/rust/gpu-acceleration|GPU Acceleration Design]] — authoritative source for all GPU architecture decisions
> - [[requirements/gpu|GPU Requirements]] — FR-GPU-001 through FR-GPU-006
> - [[requirements/testing-gaps|Testing Gaps]] — GAP-GPU-002–006
> - [[plans/rust/phase-26-preparedcodec-calibration|Phase 26]] — PreparedCodec prerequisite
> - [[plans/rust/phase-27.5-resident-corpus-search|Phase 27.5]] — search scoring (cosine_topk kernel)

## Prerequisites

- Phase 26 complete: `PreparedCodec` lands in `tinyquant-core`;
  `compress_prepared` entry point exists.
- Workspace MSRV remains 1.81. `tinyquant-gpu-wgpu` carries its own
  `package.rust-version = "1.87"` and is excluded from the MSRV gate job.
- A self-hosted GitHub Actions runner with a compatible GPU (or a
  software renderer such as `llvmpipe` via Mesa) is available for
  Layer 3 tests. If unavailable at phase start, Layer 3 is stubbed as
  an advisory-only workflow.

## Scope

This phase adds one new crate. The only change to existing files is:

- `rust/Cargo.toml` — add `"crates/tinyquant-gpu-wgpu"` to the
  `[workspace]` `members` array.
- `.github/workflows/rust-ci.yml` — add Layer 2 CI job.
- `.github/workflows/gpu-ci.yml` (new) — Layer 3 runtime smoke job.

The `ComputeBackend` trait is defined inside `tinyquant-gpu-wgpu`; it is
not in `tinyquant-core`. Three additive methods were added to `PreparedCodec`
in `tinyquant-core` to support opaque GPU state attachment (`set_gpu_state`,
`has_gpu_state`, `gpu_state`). No existing call sites break; the core crate
remains GPU-free. See
[[design/rust/phase-27-implementation-notes|Phase 27 Implementation Notes]]
for full scope notes.

## Deliverables

### New crate structure

```
rust/crates/tinyquant-gpu-wgpu/
├── Cargo.toml
├── src/
│   ├── lib.rs          ← re-exports WgpuBackend, WgpuContext, ComputeBackend
│   ├── context.rs      ← WgpuContext: Instance → Adapter → Device + Queue
│   ├── backend.rs      ← impl ComputeBackend for WgpuBackend
│   ├── prepared.rs     ← prepare_for_device: upload rotation + codebook to GPU
│   ├── error.rs        ← TinyQuantGpuError
│   └── pipelines/
│       ├── mod.rs
│       ├── rotate.rs   ← build + cache the rotate/inverse-rotate pipeline
│       ├── quantize.rs ← build + cache the quantize/dequantize pipeline
│       └── residual.rs ← residual encode/decode pipeline
├── shaders/
│   ├── rotate.wgsl
│   ├── quantize.wgsl
│   ├── dequantize.wgsl
│   └── residual_encode.wgsl
└── tests/
    ├── context_probe.rs   ← adapter detection; graceful absent-GPU (FR-GPU-002)
    ├── parity_compress.rs ← CPU vs wgpu numerical parity (FR-GPU-003)
    └── batch_threshold.rs ← should_use_gpu() unit tests (FR-GPU-005)
```

---

## Part A — Crate scaffolding and adapter detection (Steps 1–2)

These steps must pass on a GitHub-hosted runner with no GPU present,
because the graceful-degradation requirement (FR-GPU-002) is exercised
without hardware.

### Step 1 — Failing tests (red)

Create `tests/context_probe.rs`:

```rust
// FR-GPU-002: WgpuBackend::new() must not panic when no adapter is available.
// Run on a standard GitHub-hosted runner (no GPU).
use tinyquant_gpu_wgpu::{WgpuBackend, ComputeBackend};

/// On any runner (with or without GPU): new() returns without panic.
#[test]
fn wgpu_backend_new_does_not_panic() {
    // Block on the async constructor via a simple runtime.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = rt.block_on(WgpuBackend::new());
    // Ok or Err(NoAdapter) are both acceptable; panic is not.
    match &result {
        Ok(backend) => {
            println!("adapter found: {:?}", backend.adapter_name());
        }
        Err(e) => {
            println!("no adapter (expected on CI): {e}");
        }
    }
}

/// is_available() returns false when no adapter is present, true when present.
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
            // Confirm the stub is_available false path via a separate call.
            // WgpuBackend::unavailable() is a test-only constructor that
            // represents the no-adapter state.
            let stub = WgpuBackend::unavailable();
            assert!(!stub.is_available());
        }
    }
}
```

Create `tests/batch_threshold.rs`:

```rust
// FR-GPU-005: should_use_gpu(rows) returns false when rows < BATCH_THRESHOLD.
use tinyquant_gpu_wgpu::{WgpuBackend, GPU_BATCH_THRESHOLD};

#[test]
fn should_use_gpu_returns_false_below_threshold() {
    let cases_false = [0usize, 1, GPU_BATCH_THRESHOLD - 1];
    let cases_true  = [GPU_BATCH_THRESHOLD, GPU_BATCH_THRESHOLD + 1, 10_000];

    for rows in cases_false {
        assert!(
            !WgpuBackend::should_use_gpu(rows),
            "rows={rows} < BATCH_THRESHOLD={GPU_BATCH_THRESHOLD} must return false"
        );
    }
    for rows in cases_true {
        assert!(
            WgpuBackend::should_use_gpu(rows),
            "rows={rows} ≥ BATCH_THRESHOLD={GPU_BATCH_THRESHOLD} must return true"
        );
    }
}
```

### Step 2 — Implement `WgpuContext` and `WgpuBackend` skeleton

`Cargo.toml` for the new crate:

```toml
[package]
name            = "tinyquant-gpu-wgpu"
version.workspace = true
edition.workspace = true
rust-version    = "1.87"
publish         = false   # promoted to true in a future release phase

[dependencies]
tinyquant-core = { workspace = true }
wgpu           = { version = "22", features = ["wgsl"] }
tokio          = { version = "1", features = ["rt"] }
thiserror      = { workspace = true }

[dev-dependencies]
tokio          = { version = "1", features = ["rt", "macros"] }
```

Implement `WgpuContext::new()` exactly as shown in
[[design/rust/gpu-acceleration|GPU Acceleration Design]] §WgpuContext setup.
Add `WgpuBackend::unavailable()` as a `#[cfg(test)]` constructor that
returns a backend in the no-adapter state.

Implement `GPU_BATCH_THRESHOLD = 512` as a public constant in `lib.rs`.
Implement `WgpuBackend::should_use_gpu(rows: usize) -> bool` as a pure
function comparing `rows >= GPU_BATCH_THRESHOLD`.

---

## Part B — WGSL shaders and compile validation (Steps 3–4)

### Step 3 — Write four WGSL shaders

Write the shaders verbatim from [[design/rust/gpu-acceleration|GPU
Acceleration Design]] §WGSL kernel plan for `rotate.wgsl` and
`dequantize.wgsl`. Add `quantize.wgsl` (nearest-codebook-entry lookup)
and `residual_encode.wgsl` (elementwise FP32→FP16 store into the residual
payload buffer):

```wgsl
// shaders/quantize.wgsl
// For each element: find the codebook entry with minimum squared distance.
// Uses a 4-bit codebook (16 entries). Input and codebook are f32 arrays.
struct Dims { n_elements: u32, n_entries: u32 }

@group(0) @binding(0) var<uniform>             dims:     Dims;
@group(0) @binding(1) var<storage, read>       codebook: array<f32>;
@group(0) @binding(2) var<storage, read>       input:    array<f32>;
@group(0) @binding(3) var<storage, read_write> indices:  array<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= dims.n_elements) { return; }
    let val = input[i];
    var best_idx: u32  = 0u;
    var best_dist: f32 = 1e38;
    for (var k: u32 = 0u; k < dims.n_entries; k++) {
        let d = val - codebook[k];
        let dist = d * d;
        if (dist < best_dist) { best_dist = dist; best_idx = k; }
    }
    indices[i] = best_idx;
}
```

```wgsl
// shaders/residual_encode.wgsl
// Store (input[i] - reconstructed[i]) as a raw fp16 bit pattern.
// NOTE: WGSL does not have a native f16 type in all environments.
// We encode as u32 pairs using the f32_to_f16 bit-manipulation idiom.
@group(0) @binding(0) var<storage, read>       original:      array<f32>;
@group(0) @binding(1) var<storage, read>       reconstructed: array<f32>;
@group(0) @binding(2) var<storage, read_write> residual_u16:  array<u32>; // packed pairs

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x * 2u;
    let n = arrayLength(&original);
    if (i >= n) { return; }
    let r0 = original[i]     - reconstructed[i];
    let r1 = select(0.0, original[i+1u] - reconstructed[i+1u], i+1u < n);
    // Pack two f16 values into one u32 (little-endian: low=r0, high=r1).
    residual_u16[gid.x] = f32_to_f16(r0) | (f32_to_f16(r1) << 16u);
}

// Approximate f32-to-f16 bit conversion (IEEE 754 round-to-nearest).
fn f32_to_f16(v: f32) -> u32 {
    let bits = bitcast<u32>(v);
    let exp  = ((bits >> 23u) & 0xFFu);
    if (exp == 0u)   { return 0u; }
    if (exp == 0xFFu){ return 0x7C00u | ((bits & 0x7FFFFFu) >> 13u); }
    let h_exp  = exp - 127u + 15u;
    if (h_exp >= 31u){ return select(0x7C00u, 0u, (bits >> 31u) == 1u); }
    if (h_exp <= 0u) { return 0u; }
    return ((bits >> 31u) << 15u) | (h_exp << 10u) | ((bits >> 13u) & 0x3FFu);
}
```

> [!note] WGSL f16 alternative
> If the build target supports `wgpu::Features::SHADER_F16`, you can use
> WGSL's `f16` type directly and skip the manual bit-packing in
> `residual_encode.wgsl`. Gate this behind a runtime feature check so the
> shader falls back to the u32 approach on older adapters.

### Step 4 — Adapter-less shader compilation gate (Layer 2 CI)

`wgpu` can validate WGSL without a physical device by creating a shader
module against the `naga` backend. Add a compile-validation test:

```rust
// tests/context_probe.rs (extend)
/// Layer 2: validate all WGSL shaders compile without a physical device.
#[test]
fn all_shaders_compile_without_device() {
    // naga's front-end validates WGSL syntax and type correctness.
    // This does not require an adapter.
    let shaders = [
        ("rotate",           include_str!("../shaders/rotate.wgsl")),
        ("quantize",         include_str!("../shaders/quantize.wgsl")),
        ("dequantize",       include_str!("../shaders/dequantize.wgsl")),
        ("residual_encode",  include_str!("../shaders/residual_encode.wgsl")),
    ];
    for (name, src) in shaders {
        let module = naga::front::wgsl::parse_str(src)
            .unwrap_or_else(|e| panic!("shader '{name}' failed WGSL parse: {e}"));
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        );
        validator.validate(&module)
            .unwrap_or_else(|e| panic!("shader '{name}' failed validation: {e}"));
    }
}
```

Add `naga` as a `[dev-dependency]` in `Cargo.toml`.

Add to `.github/workflows/rust-ci.yml`:

```yaml
gpu-compile:
  name: GPU crate — compile + shader validation
  runs-on: ubuntu-22.04
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@1.87
    - run: cargo build -p tinyquant-gpu-wgpu
    - run: cargo test  -p tinyquant-gpu-wgpu --test context_probe all_shaders_compile_without_device
    - run: cargo clippy -p tinyquant-gpu-wgpu -- -D warnings
```

---

## Part C — Batch compress / decompress (Steps 5–6)

These steps require either a physical GPU or a software renderer
(`wgpu`'s `llvmpipe` via Mesa on Linux). Gate them behind
`#[cfg(feature = "require-device")]` or in the Layer 3 CI workflow.

### Step 5 — Parity test (red)

Create `tests/parity_compress.rs`:

```rust
// FR-GPU-003: GPU compress_batch must agree with CPU Codec::compress
// to within max_delta ≤ 1e-3 per element.
//
// Requires a wgpu adapter. Skipped automatically when no adapter is found.
use tinyquant_core::{codec::{Codec, CodecConfig, PreparedCodec}, codebook::Codebook};
use tinyquant_gpu_wgpu::{WgpuBackend, ComputeBackend};

fn skip_if_no_adapter() -> Option<WgpuBackend> {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(WgpuBackend::new()).ok()
}

/// FR-GPU-003: GPU and CPU decompress output agrees to within 1e-3.
#[test]
fn gpu_decompress_matches_cpu_within_tolerance() {
    let Some(mut backend) = skip_if_no_adapter() else {
        eprintln!("SKIP: no wgpu adapter");
        return;
    };

    let config   = CodecConfig::new(4, 42, 768, false).unwrap();
    let training: Vec<f32> = (0..256 * 768).map(|i| (i as f32 * 0.001).sin()).collect();
    let codebook = Codebook::train(&config, &training).unwrap();
    let mut prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();

    backend.prepare_for_device(&mut prepared).unwrap();

    let n_vectors = 512;
    let dim = 768;
    let input: Vec<f32> = (0..n_vectors * dim)
        .map(|i| (i as f32 * 0.001).cos())
        .collect();

    // CPU path: compress then decompress.
    let mut cpu_out = vec![0f32; n_vectors * dim];
    for i in 0..n_vectors {
        let v = &input[i * dim..(i + 1) * dim];
        let cv = Codec::new().compress_prepared(v, &prepared).unwrap();
        let mut row = vec![0f32; dim];
        Codec::new().decompress_prepared_into(&cv, &prepared, &mut row).unwrap();
        cpu_out[i * dim..(i + 1) * dim].copy_from_slice(&row);
    }

    // GPU path: compress_batch → decompress_batch_into.
    let compressed = backend.compress_batch(&input, n_vectors, dim, &prepared).unwrap();
    let mut gpu_out = vec![0f32; n_vectors * dim];
    backend.decompress_batch_into(&compressed, &prepared, &mut gpu_out).unwrap();

    let max_delta = cpu_out.iter().zip(&gpu_out)
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
    let config   = CodecConfig::new(4, 42, 64, false).unwrap();
    let codebook = Codebook::train(&config, &vec![0f32; 256 * 64]).unwrap();
    let mut prepared = PreparedCodec::new(config, codebook).unwrap();

    // Call twice — both must succeed with Ok(()).
    backend.prepare_for_device(&mut prepared).unwrap();
    backend.prepare_for_device(&mut prepared).unwrap();
    // has_gpu_state must be true after the first call and stay true.
    assert!(prepared.has_gpu_state());
}
```

### Step 6 — Implement `compress_batch` and `decompress_batch_into`

`WgpuBackend::compress_batch` follows the synchronization model from
[[design/rust/gpu-acceleration|GPU Acceleration Design]] §Synchronization
model: one `CommandEncoder` per batch, three sequential compute passes
(rotate → quantize → residual_encode), one `Queue::submit`, one
`Buffer::map_async` readback. No overlapping submissions.

`WgpuBackend::decompress_batch_into` reverses the pipeline: residual
decode → dequantize → inverse-rotate.

`prepare_for_device` uploads the rotation matrix and codebook to device
buffers and stores an opaque state handle via `PreparedCodec::set_gpu_state`.
If `has_gpu_state()` is already true, return `Ok(())` immediately.

---

## Part D — Layer 3 CI stub (Step 7)

### Step 7 — Create `gpu-ci.yml`

Create `.github/workflows/gpu-ci.yml`:

```yaml
name: GPU runtime smoke
on:
  push:
    branches: [main]
    paths:
      - "rust/crates/tinyquant-gpu-wgpu/**"
      - ".github/workflows/gpu-ci.yml"
  pull_request:
    paths:
      - "rust/crates/tinyquant-gpu-wgpu/**"

jobs:
  gpu-smoke:
    name: wgpu runtime smoke
    runs-on: [self-hosted, gpu]   # label on the GPU runner
    continue-on-error: true       # advisory until runner is fully provisioned
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.87
      - name: Run parity tests (requires wgpu adapter)
        run: cargo test -p tinyquant-gpu-wgpu --test parity_compress
```

> [!note] Advisory mode
> `continue-on-error: true` keeps the PR gate non-blocking until the
> self-hosted GPU runner is reliably provisioned. Change to
> `continue-on-error: false` once the runner is stable.

---

## Steps summary

| Step | Gap(s) closed | File | Est. effort |
|---|---|---|---|
| 1–2 | GAP-GPU-002, GAP-GPU-005 | `context_probe.rs`, `batch_threshold.rs`, crate skeleton | 3 h |
| 3–4 | — | Four WGSL shaders, `rust-ci.yml` Layer 2 job | 2 h |
| 5–6 | GAP-GPU-003, GAP-GPU-006 | `parity_compress.rs`, `compress_batch`, `decompress_batch_into`, `prepare_for_device` | 4–6 h |
| 7 | — | `gpu-ci.yml` Layer 3 stub | 0.5 h |

GAP-GPU-004 (throughput benchmark ≤ 5 ms on discrete GPU) is confirmed or
refuted after Step 6 by running the criterion bench on the self-hosted runner.
It is not blocked on this phase but should be measured before Phase 27 is
declared complete.

---

## Acceptance criteria

- [ ] `cargo build -p tinyquant-gpu-wgpu` succeeds at MSRV 1.87 on ubuntu-22.04.
- [ ] `all_shaders_compile_without_device` passes on GitHub-hosted runner.
- [ ] `wgpu_backend_new_does_not_panic` passes on a runner with no GPU.
- [ ] `should_use_gpu_returns_false_below_threshold` passes.
- [ ] `cargo clippy --no-deps -p tinyquant-gpu-wgpu -- -D warnings` clean (`--no-deps` excludes pre-existing `tinyquant-core` lints at Rust 1.87, which are tracked separately in the 1.81 clippy job).
- [ ] `gpu-ci.yml` workflow exists and runs (advisory).
- [ ] `parity_compress.rs` and `prepare_for_device_is_idempotent` pass on a
      runner with a wgpu-compatible adapter.
- [ ] GAP-GPU-002, GAP-GPU-003, GAP-GPU-005, GAP-GPU-006 updated to
      `Gap: None.` in `docs/requirements/gpu.md`.
- [ ] `tinyquant-core` dependency graph unchanged (no GPU imports); `cargo tree`
      CI check (Phase 26) continues to pass.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `llvmpipe` software renderer unavailable on GitHub-hosted runners | Medium | Medium | Use `cargo test` behind `skip_if_no_adapter()` guard; Layer 3 on self-hosted |
| WGSL f16 feature not supported by all adapters | Medium | Low | Fallback to u32 bit-packing in `residual_encode.wgsl` (already drafted) |
| `wgpu` MSRV higher than 1.87 in a future minor bump | Low | Low | Exact-version pin in Cargo.toml; review on each wgpu bump |
| GPU parity fails > 1e-3 due to FP32 GEMM reduction order | Low | High | Relax tolerance and document if measurement shows > 1e-3 delta is inherent |

## See also

- [[design/rust/gpu-acceleration|GPU Acceleration Design]] — §WgpuContext setup, §WGSL kernel plan, §Synchronization model
- [[requirements/gpu|GPU Requirements]] — FR-GPU-001 through FR-GPU-006
- [[plans/rust/phase-26-preparedcodec-calibration|Phase 26]] — PreparedCodec prerequisite
- [[plans/rust/phase-27.5-resident-corpus-search|Phase 27.5]] — cosine_topk GPU scoring
- [[plans/rust/phase-28-cuda-backend|Phase 28]] — optional CUDA specialist path
