---
title: "Phase 28: Optional CUDA Backend"
tags:
  - plans
  - rust
  - phase-28
  - gpu
  - cuda
  - cust
date-created: 2026-04-15
status: draft
category: planning
---

# Phase 28: Optional CUDA Backend

> [!info] Goal
> Introduce `tinyquant-gpu-cuda` as a strictly optional, NVIDIA-specialist
> crate that implements `ComputeBackend` using `cust` (CUDA Driver API
> bindings for Rust) and PTX kernels for quantize, dequantize, and cosine
> similarity scoring.
>
> This phase is **contingent** on Phase 27.5 throughput measurements:
> if `WgpuBackend::cosine_topk` already meets the stretch goal (≤ 2 ms
> for 10 000 × dim=1536 on an RTX 3060), the CUDA path is low-value and
> this phase may be deferred indefinitely. Proceed only when profiling
> shows a measurable gap that only CUDA can close.
>
> The CPU path (`tinyquant-core`) and the cross-platform GPU path
> (`tinyquant-gpu-wgpu`) are not modified. `tinyquant-gpu-cuda` is
> additive and publish = false initially.

> [!note] Reference docs
> - [[design/rust/gpu-acceleration|GPU Acceleration Design]] — §tinyquant-gpu-cuda, §Crate choice rationale, §MSRV isolation
> - [[requirements/gpu|GPU Requirements]] — FR-GPU-007
> - [[requirements/testing-gaps|Testing Gaps]] — GAP-GPU-007
> - [[plans/rust/phase-27.5-resident-corpus-search|Phase 27.5]] — prerequisite; throughput baseline

## Prerequisites

- Phase 27.5 complete: `WgpuBackend::cosine_topk` functional; FR-GPU-004
  throughput measured.
- Decision made (by perf lead) that a CUDA-specialist path is worth the
  distribution complexity based on Phase 27.5 measurements.
- NVIDIA driver + CUDA toolkit (≥ 12.x) on the self-hosted CI runner.
- `cust` crate requirements understood; MSRV for `cust` pinned.

## Scope

New crate only. No changes to `tinyquant-core`, `tinyquant-gpu-wgpu`,
or any existing crate. The `ComputeBackend` trait used here is the same
trait defined in `tinyquant-gpu-wgpu`; `tinyquant-gpu-cuda` takes it as
an explicit path reference rather than re-defining it.

Only the following existing files change:
- `rust/Cargo.toml` — add `"crates/tinyquant-gpu-cuda"` to `members`.
- `.github/workflows/gpu-ci.yml` — add CUDA compile-check job.

## Deliverables

### New crate structure

```
rust/crates/tinyquant-gpu-cuda/
├── Cargo.toml
├── build.rs              ← compile PTX kernels; link libcuda
├── src/
│   ├── lib.rs            ← re-exports CudaBackend; is_available() always-false stub
│   ├── context.rs        ← cust::quick_init, module + stream management
│   ├── backend.rs        ← impl ComputeBackend for CudaBackend
│   └── kernels/
│       ├── mod.rs
│       ├── quantize.ptx  ← precompiled PTX (or .cu source for nvcc)
│       ├── dequantize.ptx
│       └── cosine.ptx
└── tests/
    ├── stub_compile.rs   ← builds without CUDA toolkit (FR-GPU-007)
    └── parity.rs         ← CPU vs CUDA parity (requires CUDA device)
```

---

## Part A — Stub build and FR-GPU-007 (Steps 1–2)

FR-GPU-007 requires that the crate compiles to a stub with no link errors
when the `cuda` feature is disabled and no CUDA toolkit is installed.
This is the highest-priority requirement because it gates whether the
crate can be listed as an optional dependency for non-NVIDIA users.

### Step 1 — Failing tests (red)

Create `tests/stub_compile.rs`:

```rust
// FR-GPU-007: CudaBackend::is_available() returns false when compiled
// without the cuda feature (no CUDA toolkit required).
//
// This test must pass on a machine with NO CUDA installation.
use tinyquant_gpu_cuda::CudaBackend;

#[test]
fn cuda_backend_stub_is_available_returns_false() {
    // When built without --features cuda, CudaBackend is a zero-size stub.
    assert!(
        !CudaBackend::is_available(),
        "stub CudaBackend must always return is_available = false"
    );
}

#[test]
fn cuda_backend_stub_name_is_descriptive() {
    assert!(
        CudaBackend::stub_name().contains("CUDA"),
        "stub name must mention CUDA for diagnostics"
    );
}
```

### Step 2 — Implement the stub

`Cargo.toml`:

```toml
[package]
name        = "tinyquant-gpu-cuda"
version.workspace = true
edition.workspace = true
rust-version = "1.87"   # or cust's requirement; pin after evaluating
publish     = false

[features]
default = []
cuda    = ["dep:cust"]

[dependencies]
tinyquant-core       = { workspace = true }
tinyquant-gpu-wgpu   = { path = "../tinyquant-gpu-wgpu" }  # for ComputeBackend trait
cust                 = { version = "0.3", optional = true }
thiserror            = { workspace = true }
```

`src/lib.rs` (stub path — no `cuda` feature):

