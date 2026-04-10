---
title: Rust Port — FFI and Bindings
tags:
  - design
  - rust
  - ffi
  - pyo3
  - c-abi
date-created: 2026-04-10
status: draft
category: design
---

# Rust Port — FFI and Bindings

> [!info] Purpose
> Describe the two binding surfaces — the `tinyquant-sys` C ABI and
> the `tinyquant-py` pyo3 binding — and the invariants they must
> preserve relative to the Python gold standard.

## Binding 1: `tinyquant-py` (Python)

### Product shape

- Package name on PyPI: `tinyquant-rs`
- Importable module: `tinyquant_rs`
- Build tool: `maturin`
- Python compatibility: `abi3-py312` (single wheel works for 3.12+)
- Wheel platforms: `manylinux_2_28_x86_64`, `macosx_11_0_arm64`,
  `macosx_11_0_x86_64`, `win_amd64`

### Class surface (mirrors Python exactly)

```python
# tinyquant_rs/__init__.py
from tinyquant_rs._core import (
    # codec
    CodecConfig,
    Codebook,
    RotationMatrix,
    CompressedVector,
    Codec,
    compress,
    decompress,
    # errors
    DimensionMismatchError,
    ConfigMismatchError,
    CodebookIncompatibleError,
    DuplicateVectorError,
    # corpus
    CompressionPolicy,
    VectorEntry,
    Corpus,
    CorpusCreated,
    VectorsInserted,
    CorpusDecompressed,
    CompressionPolicyViolationDetected,
    # backend
    SearchBackend,
    SearchResult,
    BruteForceBackend,
)
from tinyquant_rs._core import __version__
```

Every symbol the Python package re-exports is available under the
same name in `tinyquant_rs`. The sub-packages (`tinyquant_rs.codec`,
`tinyquant_rs.corpus`, `tinyquant_rs.backend`) are populated with the
same members in the same order so that `dir(tinyquant_rs.codec) ==
dir(tinyquant_cpu.codec)` on both implementations.

### Method signature parity

Example: `CodecConfig`:

```rust
// tinyquant-py/src/codec.rs
use pyo3::prelude::*;
use tinyquant_core::codec::CodecConfig as CoreConfig;

#[pyclass(name = "CodecConfig", frozen, eq, hash)]
#[derive(Clone)]
pub struct PyCodecConfig {
    inner: CoreConfig,
}

#[pymethods]
impl PyCodecConfig {
    #[new]
    #[pyo3(signature = (bit_width, seed, dimension, residual_enabled = true))]
    fn new(
        bit_width: u8,
        seed: u64,
        dimension: u32,
        residual_enabled: bool,
    ) -> PyResult<Self> {
        let inner = CoreConfig::new(bit_width, seed, dimension, residual_enabled)
            .map_err(map_codec_error)?;
        Ok(Self { inner })
    }

    #[getter]
    fn bit_width(&self) -> u8 { self.inner.bit_width() }
    #[getter]
    fn seed(&self) -> u64 { self.inner.seed() }
    #[getter]
    fn dimension(&self) -> u32 { self.inner.dimension() }
    #[getter]
    fn residual_enabled(&self) -> bool { self.inner.residual_enabled() }
    #[getter]
    fn num_codebook_entries(&self) -> u32 { self.inner.num_codebook_entries() }
    #[getter]
    fn config_hash(&self) -> String { self.inner.config_hash().to_string() }

    fn __repr__(&self) -> String {
        format!(
            "CodecConfig(bit_width={}, seed={}, dimension={}, residual_enabled={})",
            self.inner.bit_width(),
            self.inner.seed(),
            self.inner.dimension(),
            if self.inner.residual_enabled() { "True" } else { "False" },
        )
    }
}
```

The `frozen` flag matches Python's frozen dataclass behavior —
attribute assignment raises `AttributeError`. `eq` derives
`__eq__`/`__ne__` from the Rust `PartialEq`. `hash` derives `__hash__`.

### NumPy interop

The pyo3 binding uses the `numpy` crate to expose zero-copy views
into NumPy f32 arrays:

