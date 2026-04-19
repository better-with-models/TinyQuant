---
title: "Phase 11: Rust Workspace Scaffold"
tags:
  - plans
  - rust
  - phase-11
  - scaffolding
date-created: 2026-04-10
status: complete
category: planning
---

# Phase 11: Rust Workspace Scaffold

> [!info] Goal
> Create the Cargo workspace under `rust/` with all crates declared,
> a pinned toolchain, the lint wall, one smoke test per crate, and a
> `rust-ci.yml` GitHub Actions workflow that runs fmt + clippy + test
> and is green on an otherwise empty codebase.

> [!note] Reference docs
> - [[design/rust/README|Rust Port Overview]]
> - [[design/rust/crate-topology|Crate Topology]]
> - [[design/rust/feature-flags|Feature Flags]]
> - [[design/rust/ci-cd|CI/CD]]

## Prerequisites

- Phase 10 complete (Python `v0.1.1` released).
- Local Rust toolchain 1.78+.
- Developer has Docker available for the WASM/embedded target
  sanity check (optional).

## Deliverables

### Files to create

| File | Purpose |
|------|---------|
| `rust/Cargo.toml` | Workspace root |
| `rust/rust-toolchain.toml` | Pinned `1.78.0` stable toolchain |
| `rust/.cargo/config.toml` | Lint walls, profile.bench settings |
| `rust/.gitignore` | Ignore `target/`, `Cargo.lock` only at crate level |
| `rust/README.md` | Points readers at `docs/design/rust/README.md` |
| `rust/crates/tinyquant-core/Cargo.toml` | Core crate manifest |
| `rust/crates/tinyquant-core/src/lib.rs` | `#![no_std]` lib stub + lint wall |
| `rust/crates/tinyquant-core/src/prelude.rs` | Empty prelude |
| `rust/crates/tinyquant-core/src/types.rs` | Empty (placeholder for phase 12) |
| `rust/crates/tinyquant-core/tests/smoke.rs` | Single `#[test]` asserting crate compiles |
| `rust/crates/tinyquant-io/Cargo.toml` | IO crate manifest |
| `rust/crates/tinyquant-io/src/lib.rs` | Empty lib + lint wall |
| `rust/crates/tinyquant-io/tests/smoke.rs` | Single `#[test]` |
| `rust/crates/tinyquant-bruteforce/Cargo.toml` | Backend crate manifest |
| `rust/crates/tinyquant-bruteforce/src/lib.rs` | Empty lib + lint wall |
| `rust/crates/tinyquant-bruteforce/tests/smoke.rs` | Single `#[test]` |
| `rust/crates/tinyquant-pgvector/Cargo.toml` | Adapter crate manifest |
| `rust/crates/tinyquant-pgvector/src/lib.rs` | Empty lib + lint wall |
| `rust/crates/tinyquant-pgvector/tests/smoke.rs` | Single `#[test]` |
| `rust/crates/tinyquant-sys/Cargo.toml` | C ABI crate manifest |
| `rust/crates/tinyquant-sys/src/lib.rs` | Empty `extern "C"` stub |
| `rust/crates/tinyquant-sys/cbindgen.toml` | Header gen config |
| `rust/crates/tinyquant-sys/build.rs` | cbindgen driver |
| `rust/crates/tinyquant-sys/tests/smoke.rs` | Single `#[test]` |
| `rust/crates/tinyquant-py/Cargo.toml` | pyo3 crate manifest |
| `rust/crates/tinyquant-py/pyproject.toml` | maturin config |
| `rust/crates/tinyquant-py/src/lib.rs` | Empty `#[pymodule]` stub |
| `rust/crates/tinyquant-py/python/tinyquant_rs/__init__.py` | Re-export stub |
| `rust/crates/tinyquant-py/python/tinyquant_rs/py.typed` | PEP 561 marker |
| `rust/crates/tinyquant-bench/Cargo.toml` | Benchmark crate manifest |
| `rust/crates/tinyquant-bench/src/lib.rs` | Empty lib |
| `rust/crates/tinyquant-bench/benches/smoke.rs` | Criterion placeholder |
| `rust/crates/tinyquant-fuzz/Cargo.toml` | Fuzz targets manifest |
| `rust/crates/tinyquant-fuzz/fuzz_targets/smoke.rs` | Empty fuzz target |
| `rust/xtask/Cargo.toml` | xtask manifest |
| `rust/xtask/src/main.rs` | `cargo xtask --help` stub |
| `.github/workflows/rust-ci.yml` | Per-PR workflow |

### `rust/Cargo.toml` specification

