---
title: "Phase 22: Pyo3 Binding, C ABI, Standalone Binary, and Multi-arch Release"
tags:
  - plans
  - rust
  - phase-22
  - pyo3
  - c-abi
  - cli
  - release
date-created: 2026-04-10
status: draft
category: planning
---

# Phase 22: Pyo3 Binding, C ABI, Standalone Binary, and Multi-arch Release

> [!info] Goal
> Land the final release surface for the Rust port:
>
> 1. `tinyquant-py` pyo3 wheel with full behavioral parity against
>    `tinyquant_cpu`.
> 2. `tinyquant-sys` C ABI (`cdylib` + `staticlib`) with committed
>    header.
> 3. **A new `tinyquant-cli` crate that produces the `tinyquant`
>    standalone binary**, shipping Cargo library crates and a
>    stand-alone executable in lockstep.
> 4. Cross-compiled release artifacts for **every Tier-1 target**
>    (Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64,
>    Windows x86_64) plus Tier-2 `i686-pc-windows-msvc` and
>    `x86_64-unknown-freebsd`.
> 5. Tag-triggered `rust-release.yml` workflow that publishes
>    wheels, crates, container image, and binary archives.

> [!note] Reference docs
> - [[design/rust/ffi-and-bindings|FFI and Bindings]]
> - [[design/rust/release-strategy|Release and Versioning]]
> - [[design/rust/feature-flags|Feature Flags]]
> - [[design/rust/ci-cd|CI/CD]]

## Prerequisites

- Phase 21 complete (calibration + bench gates).
- crates.io trusted publisher configured for the repo.
- PyPI OIDC configured.
- GHCR container permissions granted to the workflow.

## Deliverables overview

This phase ships **three binding products** and **one first-party
executable**, plus a release pipeline that builds every combination
in one workflow run.

| Product | Artifact type | Distribution | Consumers |
|---|---|---|---|
| `tinyquant-core` et al. | `rlib` on crates.io | `cargo add tinyquant-core` | Rust library users |
| `tinyquant-sys` | `cdylib`, `staticlib`, `tinyquant.h` | crates.io + GitHub release assets | C/C++/FFI consumers |
| `tinyquant-py` | Python wheel (`tinyquant_rs`) | PyPI | Python users |
| **`tinyquant-cli`** (new) | **Standalone binary `tinyquant` / `tinyquant.exe`** | **GitHub Releases per target + ghcr.io container** | **Operators, CI jobs, shell scripts** |

### Files to create

| File | Purpose |
|------|---------|
| `rust/crates/tinyquant-py/...` | See the [[design/rust/ffi-and-bindings|FFI]] doc |
| `rust/crates/tinyquant-sys/...` | See the [[design/rust/ffi-and-bindings|FFI]] doc |
| `rust/crates/tinyquant-cli/Cargo.toml` | **New** binary crate |
| `rust/crates/tinyquant-cli/src/main.rs` | CLI entry point |
| `rust/crates/tinyquant-cli/src/commands/mod.rs` | Subcommand router |
| `rust/crates/tinyquant-cli/src/commands/codec_train.rs` | `tinyquant codec train` |
| `rust/crates/tinyquant-cli/src/commands/codec_compress.rs` | `tinyquant codec compress` |
| `rust/crates/tinyquant-cli/src/commands/codec_decompress.rs` | `tinyquant codec decompress` |
| `rust/crates/tinyquant-cli/src/commands/corpus_ingest.rs` | `tinyquant corpus ingest` |
| `rust/crates/tinyquant-cli/src/commands/corpus_search.rs` | `tinyquant corpus search` (via brute force) |
| `rust/crates/tinyquant-cli/src/commands/info.rs` | `tinyquant info` (version, ISA, features) |
| `rust/crates/tinyquant-cli/src/commands/verify.rs` | `tinyquant verify` (file integrity) |
| `rust/crates/tinyquant-cli/src/io.rs` | Input parsers (CSV, NumPy `.npy`, binary f32) |
| `rust/crates/tinyquant-cli/tests/cli_smoke.rs` | `assert_cmd`-based e2e tests |
| `rust/crates/tinyquant-cli/README.md` | User-facing CLI docs |
| `.github/workflows/rust-release.yml` | Tag-triggered workflow |
| `rust/Dockerfile` | Multi-stage build for the CLI container image |
| `rust/.dockerignore` | Excludes `target/`, test fixtures |