```rust
use numpy::{PyArray1, PyArray2, PyReadonlyArray1, PyReadonlyArray2};

#[pymethods]
impl PyCodec {
    fn compress<'py>(
        &self,
        py: Python<'py>,
        vector: PyReadonlyArray1<'py, f32>,
        config: &PyCodecConfig,
        codebook: &PyCodebook,
    ) -> PyResult<Py<PyCompressedVector>> {
        let slice = vector.as_slice()?;
        let cv = self
            .inner
            .compress(slice, &config.inner, &codebook.inner)
            .map_err(map_codec_error)?;
        Py::new(py, PyCompressedVector { inner: cv })
    }

    fn compress_batch<'py>(
        &self,
        py: Python<'py>,
        vectors: PyReadonlyArray2<'py, f32>,
        config: &PyCodecConfig,
        codebook: &PyCodebook,
    ) -> PyResult<Vec<Py<PyCompressedVector>>> {
        let arr = vectors.as_array();
        let (rows, cols) = arr.dim();
        let slice = arr.as_slice().ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("vectors must be C-contiguous")
        })?;
        let cvs = self
            .inner
            .compress_batch(slice, rows, cols, &config.inner, &codebook.inner, /*parallelism*/ rayon_parallelism())
            .map_err(map_codec_error)?;
        cvs.into_iter()
            .map(|cv| Py::new(py, PyCompressedVector { inner: cv }))
            .collect()
    }

    fn decompress<'py>(
        &self,
        py: Python<'py>,
        compressed: &PyCompressedVector,
        config: &PyCodecConfig,
        codebook: &PyCodebook,
    ) -> PyResult<Py<PyArray1<f32>>> {
        let out = self
            .inner
            .decompress(&compressed.inner, &config.inner, &codebook.inner)
            .map_err(map_codec_error)?;
        Ok(PyArray1::from_vec_bound(py, out).unbind())
    }
}
```

Key property: no Python-side allocation for input vectors; the
`PyReadonlyArray1<'py, f32>` borrows the NumPy buffer directly.

### Parity test plan

The pyo3 binding ships with a dedicated parity suite that imports
both `tinyquant_cpu` and `tinyquant_rs` and asserts identical behavior:

```python
# tinyquant-py/tests/python/test_parity.py
import numpy as np
import pytest
import tinyquant_cpu as py
import tinyquant_rs as rs

@pytest.fixture(params=[(2, 0, 64), (4, 42, 768), (8, 999, 1536)])
def config_pair(request):
    bw, seed, dim = request.param
    return (
        py.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim),
        rs.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim),
    )

def test_config_hash_parity(config_pair):
    py_cfg, rs_cfg = config_pair
    assert py_cfg.config_hash == rs_cfg.config_hash

def test_compress_parity(config_pair):
    py_cfg, rs_cfg = config_pair
    dim = py_cfg.dimension
    rng = np.random.default_rng(123)
    training = rng.standard_normal((10_000, dim)).astype(np.float32)
    vec = rng.standard_normal(dim).astype(np.float32)

    py_cb = py.codec.Codebook.train(training, py_cfg)
    rs_cb = rs.codec.Codebook.train(training, rs_cfg)

    # Codebook entries byte-identical (ChaCha+quantile path on both sides)
    np.testing.assert_array_equal(py_cb.entries, rs_cb.entries)

    py_cv = py.codec.compress(vec, py_cfg, py_cb)
    rs_cv = rs.codec.compress(vec, rs_cfg, rs_cb)

    assert py_cv.to_bytes() == rs_cv.to_bytes()

def test_decompress_parity(config_pair):
    py_cfg, rs_cfg = config_pair
    dim = py_cfg.dimension
    rng = np.random.default_rng(7)
    training = rng.standard_normal((5_000, dim)).astype(np.float32)
    vec = rng.standard_normal(dim).astype(np.float32)
    py_cb = py.codec.Codebook.train(training, py_cfg)
    rs_cb = rs.codec.Codebook.train(training, rs_cfg)
    py_cv = py.codec.compress(vec, py_cfg, py_cb)
    rs_cv = rs.codec.compress(vec, rs_cfg, rs_cb)
    py_out = py.codec.decompress(py_cv, py_cfg, py_cb)
    rs_out = rs.codec.decompress(rs_cv, rs_cfg, rs_cb)
    max_err = float(np.max(np.abs(py_out - rs_out)))
    assert max_err < 1e-5, f"decompress parity failed: max={max_err}"
```

