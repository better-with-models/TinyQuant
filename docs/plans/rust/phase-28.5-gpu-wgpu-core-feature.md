---
title: "Phase 28.5: tinyquant-core gpu-wgpu Feature Integration"
tags:
  - plans
  - rust
  - phase-28.5
  - gpu
  - wgpu
  - feature-flags
  - publishing
date-created: 2026-04-19
status: planned
category: planning
---

# Phase 28.5: `tinyquant-core` `gpu-wgpu` Feature Integration

> [!info] Goal
> Make GPU acceleration available to external Rust users without requiring
> them to depend on `tinyquant-gpu-wgpu` directly. Integrate the wgpu
> backend as an optional Cargo feature of `tinyquant-core` so the public
> API is `tinyquant-core = { features = ["gpu-wgpu"] }`. Reserve the
> `tinyquant-gpu-wgpu` crate name on crates.io as a prerequisite.
>
> No new compute logic. Every kernel, pipeline, and test introduced in
> Phases 27–28 ships unchanged. This phase is purely about the public
> surface and the crate publication graph.

> [!note] Reference docs
> - [[design/rust/gpu-acceleration|GPU Acceleration Design]]
> - [[design/rust/feature-flags|Feature Flags and Optional Dependencies]]
> - [[plans/rust/phase-27-wgpu-wgsl-kernels|Phase 27]] — crate skeleton
> - [[plans/rust/phase-28-wgpu-pipeline-caching|Phase 28]] — caching, residual, preference
> - [[plans/rust/phase-29-cuda-backend|Phase 29]] — CUDA (follows same pattern)

## Problem

`tinyquant-gpu-wgpu` is `publish = false` and no published crate depends on
it. External Rust users who want GPU-accelerated batch compression have no way
to opt in: `cargo add tinyquant-gpu-wgpu` fails, and accessing it requires
cloning the full workspace.

The README documents a GPU recipe (`use tinyquant_gpu_wgpu::WgpuBackend`) that
is currently unreachable from any published release. Users who add
`tinyquant-core` to their project get CPU SIMD + Rayon with no upgrade path to
GPU without manually changing their dependency graph.

## Solution

1. Enable publication of `tinyquant-gpu-wgpu` (reserve name on crates.io at
   `1.0.0` — matching the workspace version).
2. Add `tinyquant-gpu-wgpu` as an optional dependency of `tinyquant-core`
   gated behind a `gpu-wgpu` feature flag that also enables `std`.
3. Add feature-gated GPU error variants (`GpuUnavailable`, `GpuError`) to
   `CodecError` using `From<TinyQuantGpuError>` conversion.
4. Add `Codec::compress_batch_gpu_with` — a feature-gated batch-compress method
   that accepts any `B: ComputeBackend` and routes to the GPU when
   `rows >= GPU_BATCH_THRESHOLD`, otherwise falls through to the CPU path.
5. Re-export the GPU public surface from `tinyquant-core` under the feature
   flag so callers only need one `cargo add`.
6. Update the feature-flags design doc to document the new `gpu-wgpu` row.
7. Update CI to cover the `--features gpu-wgpu` build path.

Phase 29 (CUDA) follows the identical pattern: a `gpu-cuda` feature on
`tinyquant-core` that gates `tinyquant-gpu-cuda`.

## Prerequisites

- Phase 28 complete (`CachedPipelines`, `BackendPreference`,
  `load_pipelines` lifecycle all landed and tested). ✅
- `cargo test -p tinyquant-gpu-wgpu` green on CI. ✅
- All 7 publishable crates already registered on crates.io at `1.0.0`; this
  phase adds `tinyquant-gpu-wgpu` to the set.

## Scope

This phase does not add, change, or delete any WGSL shader, kernel, or test
in `tinyquant-gpu-wgpu`. It only:

- Adjusts `publish` flag and package metadata in `tinyquant-gpu-wgpu`,
- Wires existing GPU code behind a feature gate in `tinyquant-core`,
- Adds one new method and two new error variants in `tinyquant-core`,
- Extends CI to exercise that gate, and
- Adds a `gpu-wgpu` feature stub in `tinyquant-py`.

---

## Design

### D1. `no_std` compatibility

`tinyquant-core` is `#![no_std]` (requires `alloc` only). `tinyquant-gpu-wgpu`
depends on `wgpu`, which is a std crate. The `gpu-wgpu` feature **must**
also enable `std`:

```toml
gpu-wgpu = ["dep:tinyquant-gpu-wgpu", "std"]
```

This follows the same pattern as `simd = ["std"]` already in the crate.
Consequence: code gated behind `#[cfg(feature = "gpu-wgpu")]` can freely
use `std::` because `std` is guaranteed to be enabled.

The no_std cross-compile job (`thumbv7em-none-eabihf`) runs **without**
`--features gpu-wgpu`, so the embedded build path is unaffected.

### D2. MSRV split

The workspace MSRV is `1.81` (from `[workspace.package]`). `tinyquant-gpu-wgpu`
overrides this to `rust-version = "1.87"`. When a user enables
`--features gpu-wgpu` on `tinyquant-core`, they must be using Rust ≥ 1.87.

The `tinyquant-core` `Cargo.toml` comment must document this:

```toml
# IMPORTANT: the `gpu-wgpu` optional dependency overrides the effective
# MSRV to 1.87 when the feature is enabled. The base crate MSRV remains
# 1.81 for all other feature combinations.
tinyquant-gpu-wgpu = { ... optional = true }
```

The CI `gpu-feature-gate` job must use toolchain `1.87.0`, consistent with
the existing `gpu-compile` job.

### D3. Async/sync boundary

`WgpuBackend::new()` and `WgpuBackend::new_with_preference()` are `async`.
`Codec::compress_batch_gpu_with` is synchronous. The wgpu backend already
handles this internally: all public `ComputeBackend` trait methods
(`compress_batch`, `decompress_batch_into`, `prepare_for_device`) are
synchronous — the async operations are confined inside the backend
implementation. **No executor bridging (`pollster`, `tokio::block_on`) is
needed in `tinyquant-core`.**

Callers of `compress_batch_gpu_with` are responsible for creating and holding
a `WgpuBackend` (which requires an async context for `new()` / `new_with_preference()`).
`tinyquant-core` only calls synchronous trait methods.

### D4. New API: `Codec::compress_batch_gpu_with`

Rather than modifying `compress_batch_with` to create a backend internally
(which would require embedding an async executor and adds mandatory GPU
initialization overhead for all callers), this phase adds a separate
`compress_batch_gpu_with` method. This is:

- **Explicit**: callers opt in to GPU routing by calling this method.
- **Caller-managed backend**: the `WgpuBackend` (or any `ComputeBackend`)
  is constructed and owned by the caller, amortising async initialization
  across many batch calls.
- **Transparent fallback**: when `rows < GPU_BATCH_THRESHOLD`, falls through
  to the CPU path without requiring the caller to check the threshold
  themselves.

Full signature:

```rust
#[cfg(feature = "gpu-wgpu")]
#[doc(cfg(feature = "gpu-wgpu"))]
pub fn compress_batch_gpu_with<B>(
    &self,
    vectors: &[f32],
    rows: usize,
    cols: usize,
    prepared: &mut PreparedCodec,
    backend: &mut B,
    parallelism: Parallelism,
) -> Result<Vec<CompressedVector>, CodecError>
where
    B: tinyquant_gpu_wgpu::ComputeBackend,
```

Routing logic:

```rust
if rows >= tinyquant_gpu_wgpu::GPU_BATCH_THRESHOLD {
    backend
        .prepare_for_device(prepared)
        .map_err(CodecError::from)?;
    backend
        .compress_batch(vectors, rows, cols, prepared)
        .map_err(CodecError::from)
} else {
    self.compress_batch_with(
        vectors, rows, cols,
        prepared.config(), prepared.codebook(),
        parallelism,
    )
}
```

> [!note] `prepare_for_device` idempotency
> `ComputeBackend::prepare_for_device` is documented as idempotent — calling
> it repeatedly on the same `PreparedCodec` does not upload buffers twice.
> Calling it before every `compress_batch_gpu_with` is therefore safe, though
> callers who call the method in a tight loop may want to call it once
> explicitly beforehand and then use the GPU path directly.

### D5. Error model

`CodecError` is `#[non_exhaustive]`, `Clone`, `PartialEq`, `Eq`, and uses
`alloc::sync::Arc<str>` for string-valued variants (see `BackendError::Adapter`
for the existing pattern). The two new GPU variants follow the same convention:

```rust
/// GPU backend is not available or failed to initialize.
///
/// Returned by [`Codec::compress_batch_gpu_with`] when `WgpuBackend::new()`
/// fails (no adapter, driver crash, or headless environment without a
/// software renderer).
#[cfg(feature = "gpu-wgpu")]
#[error("GPU unavailable: {0}")]
GpuUnavailable(alloc::sync::Arc<str>),

/// A GPU compute operation returned an error.
///
/// Distinct from [`CodecError::GpuUnavailable`] — the backend was
/// successfully initialized but the kernel dispatch or readback failed.
#[cfg(feature = "gpu-wgpu")]
#[error("GPU error: {0}")]
GpuError(alloc::sync::Arc<str>),
```

`From<tinyquant_gpu_wgpu::TinyQuantGpuError>` is implemented in
`tinyquant-core/src/errors.rs` under the feature gate:

```rust
#[cfg(feature = "gpu-wgpu")]
impl From<tinyquant_gpu_wgpu::TinyQuantGpuError> for CodecError {
    fn from(e: tinyquant_gpu_wgpu::TinyQuantGpuError) -> Self {
        use tinyquant_gpu_wgpu::TinyQuantGpuError;
        match &e {
            TinyQuantGpuError::NoAdapter(_) | TinyQuantGpuError::NoPreferredAdapter => {
                CodecError::GpuUnavailable(e.to_string().into())
            }
            _ => CodecError::GpuError(e.to_string().into()),
        }
    }
}
```

> [!warning] `CodecError` is `Clone + PartialEq + Eq`
> Both new variants use `Arc<str>`, which satisfies all three traits. Do not
> use `String` (not `Clone`-cheap on the eq path) or `Box<str>` (no `Clone`
> without extra impl).

### D6. crates.io publication requirements