## Steps (TDD order)

### Part A — `tinyquant-py` pyo3 wheel

- [ ] **Step 1: Failing parity test**

```python
# tinyquant-py/tests/python/test_parity.py
import numpy as np
import pytest
import tinyquant_cpu as py
import tinyquant_rs as rs

def test_config_hash_parity():
    for bw, seed, dim in [(4, 42, 768), (2, 0, 64), (8, 999, 1536)]:
        py_cfg = py.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        assert py_cfg.config_hash == rs_cfg.config_hash
```

- [ ] **Step 2: Implement `src/codec.rs`, `src/corpus.rs`,
      `src/backend.rs`, `src/errors.rs`, `src/numpy_bridge.rs`**

Follow [[design/rust/ffi-and-bindings|FFI and Bindings]] §Binding 1.
Key points:

- `#[pyclass(frozen, eq, hash)]` on value objects.
- GIL release via `py.allow_threads(...)` on batch methods.
- NumPy zero-copy views with `PyReadonlyArray1` / `PyReadonlyArray2`.
- Exception classes created with `create_exception!` matching the
  Python package names.

- [ ] **Step 3: `python/tinyquant_rs/__init__.py`**

Re-exports every public name under the sub-packages
`tinyquant_rs.codec`, `tinyquant_rs.corpus`,
`tinyquant_rs.backend` so that downstream code can swap the import.

- [ ] **Step 4: `pyproject.toml`**

```toml
[build-system]
requires = ["maturin>=1.5"]
build-backend = "maturin"

[project]
name = "tinyquant-rs"
version = "0.1.0"
description = "Ultra-high-performance Rust port of TinyQuant, the CPU-only vector quantization codec."
requires-python = ">=3.12"
license = { text = "Apache-2.0" }
classifiers = [
    "Programming Language :: Python :: 3",
    "Programming Language :: Rust",
    "License :: OSI Approved :: Apache Software License",
    "Operating System :: POSIX :: Linux",
    "Operating System :: MacOS",
    "Operating System :: Microsoft :: Windows",
]

[tool.maturin]
python-source = "python"
module-name = "tinyquant_rs._core"
bindings = "pyo3"
features = ["pyo3/extension-module", "pyo3/abi3-py312"]
strip = true
```

- [ ] **Step 5: Build and smoke-test the wheel**

```bash
cd rust/crates/tinyquant-py
maturin develop --release
python -c "import tinyquant_rs; print(tinyquant_rs.__version__)"
pytest tests/python/test_parity.py -v
```

- [ ] **Step 6: Expand parity suite** to cover:

- `Codebook.train` byte parity.
- `CompressedVector.to_bytes` byte parity.
- `Corpus.insert` + `decompress` round trip.
- Exception class names and hierarchies.
- Batch methods: `compress_batch`, `decompress_batch`.
- NumPy buffer protocol / zero-copy verification.

### Part B — `tinyquant-sys` C ABI

- [ ] **Step 7: Failing C ABI test**

```rust
// tinyquant-sys/tests/abi_smoke.rs
#[test]
fn config_handle_round_trip() {
    let mut handle: *mut CodecConfigHandle = std::ptr::null_mut();
    let mut err = TinyQuantError::default();
    let kind = unsafe {
        tq_codec_config_new(4, 42, 768, true, &mut handle, &mut err)
    };
    assert_eq!(kind, TinyQuantErrorKind::Ok);
    assert!(!handle.is_null());
    unsafe {
        assert_eq!(tq_codec_config_bit_width(handle), 4);
        tq_codec_config_free(handle);
    }
}
```