```rust
// tinyquant-gpu-cuda/src/lib.rs
//
// Stub implementation active when the `cuda` feature is NOT enabled.
// Compiles cleanly without a CUDA toolkit installed (FR-GPU-007).

pub use crate::backend::CudaBackend;

pub mod backend;

#[cfg(feature = "cuda")]
pub mod context;
#[cfg(feature = "cuda")]
pub mod kernels;
```

`src/backend.rs`:

```rust
/// NVIDIA CUDA specialist backend for TinyQuant.
///
/// When the `cuda` feature is disabled (default), this is a zero-size stub.
/// `is_available()` always returns false. No link to libcuda occurs.
pub struct CudaBackend {
    #[cfg(feature = "cuda")]
    context: crate::context::CudaContext,
}

impl CudaBackend {
    /// Construct the backend. On stub builds, always succeeds and
    /// `is_available()` returns false.
    pub fn new() -> Result<Self, CudaBackendError> {
        #[cfg(feature = "cuda")]
        {
            let context = crate::context::CudaContext::init()?;
            Ok(Self { context })
        }
        #[cfg(not(feature = "cuda"))]
        {
            Ok(Self {})
        }
    }

    pub fn is_available() -> bool {
        #[cfg(feature = "cuda")]
        { crate::context::CudaContext::device_count() > 0 }
        #[cfg(not(feature = "cuda"))]
        { false }
    }

    pub fn stub_name() -> &'static str {
        "CUDA (stub — build with --features cuda to enable)"
    }
}

impl Default for CudaBackend {
    fn default() -> Self { Self::new().expect("CudaBackend stub must always succeed") }
}
```

Add a `CudaBackendError` type derived from `thiserror::Error`.

Add to `.github/workflows/gpu-ci.yml`:

```yaml
cuda-stub-compile:
  name: CUDA stub compile-check (no toolkit required)
  runs-on: ubuntu-22.04
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@1.87
    - name: Build stub (no cuda feature)
      run: cargo +1.87.0 check -p tinyquant-gpu-cuda
    - name: Test stub
      run: cargo +1.87.0 test -p tinyquant-gpu-cuda --test stub_compile
    - name: Clippy stub
      run: cargo clippy -p tinyquant-gpu-cuda -- -D warnings
```

This job runs on a standard GitHub-hosted runner with no CUDA toolkit —
that is precisely the condition FR-GPU-007 requires to pass.

---

## Part B — CUDA context and PTX kernels (Steps 3–5)

Steps 3–5 require the `cuda` feature and a machine with an NVIDIA driver
and CUDA toolkit. They run in a separate CI leg on the self-hosted GPU runner.

### Step 3 — `CudaContext` and stream management

Implement `context.rs` with `cust::quick_init()`:

```rust
// src/context.rs — only compiled with feature = "cuda"
use cust::prelude::*;

pub struct CudaContext {
    _ctx:    cust::context::Context,
    stream:  cust::stream::Stream,
    module:  cust::module::Module,
}

impl CudaContext {
    pub fn init() -> Result<Self, CudaBackendError> {
        cust::quick_init().map_err(CudaBackendError::Init)?;
        let ctx = cust::context::Context::new(Device::get_device(0)?)
            .map_err(CudaBackendError::Context)?;
        let stream = Stream::new(StreamFlags::NON_BLOCKING, None)
            .map_err(CudaBackendError::Stream)?;
        let module = Module::from_ptx(include_str!("../kernels/quantize.ptx"), &[])
            .map_err(CudaBackendError::Module)?;
        Ok(Self { _ctx: ctx, stream, module })
    }

    pub fn device_count() -> usize {
        cust::device::Device::num_devices().unwrap_or(0) as usize
    }
}
```

### Step 4 — PTX kernels

Write `kernels/quantize.ptx` (or provide `.cu` source for `nvcc` compilation
in `build.rs`). The quantize kernel is semantically identical to the WGSL
version — nearest-entry lookup over a 4-bit codebook:

```ptx
// kernels/quantize.ptx
// Nearest codebook entry lookup for 4-bit quantization.
// Grid: (n_elements / 256, 1, 1) blocks; blockDim: (256, 1, 1).
.version 7.0
.target sm_75
.address_size 64

.visible .entry quantize_kernel(
    .param .u64 codebook_ptr,
    .param .u64 input_ptr,
    .param .u64 indices_ptr,
    .param .u32 n_elements,
    .param .u32 n_entries
) {
    // ... (standard PTX addressing + loop over n_entries) ...
}
```

> [!note] Build strategy for PTX
> Prefer shipping pre-compiled `.ptx` files over requiring `nvcc` in the
> user's build environment. Pre-compile with `nvcc -arch=sm_75 -ptx`
> targeting sm_75 (Turing; RTX 20/30 series). `cust`'s `Module::from_ptx`
> handles JIT compilation to device-native ISA at runtime.
>
> If the crate is eventually published, gate `.ptx` behind LFS or generate
> from embedded CUDA C source using `bindgen` + `cc` crate in `build.rs`.

Write equivalent PTX files for `dequantize.ptx` and `cosine.ptx`.