Full parity suite covers compressed-vector `to_bytes`/`from_bytes`,
error types, batch operations, corpus lifecycle, and brute-force
search results.

### Avoiding GIL contention

Hot methods release the GIL during compute:

```rust
fn compress_batch<'py>(
    &self,
    py: Python<'py>,
    vectors: PyReadonlyArray2<'py, f32>,
    config: &PyCodecConfig,
    codebook: &PyCodebook,
) -> PyResult<Vec<Py<PyCompressedVector>>> {
    let arr = vectors.as_array();
    let (rows, cols) = arr.dim();
    let slice = arr.as_slice().ok_or_else(|| /* ... */)?;
    let cfg = config.inner.clone();
    let cb  = codebook.inner.clone();

    let cvs = py.allow_threads(move || {
        Codec::new().compress_batch(slice, rows, cols, &cfg, &cb, rayon_parallelism())
    }).map_err(map_codec_error)?;

    cvs.into_iter().map(|cv| Py::new(py, PyCompressedVector { inner: cv })).collect()
}
```

This lets the batch path parallelize with rayon while other Python
threads keep running.

### Version guard

```python
# tinyquant_rs/__init__.py
from ._core import __version__

MIN_MATCHING_PY = (0, 1, 1)

import tinyquant_cpu as _py
_py_version = tuple(int(x) for x in _py.__version__.split("."))
if _py_version < MIN_MATCHING_PY:
    import warnings
    warnings.warn(
        f"tinyquant_cpu {_py.__version__} is older than the parity target "
        f"{MIN_MATCHING_PY}; behavioral parity is not guaranteed."
    )
```

## Binding 2: `tinyquant-sys` (C ABI)

### Product shape

- Crate: `tinyquant-sys`
- Artifacts: `libtinyquant.so`, `tinyquant.dll`, `libtinyquant.dylib`,
  `libtinyquant.a`, `tinyquant.h`
- Header generator: `cbindgen` with `include_guard = "TINYQUANT_H"`
- Consumers: Rust agents outside the Cargo workspace, C++ glue, any
  language with a C FFI

### ABI contract (from `tinyquant.h`)