`tinyquant-gpu-wgpu/Cargo.toml` currently lacks the mandatory fields for
`cargo publish`. The following must be present before publishing succeeds:

| Field | Value / Source |
|---|---|
| `name` | `"tinyquant-gpu-wgpu"` |
| `version` | `version.workspace = true` (→ `"1.0.0"`) |
| `edition` | `edition.workspace = true` |
| `license` | `license.workspace = true` (→ `"Apache-2.0"`) |
| `repository` | `repository.workspace = true` (→ GitHub URL) |
| `description` | New value (see Step 1) |
| `keywords` | `["quantization", "embeddings", "wgpu", "gpu", "machine-learning"]` |
| `categories` | `["compression", "science", "wasm"]` |

The `rust-version = "1.87"` override is already present and must stay.
`publish = false` is removed. `authors` can be inherited from workspace.

> [!caution] `rust-version` override precedence
> The crate explicitly sets `rust-version = "1.87"`, which overrides the
> workspace value of `"1.81"`. This is intentional and must not be removed
> or replaced with `rust-version.workspace = true` (which would lower the
> effective MSRV and allow broken builds on 1.81 toolchains).

---

## Deliverables

### Step 1 — Enable `tinyquant-gpu-wgpu` for publication

**File:** `rust/crates/tinyquant-gpu-wgpu/Cargo.toml`

Replace:

```toml
[package]
name            = "tinyquant-gpu-wgpu"
version.workspace = true
edition.workspace = true
rust-version    = "1.87"
publish         = false
description     = "wgpu/WGSL GPU-accelerated batch compression backend for TinyQuant"
```

With:

```toml
[package]
name             = "tinyquant-gpu-wgpu"
version.workspace  = true
edition.workspace  = true
rust-version     = "1.87"
license.workspace  = true
repository.workspace = true
description      = "wgpu/WGSL GPU-accelerated batch compression backend for TinyQuant \
                    (optional dependency — prefer `tinyquant-core` with the `gpu-wgpu` feature)"
keywords         = ["quantization", "embeddings", "wgpu", "gpu", "machine-learning"]
categories       = ["compression", "science", "wasm"]
```

The `description` copy guides crates.io search hits toward the feature-flag
path rather than the standalone crate.

Then verify publication readiness:

```bash
cd rust
cargo publish -p tinyquant-gpu-wgpu --dry-run
# exits 0 → safe to proceed
cargo publish -p tinyquant-gpu-wgpu
```

### Step 2 — Add `gpu-wgpu` optional dependency to `tinyquant-core`

**File:** `rust/crates/tinyquant-core/Cargo.toml`

Add to `[features]`:

```toml
[features]
default = []
std     = []
simd    = ["std"]
avx512  = ["simd"]
# Optional GPU acceleration via wgpu/WGSL.
# Enabling this feature raises the effective MSRV from 1.81 to 1.87
# (wgpu's requirement) and switches the crate from no_std to std.
gpu-wgpu = ["dep:tinyquant-gpu-wgpu", "std"]
```

Add to `[dependencies]`:

```toml
# Optional: GPU acceleration backend. Only compiled when the `gpu-wgpu` feature
# is enabled. The `dep:` prefix prevents the optional dep from implicitly
# creating a `tinyquant-gpu-wgpu` feature; the only public feature name is
# `gpu-wgpu` (stabilised in Rust 1.60, Cargo edition = "2021").
tinyquant-gpu-wgpu = { path = "../tinyquant-gpu-wgpu", version = "=1.0.0", optional = true }
```

### Step 3 — Add GPU error variants to `CodecError`

**File:** `rust/crates/tinyquant-core/src/errors.rs`

Append to the `CodecError` enum body (after `InvalidResidualFlag`):

```rust
    /// GPU backend is not available or failed to initialize.
    ///
    /// Only present when the `gpu-wgpu` feature is enabled.
    #[cfg(feature = "gpu-wgpu")]
    #[error("GPU unavailable: {0}")]
    GpuUnavailable(alloc::sync::Arc<str>),

    /// A GPU compute operation returned an error.
    ///
    /// Only present when the `gpu-wgpu` feature is enabled.
    #[cfg(feature = "gpu-wgpu")]
    #[error("GPU error: {0}")]
    GpuError(alloc::sync::Arc<str>),
```

Add `From` impl below the enum (still in `errors.rs`):

```rust
#[cfg(feature = "gpu-wgpu")]
impl From<tinyquant_gpu_wgpu::TinyQuantGpuError> for CodecError {
    fn from(e: tinyquant_gpu_wgpu::TinyQuantGpuError) -> Self {
        use tinyquant_gpu_wgpu::TinyQuantGpuError;
        match &e {
            TinyQuantGpuError::NoAdapter(_) | TinyQuantGpuError::NoPreferredAdapter => {
                CodecError::GpuUnavailable(e.to_string().into())
            }
            _ => CodecError::GpuError(e.to_string().into()),
        }
    }
}
```

> [!caution] Match exhaustiveness
> `TinyQuantGpuError` may gain new variants in future phases (e.g. the CUDA
> backend). The `From` impl uses a wildcard `_` arm for non-`NoAdapter`
> variants so it does not need to be updated when new variants are added to
> the GPU error type. This is intentional.