- [ ] **Step 8: Implement `src/lib.rs`, `src/handle.rs`,
      `src/codec_abi.rs`, `src/corpus_abi.rs`, `src/error.rs`**

Follow [[design/rust/ffi-and-bindings|FFI and Bindings]] §Binding 2.
Each `extern "C" fn` wraps its body in `catch_unwind`.

- [ ] **Step 9: `build.rs` runs cbindgen**

Writes `include/tinyquant.h`. CI fails on header drift via
`git diff --exit-code rust/crates/tinyquant-sys/include/`.

- [ ] **Step 10: C consumer smoke test**

```bash
# rust/crates/tinyquant-sys/tests/c/smoke.c
#include "../../include/tinyquant.h"
int main(void) {
    TinyQuantError err = {0};
    CodecConfigHandle* cfg = NULL;
    if (tq_codec_config_new(4, 42, 768, true, &cfg, &err) != TQ_OK) return 1;
    if (tq_codec_config_bit_width(cfg) != 4) return 2;
    tq_codec_config_free(cfg);
    return 0;
}
```

A `build.rs`-generated `tests/abi_c_smoke.rs` compiles this with
`cc` crate at test time and runs it.

### Part C — **`tinyquant-cli` standalone binary (CRITICAL)**

- [ ] **Step 11: Scaffold `rust/crates/tinyquant-cli/Cargo.toml`**

```toml
[package]
name = "tinyquant-cli"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "CLI front-end for the TinyQuant codec and corpus tooling."
readme = "README.md"
default-run = "tinyquant"

[[bin]]
name = "tinyquant"
path = "src/main.rs"

[dependencies]
tinyquant-core = { path = "../tinyquant-core", version = "0.1.0", features = ["std", "simd"] }
tinyquant-io   = { path = "../tinyquant-io",   version = "0.1.0", features = ["simd", "mmap", "rayon"] }
tinyquant-bruteforce = { path = "../tinyquant-bruteforce", version = "0.1.0", features = ["simd"] }
clap = { version = "4", features = ["derive", "wrap_help"] }
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rayon = "1"
indicatif = "0.17"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }

[features]
default = ["jemalloc"]
jemalloc = ["dep:jemallocator"]

[target.'cfg(not(target_env = "msvc"))'.dependencies]
jemallocator = { version = "0.5", optional = true }
```

- [ ] **Step 12: Failing CLI smoke test**

```rust
// rust/crates/tinyquant-cli/tests/cli_smoke.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn info_subcommand_prints_version() {
    Command::cargo_bin("tinyquant")
        .unwrap()
        .arg("info")
        .assert()
        .success()
        .stdout(predicate::str::contains("tinyquant"))
        .stdout(predicate::str::contains("version"));
}

#[test]
fn codec_train_writes_codebook_file(tmp: tempfile::TempDir) {
    let training = tmp.path().join("training.f32.bin");
    write_random_f32_bin(&training, 1000, 64);
    let codebook = tmp.path().join("codebook.bin");

    Command::cargo_bin("tinyquant")
        .unwrap()
        .args([
            "codec", "train",
            "--input", training.to_str().unwrap(),
            "--rows", "1000",
            "--cols", "64",
            "--bit-width", "4",
            "--seed", "42",
            "--output", codebook.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(codebook.exists());
}

#[test]
fn codec_compress_decompress_round_trip_via_cli(tmp: tempfile::TempDir) {
    // 1. tinyquant codec train
    // 2. tinyquant codec compress ... --output compressed.bin
    // 3. tinyquant codec decompress compressed.bin --output recovered.f32.bin
    // 4. assert MSE < 1e-2
}
```

- [ ] **Step 13: Implement `src/main.rs` with clap derive**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "tinyquant",
    version,
    about = "CPU-only vector quantization codec",
    long_about = None,
)]
struct Cli {
    /// Log filter (e.g., `info`, `tinyquant_cli=debug`).
    #[arg(long, env = "TINYQUANT_LOG", default_value = "info")]
    log: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print build metadata, feature set, and detected ISA.
    Info,