```c
#ifndef TINYQUANT_H
#define TINYQUANT_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Version */
#define TINYQUANT_VERSION_MAJOR 0
#define TINYQUANT_VERSION_MINOR 1
#define TINYQUANT_VERSION_PATCH 0

typedef enum TinyQuantErrorKind {
    TQ_OK = 0,
    TQ_ERR_UNSUPPORTED_BIT_WIDTH = 1,
    /* ... same variants as the Rust enum ... */
    TQ_ERR_PANIC = 254,
    TQ_ERR_UNKNOWN = 255,
} TinyQuantErrorKind;

typedef struct TinyQuantError {
    TinyQuantErrorKind kind;
    char* message; /* owned; free with tq_error_free */
} TinyQuantError;

void tq_error_free(TinyQuantError* err);

/* Opaque handles */
typedef struct CodecConfigHandle CodecConfigHandle;
typedef struct CodebookHandle CodebookHandle;
typedef struct CompressedVectorHandle CompressedVectorHandle;
typedef struct CorpusHandle CorpusHandle;
typedef struct BruteForceBackendHandle BruteForceBackendHandle;

/* Thread pool */
TinyQuantErrorKind tq_set_num_threads(uintptr_t n);

/* CodecConfig */
TinyQuantErrorKind tq_codec_config_new(
    uint8_t bit_width,
    uint64_t seed,
    uint32_t dimension,
    bool residual_enabled,
    CodecConfigHandle** out,
    TinyQuantError* err);

void tq_codec_config_free(CodecConfigHandle* handle);
uint8_t  tq_codec_config_bit_width(const CodecConfigHandle* h);
uint64_t tq_codec_config_seed(const CodecConfigHandle* h);
uint32_t tq_codec_config_dimension(const CodecConfigHandle* h);
bool     tq_codec_config_residual_enabled(const CodecConfigHandle* h);

/* Returns a borrowed NUL-terminated string; valid until handle freed. */
const char* tq_codec_config_hash(const CodecConfigHandle* h);

/* Codebook */
TinyQuantErrorKind tq_codebook_train(
    const float* training_vectors,
    uintptr_t rows,
    uintptr_t cols,
    const CodecConfigHandle* config,
    CodebookHandle** out,
    TinyQuantError* err);

void tq_codebook_free(CodebookHandle* handle);

/* Codec single-vector */
TinyQuantErrorKind tq_codec_compress(
    const CodecConfigHandle* config,
    const CodebookHandle* codebook,
    const float* vector,
    uintptr_t vector_len,
    CompressedVectorHandle** out,
    TinyQuantError* err);

TinyQuantErrorKind tq_codec_decompress(
    const CodecConfigHandle* config,
    const CodebookHandle* codebook,
    const CompressedVectorHandle* compressed,
    float* out,               /* caller-allocated, length = dimension */
    uintptr_t out_len,
    TinyQuantError* err);

/* Codec batch */
TinyQuantErrorKind tq_codec_compress_batch(
    const CodecConfigHandle* config,
    const CodebookHandle* codebook,
    const float* vectors,     /* row-major, rows * cols floats */
    uintptr_t rows,
    uintptr_t cols,
    CompressedVectorHandle** out, /* caller: array of `rows` slots */
    TinyQuantError* err);

TinyQuantErrorKind tq_codec_decompress_batch(
    const CodecConfigHandle* config,
    const CodebookHandle* codebook,
    const CompressedVectorHandle* const* compressed, /* array of `rows` */
    uintptr_t rows,
    float* out,               /* row-major, rows * cols */
    uintptr_t out_len,
    TinyQuantError* err);

/* CompressedVector serialization */
TinyQuantErrorKind tq_compressed_vector_to_bytes(
    const CompressedVectorHandle* h,
    uint8_t** out_bytes,      /* allocated by tinyquant; free with tq_bytes_free */
    uintptr_t* out_len,
    TinyQuantError* err);

TinyQuantErrorKind tq_compressed_vector_from_bytes(
    const uint8_t* data,
    uintptr_t data_len,
    CompressedVectorHandle** out,
    TinyQuantError* err);

void tq_bytes_free(uint8_t* ptr, uintptr_t len);
void tq_compressed_vector_free(CompressedVectorHandle* h);

#ifdef __cplusplus
}  /* extern "C" */
#endif

#endif  /* TINYQUANT_H */
```

### Ownership rules (engraved in the header comments)

1. All `*_new`/`*_train`/`*_compress*` functions that return a handle
   transfer ownership to the caller. The caller must call the
   corresponding `_free` function.
2. Strings returned by `tq_codec_config_hash` are borrowed from the
   handle and valid only until the handle is freed. Callers must
   `strdup` if they need to outlive the handle.
3. Byte buffers returned by `tq_compressed_vector_to_bytes` are owned
   by the caller and must be freed with `tq_bytes_free` (which knows
   the Rust allocator layout).
4. Caller-provided buffers (`float* out` for decompress) must be
   sized correctly; the function writes exactly `dimension`
   (or `rows * cols`) floats.
5. All handles are `!Send` at the C level — the ABI is single-threaded
   unless the caller explicitly serializes access.

### Panic discipline

Every `extern "C" fn` wraps its body in
`std::panic::catch_unwind(AssertUnwindSafe(|| { ... }))`. A panic
becomes `TQ_ERR_PANIC` with a message captured from the panic
payload. This guarantees no undefined behavior on a Rust panic
crossing the C boundary.

### Header generation

`build.rs` runs `cbindgen` on release builds:

```rust
// tinyquant-sys/build.rs
fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let target_dir = std::path::PathBuf::from(&crate_dir).join("include");
    std::fs::create_dir_all(&target_dir).unwrap();
    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(cbindgen::Config::from_file(
            std::path::Path::new(&crate_dir).join("cbindgen.toml"),
        ).unwrap())
        .generate()
        .expect("cbindgen failed")
        .write_to_file(target_dir.join("tinyquant.h"));
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=src");
}
```