### Step 4 — Add `Codec::compress_batch_gpu_with`

**File:** `rust/crates/tinyquant-core/src/codec/service.rs`

After the existing `compress_batch_with` method (line ~280), add a new `impl`
block that is compiled only with the feature:

```rust
#[cfg(feature = "gpu-wgpu")]
impl Codec {
    /// Batch compress with automatic GPU routing.
    ///
    /// When `rows >= GPU_BATCH_THRESHOLD` (currently 512), the batch is
    /// dispatched to `backend` via [`tinyquant_gpu_wgpu::ComputeBackend`].
    /// Otherwise the call falls through to the CPU path using `parallelism`.
    ///
    /// # Caller responsibility
    ///
    /// `backend` must have been created and initialized by the caller (which
    /// requires an async executor). This method only calls synchronous trait
    /// methods on the backend.
    ///
    /// `prepared` is passed by mutable reference so `prepare_for_device` can
    /// attach GPU-resident state (idempotent — safe to call before every batch).
    ///
    /// # Async-context safety
    ///
    /// Do **not** call this function from an async context that owns an active
    /// tokio or other executor on the same thread. `wgpu` may internally call
    /// `device.poll(wgpu::Maintain::Wait)`, which blocks the thread.
    ///
    /// # Errors
    ///
    /// - [`CodecError::GpuUnavailable`] if `prepare_for_device` fails
    ///   (no adapter or driver unavailable).
    /// - [`CodecError::GpuError`] if the GPU kernel dispatch or readback fails.
    /// - All errors from [`Codec::compress_batch_with`] when the CPU fallback
    ///   is taken.
    #[doc(cfg(feature = "gpu-wgpu"))]
    pub fn compress_batch_gpu_with<B>(
        &self,
        vectors: &[f32],
        rows: usize,
        cols: usize,
        prepared: &mut PreparedCodec,
        backend: &mut B,
        parallelism: Parallelism,
    ) -> Result<Vec<CompressedVector>, CodecError>
    where
        B: tinyquant_gpu_wgpu::ComputeBackend,
    {
        if rows >= tinyquant_gpu_wgpu::GPU_BATCH_THRESHOLD {
            backend
                .prepare_for_device(prepared)
                .map_err(CodecError::from)?;
            backend
                .compress_batch(vectors, rows, cols, prepared)
                .map_err(CodecError::from)
        } else {
            self.compress_batch_with(
                vectors,
                rows,
                cols,
                prepared.config(),
                prepared.codebook(),
                parallelism,
            )
        }
    }
}
```

> [!note] Why a separate `impl` block?
> Placing the GPU methods in a separate `#[cfg(feature = "gpu-wgpu")] impl Codec`
> block keeps `service.rs` readable and avoids `#[cfg]` annotations on
> individual `use` statements inside the method. The existing `impl Codec`
> block remains feature-agnostic.

### Step 5 — Re-export the GPU public surface from `tinyquant-core`

**File:** `rust/crates/tinyquant-core/src/lib.rs`

Append after the existing `pub mod` declarations:

```rust
/// GPU acceleration types and backend trait.
///
/// Only present when the `gpu-wgpu` Cargo feature is enabled.
#[cfg(feature = "gpu-wgpu")]
#[doc(cfg(feature = "gpu-wgpu"))]
pub mod gpu {
    pub use tinyquant_gpu_wgpu::{
        AdapterCandidate,
        BackendPreference,
        ComputeBackend,
        TinyQuantGpuError,
        WgpuBackend,
        GPU_BATCH_THRESHOLD,
    };
}

// Flat re-exports for ergonomic use without the `gpu` module prefix.
#[cfg(feature = "gpu-wgpu")]
pub use tinyquant_gpu_wgpu::{
    AdapterCandidate,
    BackendPreference,
    ComputeBackend,
    TinyQuantGpuError,
    WgpuBackend,
    GPU_BATCH_THRESHOLD,
};
```

Callers can then write:

```rust
use tinyquant_core::{WgpuBackend, BackendPreference, ComputeBackend};
```

The `gpu` module re-export is an additional namespace for callers who prefer
explicit namespacing (`tinyquant_core::gpu::WgpuBackend`). Both paths are
available and equivalent.

### Step 6 — Update `feature-flags` design doc

**File:** `docs/design/rust/feature-flags.md`

In the `## Feature matrix` section, add a new subsection after `tinyquant-core`
(before `tinyquant-io`):

````markdown
#### `gpu-wgpu` feature (Phase 28.5)

```toml
# Enables GPU-accelerated batch compression via tinyquant-gpu-wgpu.
# Implies `std`. Raises the effective MSRV from 1.81 to 1.87 (wgpu requirement).
gpu-wgpu = ["dep:tinyquant-gpu-wgpu", "std"]
```

Updated build matrix for `tinyquant-core`:

| Features | MSRV | Purpose |
|---|---|---|
| (none) | 1.81 | `no_std + alloc` embedded/WASM build |
| `std` | 1.81 | Standard baseline |
| `std,simd` | 1.81 | Default for leaf crates |
| `std,simd,rayon` | 1.81 | Integration with `tinyquant-io` |
| `gpu-wgpu` | 1.87 | GPU-accelerated batch compression |
````