```toml
[workspace]
resolver = "2"
members = [
    "crates/tinyquant-core",
    "crates/tinyquant-io",
    "crates/tinyquant-bruteforce",
    "crates/tinyquant-pgvector",
    "crates/tinyquant-sys",
    "crates/tinyquant-py",
    "crates/tinyquant-bench",
    "crates/tinyquant-fuzz",
    "xtask",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.78"
license = "Apache-2.0"
repository = "https://github.com/better-with-models/TinyQuant"
homepage = "https://github.com/better-with-models/TinyQuant"
readme = "../README.md"

[workspace.dependencies]
thiserror = "1"
half = { version = "2", default-features = false }
bytemuck = { version = "1", features = ["derive"] }
sha2 = { version = "0.10", default-features = false }
hex = { version = "0.4", default-features = false, features = ["alloc"] }

[profile.release]
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
debug = 0
incremental = false
overflow-checks = false

[profile.release-with-debug]
inherits = "release"
debug = 2
strip = "none"

[profile.bench]
inherits = "release-with-debug"
```

## Steps (TDD order)

- [ ] **Step 1: Create workspace directory**

```bash
mkdir -p rust/crates rust/xtask
```

Expected: new empty directories.

- [ ] **Step 2: Write `rust/rust-toolchain.toml`**

```toml
[toolchain]
channel = "1.78.0"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

- [ ] **Step 3: Write `rust/Cargo.toml`** (full content above).

- [ ] **Step 4: Write `rust/.cargo/config.toml`**

```toml
[build]
rustflags = ["-D", "warnings"]

[profile.bench]
# host tuning; see docs/design/rust/benchmark-harness.md
```

- [ ] **Step 5: Run `cargo build --workspace` — expect failure**

```bash
cd rust
cargo build --workspace
```

Expected: failure because no crates exist yet. This is the red step.

- [ ] **Step 6: Scaffold `tinyquant-core`**

Create `rust/crates/tinyquant-core/Cargo.toml`:

```toml
[package]
name = "tinyquant-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Ultra-high-performance vector quantization codec (no_std core)."

[dependencies]
thiserror = { workspace = true }
half = { workspace = true }
sha2 = { workspace = true }
hex = { workspace = true }

[features]
default = []
std = []
simd = ["std"]
```

Create `rust/crates/tinyquant-core/src/lib.rs`:

```rust
#![no_std]
#![deny(
    warnings,
    missing_docs,
    unsafe_op_in_unsafe_fn,
    clippy::all,
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
)]
#![allow(clippy::module_name_repetitions, clippy::must_use_candidate)]

//! Core codec, corpus, and backend protocol types for the TinyQuant
//! Rust port. See `docs/design/rust/crate-topology.md` for the layout.

extern crate alloc;

pub mod prelude;
pub mod types;
```

Create `rust/crates/tinyquant-core/src/prelude.rs`:

```rust
//! Convenience re-exports. Populated in subsequent phases.
```

Create `rust/crates/tinyquant-core/src/types.rs`:

```rust
//! Shared type aliases. Populated in phase 12.
```

Create `rust/crates/tinyquant-core/tests/smoke.rs`:

```rust
//! Smoke test: crate compiles and imports cleanly.

#[test]
fn crate_compiles() {
    // Intentionally trivial. This test exists so that phase 11's
    // failing red step produces a concrete target for phase 12 to
    // replace with real behavior.
    let _ = core::mem::size_of::<usize>();
}
```

- [ ] **Step 7: Run `cargo build -p tinyquant-core`** — expect success.

```bash
cd rust
cargo build -p tinyquant-core
```

- [ ] **Step 8: Run `cargo test -p tinyquant-core`** — expect success.

```bash
cargo test -p tinyquant-core
```

Expected: `1 passed`.

- [ ] **Step 9: Scaffold the remaining six library crates**

Repeat step 6 for each of:

- `tinyquant-io` (depends on `tinyquant-core`, feature `simd` enabled by default)
- `tinyquant-bruteforce` (depends on `tinyquant-core`)
- `tinyquant-pgvector` (depends on `tinyquant-core`; no default features)
- `tinyquant-sys` (depends on `tinyquant-core`, `tinyquant-io`, `tinyquant-bruteforce`; `crate-type = ["cdylib", "staticlib", "rlib"]`)
- `tinyquant-py` (depends on `tinyquant-core`, `tinyquant-io`, `tinyquant-bruteforce`; `crate-type = ["cdylib"]`; pyo3 + numpy workspace deps)
- `tinyquant-bench` (`publish = false`, criterion dep)
- `tinyquant-fuzz` (`publish = false`, libfuzzer-sys dep; target-only)

Each crate gets a `src/lib.rs` with the same lint wall header (with
`#![no_std]` removed for the `std`-only crates) and a
`tests/smoke.rs` with a single passing test.

