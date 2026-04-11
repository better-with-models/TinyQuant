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
| `COMPATIBILITY.md` | Repo-root compatibility matrix (see subsection below) |

### CLI I/O format specification

The CLI must accept multiple input representations for the same
logical data (an `f32` matrix) and emit results in whichever format
a downstream pipeline consumes. Every subcommand that takes a vector
matrix uses `--format` to pick the parser, and every subcommand that
emits vector data uses `--format` to pick the writer.

| Format | CLI flag | Extension | Parser / writer | Notes |
|---|---|---|---|---|
| Raw f32 binary | `--format f32` (default for `codec train`) | `.f32.bin` | `tinyquant-io::raw_f32` | Little-endian, row-major, **no header**; `--rows` and `--cols` are required |
| NumPy `.npy` v1.0 | `--format npy` | `.npy` | `npyz` crate | Validate dtype `== float32`, shape is 2-D; **reject** object dtypes, structured dtypes, and Fortran-order arrays with exit code 2 |
| CSV | `--format csv` | `.csv` | `csv` crate | `--delimiter ,` (default), `--has-header` (default on); one vector per row; reject non-numeric cells with `CSV parse error at row N, column M: '<cell>'` |
| JSONL | `--format jsonl` | `.jsonl` | `serde_json::Deserializer::from_reader` | One JSON object per line: `{"id": "...", "vector": [f32; dim], "metadata": {...}}`; used by both `corpus ingest` and `corpus search` input |
| JSON (output) | `--format json` (default for `corpus search`) | `.json` | `serde_json` | Canonical: `{"results": [{"id": "...", "score": 0.87}, ...]}`; default for search results |
| Table (output) | `--format table` | n/a | `comfy-table` | Human-readable ASCII table; respects `NO_COLOR` |
| CSV (output) | `--format csv` | `.csv` | `csv` crate | Shell-pipeline friendly; header row is always emitted |

Round-trip matrix (a check run by the `cli_smoke.rs` integration
tests — every combination is exercised at least once):

| Input format | Output format | Round-trip loss | Notes |
|---|---|---|---|
| `f32` raw | `f32` raw | 0 bytes (bit-exact via compress→decompress against MSE tolerance) | The canonical round trip; `MSE < 1e-2` on 10 000-vector sweep |
| `.npy` | `.npy` | 0 bytes | `npyz` preserves dtype and shape; header is regenerated |
| csv | csv | Float-format preserved at `{:.9}` | Human-readable; round trip is lossy at the ASCII level but bit-stable after `f32::from_str` |
| jsonl | jsonl | Lossless at `serde_json::Number` precision | `id` and `metadata` passthrough verbatim |
| `.npy` | csv | Lossy on float text | One-way migration path |
| csv | `.npy` | 0 bytes | Ingestion from legacy pipelines |
| jsonl | `.npy` | 0 bytes | Drops `id` and `metadata`; emits a WARN log |
| `f32` raw | jsonl | Synthetic `id = "{row_index}"` | Upgrade path; emits INFO log |

All formats reject BOM markers, NaN cells (explicitly logged with row
index), and files whose row count does not match `--rows` when that
flag is supplied.

### CLI subcommand reference

Every subcommand is exhaustively specified here so the clap derive
layout in Step 13 is a mechanical translation. Exit codes follow the
existing [[design/rust/ffi-and-bindings|FFI and Bindings]] table in
§Binding 3.

| Command | Purpose | Required args | Optional args | Exit codes |
|---|---|---|---|---|
| `tinyquant info` | Print build metadata, enabled features, detected ISA, CPU count | none | none | 0 |
| `tinyquant codec train` | Train a codebook from raw FP32 input | `--input <PATH>`, `--rows <N>`, `--cols <N>`, `--bit-width <{2,4,8}>`, `--seed <u64>`, `--output <PATH>` | `--residual` (default on), `--format {f32,npy,csv}`, `--progress`, `--threads <N>` | 0, 2 (invalid args), 3 (IO), 70 (panic) |
| `tinyquant codec compress` | Compress one or many vectors into a corpus file | `--input <PATH>`, `--config-json <PATH>`, `--codebook <PATH>`, `--output <PATH>` | `--threads <N>`, `--format {f32,npy,csv,jsonl}`, `--progress` | 0, 2, 3, 70 |
| `tinyquant codec decompress` | Decompress a corpus file back to raw FP32 | `--input <PATH>`, `--config-json <PATH>`, `--codebook <PATH>`, `--output <PATH>` | `--threads <N>`, `--format {f32,npy,csv}` | 0, 2, 3, 70 |
| `tinyquant codec verify` | Validate magic, header, and per-vector checksums of a corpus file | `--input <PATH>` | none | 0, 4 (verify failed), 3, 70 |
| `tinyquant corpus ingest` | Ingest raw FP32 vectors into a new corpus file with a policy | `--input <PATH>`, `--config-json <PATH>`, `--codebook <PATH>`, `--corpus-path <PATH>`, `--policy {compress,passthrough,fp16}` | `--threads <N>`, `--format`, `--progress` | 0, 2, 3, 70 |
| `tinyquant corpus decompress` | Decompress every vector in a corpus file to a raw FP32 matrix | `--corpus-path <PATH>`, `--output <PATH>` | `--format {f32,npy,csv}`, `--progress` | 0, 2, 3, 70 |
| `tinyquant corpus search` | Brute-force search a query vector against a corpus file | `--corpus <PATH>`, `--query <PATH>`, `--top-k <N>` | `--format {json,table,csv}` (default `json`) | 0, 2, 3, 70 |
| `tinyquant verify <PATH>` | Umbrella verifier that delegates by the first 8 magic bytes | `PATH` | none | 0, 4, 3, 70 |
| `tinyquant --generate-completion <SHELL>` | Emit `bash` / `zsh` / `fish` / `powershell` completions to stdout | `<SHELL>` | none | 0, 2 |
| `tinyquant --generate-man` | Emit the `clap_mangen` man page to stdout | none | none | 0 |