Also update the `## CI feature matrix` table to add:

```toml
| `gpu-wgpu` | `cargo build -p tinyquant-core --features gpu-wgpu` (1.87 job) |
| `gpu-wgpu` tests | `cargo test -p tinyquant-core --features gpu-wgpu` (software renderer) |
```

Update the `> [!note] GPU crates excluded from the 1.81 MSRV gate` callout to
mention that the new `gpu-feature-gate` CI job now also validates
`tinyquant-core --features gpu-wgpu`.

### Step 7 — CI: `gpu-feature-gate` job

**File:** `.github/workflows/rust-ci.yml`

Add a new job after the `gpu-compile` job:

```yaml
  gpu-feature-gate:
    name: build + test (tinyquant-core gpu-wgpu feature)
    runs-on: ubuntu-22.04
    # This job is independent of the runner-selection matrix because it always
    # runs on a fixed GitHub-hosted image (needs mesa software renderer).
    env:
      RUSTUP_TOOLCHAIN: "1.87.0"
      # Force wgpu to use the OpenGL software renderer available in mesa.
      WGPU_BACKEND: "gl"
      MESA_NO_ERROR: "1"
    defaults:
      run:
        working-directory: rust
    steps:
      - uses: actions/checkout@v5
        with:
          lfs: false
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: "1.87.0"
          components: clippy
      - name: install mesa EGL (software renderer)
        run: sudo apt-get update -q && sudo apt-get install -y libegl1-mesa-dev libgl1-mesa-dri
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust -> target
          key: gpu-feature-gate-1.87
      - name: build tinyquant-core with gpu-wgpu feature
        run: cargo build -p tinyquant-core --features gpu-wgpu
      - name: test tinyquant-core with gpu-wgpu feature (software renderer)
        run: cargo test -p tinyquant-core --features gpu-wgpu
      - name: clippy tinyquant-core with gpu-wgpu feature
        run: cargo clippy --no-deps -p tinyquant-core --features gpu-wgpu -- -D warnings
      - name: clippy tinyquant-core without gpu-wgpu feature
        run: cargo clippy --no-deps -p tinyquant-core -- -D warnings
      - name: verify no_std still compiles without gpu-wgpu
        run: |
          rustup target add thumbv7em-none-eabihf
          cargo build -p tinyquant-core --no-default-features --target thumbv7em-none-eabihf
```

> [!note] Software renderer availability
> The `ubuntu-22.04` GitHub-hosted image ships mesa but does not install
> `libegl1-mesa-dev` by default. The explicit `apt-get install` step is
> required; without it `wgpu` cannot find an EGL context for the `gl`
> backend and all tests skip/panic.

> [!note] Cache key separation
> The `key: gpu-feature-gate-1.87` cache key prevents cache collisions with
> the `1.81.0`-toolchain jobs that also use `rust -> target` as the
> workspace mapping.

### Step 8 — `tinyquant-py` feature stub (optional, logistics deferred)

**File:** `rust/crates/tinyquant-py/Cargo.toml`

Add to `[features]`:

```toml
# Enables GPU-accelerated batch compression in the Python wheel.
# The wheel build pipeline must pass `--features gpu-wgpu` to maturin.
# Wheel logistics (separate wheel artifact, PyPI extras) are deferred
# to Phase 29 or the next release planning turn.
gpu-wgpu = ["tinyquant-core/gpu-wgpu"]
```

No further changes to `tinyquant-py` are required in this phase.
The PyO3 bindings do not expose `WgpuBackend` directly; GPU routing
is transparent at the `compress_batch_with`-equivalent call site.

---

## Files to change

| File | Change |
|---|---|
| `rust/crates/tinyquant-gpu-wgpu/Cargo.toml` | Remove `publish = false`; add `license`, `repository`, `keywords`, `categories`; update `description` |
| `rust/crates/tinyquant-core/Cargo.toml` | Add `gpu-wgpu` feature; add optional `tinyquant-gpu-wgpu` dep with `dep:` prefix |
| `rust/crates/tinyquant-core/src/errors.rs` | Add `GpuUnavailable` and `GpuError` variants (feature-gated); add `From<TinyQuantGpuError>` impl |
| `rust/crates/tinyquant-core/src/codec/service.rs` | Add `#[cfg(feature = "gpu-wgpu")] impl Codec` block with `compress_batch_gpu_with` |
| `rust/crates/tinyquant-core/src/lib.rs` | Add `#[cfg(feature = "gpu-wgpu")]` re-exports (`pub mod gpu` + flat `pub use`) |
| `rust/crates/tinyquant-py/Cargo.toml` | Add `gpu-wgpu` feature stub |
| `.github/workflows/rust-ci.yml` | Add `gpu-feature-gate` job |
| `docs/design/rust/feature-flags.md` | Add `gpu-wgpu` row to `tinyquant-core` feature matrix; update CI table |
| `README.md` and `.github/README.md` | Update GPU recipe to show `features = ["gpu-wgpu"]` on `tinyquant-core` |

---

## Tests

### Test file: `rust/crates/tinyquant-core/tests/gpu_feature_gate.rs`