The generated header is committed to source control so that C/C++
consumers can depend on a specific release without running the Rust
build. A CI job regenerates it and fails if the diff is non-empty,
forcing the developer to commit header changes alongside Rust changes.

### ABI stability

- Minor releases (0.1.x → 0.1.y) may add new functions but must not
  remove or rename existing ones.
- Enum discriminants are fixed at the values declared in v0.1.0 and
  are monotonically allocated thereafter.
- Struct layouts for `TinyQuantError` and any future `#[repr(C)]`
  public type are frozen at v1.0.0; v0.x.y is pre-stable and may
  change, but each change is documented in `CHANGELOG.md`.

## Binding 3: `tinyquant-cli` (standalone binary)

Although strictly not an "FFI" target, the CLI is the third release
surface and shares the same design invariants as the pyo3 and C ABI
bindings: it must not reimplement any codec logic, must surface
errors via the same error enums, and must ship identical behavior
on every supported OS/architecture.

### Product shape

- Crate: `tinyquant-cli`
- Binary: `tinyquant` (or `tinyquant.exe` on Windows)
- Dependencies: `tinyquant-core`, `tinyquant-io`,
  `tinyquant-bruteforce`, `clap`, `anyhow`, `tracing`
- Distribution:
  - Source via `cargo install tinyquant-cli` from crates.io
  - Pre-built archives on GitHub Releases per target triple
  - Multi-arch container image on GHCR
    (`ghcr.io/better-with-models/tinyquant:<tag>`)

### Subcommand surface

| Command | Purpose |
|---|---|
| `tinyquant info` | Print version, git commit, rustc, target, enabled features, detected ISA, CPU count |
| `tinyquant codec train` | Train a codebook from a raw FP32 matrix file |
| `tinyquant codec compress` | Compress raw FP32 input into a Level-2 corpus file |
| `tinyquant codec decompress` | Decompress a corpus file back to FP32 |
| `tinyquant corpus ingest` | Ingest raw FP32 vectors into a corpus file with a policy |
| `tinyquant corpus decompress` | Decompress every vector in a corpus file |
| `tinyquant corpus search` | Brute-force search a query against a corpus file |
| `tinyquant verify <file>` | Validate magic, version, dimensions, and checksums |
| `tinyquant --generate-completion {bash,zsh,fish,powershell}` | Shell completions |
| `tinyquant --generate-man` | man(1) page |

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | User error (bad arguments, missing file) |
| 2 | Codec / corpus error (dimension mismatch, bad bit width) |
| 3 | IO error (truncated, bad magic, permission) |
| 4 | Parity or calibration gate failed (used by `verify`) |
| 70 | Internal bug (caught panic) |

### Why ship the CLI alongside the Cargo libraries

Three consumer groups depend on having a prebuilt binary instead of
a library:

1. **Operators and sysadmins** who need to compress a directory of
   embedding files via a shell script. They should not install a
   Rust toolchain.
2. **CI jobs and Dockerfiles** that want a single static binary to
   pre-process training data or validate corpus files.
3. **better-router and nordic-mcp** integration glue that can `exec`
   the binary with a subprocess call rather than linking against
   `tinyquant-sys` directly when the performance is not critical.

The Cargo library crates still cover the high-performance embedded
use case. The CLI is additive; it never replaces them.

## Why three bindings instead of one

The pyo3 binding is the drop-in replacement for `tinyquant_cpu`. The
C ABI is the integration surface for non-Python consumers (the
better-router Rust agents in particular). The CLI is the operator
experience — a single prebuilt binary that runs anywhere. The three
serve disjoint audiences and have disjoint stability commitments;
trying to produce one binding that serves all three would compromise
every one of them.

## See also

- [[design/rust/type-mapping|Type Mapping from Python]]
- [[design/rust/error-model|Error Model]]
- [[design/rust/release-strategy|Release and Versioning]]
- [[design/rust/testing-strategy|Testing Strategy]]