Error messages always follow the shape
`error: <category>: <detail> [at <path>:<line>]` and are printed to
stderr.  Progress bars are written to stderr, never stdout, so
`tinyquant ... --format json > out.json` stays clean.

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

#### Maturin wheel test matrix

Every release wheel is built and tested under a fixed target matrix.
The abi3 strategy means one wheel per OS+arch serves every Python
`>= 3.12`, so the matrix axis is smaller than the per-version
alternative but still has to be exercised end-to-end on each runner.

| Python | Platform target | Runner | manylinux / macOS min | Wheel tag |
|---|---|---|---|---|
| 3.12, 3.13 (via abi3) | `x86_64-unknown-linux-gnu` | `ubuntu-22.04` | `manylinux_2_17` (glibc >= 2.17) | `cp312-abi3-manylinux_2_17_x86_64` |
| 3.12, 3.13 (via abi3) | `aarch64-unknown-linux-gnu` | `ubuntu-22.04-arm` | `manylinux_2_28` | `cp312-abi3-manylinux_2_28_aarch64` |
| 3.12, 3.13 (via abi3) | `x86_64-apple-darwin` | `macos-13` | `>= 10.14` | `cp312-abi3-macosx_10_14_x86_64` |
| 3.12, 3.13 (via abi3) | `aarch64-apple-darwin` | `macos-14` | `>= 11.0` | `cp312-abi3-macosx_11_0_arm64` |
| 3.12, 3.13 (via abi3) | `x86_64-pc-windows-msvc` | `windows-2022` | `>= 10` | `cp312-abi3-win_amd64` |

Per-runner steps:

1. Build the wheel: `maturin build --release --strip --manylinux auto`
   (auditwheel runs under the hood for Linux and rewrites the tag).
2. Install into a fresh venv and run the parity suite:
   `python -m venv .venv && .venv/bin/pip install --force-reinstall target/wheels/tinyquant_rs-*.whl tinyquant-cpu && .venv/bin/pytest tests/python/test_parity.py`.
3. Inspect wheel contents: `python -m zipfile -l <wheel>` must show
   exactly `tinyquant_rs/_core.abi3.so` (or `.pyd` on Windows) plus
   the Python-source package files — and **nothing else** (no
   `.debug/`, no stray `.rs` sources, no test fixtures).
4. Confirm the abi3 import compatibility flag: the wheel file name
   must contain `abi3` and the `ELF` / `Mach-O` / `PE` binary must
   export `PyInit__core` with the limited API tag.

Key `[tool.maturin]` settings enforced by release:

```toml
[tool.maturin]
python-source = "python"
module-name = "tinyquant_rs._core"
bindings = "pyo3"
features = ["pyo3/extension-module", "pyo3/abi3-py312"]
strip = true
```

`strip = true` removes debuginfo from the built shared object so the
wheel payload stays under the ~5 MiB budget.

- [ ] **Step 6: Expand the parity suite**

The `tinyquant-py/tests/python/test_parity.py` suite must prove
behavioral parity with `tinyquant-cpu` at the byte level. The full
breakdown of test functions:

1. **`test_config_hash_parity`** — 20 `(bit_width, seed, dim)` tuples
   cross-checked: bit widths `{2, 4, 8}`, seeds
   `{0, 1, 42, 123, 999}`, dims `{64, 384, 768, 1536}` (intersected
   to 20 representative triples). Each tuple asserts
   `py_cfg.config_hash == rs_cfg.config_hash` and
   `hash(py_cfg) == hash(rs_cfg)` (not strict, but warning if differs).
2. **`test_codebook_train_parity`** — trains with the same seed and
   input buffer on both sides and asserts byte-identical
   `.entries` via `np.testing.assert_array_equal`. Repeats for
   bw `{2, 4, 8}` and dim `{64, 768, 1536}`.
3. **`test_compressed_vector_to_bytes_parity`** — compresses 50
   vectors drawn from `numpy.random.default_rng(7)` and asserts
   `py_cv.to_bytes() == rs_cv.to_bytes()` for every vector.
4. **`test_corpus_lifecycle_parity`** — creates an empty corpus on
   both sides, inserts a batch of 100 vectors, decompresses all,
   and asserts byte equality of the decompressed buffers.
5. **`test_exception_hierarchy`** — for every pyo3-exposed exception
   class in `tinyquant_rs.codec` / `.corpus` / `.backend`, assert the
   same `__name__` and the same `__mro__[1]` (base class) as the
   matching `tinyquant_cpu` class. Catches accidental renames.
6. **`test_batch_methods`** — `compress_batch` / `decompress_batch`
   byte parity at `n ∈ {1, 16, 256}` with randomized inputs.
7. **`test_numpy_zero_copy`** — pass a NumPy array into `compress`
   via `PyReadonlyArray2`, verify no copy via the data-pointer
   equality check pattern (capture `arr.ctypes.data` before and
   assert the Rust side did not reallocate — under
   `pyo3.allow_threads(False)` to keep the GIL for the check).
8. **`test_threading_safety`** — spawn 4 Python threads, run
   `compress_batch` concurrently with the GIL released from each
   thread, assert no panics, no `PyErr` leaks, and bit-identical
   outputs in every thread.

