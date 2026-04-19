---
title: Rust Port — Feature Flags and Optional Dependencies
tags:
  - design
  - rust
  - features
  - cargo
date-created: 2026-04-10
status: draft
category: design
---

# Rust Port — Feature Flags and Optional Dependencies

> [!info] Purpose
> Lay out the Cargo feature matrix across all crates so that
> downstream consumers can pull exactly what they need and no more.

## Principles

1. **Default features are minimal.** A dependent adding a crate
   without specifying features should get a working, correct, but
   not-necessarily-fast build.
2. **Features are additive.** No feature ever removes behavior or
   API surface. Turning a feature on should only add.
3. **No `mutually-exclusive-feature` traps.** The Cargo model does
   not support them cleanly; we enforce mutual exclusion through
   error messages rather than feature arithmetic.
4. **Every feature combination that ships is CI-tested.** No
   "optional paths" that are never exercised.

## Feature matrix

### `tinyquant-core`

```toml
[features]
default = []

# Enable std-dependent functionality (rotation cache with a proper
# mutex, runtime feature detection, clocks). Without it the crate is
# no_std + alloc.
std = []

# Portable SIMD kernels. Requires std.
simd = ["std"]

# Additional unstable kernels that require nightly Rust.
simd-nightly = ["simd"]

# Use rayon for internal parallelism hints. NOT enabled here — parallelism
# is driven from the consumer via the Parallelism enum. This feature only
# adds conveniences like a default rayon-backed Parallelism::Custom
# construction helper.
rayon = ["std", "dep:rayon"]
```

Build matrix:

| Features | Purpose |
|---|---|
| (none) | `no_std + alloc` smoke build (embedded, WASM) |
| `std` | Standard baseline |
| `std,simd` | Default for leaf crates |
| `std,simd,rayon` | Integration with `tinyquant-io` |

### `tinyquant-io`

```toml
[features]
default = ["simd"]
simd = ["tinyquant-core/simd"]
mmap = ["dep:memmap2"]
rayon = ["dep:rayon", "tinyquant-core/rayon"]
all = ["simd", "mmap", "rayon"]
```

Defaults include `simd` because an IO-using consumer is almost
always std-capable.

### `tinyquant-bruteforce`

```toml
[features]
default = ["simd"]
simd = ["tinyquant-core/simd"]
rayon = ["dep:rayon"]
```

### `tinyquant-pgvector`

```toml
[features]
default = []
test-containers = ["dep:testcontainers"]
```

`test-containers` activates the dependency on `testcontainers` (Rust
crate) for integration tests that spin up a PostgreSQL container
with `pgvector` extension. Not included in `default` because it
requires Docker.

### `tinyquant-sys`

```toml
[features]
default = ["simd"]
simd = ["tinyquant-core/simd", "tinyquant-io/simd", "tinyquant-bruteforce/simd"]
mmap = ["tinyquant-io/mmap"]
rayon = ["tinyquant-core/rayon", "tinyquant-io/rayon", "tinyquant-bruteforce/rayon"]
jemalloc = ["dep:jemallocator"]
mimalloc = ["dep:mimalloc"]

# cdylib + staticlib default
all = ["simd", "mmap", "rayon"]
```

`jemalloc` and `mimalloc` are mutually exclusive. The `lib.rs`
asserts:

```rust
#[cfg(all(feature = "jemalloc", feature = "mimalloc"))]
compile_error!("Features `jemalloc` and `mimalloc` are mutually exclusive");
```

### `tinyquant-py`

```toml
[features]
default = ["simd", "rayon"]
simd = ["tinyquant-core/simd"]
rayon = ["tinyquant-core/rayon", "tinyquant-io/rayon", "tinyquant-bruteforce/rayon", "dep:rayon"]
numpy = ["dep:numpy"]  # always on for defaults
# abi3-py312 is specified on pyo3 directly, not as a tinyquant feature
```

### `tinyquant-gpu-wgpu` (Phase 27)

```toml
[package]
name = "tinyquant-gpu-wgpu"
# MSRV override: wgpu requires Rust ≥ 1.87. This field overrides the
# workspace rust-version (1.81) for this crate only. Cargo respects
# the per-package field when it exists.
rust-version = "1.87"

[features]
default = []

# Enable compile-time shader validation (wgpu naga validator).
# Default off: validator adds ~6 s to cold build; only needed in dev.
validate-shaders = []

# Enable dhat heap profiling on GPU buffer allocations.
dhat-heap = ["dep:dhat"]
```

Build matrix:

| Features | Purpose |
|---|---|
| (none) | Default — adapter probe + WGSL compile, no extras |
| `validate-shaders` | Shader-compile CI job |
| `dhat-heap` | GPU allocation profiling only |

`wgpu` is declared as:

```toml
[dependencies]
wgpu = { version = "22", default-features = false, features = ["wgsl"] }
tinyquant-core = { path = "../tinyquant-core" }
```

`default-features = false` keeps the dep from pulling in `naga`'s
GLSL and HLSL parsers, which are not needed.

### `tinyquant-gpu-cuda` (Phase 29)