New integration test file, compiled only with `--features gpu-wgpu`. Open with:

```rust
//! Integration tests for the `gpu-wgpu` feature gate of `tinyquant-core`.
//!
//! These tests run against the software renderer (`WGPU_BACKEND=gl`).
//! They verify: re-exports are accessible, routing is correct, errors
//! convert correctly, and parity with the CPU path holds.

#![cfg(feature = "gpu-wgpu")]
```

#### `gpu_types_accessible_via_tinyquant_core`

Smoke test. Verifies that all re-exported GPU types compile and are reachable
from `tinyquant_core::`:

```rust
#[test]
fn gpu_types_accessible_via_tinyquant_core() {
    use tinyquant_core::{
        AdapterCandidate, BackendPreference, ComputeBackend,
        TinyQuantGpuError, WgpuBackend, GPU_BATCH_THRESHOLD,
    };
    // GPU_BATCH_THRESHOLD is a const — access it to verify it's compiled in.
    assert!(GPU_BATCH_THRESHOLD > 0);
    // BackendPreference variants must be accessible.
    let _ = BackendPreference::Auto;
    let _ = BackendPreference::Software;
}
```

#### `gpu_batch_threshold_matches_gpu_crate_constant`

Verifies the re-exported constant equals the value in `tinyquant-gpu-wgpu`:

```rust
#[test]
fn gpu_batch_threshold_matches_gpu_crate_constant() {
    assert_eq!(
        tinyquant_core::GPU_BATCH_THRESHOLD,
        tinyquant_gpu_wgpu::GPU_BATCH_THRESHOLD,
    );
}
```

#### `compress_batch_gpu_with_below_threshold_uses_cpu_path`

Verifies that `rows < GPU_BATCH_THRESHOLD` falls through to CPU and produces
the same output as `compress_batch_with`:

```rust
#[tokio::test]
async fn compress_batch_gpu_with_below_threshold_uses_cpu_path() {
    use tinyquant_core::{
        codec::{Codec, CodecConfig, Codebook, Parallelism, PreparedCodec},
        WgpuBackend,
    };
    let config = CodecConfig::new(4, 64, 42, false).unwrap();
    let vectors: Vec<f32> = (0..32 * 64)
        .map(|i| (i as f32) / 100.0)
        .collect();
    let codebook = /* build from training data */;
    let mut prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();
    let mut backend = WgpuBackend::new_with_preference(
        tinyquant_core::BackendPreference::Software
    ).await.unwrap();

    let rows = 32; // < GPU_BATCH_THRESHOLD (512)
    let codec = Codec::new();

    let cpu_result = codec
        .compress_batch_with(&vectors, rows, 64, &config, &codebook, Parallelism::Serial)
        .unwrap();
    let gpu_path_result = codec
        .compress_batch_gpu_with(&vectors, rows, 64, &mut prepared, &mut backend, Parallelism::Serial)
        .unwrap();

    // CPU fallback must produce byte-identical output.
    assert_eq!(cpu_result.len(), gpu_path_result.len());
    for (a, b) in cpu_result.iter().zip(gpu_path_result.iter()) {
        assert_eq!(a.indices(), b.indices());
        assert_eq!(a.config_hash(), b.config_hash());
    }
}
```

#### `compress_batch_gpu_with_above_threshold_succeeds`

Verifies GPU path is taken and completes successfully with the software
renderer when `rows >= GPU_BATCH_THRESHOLD`:

```rust
#[tokio::test]
async fn compress_batch_gpu_with_above_threshold_succeeds() {
    use tinyquant_core::{
        codec::{Codec, CodecConfig, Parallelism, PreparedCodec},
        WgpuBackend, GPU_BATCH_THRESHOLD,
    };
    let dim = 64usize;
    let rows = GPU_BATCH_THRESHOLD; // exactly at threshold
    let config = CodecConfig::new(4, dim as u32, 42, false).unwrap();
    let vectors: Vec<f32> = (0..rows * dim).map(|i| (i as f32) / 1000.0).collect();
    let codebook = /* build */;
    let mut prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();
    let mut backend = WgpuBackend::new_with_preference(
        tinyquant_core::BackendPreference::Software
    ).await.unwrap();

    let result = Codec::new().compress_batch_gpu_with(
        &vectors, rows, dim, &mut prepared, &mut backend, Parallelism::Serial,
    );
    assert!(result.is_ok(), "GPU compress at threshold must succeed: {:?}", result);
    assert_eq!(result.unwrap().len(), rows);
}
```

#### `compress_batch_gpu_with_parity_against_cpu`

Parity test. Output of GPU path must match CPU path element-by-element
for all rows when the software renderer is available:

```rust
#[tokio::test]
async fn compress_batch_gpu_with_parity_against_cpu() {
    // Use a small dim (64) and rows above threshold (512) to keep the test
    // under 10 s on the software renderer.
    let dim = 64usize;
    let rows = 512usize;
    // ... setup config, codebook, vectors ...
    let cpu_out = codec.compress_batch_with(&vectors, rows, dim, &config, &codebook, Parallelism::Serial).unwrap();
    let gpu_out = codec.compress_batch_gpu_with(&vectors, rows, dim, &mut prepared, &mut backend, Parallelism::Serial).unwrap();
    for (row, (c, g)) in cpu_out.iter().zip(gpu_out.iter()).enumerate() {
        assert_eq!(c.indices(), g.indices(), "parity failure at row {row}");
        assert_eq!(c.config_hash(), g.config_hash());
        assert_eq!(c.bit_width(), g.bit_width());
    }
}
```