Each test asserts **strict byte equality** where possible; only
decompress paths that round-trip through quantization compare with
`assert_allclose(atol=0)` against the Python reference on the same
codebook. The full suite must finish in under 90 seconds on the
x86_64 wheel smoke job.

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
Every `extern "C" fn` wraps its body in
`std::panic::catch_unwind(AssertUnwindSafe(|| { ... }))` and
converts any panic payload to `TinyQuantError { kind: TQ_ERR_PANIC,
message: <captured> }`. Handles are declared as
`#[repr(C)] struct CodecConfigHandle { _private: [u8; 0] }` opaque
types so the C side can only pass pointers, never touch fields.
Errors are returned via out-pointer `TinyQuantError*` whose owned
`char*` is freed via `tq_error_free`. Because `tinyquant-sys` is
the only crate in the workspace that needs raw FFI, the
`#![forbid(unsafe_code)]` crate attribute is dropped **only** for
`src/codec_abi.rs` and `src/corpus_abi.rs` via narrowed
`#[allow(unsafe_code)]` on the module (see
[[design/rust/phase-14-implementation-notes|Phase 14 Lessons]] §L7
for the narrow-allow pattern).

- [ ] **Step 9: `build.rs` runs cbindgen**

Writes `include/tinyquant.h` on every `cargo build -p tinyquant-sys`.
CI fails on header drift via
`git diff --exit-code rust/crates/tinyquant-sys/include/tinyquant.h`.

#### cbindgen drift guard

The cbindgen output is load-bearing for every C consumer, so we
treat the generated header as a committed source file, not a build
artifact.

- The `build.rs` script regenerates `include/tinyquant.h` on every
  `cargo build -p tinyquant-sys` (release or debug) and writes the
  file in-place. `cargo:rerun-if-changed=src` ensures the build
  re-runs when any Rust source changes.
- CI job `rust-ci/abi-header-drift` runs
  `cargo build -p tinyquant-sys --release` followed by
  `git diff --exit-code rust/crates/tinyquant-sys/include/tinyquant.h`.
  A non-zero diff means the committed header is stale relative to
  the Rust source; the PR must regenerate and commit the header
  before merging.
- The header is **committed to the repo**, not generated on install,
  so downstream C consumers can use the crate (or a plain header
  vendor drop) without a Rust toolchain.
- Versioning is tied to the Cargo package: the header carries a
  `TINYQUANT_H_VERSION` macro emitted by `cbindgen.toml` via the
  `header = "#define TINYQUANT_H_VERSION \"@version@\""` hook, and
  a Rust-side `const_assert!` checks that the string matches
  `env!("CARGO_PKG_VERSION")`. Mismatch is a compile-time error,
  so the header can never drift from the crate version.
- A separate smoke test (`tests/abi_header_compile.rs`) compiles a
  one-line C file that only `#include`s the header and expands the
  version macro, to catch macro-expansion regressions that would
  slip past the `cc` crate's "it links" check.
- The drift guard is repeated in the `rust-release.yml` workflow's
  `gate` stage so that a tag push cannot publish a crate whose
  header is stale.

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
`cc` crate at test time and runs it. The full C-ABI test surface
is:

| Test file | Purpose |
|---|---|
| `tests/abi_smoke.rs` | Rust-side sanity of every exported fn; calls each `tq_*` function through its Rust re-export and asserts ownership/error/out-parameter semantics |
| `tests/abi_c_smoke.rs` | Compiles `tests/c/smoke.c` at test time via the `cc` crate and runs it; catches C compilability and link-time issues |
| `tests/abi_cxx_smoke.rs` | Compiles `tests/cxx/smoke.cc` to catch C++ consumer issues (name mangling, `extern "C"` linkage, `nullptr` vs `NULL`, `bool` vs `uint8_t`) |
| `tests/abi_header_compile.rs` | Parses `include/tinyquant.h` with `bindgen` at test time to verify it is valid C **independent of our own emission** — a second-source check |
| `tests/abi_handle_lifetime.rs` | Stress test for handle double-free, use-after-free, and null-pointer paths; each must fail cleanly with `TQ_ERR_INVALID_HANDLE` and never UB |
| `tests/abi_panic_crossing.rs` | Deliberately triggers a Rust panic inside every `extern "C"` entry point and asserts the wrapper returns `TQ_ERR_PANIC` with a non-NULL message |

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

#### CLI feature flag matrix

`tinyquant-cli` exposes a narrow set of Cargo features whose
defaults are tuned for operator ergonomics. The set must be kept in
sync with [[design/rust/feature-flags|Feature Flags]] §`tinyquant-sys`
(the CLI is the only `bin` consumer of the same feature graph).

```toml
[features]
default = ["jemalloc", "rayon", "simd", "mmap", "progress"]
jemalloc = ["dep:jemallocator"]
rayon    = ["tinyquant-core/rayon", "tinyquant-io/rayon", "tinyquant-bruteforce/rayon"]
simd     = ["tinyquant-core/simd", "tinyquant-io/simd", "tinyquant-bruteforce/simd"]
mmap     = ["tinyquant-io/mmap"]
progress = ["dep:indicatif"]
tracing-json = ["tracing-subscriber/json"]
```