    /// Codec subcommands (train, compress, decompress, verify).
    #[command(subcommand)]
    Codec(CodecCmd),

    /// Corpus subcommands (ingest, decompress, search via brute force).
    #[command(subcommand)]
    Corpus(CorpusCmd),

    /// Verify a serialized CompressedVector or corpus file.
    Verify { path: std::path::PathBuf },
}

#[derive(Subcommand)]
enum CodecCmd {
    /// Train a codebook from raw FP32 input.
    Train {
        #[arg(long)] input: std::path::PathBuf,
        #[arg(long)] rows: usize,
        #[arg(long)] cols: usize,
        #[arg(long)] bit_width: u8,
        #[arg(long)] seed: u64,
        #[arg(long, default_value_t = true)] residual: bool,
        #[arg(long)] output: std::path::PathBuf,
    },
    /// Compress one or many vectors into a corpus file.
    Compress {
        #[arg(long)] input: std::path::PathBuf,
        #[arg(long)] config_json: std::path::PathBuf,
        #[arg(long)] codebook: std::path::PathBuf,
        #[arg(long)] output: std::path::PathBuf,
        #[arg(long, default_value_t = num_cpus::get())] threads: usize,
    },
    /// Decompress a corpus file back to raw FP32.
    Decompress {
        #[arg(long)] input: std::path::PathBuf,
        #[arg(long)] config_json: std::path::PathBuf,
        #[arg(long)] codebook: std::path::PathBuf,
        #[arg(long)] output: std::path::PathBuf,
    },
}

#[derive(Subcommand)]
enum CorpusCmd {
    /// Ingest an FP32 matrix into a new corpus file.
    Ingest { /* … */ },
    /// Decompress every vector in a corpus file to a raw FP32 matrix.
    Decompress { /* … */ },
    /// Brute-force search a query vector against a corpus file.
    Search {
        #[arg(long)] corpus: std::path::PathBuf,
        #[arg(long)] query: std::path::PathBuf,
        #[arg(long, default_value_t = 10)] top_k: usize,
        #[arg(long, default_value = "json")] format: OutputFormat,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum OutputFormat { Json, Csv, Table }

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log)?;
    match cli.command {
        Command::Info => commands::info::run(),
        Command::Codec(c) => commands::codec::dispatch(c),
        Command::Corpus(c) => commands::corpus::dispatch(c),
        Command::Verify { path } => commands::verify::run(&path),
    }
}
```

- [ ] **Step 14: Implement the subcommand modules**

Each subcommand:

- Reads inputs via `tinyquant-io` (zero-copy when possible).
- Uses `indicatif::ProgressBar` on long-running batches.
- Uses rayon for batch compress/decompress, honoring `--threads`.
- Writes outputs via `tinyquant-io::codec_file::CodecFileWriter`.
- Prints structured logs via `tracing`.
- Returns exit code 0 on success, non-zero on error with a clear
  message (`anyhow::Result` + `main` sugar).

- [ ] **Step 15: Implement `commands::info`**

```rust
pub fn run() -> anyhow::Result<()> {
    println!("tinyquant {}", env!("CARGO_PKG_VERSION"));
    println!("git commit:   {}", option_env!("GIT_COMMIT").unwrap_or("unknown"));
    println!("rustc:        {}", option_env!("RUSTC_VERSION").unwrap_or("unknown"));
    println!("target:       {}", env!("TARGET"));
    println!("profile:      {}", env!("PROFILE"));
    println!("features:     {}", enabled_features());
    println!("detected isa: {:?}", tinyquant_core::codec::dispatch::current());
    println!("cpu count:    {}", num_cpus::get());
    Ok(())
}
```

A `build.rs` script captures `GIT_COMMIT`, `RUSTC_VERSION`,
`TARGET`, `PROFILE` from the environment and emits them via
`cargo:rustc-env=`.

- [ ] **Step 16: Run CLI smoke tests — expect pass**

```bash
cd rust
cargo test -p tinyquant-cli
```

- [ ] **Step 17: Man page and shell completion generation**

Add a `--generate-completion {bash,zsh,fish,powershell}` flag that
uses `clap_complete` to emit completions to stdout. Add a
`--generate-man` flag using `clap_mangen`.

Release workflow calls both during cross-compilation and bundles
the outputs into the release archive.

### Part D — Cross-architecture release pipeline

- [ ] **Step 18: Dockerfile for CLI container image**

```dockerfile
# rust/Dockerfile
FROM rust:1.78.0-bookworm AS builder
WORKDIR /build
COPY rust /build/rust
WORKDIR /build/rust
RUN cargo build -p tinyquant-cli --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
COPY --from=builder /build/rust/target/release/tinyquant /usr/local/bin/tinyquant
USER nonroot
ENTRYPOINT ["/usr/local/bin/tinyquant"]
CMD ["info"]
```

- [ ] **Step 19: Write `rust-release.yml`**

```yaml
name: rust-release