- [ ] **Step 10: Scaffold `xtask`**

`rust/xtask/Cargo.toml`:

```toml
[package]
name = "xtask"
version.workspace = true
edition.workspace = true
publish = false

[[bin]]
name = "xtask"
path = "src/main.rs"
```

`rust/xtask/src/main.rs`:

```rust
fn main() {
    let cmd = std::env::args().nth(1);
    match cmd.as_deref() {
        Some("help") | None => {
            println!("xtask: TinyQuant Rust workspace tasks");
            println!();
            println!("Usage: cargo xtask <command>");
            println!();
            println!("Commands:");
            println!("  help      Show this message");
            println!("  fmt       cargo fmt --all");
            println!("  lint      cargo clippy --workspace --all-targets -- -D warnings");
            println!("  test      cargo test --workspace");
        }
        Some("fmt") => run("cargo", &["fmt", "--all"]),
        Some("lint") => run("cargo", &[
            "clippy", "--workspace", "--all-targets", "--all-features",
            "--", "-D", "warnings",
        ]),
        Some("test") => run("cargo", &["test", "--workspace", "--all-features"]),
        Some(other) => {
            eprintln!("unknown xtask command: {other}");
            std::process::exit(2);
        }
    }
}

fn run(program: &str, args: &[&str]) {
    let status = std::process::Command::new(program)
        .args(args)
        .status()
        .expect("failed to spawn");
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}
```

- [ ] **Step 11: Run full workspace build**

```bash
cd rust
cargo build --workspace
```

Expected: all crates build. Add a `.cargo/alias` so that
`cargo xtask` is available globally.

- [ ] **Step 12: Run full workspace tests**

```bash
cargo test --workspace
```

Expected: one smoke test per crate passes. Total should be around
`8 passed; 0 failed`.

- [ ] **Step 13: Run `cargo fmt --all`**

```bash
cargo fmt --all
```

Expected: no diff (we wrote already-formatted code).

- [ ] **Step 14: Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`**

Expected: no warnings. If there are, fix them — no suppressions.

- [ ] **Step 15: `no_std` build smoke check**

```bash
rustup target add thumbv7em-none-eabihf
cargo build -p tinyquant-core --no-default-features --target thumbv7em-none-eabihf
```

Expected: `tinyquant-core` builds for the Cortex-M4F target.

- [ ] **Step 16: Write `.github/workflows/rust-ci.yml`**

```yaml
name: rust-ci

on:
  pull_request:
    paths:
      - 'rust/**'
      - '.github/workflows/rust-ci.yml'
      - 'docs/design/rust/**'
      - 'docs/plans/rust/**'
  push:
    branches: [main]
    paths:
      - 'rust/**'
      - '.github/workflows/rust-ci.yml'

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  check:
    name: fmt + clippy + test
    runs-on: ubuntu-22.04
    defaults:
      run:
        working-directory: rust
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.78.0
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: fmt
        run: cargo fmt --all -- --check
      - name: clippy
        run: cargo clippy --workspace --all-targets --all-features -- -D warnings
      - name: test
        run: cargo test --workspace --all-features
      - name: no_std build
        run: |
          rustup target add thumbv7em-none-eabihf
          cargo build -p tinyquant-core --no-default-features --target thumbv7em-none-eabihf
```

- [ ] **Step 17: Commit**

```bash
git add rust/ .github/workflows/rust-ci.yml docs/design/rust docs/plans/rust docs/roadmap.md
git commit -m "feat(rust): scaffold Cargo workspace and rust-ci workflow"
```

## Acceptance criteria

- `cargo build --workspace` succeeds on Linux x86_64.
- `cargo test --workspace` reports one passing test per library crate.
- `cargo fmt --all -- --check` has no diff.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  produces no warnings.
- `cargo build -p tinyquant-core --no-default-features --target thumbv7em-none-eabihf`
  succeeds.
- `.github/workflows/rust-ci.yml` exists and is syntactically valid YAML.
- No file outside `rust/`, `.github/workflows/rust-ci.yml`, `docs/`
  has been modified.

## Out of scope

- Any codec logic.
- SIMD kernels.
- pyo3 bindings beyond the stub.
- Actual benchmarks (criterion smoke target is a placeholder).

## See also

- [[plans/rust/phase-12-shared-types-and-errors|Phase 12]]
- [[design/rust/crate-topology|Crate Topology]]
- [[design/rust/ci-cd|CI/CD]]