| Feature | Default | Effect | Notes |
|---|---|---|---|
| `jemalloc` | on (non-MSVC) | Uses `jemallocator` as the global allocator on Linux/macOS/FreeBSD | Larger binary, faster allocation; `cfg(not(target_env = "msvc"))` gates the dep; **explicitly off for musl** (see Risk R22.4) |
| `rayon` | on | Parallel batch compress/decompress honoring `--threads` | Can be disabled for deterministic single-threaded runs |
| `simd` | on | Portable SIMD kernels from `tinyquant-core` | Required for meeting the Phase 21 bench budgets on x86_64 and aarch64. For Tier-2 targets (`i686-pc-windows-msvc`, `x86_64-unknown-freebsd`) the feature is still built-in but the budget gate is advisory — see Phase 21 §Bench budget schema. |
| `mmap` | on | mmap-backed corpus file reads in `corpus decompress`, `corpus search` | Falls back to `std::fs::read` when disabled |
| `progress` | on | `indicatif` progress bars to stderr on long-running batches | Escape hatches: `--no-progress` flag, `NO_COLOR=1` disables colorization, `TERM=dumb` disables bars entirely |
| `tracing-json` | off | JSON-formatted logs via `tracing-subscriber` for CI pipelines | Enabled in the release container image so `docker logs` produces structured output |

**Size budget.** A stripped release binary must be ≤ 8 MiB on
`x86_64-unknown-linux-gnu` with the default feature set. The
release workflow enforces this via
`stat --format='%s' target/x86_64-unknown-linux-gnu/release/tinyquant`
and fails if the result exceeds 8 × 1024 × 1024. Exceeding the
budget requires either a feature flag being moved out of default or
an explicit budget bump with reviewer sign-off.

#### CLI smoke test matrix

Every standalone binary produced by the release pipeline must pass
a common set of smoke tests on a platform-appropriate runner. The
matrix mirrors the Part D release matrix and the two must stay in
sync (xtask grep: see `## CI integration` below).

| Target triple | Runner | Test cases | Expected outcome |
|---|---|---|---|
| `x86_64-unknown-linux-gnu` | `ubuntu-22.04` | `info`, `codec train`, `codec compress`, `codec decompress`, `verify` | All commands exit 0; round-trip MSE < 1e-2 |
| `aarch64-unknown-linux-gnu` | `ubuntu-22.04-arm` | same | same |
| `x86_64-unknown-linux-musl` | `ubuntu-22.04` (cross) | `info`, `codec train`, `codec compress`, `codec decompress`, `verify` | Static binary; `ldd` must print "not a dynamic executable"; round-trip MSE < 1e-2 |
| `aarch64-unknown-linux-musl` | `ubuntu-22.04` (cross, qemu) | same | same |
| `x86_64-apple-darwin` | `macos-13` | same | same |
| `aarch64-apple-darwin` | `macos-14` | same | same |
| `x86_64-pc-windows-msvc` | `windows-2022` | same (`.exe` suffix) | same |
| `i686-pc-windows-msvc` | `windows-2022` | `info`, `codec train`, `codec compress`, `codec decompress`, `verify` | 32-bit `.exe`; round-trip MSE < 1e-2 |
| `x86_64-unknown-freebsd` | `ubuntu-22.04` (cross; pinned freebsd image) | `info`, `verify`, `codec train` (only) | No runnable emulator for full round-trip; build-and-link only plus `cross run` of `info` |

Each row runs a shared `scripts/cli-smoke.sh` (portable bash) or
`scripts/cli-smoke.ps1` (PowerShell) that chains the commands and
asserts the MSE threshold with a one-line Python shim (available on
every runner). The matrix is referenced twice — once here and once
in the Part D YAML — and the xtask drift check under §CI integration
enforces both to stay identical.

### Part D — Cross-architecture release pipeline

- [ ] **Step 18: Dockerfile for CLI container image**

```dockerfile
# rust/Dockerfile
#
# Reproducibility notes (see §Container reproducibility below):
#   - Base image is pinned by digest, NOT by tag.
#   - SOURCE_DATE_EPOCH is set from the git commit timestamp.
#   - --timestamp on buildx normalizes layer mtimes.
#   - strip + LTO are configured in rust/Cargo.toml [profile.release].
#
FROM rust:1.81-bookworm@sha256:REPLACE_WITH_PINNED_DIGEST AS builder
ARG SOURCE_DATE_EPOCH
ENV SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}
WORKDIR /build
COPY rust /build/rust
WORKDIR /build/rust
RUN cargo build -p tinyquant-cli --release --locked --features tracing-json

FROM gcr.io/distroless/cc-debian12:nonroot@sha256:REPLACE_WITH_PINNED_DIGEST AS runtime
COPY --from=builder /build/rust/target/release/tinyquant /usr/local/bin/tinyquant
USER nonroot
ENTRYPOINT ["/usr/local/bin/tinyquant"]
CMD ["info"]
```

#### Container reproducibility

The CLI container image must be bit-identical across rebuilds so
that downstream consumers can pin by digest and get honest SLSA
provenance. The Dockerfile achieves this with four techniques:

1. **Pinned base images by digest.** Neither `rust:1.81-bookworm`
   nor `gcr.io/distroless/cc-debian12:nonroot` is pinned by tag
   alone — both carry a `@sha256:...` suffix baked into the
   Dockerfile. The release workflow runs `docker inspect` on each
   digest before building to catch upstream tag drift early.
2. **`SOURCE_DATE_EPOCH` from the git commit.** The workflow
   computes `SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)` from the
   tagged commit, exports it as a build arg, and passes it to
   `cargo build` so the release binary embeds a stable timestamp
   in debug symbols and the built-in version string. Same commit →
   same bytes.
3. **`--timestamp` on `docker buildx build`.** The workflow passes
   `--timestamp=$SOURCE_DATE_EPOCH` (buildx normalizes layer mtimes
   to that value), so the resulting OCI manifest has identical
   layer digests regardless of when the build ran.