on:
  push:
    tags:
      - 'rust-v*.*.*'

permissions:
  contents: write
  id-token: write
  packages: write

env:
  CARGO_TERM_COLOR: always

jobs:
  # --- Stage 1: re-run the per-PR gate on the tagged commit --------------
  gate:
    uses: ./.github/workflows/rust-ci.yml

  # --- Stage 2: cross-build wheels, CLI binaries, and the static lib -----
  build:
    needs: gate
    strategy:
      fail-fast: false
      matrix:
        include:
          - { target: x86_64-unknown-linux-gnu,    os: ubuntu-22.04,      cross: false }
          - { target: aarch64-unknown-linux-gnu,   os: ubuntu-22.04,      cross: true  }
          - { target: x86_64-unknown-linux-musl,   os: ubuntu-22.04,      cross: true  }
          - { target: aarch64-unknown-linux-musl,  os: ubuntu-22.04,      cross: true  }
          - { target: x86_64-apple-darwin,         os: macos-14,          cross: false }
          - { target: aarch64-apple-darwin,        os: macos-14,          cross: false }
          - { target: x86_64-pc-windows-msvc,      os: windows-2022,      cross: false }
          - { target: i686-pc-windows-msvc,        os: windows-2022,      cross: false }
          - { target: x86_64-unknown-freebsd,      os: ubuntu-22.04,      cross: true  }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
        with: { lfs: true }
      - uses: dtolnay/rust-toolchain@stable
        with: { toolchain: 1.78.0, targets: ${{ matrix.target }} }
      - uses: Swatinem/rust-cache@v2
      - name: Install cross (linux/bsd)
        if: matrix.cross
        run: cargo install cross --locked
      - name: Build tinyquant-cli (standalone binary)
        working-directory: rust
        shell: bash
        run: |
          if [ "${{ matrix.cross }}" = "true" ]; then
            cross build -p tinyquant-cli --release --target ${{ matrix.target }} --locked
          else
            cargo build -p tinyquant-cli --release --target ${{ matrix.target }} --locked
          fi
      - name: Build tinyquant-sys (cdylib + staticlib)
        working-directory: rust
        shell: bash
        run: |
          if [ "${{ matrix.cross }}" = "true" ]; then
            cross build -p tinyquant-sys --release --target ${{ matrix.target }} --locked
          else
            cargo build -p tinyquant-sys --release --target ${{ matrix.target }} --locked
          fi
      - name: Build tinyquant-py wheel
        if: matrix.target != 'x86_64-unknown-freebsd'
        uses: PyO3/maturin-action@v1
        with:
          target: ${{ matrix.target }}
          working-directory: rust/crates/tinyquant-py
          args: --release --strip --out dist
          manylinux: auto
      - name: Package CLI archive
        shell: bash
        run: |
          cd rust/target/${{ matrix.target }}/release
          if [[ "${{ matrix.target }}" == *windows* ]]; then
            ARCHIVE="tinyquant-${{ github.ref_name }}-${{ matrix.target }}.zip"
            7z a "$ARCHIVE" tinyquant.exe
          else
            ARCHIVE="tinyquant-${{ github.ref_name }}-${{ matrix.target }}.tar.gz"
            tar czf "$ARCHIVE" tinyquant
          fi
          mv "$ARCHIVE" $GITHUB_WORKSPACE/
      - name: Upload CLI artifact
        uses: actions/upload-artifact@v4
        with:
          name: tinyquant-cli-${{ matrix.target }}
          path: tinyquant-${{ github.ref_name }}-${{ matrix.target }}.*
      - name: Upload wheel artifact
        if: matrix.target != 'x86_64-unknown-freebsd'
        uses: actions/upload-artifact@v4
        with:
          name: wheels-${{ matrix.target }}
          path: rust/crates/tinyquant-py/dist/*.whl
      - name: Upload staticlib + header
        uses: actions/upload-artifact@v4
        with:
          name: sys-${{ matrix.target }}
          path: |
            rust/target/${{ matrix.target }}/release/libtinyquant.*
            rust/target/${{ matrix.target }}/release/tinyquant.dll
            rust/crates/tinyquant-sys/include/tinyquant.h

  # --- Stage 3: publish crates.io (upstream → downstream order) ----------
  publish-crates:
    needs: build
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { toolchain: 1.78.0 }
      - name: Publish crates (ordered)
        working-directory: rust
        run: |
          cargo publish -p tinyquant-core
          cargo publish -p tinyquant-io
          cargo publish -p tinyquant-bruteforce
          cargo publish -p tinyquant-pgvector
          cargo publish -p tinyquant-sys
          cargo publish -p tinyquant-cli

  # --- Stage 4: publish Python wheels to PyPI ----------------------------
  publish-pypi:
    needs: build
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/download-artifact@v4
        with:
          pattern: wheels-*
          merge-multiple: true
          path: dist/
      - uses: pypa/gh-action-pypi-publish@release/v1
        with:
          packages-dir: dist/

  # --- Stage 5: build + push multi-arch CLI container -------------------
  publish-container:
    needs: build
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - uses: docker/setup-qemu-action@v3
      - uses: docker/setup-buildx-action@v3
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - name: Build and push
        uses: docker/build-push-action@v6
        with:
          context: .
          file: rust/Dockerfile
          platforms: linux/amd64,linux/arm64
          tags: |
            ghcr.io/better-with-models/tinyquant:${{ github.ref_name }}
            ghcr.io/better-with-models/tinyquant:latest
          push: true

  # --- Stage 6: assemble GitHub release ---------------------------------
  publish-release:
    needs: [build, publish-crates, publish-pypi, publish-container]
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/download-artifact@v4
        with: { pattern: tinyquant-cli-*, merge-multiple: true, path: cli/ }
      - uses: actions/download-artifact@v4
        with: { pattern: sys-*, merge-multiple: true, path: sys/ }
      - name: Generate release notes
        run: |
          echo "# Release ${{ github.ref_name }}" > RELEASE_NOTES.md
          # concatenated from CHANGELOG.md [Unreleased] section
      - uses: softprops/action-gh-release@v2
        with:
          files: |
            cli/*.tar.gz
            cli/*.zip
            sys/libtinyquant.*
            sys/tinyquant.dll
            sys/tinyquant.h
          body_path: RELEASE_NOTES.md
```

- [ ] **Step 20: Dry-run the release workflow**

```bash
# Locally
cargo publish --dry-run -p tinyquant-core
# And push a pre-release tag
git tag rust-v0.1.0-alpha.1 && git push origin rust-v0.1.0-alpha.1
```

Verify:

1. Every matrix target produces a CLI archive.
2. Every supported target produces a wheel.
3. Every target produces a static lib + header pair.
4. `ghcr.io/better-with-models/tinyquant:0.1.0-alpha.1` pulls
   and runs (`docker run ghcr.io/.../tinyquant:... info`).

- [ ] **Step 21: Post-release parity verification**

```bash
# Install from PyPI
python -m venv .venv && .venv/bin/pip install tinyquant-rs tinyquant-cpu
.venv/bin/pytest rust/crates/tinyquant-py/tests/python/test_parity.py

# Install CLI
curl -L https://github.com/better-with-models/TinyQuant/releases/download/rust-v0.1.0/tinyquant-rust-v0.1.0-x86_64-unknown-linux-gnu.tar.gz | tar xz
./tinyquant info
```

- [ ] **Step 22: Final commit + tag**

```bash
git add rust/crates/tinyquant-py rust/crates/tinyquant-sys \
        rust/crates/tinyquant-cli rust/Dockerfile rust/.dockerignore \
        .github/workflows/rust-release.yml
git commit -m "feat(release): pyo3 binding, C ABI, CLI binary, and multi-arch release"
git tag rust-v0.1.0
git push origin main rust-v0.1.0
```

## Acceptance criteria

- `tinyquant-rs` wheel installs cleanly on Linux x86_64/aarch64,
  macOS x86_64/aarch64, Windows x86_64 and passes the full parity
  suite against `tinyquant-cpu`.
- `tinyquant-sys` produces `libtinyquant.{so,dylib}`,
  `tinyquant.dll`, and a matching `libtinyquant.a` /
  `tinyquant.lib` plus `tinyquant.h` for every Tier-1 and Tier-2
  target.
- **`tinyquant-cli` produces a standalone `tinyquant` / `tinyquant.exe`
  binary for every matrix target and the smoke tests
  (`info`, `codec train`, `codec compress`, `codec decompress`,
  `verify`) pass on Linux x86_64/aarch64, macOS x86_64/aarch64, and
  Windows x86_64.**
- The multi-arch container image
  `ghcr.io/better-with-models/tinyquant:rust-v0.1.0` runs on both
  `linux/amd64` and `linux/arm64`.
- crates.io publishes all six library crates plus the CLI.
- `rust-release.yml` passes end-to-end on a tagged commit.
- `COMPATIBILITY.md` at the repo root lists the supported
  `(tinyquant_cpu, tinyquant_rs)` pairs and any known drift.
- Every performance goal from
  [[design/rust/goals-and-non-goals|Goals and Non-Goals]] is met or
  has an explicit risk ticket.
- Clippy + fmt clean across the full workspace.

## Out of scope

- JS/WASM bindings (reserved for a later release).
- iOS/Android wheels.
- AVX-512 specialization (Phase 23+).
- Async CLI or HTTP server mode.

## Multi-architecture release matrix summary

| Target triple | Library artifact | Standalone binary | Python wheel | Container |
|---|---|---|---|---|
| `x86_64-unknown-linux-gnu` | ✅ rlib + cdylib + staticlib | ✅ `tinyquant` | ✅ `tinyquant_rs` | ✅ linux/amd64 |
| `aarch64-unknown-linux-gnu` | ✅ | ✅ | ✅ | ✅ linux/arm64 |
| `x86_64-unknown-linux-musl` | ✅ | ✅ static | — | — |
| `aarch64-unknown-linux-musl` | ✅ | ✅ static | — | — |
| `x86_64-apple-darwin` | ✅ | ✅ | ✅ | — |
| `aarch64-apple-darwin` | ✅ | ✅ | ✅ | — |
| `x86_64-pc-windows-msvc` | ✅ | ✅ `tinyquant.exe` | ✅ | — |
| `i686-pc-windows-msvc` | ✅ | ✅ `tinyquant.exe` | — | — |
| `x86_64-unknown-freebsd` | ✅ | ✅ | — | — |

Every row of this matrix is built on every tagged release; missing
rows represent explicit non-goals (documented in
[[design/rust/release-strategy|Release Strategy]]).

## See also

- [[plans/rust/phase-21-rayon-batch-benches|Phase 21]]
- [[design/rust/ffi-and-bindings|FFI and Bindings]]
- [[design/rust/release-strategy|Release and Versioning]]
- [[design/rust/ci-cd|CI/CD]]
- [[roadmap|Implementation Roadmap]]