#### `codec_error_from_gpu_error_unavailable_variant`

Unit test for the `From<TinyQuantGpuError>` conversion:

```rust
#[test]
fn codec_error_from_gpu_error_unavailable_variant() {
    use tinyquant_core::errors::CodecError;
    use tinyquant_gpu_wgpu::TinyQuantGpuError;

    let gpu_err = TinyQuantGpuError::NoAdapter("no adapter".to_string());
    let codec_err = CodecError::from(gpu_err);
    assert!(
        matches!(codec_err, CodecError::GpuUnavailable(_)),
        "NoAdapter should map to GpuUnavailable"
    );
}
```

#### `codec_error_from_gpu_error_gpu_error_variant`

```rust
#[test]
fn codec_error_from_gpu_error_gpu_error_variant() {
    use tinyquant_core::errors::CodecError;
    use tinyquant_gpu_wgpu::TinyQuantGpuError;

    // Use any non-NoAdapter variant.
    let gpu_err = TinyQuantGpuError::ShaderCompile("test shader failure".to_string());
    let codec_err = CodecError::from(gpu_err);
    assert!(
        matches!(codec_err, CodecError::GpuError(_)),
        "ShaderCompile should map to GpuError"
    );
}
```

#### `prepare_for_device_is_idempotent_via_codec`

Verifies that calling `compress_batch_gpu_with` twice with the same
`PreparedCodec` does not error (second call hits the already-prepared state):

```rust
#[tokio::test]
async fn prepare_for_device_is_idempotent_via_codec() {
    // rows >= GPU_BATCH_THRESHOLD to ensure prepare_for_device is called
    let result1 = codec.compress_batch_gpu_with(...).unwrap();
    let result2 = codec.compress_batch_gpu_with(...).unwrap();
    assert_eq!(result1.len(), result2.len());
}
```

#### `gpu_wgpu_feature_does_not_break_cpu_only_tests`

Run the existing `compress_batch` and `decompress_batch_into` tests with
`--features gpu-wgpu` to confirm the CPU path still works when the GPU feature
is present. These are the existing tests in `service.rs` — no new code needed;
just verify CI runs them with the feature flag.

### Compile-fail test (optional)

If `trybuild` is added as a dev-dependency, a compile-fail test can verify
the feature gate is active:

**File:** `rust/crates/tinyquant-core/tests/ui/no_gpu_without_feature.rs`

```rust
// This file must NOT compile without --features gpu-wgpu.
use tinyquant_core::WgpuBackend; // error[E0432]: unresolved import
```

Gated in the test harness as `#[cfg(not(feature = "gpu-wgpu"))]`.

---

## Acceptance criteria

- [ ] `cargo publish --dry-run -p tinyquant-gpu-wgpu` exits 0.
- [ ] `cargo build -p tinyquant-core --features gpu-wgpu` exits 0 with toolchain
      `1.87.0`.
- [ ] `cargo build -p tinyquant-core` (no GPU feature) exits 0 and produces
      no references to `tinyquant_gpu_wgpu` in the binary (verified with
      `cargo tree -p tinyquant-core | grep tinyquant-gpu-wgpu` — must be absent
      without the feature).
- [ ] `use tinyquant_core::WgpuBackend` compiles with `--features gpu-wgpu`.
- [ ] `use tinyquant_core::WgpuBackend` produces `error[E0432]` without the
      feature (confirmed by compile-fail test or manual check).
- [ ] `use tinyquant_core::ComputeBackend` is accessible with `--features gpu-wgpu`.
- [ ] `cargo test -p tinyquant-core --features gpu-wgpu` green with
      `WGPU_BACKEND=gl` (software renderer).
- [ ] `gpu-feature-gate` CI job green on `ubuntu-22.04`.
- [ ] `cargo clippy --no-deps -p tinyquant-core --features gpu-wgpu -- -D warnings`
      clean.
- [ ] `cargo clippy --no-deps -p tinyquant-core -- -D warnings` clean (no GPU
      feature — ensures no stray imports or dead-code warnings).
- [ ] `cargo build -p tinyquant-core --no-default-features --target thumbv7em-none-eabihf`
      exits 0 (no_std unbroken).
- [ ] `cargo publish --dry-run -p tinyquant-core` exits 0 with the new feature
      defined.
- [ ] `GpuUnavailable` and `GpuError` are absent from `CodecError` without the
      feature (no phantom variants in non-GPU builds).
- [ ] Parity test confirms GPU and CPU paths produce byte-identical indices for
      the same input on the software renderer.

---

## Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `TinyQuantGpuError` variant names differ from what the `From` impl expects | Low | Medium | Read `tinyquant-gpu-wgpu/src/error.rs` before writing the `From` impl; match on the actual variant names |
| `tinyquant-gpu-wgpu` 1.0.0 publish fails due to missing mandatory fields | Low | Low | Verify all required fields (description, keywords, categories, license, repository) are present before `cargo publish` |
| Software renderer (`WGPU_BACKEND=gl`) unavailable on `ubuntu-22.04` CI image | Low | Medium | Explicit `apt-get install libegl1-mesa-dev libgl1-mesa-dri` step; validate locally first with `docker run ubuntu:22.04` |
| `#[cfg(feature = "gpu-wgpu")]` on `CodecError` variants breaks `thiserror` derive | Low | Low | `thiserror` respects `#[cfg]` attributes on enum variants correctly (confirmed in thiserror v2 docs) |
| `compress_batch_gpu_with` panics when called from inside a `tokio::runtime::block_on` | Medium | Medium | Document the async-context constraint in the doc comment (`# Async-context safety` section); consider adding a `#[track_caller]` assertion in a follow-up |
| Mesa EGL apt package name differs between Ubuntu LTS versions | Low | Low | Use `libegl1-mesa-dev` (stable across 20.04/22.04/24.04); add `libgl1-mesa-dri` as fallback |
| `cargo tree` includes `tinyquant-gpu-wgpu` in `tinyquant-core` binary without feature | Low | High | Verify with `cargo tree -p tinyquant-core --no-default-features` — must show no GPU crate |
| `avx512` Cargo feature interacts unexpectedly with `gpu-wgpu` | Very Low | Low | Both features imply `std`; no WGSL or SIMD code shares state; no conflict expected |
| CPU fallback in `compress_batch_gpu_with` rebuilds rotation matrix | Medium | Low | This is the same cost as `compress_batch_with` today. A future phase can add `compress_batch_prepared_gpu_with` that skips the rebuild. Document as a known perf limitation in the method doc comment |
| `tinyquant-py` wheel build fails with `--features gpu-wgpu` on platforms without wgpu support (e.g. musl/WASM) | Low | Medium | Feature is not in the default wheel build; gate the maturin `--features gpu-wgpu` step behind a separate CI job controlled by an env flag |
| Version `1.0.0` on crates.io already taken for `tinyquant-gpu-wgpu` under a different author | Very Low | High | Check with `cargo search tinyquant-gpu-wgpu` before publishing; if taken, rename per the crate-topology design doc |

## Pitfalls

> [!warning] `dep:` prefix is mandatory
> Without `dep:tinyquant-gpu-wgpu` in the feature definition, Cargo
> (edition 2021) still works but implicitly creates a `tinyquant-gpu-wgpu`
> feature alias. This confuses downstream users who see two feature names
> (`gpu-wgpu` and `tinyquant-gpu-wgpu`) for the same thing. Always use
> `dep:` to suppress the implicit alias.

> [!warning] `gpu-wgpu` must imply `std`
> If you forget `"std"` in `gpu-wgpu = ["dep:tinyquant-gpu-wgpu", "std"]`,
> the build will fail with confusing linker or missing-symbol errors when wgpu
> tries to pull in OS primitives that are absent from the no_std build. The
> compiler error appears in wgpu, not in your code.

> [!warning] Toolchain 1.87 only for the GPU feature
> The `gpu-feature-gate` CI job must use `1.87.0`. Accidentally running it
> with `1.81.0` will fail at the wgpu transitive deps with confusing edition
> errors. Use `RUSTUP_TOOLCHAIN: "1.87.0"` env at the job level (same pattern
> as the existing `gpu-compile` job) so it applies to all steps without
> per-step annotation.

> [!warning] `#[non_exhaustive]` + feature-gated variants
> `CodecError` is `#[non_exhaustive]`. Adding `#[cfg(feature = "gpu-wgpu")]`
> variants does not break existing match statements in downstream code because
> the `#[non_exhaustive]` attribute already requires a wildcard arm. However,
> within `tinyquant-core` itself (which is in the defining crate), exhaustive
> matches must be updated to include the new variants — or use a `cfg`-gated
> arm. The CI clippy pass will catch any exhaustiveness failures.

> [!warning] `PreparedCodec` must be built from the same `(config, codebook)` as the GPU path
> `backend.compress_batch(vectors, rows, cols, prepared)` uses the rotation
> matrix inside `PreparedCodec` to rotate on the GPU. If `prepared` was built
> from a different config than the one used to train the codebook, the output
> will be silently wrong (not an error). This is the same silent misuse risk
> as the CPU `compress_prepared` path — document it clearly.

> [!warning] `prepare_for_device` uploads rotation + codebook buffers
> The first call to `prepare_for_device` performs a GPU upload. For large
> codebooks or rotation matrices (e.g. dim=4096), this can be tens of
> milliseconds. Do not call `compress_batch_gpu_with` in a tight loop with a
> fresh `PreparedCodec` every iteration — reuse the `PreparedCodec` across
> calls.

## See also

- [[plans/rust/phase-28-wgpu-pipeline-caching|Phase 28]] — wgpu surface this integrates
- [[plans/rust/phase-29-cuda-backend|Phase 29]] — CUDA follows same pattern
- [[design/rust/feature-flags|Feature Flags]] — feature-flag conventions
- [[design/rust/gpu-acceleration|GPU Acceleration Design]]
- [[design/rust/error-model|Error Model]] — `CodecError` variant taxonomy