4. **Deterministic profile.** `rust/Cargo.toml` sets
   `[profile.release]` with `codegen-units = 1`, `lto = "fat"`,
   `strip = true`, and `debug = false` — matches the release-strategy
   doc §Reproducible builds.

**Verification.** The `rust-release.yml` workflow adds a
`container-reproducibility` job that builds the image twice (once
in the `publish-container` stage, once in a second ephemeral job)
and asserts
`docker inspect --format '{{.Id}}' tinyquant:test-1 == tinyquant:test-2`.
A mismatch fails the release. Document each of the four techniques
with comments at the top of `rust/Dockerfile` so future edits don't
silently break determinism.

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
          - { target: x86_64-apple-darwin,         os: macos-13,          cross: false }
          - { target: aarch64-apple-darwin,        os: macos-14,          cross: false }
          - { target: x86_64-pc-windows-msvc,      os: windows-2022,      cross: false }
          - { target: i686-pc-windows-msvc,        os: windows-2022,      cross: false }
          - { target: x86_64-unknown-freebsd,      os: ubuntu-22.04,      cross: true  }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
        with: { lfs: true }
      - uses: dtolnay/rust-toolchain@stable
        with: { toolchain: 1.81.0, targets: ${{ matrix.target }} }  # MUST match rust/rust-toolchain.toml — see Phase 14 §L3 and the check-toolchain-version xtask
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
        with: { toolchain: 1.81.0 }  # matches rust/rust-toolchain.toml (Phase 14 §L3)
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

#### Release dry-run playbook

Before tagging the real release, run the dry-run playbook end-to-end
on a pre-release tag. Every step must be green before the final
commit+tag. This is gated on a clean `rust-release.yml` run against
a `rust-v0.1.0-alpha.1` tag — see [[design/rust/phase-14-implementation-notes|Phase 14 Lessons]]
§L1 for why "green locally" is not a substitute for a successful
workflow run on the exact commit.

- **Step a — Crate publish dry run.** `cargo publish --dry-run`
  for each crate in the documented publish order
  ([[design/rust/crate-topology|Crate Topology]] §Publish order):
  ```bash
  cargo publish --dry-run -p tinyquant-core
  cargo publish --dry-run -p tinyquant-io
  cargo publish --dry-run -p tinyquant-bruteforce
  cargo publish --dry-run -p tinyquant-pgvector
  cargo publish --dry-run -p tinyquant-sys
  cargo publish --dry-run -p tinyquant-cli
  ```
  Every invocation must exit 0; any packaging error (missing
  `README.md`, missing license, unpublished path-only dep) is a
  blocker.
- **Step b — Local wheel build and install.** Build the wheel with
  `maturin build --release --strip` and force-install it into a
  clean venv:
  ```bash
  python -m venv .venv-dryrun
  .venv-dryrun/bin/pip install --force-reinstall target/wheels/*.whl
  ```
- **Step c — Version smoke.** In the fresh venv, `python -c "import
  tinyquant_rs; print(tinyquant_rs.__version__)"` must print
  exactly the workspace version string.
- **Step d — Cross-compile smoke.**
  `cross build -p tinyquant-cli --target aarch64-unknown-linux-gnu --release`
  to smoke-test the cross path locally before entrusting it to CI.
  Repeat for `aarch64-unknown-linux-musl` if the workstation has
  qemu registered.
- **Step e — Pre-release tag.** Push `rust-v0.1.0-alpha.1` and
  watch the full workflow:
  ```bash
  git tag rust-v0.1.0-alpha.1
  git push origin rust-v0.1.0-alpha.1
  gh run watch --workflow rust-release.yml
  ```
  Confirm every matrix artifact attaches to the GitHub pre-release
  and that both PyPI and crates.io show the alpha.
- **Step f — CLI archive smoke.** Download the
  `x86_64-unknown-linux-gnu` CLI archive from the pre-release,
  extract, and run:
  ```bash
  ./tinyquant info
  ./tinyquant codec train --input sample.f32.bin --rows 1000 --cols 64 \
                          --bit-width 4 --seed 42 --output cb.bin
  ./tinyquant codec compress --input sample.f32.bin --config-json cfg.json \
                             --codebook cb.bin --output compressed.bin
  ./tinyquant codec decompress --input compressed.bin --config-json cfg.json \
                               --codebook cb.bin --output recovered.f32.bin
  python -c "import numpy as np; a = np.fromfile('sample.f32.bin', dtype='f32'); b = np.fromfile('recovered.f32.bin', dtype='f32'); mse = float(((a - b)**2).mean()); assert mse < 1e-2, mse"
  ```
- **Step g — Wheel install from pre-release.** Download the wheel
  tagged `cp312-abi3-manylinux_2_17_x86_64`, install via
  `pip install tinyquant_rs-0.1.0a1-cp312-abi3-manylinux_2_17_x86_64.whl`,
  and run the full parity suite vs a pinned `tinyquant-cpu`.
- **Step h — Container pull + run.**
  ```bash
  docker pull ghcr.io/better-with-models/tinyquant:rust-v0.1.0-alpha.1
  docker run --rm ghcr.io/better-with-models/tinyquant:rust-v0.1.0-alpha.1 info
  ```
  Must print the same `info` output as Step f.
- **Step i — Clean up the pre-release.** After confirming every
  stage, delete the pre-release tag and all its artifacts (`gh
  release delete rust-v0.1.0-alpha.1 --cleanup-tag`) before tagging
  the real `rust-v0.1.0`. This keeps the public release history
  uncluttered and prevents accidental consumer pinning to the
  alpha.

> [!warning] Phase 14 L1 — CI health is the gate
> Do not proceed to Step 22 until the pre-release dry-run produces
> a green `rust-release.yml` run on the exact commit you intend to
> tag. `gh run list --workflow rust-release.yml --limit 5` must
> show only `completed success` for the pre-release tag.