```toml
[package]
rust-version = "1.87"

[features]
default = []

# Enable CUDA device enumeration and kernel launch.
# Off by default: build fails if no CUDA toolkit is installed.
cuda = ["dep:cust"]
```

When the `cuda` feature is absent the crate compiles to an empty stub
so downstream code that conditionally enables it does not need `#[cfg]`
fences everywhere — `CudaBackend::is_available()` always returns
`false` without the feature.

### `tinyquant-bench`

```toml
[features]
default = ["simd", "rayon"]
simd = ["tinyquant-core/simd", "tinyquant-io/simd"]
rayon = ["tinyquant-core/rayon", "tinyquant-io/rayon"]
dhat-heap = ["dep:dhat"]  # for memory bench only
```

## CI feature matrix

`rust-ci.yml` runs the following combinations to catch feature
regressions:

| Job | Command |
|---|---|
| `default` | `cargo test --workspace` |
| `no-default` | `cargo test --workspace --no-default-features` |
| `all-features` | `cargo test --workspace --all-features` |
| `core-nostd` | `cargo build -p tinyquant-core --no-default-features --target thumbv7em-none-eabihf` |
| `core-std-only` | `cargo test -p tinyquant-core --no-default-features --features std` |
| `core-std-simd` | `cargo test -p tinyquant-core --features std,simd` |
| `io-minimal` | `cargo test -p tinyquant-io --no-default-features` |
| `sys-jemalloc` | `cargo test -p tinyquant-sys --features simd,jemalloc` |
| `sys-mimalloc` | `cargo test -p tinyquant-sys --features simd,mimalloc` |

Total: 9 cargo invocations. All run in parallel on a matrix job.

> [!note] GPU crates excluded from the 1.81 MSRV gate
> The workspace MSRV check (`cargo +1.81.0 check`) explicitly excludes
> `tinyquant-gpu-wgpu` and `tinyquant-gpu-cuda` via
> `--exclude tinyquant-gpu-wgpu --exclude tinyquant-gpu-cuda` because
> those crates declare `package.rust-version = "1.87"`. A separate CI
> job (`gpu-compile-check`) runs `cargo +1.87.0 check -p
> tinyquant-gpu-wgpu` on `ubuntu-22.04` without a physical GPU, gating
> only on successful compilation and shader validation.

## Feature interactions audit

| Interaction | Allowed | Behavior |
|---|---|---|
| `simd` without `std` in `tinyquant-core` | no | `compile_error!` — `simd` depends on `std` |
| `rayon` without `std` | no | `compile_error!` |
| `jemalloc` + `mimalloc` in `tinyquant-sys` | no | `compile_error!` |
| `mmap` without `std` | no | implicit via `memmap2` dep |
| `test-containers` in production builds of `tinyquant-pgvector` | no | tests-only feature; release builds reject it |

## `no_std` story

`tinyquant-core --no-default-features` compiles and runs on:

- `thumbv7em-none-eabihf` (Cortex-M4F)
- `riscv32imc-unknown-none-elf` (ESP32-C3)
- `wasm32-unknown-unknown`

With no SIMD, no mmap, no rayon, no std. The core codec works; only
the rotation matrix cache uses a `spin::Mutex` instead of
`std::sync::Mutex`. Because the rotation matrix is dominated by the
QR decomposition (which `faer`'s `no_std` mode supports), performance
on embedded targets is bounded by matrix size and memory.

The CI matrix includes a build-only job for each `no_std` target:

```yaml
- name: no_std build (Cortex-M4F)
  run: |
    rustup target add thumbv7em-none-eabihf
    cargo build -p tinyquant-core --no-default-features --target thumbv7em-none-eabihf

- name: no_std build (WASM)
  run: |
    rustup target add wasm32-unknown-unknown
    cargo build -p tinyquant-core --no-default-features --target wasm32-unknown-unknown
```

Running tests on these targets is not feasible without a simulator,
so we rely on `cargo test --no-default-features` on the host to
exercise the code paths (host target still compiles with no SIMD
flag).

## Optional dependencies

All optional dependencies declared as:

```toml
rayon = { workspace = true, optional = true }
memmap2 = { workspace = true, optional = true }
faer = { workspace = true, default-features = false }  # always on but lean
numpy = { workspace = true, optional = true }
jemallocator = { workspace = true, optional = true }
mimalloc = { workspace = true, optional = true }
testcontainers = { workspace = true, optional = true }
dhat = { workspace = true, optional = true }
```

Consumers always pin features in their Cargo.toml:

```toml
# Downstream application
tinyquant-core = { version = "0.1", features = ["std", "simd"] }
tinyquant-io = { version = "0.1" }  # gets default = ["simd"]
```

## Documentation per feature

Every public item has a `#[doc(cfg(feature = "..."))]` attribute
where appropriate, so `cargo doc --all-features` renders feature
badges next to feature-gated items.

## Versioning and features

Adding a new feature in a minor release is safe (additive). Removing
a feature is a breaking change and requires a major version bump.
Making an existing feature a default is a breaking change for
downstreams using `default-features = false`.

## See also

- [[design/rust/crate-topology|Crate Topology]]
- [[design/rust/ci-cd|CI/CD]]
- [[design/rust/release-strategy|Release and Versioning]]