### Step 5 — Parity test (requires CUDA device)

Create `tests/parity.rs`:

```rust
// CPU vs CUDA parity for compress + decompress + cosine_topk.
// Requires the `cuda` feature AND a CUDA device. Skip otherwise.
use tinyquant_core::{codec::{Codec, CodecConfig, PreparedCodec}, codebook::Codebook};
use tinyquant_gpu_cuda::CudaBackend;
use tinyquant_gpu_wgpu::ComputeBackend;

fn skip_if_no_cuda() -> Option<CudaBackend> {
    if !CudaBackend::is_available() { return None; }
    CudaBackend::new().ok()
}

/// CUDA decompress output must agree with CPU within max_delta ≤ 1e-3.
#[test]
#[cfg(feature = "cuda")]
fn cuda_decompress_matches_cpu_within_tolerance() {
    let Some(mut backend) = skip_if_no_cuda() else {
        eprintln!("SKIP: no CUDA device");
        return;
    };

    let config   = CodecConfig::new(4, 42, 768, false).unwrap();
    let training = vec![0f32; 256 * 768];
    let codebook = Codebook::train(&config, &training).unwrap();
    let mut prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();
    backend.prepare_for_device(&mut prepared).unwrap();

    let n = 512usize;
    let dim = 768usize;
    let input: Vec<f32> = (0..n * dim).map(|i| (i as f32 * 0.001).cos()).collect();

    let compressed = backend.compress_batch(&input, n, dim, &prepared).unwrap();
    let mut cuda_out = vec![0f32; n * dim];
    backend.decompress_batch_into(&compressed, &prepared, &mut cuda_out).unwrap();

    let mut cpu_out = vec![0f32; n * dim];
    for i in 0..n {
        let v = &input[i * dim..(i + 1) * dim];
        let cv = Codec::new().compress_prepared(v, &prepared).unwrap();
        Codec::new()
            .decompress_prepared_into(&cv, &prepared, &mut cpu_out[i * dim..(i + 1) * dim])
            .unwrap();
    }

    let max_delta = cpu_out.iter().zip(&cuda_out)
        .map(|(c, g)| (c - g).abs())
        .fold(0f32, f32::max);

    assert!(
        max_delta <= 1e-3,
        "max delta CUDA vs CPU = {max_delta:.6e}, must be ≤ 1e-3"
    );
}
```

---

## Steps summary

| Step | Gap(s) closed | File | Est. effort |
|---|---|---|---|
| 1–2 | GAP-GPU-007 | `stub_compile.rs`, `backend.rs` (stub), `gpu-ci.yml` CUDA job | 2 h |
| 3 | — | `context.rs` (`cuda` feature-gated) | 1.5 h |
| 4 | — | Three PTX kernel files | 3–5 h |
| 5 | — | `parity.rs` (requires CUDA device) | 1 h |

---

## Acceptance criteria

- [ ] `cargo +1.87.0 check -p tinyquant-gpu-cuda` (no cuda feature) succeeds
      on ubuntu-22.04 without CUDA toolkit.
- [ ] `cargo test -p tinyquant-gpu-cuda --test stub_compile` — both stub
      tests pass on a no-CUDA machine.
- [ ] `cargo clippy -p tinyquant-gpu-cuda -- -D warnings` clean on stub
      build.
- [ ] `gpu-ci.yml::cuda-stub-compile` job added and passing on standard
      GitHub-hosted runner.
- [ ] `cuda_decompress_matches_cpu_within_tolerance` passes (max delta ≤ 1e-3)
      on the self-hosted CUDA runner.
- [ ] GAP-GPU-007 updated to `Gap: None.` in `docs/requirements/gpu.md`.
- [ ] `tinyquant-core` no-GPU CI check (Phase 26) still passes — `cust` not
      in `tinyquant-core` dep tree.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `cust` MSRV higher than 1.87; conflicts with workspace | Low | Medium | Exact-version pin; document MSRV in crate README; isolate in CI leg |
| Pre-compiled PTX not portable across SM architectures | Medium | Medium | Compile for sm_75 (Turing baseline); document that sm_70 (Volta) is unsupported; provide sm_80 (Ampere) PTX as an optional build artefact |
| CUDA toolkit availability on self-hosted runner varies | High | Low | Stub compile-check passes on standard runner; parity test advisory-only until runner is stable |
| Phase 27.5 meets the stretch goal; CUDA path has negligible benefit | Medium | Low | This risk is anticipated — document the measurement and defer Phase 28 if wgpu is sufficient |

## See also

- [[design/rust/gpu-acceleration|GPU Acceleration Design]] — §tinyquant-gpu-cuda, §Crate choice rationale
- [[requirements/gpu|GPU Requirements]] — FR-GPU-007
- [[requirements/testing-gaps|Testing Gaps]] — GAP-GPU-007
- [[plans/rust/phase-27.5-resident-corpus-search|Phase 27.5]] — throughput baseline decision gate
- [[design/rust/feature-flags|Feature Flags and Optional Dependencies]] §tinyquant-gpu-cuda
- [[design/rust/risks-and-mitigations|Risks and Mitigations]] §R23