- [ ] **Step 22: Final commit + tag**

```bash
git add rust/crates/tinyquant-py rust/crates/tinyquant-sys \
        rust/crates/tinyquant-cli rust/Dockerfile rust/.dockerignore \
        .github/workflows/rust-release.yml COMPATIBILITY.md
git commit -m "feat(release): pyo3 binding, C ABI, CLI binary, and multi-arch release"
git tag rust-v0.1.0
git push origin main rust-v0.1.0
```

### Clippy profile gotchas

The workspace lint profile is the same one
[[design/rust/phase-14-implementation-notes|Phase 14 Lessons]] §L7
called out (`pedantic + nursery + unwrap_used + expect_used + panic
+ indexing_slicing + cognitive_complexity`), plus two additions for
the binding crates. Phase 22 code will trip the following lints
unless they are handled up front:

- **`unsafe_code` in `tinyquant-sys`.** The `extern "C" fn` bodies
  need raw pointer dereferences and `catch_unwind`. Rather than
  loosening the crate-wide `#![forbid(unsafe_code)]`, apply a
  narrow `#[allow(unsafe_code)]` at the **module** level on
  `src/codec_abi.rs` and `src/corpus_abi.rs` only. All other
  modules in the crate keep the forbid.
- **`must_use_candidate` on pyo3 functions returning `Result`.**
  Every `#[pyfunction]` and `#[pymethod]` that returns a
  `PyResult<T>` must carry `#[must_use]` — clippy nursery flags the
  missing attribute and the release gate is strict.
- **`cast_precision_loss` / `cast_possible_truncation` on
  Python→Rust argument conversions.** Narrow `#[allow]` on the
  individual function, naming the exact lints, following the Phase
  14 `Codebook::train` precedent. Never blanket-allow across the
  module.
- **`missing_errors_doc` / `missing_panics_doc` on every
  `#[pyfunction]`.** Each exported pyo3 function needs a
  rustdoc section listing the `PyErr` subclasses it can raise (for
  `missing_errors_doc`) and a statement that panics are caught and
  converted (for `missing_panics_doc`). This is both a lint pass
  and the canonical place to document the exception contract.
- **`indexing_slicing` in CLI arg parsing.** `clap`'s derived
  parser already validates, but any hand-rolled index into
  `std::env::args` is a lint failure. Use `.get(..)` or route
  through clap's validators; prefer `args[0]` → `args.get(0)`.
- **`unwrap_used` / `expect_used` ban, including `main()`.** Even
  the `fn main() -> anyhow::Result<()>` path must avoid
  `.unwrap()` / `.expect()`. Use `?` throughout and only surface
  panics via the wrapped `catch_unwind` boundary. The same pattern
  applies to the pyo3 binding's `PyResult` returns.
- **`trivially_copy_pass_by_ref` on `f32`/`u8`/`u32` helpers**
  (Phase 14 L7 recurrence). Keep value semantics on numeric
  arguments; `&f32` in a helper will trip the lint.
- **`redundant_pub_crate`** on items inside `pub(crate)` modules.
  Use `pub` for inner items when the containing module is already
  `pub(crate)` — same recipe as Phase 14 §`codec::quantize`.

Re-read this list before the first `cargo xtask lint` run on any
new Phase 22 module. These are recurring patterns the test suites
will not catch.

### COMPATIBILITY.md

Phase 22 creates a new repo-root file `COMPATIBILITY.md` that is
the authoritative matrix of supported
`(tinyquant_cpu, tinyquant_rs)` pairs. The initial shape:

- **Location:** repo root, alongside `README.md`, `CHANGELOG.md`,
  and `CLAUDE.md`.
- **Content:** a single table with columns
  `tinyquant_cpu version | tinyquant_rs version | known drift |
  notes`, one row per Rust release.
- **Initial entry for `rust-v0.1.0`:**
  `(cpu 0.1.1, rs 0.1.0, known drift: none, notes: full parity on
  the gold corpus; parity suite covers config_hash, codebook.train
  bytes, compressed_vector.to_bytes bytes, corpus lifecycle, batch
  methods, and exception hierarchies)`.
- **Update cadence:** every release of either package adds a new
  row. Drifts are documented explicitly with a pointer to the
  corresponding issue and a mitigation plan.
- **Ownership:** Phase 22 creates the file; every subsequent phase
  (starting with a hypothetical Phase 23) maintains it as part of
  its own release checklist.
- **Validation:** an `xtask compatibility-check` reads the table
  and asserts that the current workspace version matches the
  latest row; CI runs this on every PR touching
  `rust/**/Cargo.toml`.

The file complements [[design/rust/release-strategy|Release and
Versioning]] §Compatibility with the Python package — the design
doc describes the policy, `COMPATIBILITY.md` is the ledger.

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
- SBOM, cosign signatures, and SLSA provenance attached to every
  release artifact (see `## Supply-chain attestations`).

## Supply-chain attestations

Every Phase 22 release artifact — wheel, crate, CLI archive,
container image, and staticlib bundle — ships with supply-chain
metadata so downstream consumers can verify provenance without
trusting the release pipeline blindly.

- **`cargo publish` via trusted publisher (OIDC).** crates.io uses
  the trusted-publisher OIDC integration, so the release workflow
  never needs a long-lived `CARGO_REGISTRY_TOKEN`. Setup:
  1. On crates.io, open the `tinyquant-*` crate settings and add a
     GitHub Actions trusted publisher pointing at
     `better-with-models/TinyQuant`, workflow `rust-release.yml`,
     and the `rust-v*` tag ref.
  2. Repeat for every published crate (`tinyquant-core`,
     `tinyquant-io`, `tinyquant-bruteforce`, `tinyquant-pgvector`,
     `tinyquant-sys`, `tinyquant-cli`).
  3. Add `id-token: write` to the workflow permissions block
     (already present in the Step 19 YAML).
  4. Remove any lingering `CARGO_REGISTRY_TOKEN` secret from the
     repo.
- **PyPI via OIDC trusted publisher.** Use
  `pypa/gh-action-pypi-publish@release/v1`. The trusted publisher
  is configured at `https://pypi.org/manage/publishing/` for the
  `tinyquant-rs` project, pointing at `better-with-models/TinyQuant`,
  workflow `rust-release.yml`, environment `release`, and the
  `rust-v*` tag ref on `main`. No API tokens are stored in GitHub
  secrets.
- **GitHub Releases.** Published via `softprops/action-gh-release@v2`
  with `GITHUB_TOKEN` scoped to `contents: write`. The release is
  pinned to a git commit (not just a tag), so orphaned artifacts
  from partial runs can be cleanly deleted and re-attached.
- **GHCR container push.** Uses `docker/login-action@v3` with
  `GITHUB_TOKEN` scoped to `packages: write`. The login step reads
  `${{ github.actor }}` / `${{ secrets.GITHUB_TOKEN }}` — again, no
  long-lived PAT.
- **SBOM per artifact.** Every release artifact ships with a
  CycloneDX SBOM. The workflow runs:
  ```bash
  cargo install cargo-auditable --locked
  cargo auditable build --release -p tinyquant-cli
  syft dir:. -o cyclonedx-json > tinyquant-cli.cdx.json
  ```
  and attaches `tinyquant-cli.cdx.json` next to the CLI archive on
  the GitHub release. Wheels carry an in-wheel `sbom/` directory
  with the same JSON; containers have the SBOM attached via
  `cosign attach sbom`.
- **Cosign signatures.** Uses `sigstore/cosign-installer@v3` in the
  workflow, then signs every binary and container with a keyless
  OIDC identity:
  ```bash
  cosign sign --yes ghcr.io/better-with-models/tinyquant@$DIGEST
  cosign sign-blob --yes tinyquant-*.tar.gz --output-signature tinyquant-*.tar.gz.sig
  ```
  Signatures are stored in the GHCR registry (for images) and
  attached to the GitHub release (for blobs).
- **SLSA build provenance.** Uses
  `actions/attest-build-provenance@v1` on every artifact. The
  attestation is written to the GitHub attestation store and is
  discoverable via `gh attestation verify`.
- **`cargo-vet` + `cargo-audit` gates.** On every release tag the
  workflow runs:
  ```bash
  cargo install cargo-vet cargo-audit --locked
  cargo vet --locked
  cargo audit --deny warnings
  ```
  Any new advisory (even if it has not been triaged in the Vet
  database) blocks the release. The vet/audit results are attached
  to the release as `supply-chain-report.json`.

Consumers can verify a Phase 22 release with:

```bash
cosign verify ghcr.io/better-with-models/tinyquant:rust-v0.1.0 \
  --certificate-identity-regexp 'https://github.com/better-with-models/TinyQuant/\.github/workflows/rust-release\.yml@refs/tags/rust-v.*' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
gh attestation verify tinyquant-rust-v0.1.0-x86_64-unknown-linux-gnu.tar.gz \
  --owner better-with-models
```

## CI integration

Phase 22 extends the workflow surface defined in
[[design/rust/ci-cd|CI/CD]] with the new release entry point and a
set of per-PR gates that protect it.

- **`rust-release.yml` is the release entry point.** The Step 19
  YAML in this document is the source of truth for the release
  workflow — any change must land alongside a matching update to
  [[design/rust/ci-cd|CI/CD]] and
  [[design/rust/release-strategy|Release and Versioning]]. The
  workflow is tag-triggered on `rust-v*.*.*` and gates every
  downstream stage on the re-run of `rust-ci.yml` against the
  tagged commit.
- **`rust-ci.yml` gains three new jobs for Phase 22.**
  - **`abi-header-drift`** — runs
    `cargo build -p tinyquant-sys --release` then
    `git diff --exit-code rust/crates/tinyquant-sys/include/tinyquant.h`;
    blocking on every PR.
  - **`cli-smoke`** — matrix job that runs `scripts/cli-smoke.sh`
    against every row of the §CLI smoke test matrix; blocking on
    every PR that touches `rust/crates/tinyquant-cli/**`,
    advisory elsewhere.
  - **`wheel-smoke`** — matrix job that builds the wheel via
    `maturin build --release --strip` on each Python/OS pair from
    §Maturin wheel test matrix and runs the parity suite; blocking
    on every PR that touches `rust/crates/tinyquant-py/**`.
- **LFS hydration** (**Phase 14 L2**). Every build, smoke, and
  parity job uses `actions/checkout@v4` with `with: { lfs: true }`
  because the codec fixtures drive the parity tests. The release
  workflow does the same. A sibling `xtask lfs-audit` grep
  asserts every `actions/checkout@v4` occurrence in
  `.github/workflows/*.yml` is followed by an `lfs: true` block;
  the check is blocking in CI. L2's silent-132-byte-pointer-file
  failure must not recur for the fixture-heavy parity suite.
- **Design-doc vs CI drift check** (**Phase 14 L3**). A new
  `xtask ci-doc-drift` subcommand greps
  `docs/design/rust/release-strategy.md` and
  `docs/design/rust/ci-cd.md` for testable claims (e.g., "every
  build job runs on `ubuntu-22.04`", "wheels are built with
  `manylinux auto`", "publish order is core → io → bruteforce →
  pgvector → sys → cli") and asserts each claim has a matching
  line in `.github/workflows/rust-release.yml`. A mismatch blocks
  the PR. This is the L3 remediation generalized: design docs are
  not CI config, but they can be validated against it.
- **Cross-runner parity** (**Phase 14 L4**). Every parity test
  runs on **at least two distinct host CPUs per target triple** —
  the runner pool on `ubuntu-22.04` mixes AVX2 and AVX-512 hosts,
  and L4's fix was to assert numerical invariants that hold across
  both. Phase 22 inherits that contract via the Phase 20
  determinism work; any parity regression caused by SIMD ISA
  nondeterminism is a Phase 20 escape, not a Phase 22 bug. The
  `wheel-smoke` job pins its execution across runner images
  `ubuntu-22.04` and `ubuntu-22.04-arm` to force the cross-CPU
  exercise.
- **Phase exit requires a clean pre-release workflow run**
  (**Phase 14 L1**). Phase 22 is not complete until
  `rust-release.yml` produces a `completed success` run on the
  `rust-v0.1.0-alpha.1` dry-run tag. `gh run list --workflow
  rust-release.yml --limit 5` must show the alpha run as success
  before Step 22's final tag is pushed. Treat any intervening
  `completed failure` as a blocker — same gate as L1 applied
  consistently at the project exit point.
- **Post-tag monitoring.** The release workflow documents
  `gh run watch --workflow rust-release.yml` as the recommended
  operator pattern during a release — running it keeps the
  operator in the loop and surfaces mid-run failures (e.g., a
  PyPI outage, a GHCR rate limit) early enough to abort before
  partial artifacts hit consumers.

## Risks

- **R22.1** — pyo3 minor bump breaks abi3 compatibility. Pin pyo3
  to an exact minor version in `rust/Cargo.toml`
  `[workspace.dependencies]` and add a nightly job that builds the
  wheel against pyo3 main to surface upcoming breakage early.
- **R22.2** — maturin build on aarch64 needs qemu. Use
  `dtolnay/rust-toolchain@stable` + `docker/setup-qemu-action@v3`
  on every aarch64 matrix row; verify qemu availability in the
  smoke job's first step so a missing action fails fast rather
  than mid-build.
- **R22.3** — cross-compile of `tinyquant-cli` for freebsd misses
  libc symbols. Use `cross` with the pinned image
  `ghcr.io/cross-rs/x86_64-unknown-freebsd:main@sha256:...` and
  freeze the digest in the workflow alongside the base images.
- **R22.4** — jemalloc link fails on musl. Gate the `jemalloc`
  feature off for `*-musl` targets via
  `#[cfg(not(all(target_env = "musl", feature = "jemalloc")))]`
  and document the fallback to the system allocator in the
  CLI feature matrix (see §CLI feature flag matrix).
- **R22.5** — C ABI header drifts silently. Blocked by the
  `abi-header-drift` CI job (§CI integration) and the version-
  macro `const_assert!` in `tinyquant-sys`. The risk is
  closeable once both land.
- **R22.6** — release workflow partial failure leaves orphaned
  artifacts. Every stage is idempotent; publishes are pinned to a
  git commit, not just a tag; `gh release delete --cleanup-tag`
  is the recovery path in the dry-run playbook §Step i.
- **R22.7** — crates.io publish order deadlock because
  `tinyquant-cli` depends on every other crate. Mitigated by the
  explicit ordered list in the `publish-crates` job (core → io →
  bruteforce → pgvector → sys → cli), matching
  [[design/rust/crate-topology|Crate Topology]] §Publish order.
- **R22.8** — PyPI outage on release day. The `publish-pypi` job
  retries with exponential backoff (`pypa/gh-action-pypi-publish`
  supports `--skip-existing` for safe retries), and the dry-run
  catches format issues ahead of time so the real tag only hits
  PyPI once everything is known-good.
- **R22.9** — GHCR rate limit on container push. Fall back to a
  Docker Hub mirror as a secondary registry via an additional
  login + push step gated on `failure()`; tag both registries
  identically so consumers can swap the registry prefix without
  breaking scripts.
- **R22.10** — cross-runner SIMD nondeterminism breaks parity
  tests (**Phase 14 L4** recurrence). Phase 20's determinism
  contract must be green on every target in the release matrix
  before Phase 22 can tag. Treat this as an explicit prerequisite:
  if Phase 20 has any `#[ignore]`d parity tests on the rotation or
  codec path, Phase 22 cannot exit. The L4 lesson was specifically
  about runner-identity-dependent "fixes," and Phase 22 must not
  repeat it on the release matrix.

## Out of scope

- JS/WASM bindings (reserved for a later release).
- iOS / Android wheels.
- AVX-512 as a default kernel — stays behind the `simd-nightly`
  feature flag in `tinyquant-core`.
- Async CLI or HTTP server mode — the CLI remains a synchronous
  command dispatcher; HTTP belongs in `better-router`.
- Cloud-native plugins (Kubernetes operators, CRDs, Helm charts).
- Signed crates.io packages — crates.io does not yet support
  package signing upstream; we sign the CLI archives and container
  images via cosign instead.

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
- [[design/rust/feature-flags|Feature Flags]]
- [[design/rust/ci-cd|CI/CD]]
- [[design/rust/goals-and-non-goals|Goals and Non-Goals]]
- [[design/rust/phase-14-implementation-notes|Phase 14 Implementation Notes]]
- [[roadmap|Implementation Roadmap]]
