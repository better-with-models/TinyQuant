---
title: "Python Wrapper Over Rust Core: Distribution Strategies"
tags:
  - research
  - python
  - rust
  - pyo3
  - maturin
  - distribution
  - packaging
  - phase-22
date-created: 2026-04-13
status: research
category: research
---

# Python Wrapper Over Rust Core: Distribution Strategies

> [!info] Question
> How do we ship a Python package that wraps the Rust implementation
> (`tinyquant-core`) and behaves identically to `tinyquant_cpu`, with
> all compiled binaries bundled inside the package and runtime
> architecture selection?

## Context

`tinyquant_cpu` is a pure-Python implementation (hatchling wheel, numpy
only). The Rust port lives in `rust/crates/`, with `tinyquant-py` as a
PyO3 stub scheduled for Phase 22. Phase 22's plan chooses **maturin +
PyO3**, building separate per-platform wheels and letting PyPI/pip
resolve the correct one at install time.

This document surveys the full design space — including the
"all binaries in one package, runtime arch selection" variant the user
specifically asked about — and records trade-offs for each approach.

---

## The Four Approaches

### Approach A: Standard Maturin Wheels (the Phase 22 plan)

**What it is.** Maturin builds one `.whl` per `(platform, Python ABI)`
combination. Each wheel contains a single `.so`/`.pyd` extension module
compiled for that target. PyPI stores all wheels; pip downloads only
the matching one.

```
PyPI index
├── tinyquant_rs-0.1.0-cp312-abi3-manylinux_2_17_x86_64.whl   ← Linux x86
├── tinyquant_rs-0.1.0-cp312-abi3-manylinux_2_28_aarch64.whl  ← Linux arm64
├── tinyquant_rs-0.1.0-cp312-abi3-macosx_10_14_x86_64.whl     ← macOS x86
├── tinyquant_rs-0.1.0-cp312-abi3-macosx_11_0_arm64.whl       ← macOS arm64
└── tinyquant_rs-0.1.0-cp312-abi3-win_amd64.whl               ← Windows x64
```

**Python wrapper layer.** A thin pure-Python shim mirrors the
`tinyquant_cpu` namespace by re-exporting from the compiled extension:

```python
# python/tinyquant_rs/__init__.py
from tinyquant_rs._core import (   # _core = compiled .so/.pyd
    CodecConfig, Codebook, Codec,
    RotationMatrix, CompressedVector,
    compress, decompress,
)
from tinyquant_rs._core import codec, corpus, backend  # sub-packages
```

**Architecture selection.** Happens at install time — pip resolves the
correct wheel from the platform tag in the wheel filename. Zero runtime
overhead. No `platform.machine()` calls in userland.

**Build pipeline (Phase 22 plan).**

```yaml
# .github/workflows/rust-release.yml (excerpt)
strategy:
  matrix:
    include:
      - { runner: ubuntu-22.04,     target: x86_64-unknown-linux-gnu  }
      - { runner: ubuntu-22.04-arm, target: aarch64-unknown-linux-gnu }
      - { runner: macos-13,         target: x86_64-apple-darwin       }
      - { runner: macos-14,         target: aarch64-apple-darwin      }
      - { runner: windows-2022,     target: x86_64-pc-windows-msvc    }
steps:
  - uses: PyO3/maturin-action@v1
    with:
      target: ${{ matrix.target }}
      args: --release --strip --features simd
      manylinux: auto
```

**Pros.**
- Smallest installed size — user gets only one binary.
- Pip's platform-tag resolver is battle-tested.
- Standard `maturin publish` covers the full release lifecycle.
- SIMD features (`simd`, `avx512`) can be gated per wheel with no
  runtime branching in Python.

**Cons.**
- Requires a 5-runner CI matrix per release.
- Offline installs need all wheels pre-downloaded or a private index.
- Users cross-compiling or on unusual platforms (FreeBSD, Alpine musl)
  need `--no-binary` source builds or a separate sdist.

**Verdict.** Best default. Phase 22 is already spec'd this way.

---

### Approach B: Fat Wheel (All Binaries, Runtime Selection)

**What it is.** A single `.whl` ships the compiled extensions for every
platform as data files. An import hook inspects `sys.platform` and
`platform.machine()` at import time and loads the matching `.so`/`.pyd`.

```
tinyquant_rs-0.1.0-py3-none-any.whl   ← platform-neutral tag
└── tinyquant_rs/
    ├── __init__.py                    ← runtime selector
    ├── _lib/
    │   ├── linux_x86_64/
    │   │   └── _core.so
    │   ├── linux_aarch64/
    │   │   └── _core.so
    │   ├── macos_x86_64/
    │   │   └── _core.dylib
    │   ├── macos_arm64/
    │   │   └── _core.dylib
    │   └── win_amd64/
    │       └── _core.pyd
    └── codec/
        └── __init__.py
```

**Runtime selector (sketch).**

```python
# tinyquant_rs/__init__.py
import importlib
import platform
import sys
from pathlib import Path

def _load_core() -> types.ModuleType:
    arch = platform.machine().lower()
    plat = sys.platform
    key = {
        ("linux",  "x86_64"):  "linux_x86_64",
        ("linux",  "aarch64"): "linux_aarch64",
        ("darwin", "x86_64"):  "macos_x86_64",
        ("darwin", "arm64"):   "macos_arm64",
        ("win32",  "amd64"):   "win_amd64",
    }.get((plat if plat != "linux" else "linux", arch))
    if key is None:
        raise ImportError(f"No pre-built binary for {plat}/{arch}")
    lib_dir = Path(__file__).parent / "_lib" / key
    spec = importlib.util.spec_from_file_location(
        "tinyquant_rs._core",
        lib_dir / ("_core.pyd" if plat == "win32" else "_core.so"),
    )
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod

_core = _load_core()
```

**Fat wheel assembly (CI sketch).** Build all 5 wheels in a matrix,
then run a post-processing step:

```bash
# Runs after all per-platform wheels are built and downloaded as artifacts
python scripts/assemble_fat_wheel.py \
    --linux-x86  dist/tinyquant_rs-*-linux_x86_64.whl \
    --linux-arm  dist/tinyquant_rs-*-linux_aarch64.whl \
    --macos-x86  dist/tinyquant_rs-*-macosx_x86_64.whl \
    --macos-arm  dist/tinyquant_rs-*-macosx_arm64.whl  \
    --win-amd64  dist/tinyquant_rs-*-win_amd64.whl     \
    --out        dist/tinyquant_rs-0.1.0-py3-none-any.whl
```

The assembler script:
1. Unzips each platform wheel.
2. Extracts the `.so`/`.pyd` from each.
3. Places it under `_lib/<platform_key>/`.
4. Injects the runtime selector `__init__.py`.
5. Repacks with wheel tag `py3-none-any`.

**Pros.**
- Single file to distribute, cache, and install offline.
- No pip index needed — `pip install ./tinyquant_rs-0.1.0-py3-none-any.whl`
  works everywhere.
- Useful for air-gapped enterprise environments.
- Docker `COPY` of one file covers all base images.

**Cons.**
- Wheel is large: ~5× the per-platform size (typically 2–10 MB → 10–50 MB).
- PyPI upload size limit is 100 MB per file (currently fine for TinyQuant,
  but will grow with more SIMD variants).
- `py3-none-any` tag bypasses pip's ABI compatibility check — the wrong
  binary being loaded silently on a misconfigured system is a real failure
  mode.
- `platform.machine()` strings are inconsistent across OS versions
  (`x86_64` vs `AMD64` on Windows, `arm64` vs `aarch64` on macOS vs Linux).
  The selector must normalize all known aliases.
- SIMD runtime dispatch (AVX2 vs scalar fallback) can't be done per-wheel;
  needs a second layer of runtime CPUID detection inside the extension.
- Harder to audit: installing 5 binaries to use 1 is a security concern in
  some orgs.

**Known real-world examples.** HuggingFace `tokenizers` ≤ 0.10 used this
pattern. `sentencepiece` Python wheels have historically bundled large
native libs. The approach has largely been superseded by per-platform
wheels on modern pip + PyPI.

**Verdict.** Useful for offline/enterprise scenarios. Not recommended as
the primary distribution mechanism for a public PyPI package.

---

### Approach C: ctypes / CFFI Over `tinyquant-sys` cdylib

**What it is.** Skip PyO3 entirely. Build `tinyquant-sys` as a `cdylib`
for each platform. Ship the `.so`/`.dll`/`.dylib` as package data.
A pure-Python wrapper calls into the C ABI via `ctypes` or `cffi`.

```
tinyquant_rs/
├── __init__.py         ← Python API + ctypes loader
├── _lib/
│   ├── libtinyquant.so      (Linux)
│   ├── libtinyquant.dylib   (macOS)
│   └── tinyquant.dll        (Windows)
└── tinyquant.h         (for cffi parsing)
```

**ctypes loader sketch.**

```python
import ctypes, sys, platform
from pathlib import Path

_lib_map = {
    "linux":  "libtinyquant.so",
    "darwin": "libtinyquant.dylib",
    "win32":  "tinyquant.dll",
}
_lib = ctypes.CDLL(
    str(Path(__file__).parent / "_lib" / _lib_map[sys.platform])
)

# Bind signatures from tinyquant-sys C ABI
_lib.tinyquant_version.restype  = ctypes.c_uint32
_lib.tinyquant_codec_compress.argtypes = [...]  # (const float*, size_t, ...) → TQResult
_lib.tinyquant_codec_compress.restype  = ctypes.c_int

def compress(vector, config, codebook):
    ...  # marshal numpy array → c_float_p, call _lib, unmarshal result
```

**Pros.**
- Python wrapper is pure Python; no Rust compilation needed for the
  wrapper layer.
- The C ABI is stable across Rust compiler versions — wheels don't break
  when rustc is updated.
- `cffi` with ABI mode can parse the `tinyquant.h` header directly,
  reducing binding maintenance.
- Interoperable with C/C++ downstream consumers too.

**Cons.**
- Marshalling overhead: every `compress()` call copies numpy arrays
  through ctypes. For batch operations this can be significant.
- `tinyquant-sys` is a Phase 11 stub — it needs to expose a stable,
  complete C API covering all the types Phase 22 needs. That is a
  significant design effort (opaque handles, error codes, memory
  ownership).
- Error propagation through C means integer error codes + message
  buffers, not rich Python exceptions.
- No GIL release possible from the Python side (only the C library can
  release it if it knows about Python).
- ctypes has no numpy zero-copy path; cffi with buffer protocol is
  possible but complex.

**Verdict.** Good for non-Python consumers (C, C++, Go via cgo). Not
the right primary Python binding strategy when PyO3 is already in the
tree.

---

### Approach D: Subprocess / Binary Delegation

**What it is.** Ship the `tinyquant` CLI binary for each platform.
Python calls it via `subprocess.run()` with JSON or binary stdio.

**Pros.**
- Zero FFI complexity. Any language can integrate.
- Binary can be pre-built and cached independently of Python.

**Cons.**
- Per-call subprocess overhead (fork + exec + IPC) is unacceptable for
  hot-path compression of millions of vectors.
- Streaming large numpy arrays through stdin/stdout adds I/O cost.
- Not a viable replacement for the in-process Python API.

**Verdict.** Ruled out for the core API. Acceptable for CLI tooling
(`tinyquant corpus search`, `tinyquant codec train`), which Phase 22
already plans.

---

## Comparison Matrix

| Criterion | A: Standard maturin | B: Fat wheel | C: ctypes/cffi | D: subprocess |
|---|---|---|---|---|
| Install size | ✅ Small (1 binary) | ⚠ Large (5× per platform) | ✅ Small | ✅ Small |
| Offline deploy | ⚠ Needs all wheels | ✅ Single file | ✅ Single file | ✅ Single file |
| Runtime perf | ✅ Zero overhead | ✅ Zero overhead | ⚠ Marshal cost | ❌ Fork cost |
| GIL release | ✅ PyO3 `allow_threads` | ✅ PyO3 `allow_threads` | ❌ Not from Python | N/A |
| Numpy zero-copy | ✅ `PyReadonlyArray` | ✅ `PyReadonlyArray` | ⚠ cffi buffer | ❌ |
| Type safety | ✅ PyO3 `#[pyclass]` | ✅ PyO3 `#[pyclass]` | ⚠ Manual ctypes | ❌ |
| SIMD per-arch | ✅ Separate wheels | ⚠ Runtime CPUID | ✅ Separate libs | ✅ |
| PyPI-friendly | ✅ Standard | ⚠ Non-standard tag | ✅ Standard | ✅ |
| Phase 22 alignment | ✅ Already specced | ⚠ Additive | ❌ Different stack | ❌ |
| Security surface | ✅ One binary | ⚠ 5 binaries installed | ✅ One library | ✅ |

---

## Recommended Approach for TinyQuant

**Primary: Approach A (standard maturin wheels).**

This is already the Phase 22 design. It is correct for a public PyPI
package. The platform-tag resolver in pip is exactly the right layer for
architecture selection — it happens before a byte is downloaded, not at
import time.

**Additive: Approach B (fat wheel) as an enterprise artifact.**

Produce the fat wheel as a secondary CI artifact alongside the standard
wheels. It is not uploaded to PyPI but is attached to the GitHub Release
as `tinyquant_rs-<version>-py3-none-any.whl`. Enterprise users with
air-gapped environments or Docker layer caching constraints get a single
file that works on all supported platforms.

The assembly script is ~100 lines of Python and runs in the release
workflow after all per-platform wheels are built.

**Not recommended: Approach C or D for the primary Python binding.**

`tinyquant-sys` should stay focused on C/C++/non-Python consumers.
`tinyquant-py` is the right PyO3 layer, and PyO3 provides capabilities
(zero-copy numpy, GIL release, rich exceptions) that ctypes cannot
match.

---

## Implementation Notes for Phase 22

### The Python wrapper layer

The wrapper lives at `rust/crates/tinyquant-py/python/tinyquant_rs/`.
It should be a thin re-export shim, not a reimplementation:

```
tinyquant_rs/
├── __init__.py        ← version, re-export top-level names
├── codec/
│   └── __init__.py   ← re-export from _core.codec
├── corpus/
│   └── __init__.py   ← re-export from _core.corpus
└── backend/
    └── __init__.py   ← re-export from _core.backend
```

All logic lives in `_core` (the compiled extension). The pure-Python
files exist only to mirror the `tinyquant_cpu` sub-package layout so
`import tinyquant_rs.codec.CodecConfig` works identically to
`import tinyquant_cpu.codec.CodecConfig`.

### Parity test pattern

```python
# rust/crates/tinyquant-py/tests/python/test_parity.py
import numpy as np
import tinyquant_cpu as py_impl
import tinyquant_rs  as rs_impl

@pytest.mark.parametrize("bw,seed,dim", [(4, 42, 768), (2, 0, 64), (8, 999, 1536)])
def test_config_hash_parity(bw, seed, dim):
    py_cfg = py_impl.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
    rs_cfg = rs_impl.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
    assert py_cfg.config_hash == rs_cfg.config_hash

def test_round_trip_parity(trained_fixture):
    """compress with Python impl, decompress with Rust impl and vice versa."""
    vec, py_cfg, py_cb = trained_fixture
    rs_cfg = rs_impl.codec.CodecConfig(
        bit_width=py_cfg.bit_width, seed=py_cfg.seed, dimension=py_cfg.dimension
    )
    # Python → bytes → Rust
    py_cv = py_impl.codec.compress(vec, py_cfg, py_cb)
    raw = py_cv.to_bytes()
    rs_cv = rs_impl.codec.CompressedVector.from_bytes(raw)
    rs_cb = rs_impl.codec.Codebook(entries=py_cb.entries, bit_width=py_cfg.bit_width)
    reconstructed = rs_impl.codec.decompress(rs_cv, rs_cfg, rs_cb)
    np.testing.assert_allclose(reconstructed, vec, atol=1e-3)
```

### maturin pyproject.toml (in `rust/crates/tinyquant-py/`)

```toml
[build-system]
requires = ["maturin>=1.5,<2"]
build-backend = "maturin"

[project]
name = "tinyquant-rs"
requires-python = ">=3.12"
dependencies = ["numpy>=1.26"]

[tool.maturin]
python-source = "python"
module-name   = "tinyquant_rs._core"
bindings      = "pyo3"
features      = ["pyo3/extension-module", "pyo3/abi3-py312"]
strip         = true
```

The `abi3-py312` feature instructs PyO3 to use the stable ABI
(`Py_LIMITED_API`), so one wheel serves all Python 3.12+ releases
without rebuilding per minor version.

### Fat wheel assembly script location

`scripts/packaging/assemble_fat_wheel.py` — runs as a release-workflow
step after all per-platform wheel artifacts are downloaded. The output
wheel is uploaded as a GitHub Release asset but **not** pushed to PyPI.

### `platform.machine()` normalization table

```python
_ARCH_ALIASES = {
    "x86_64":  "x86_64",
    "amd64":   "x86_64",   # Windows
    "AMD64":   "x86_64",   # Windows sys.version_info
    "aarch64": "aarch64",
    "arm64":   "aarch64",  # macOS
}
```

Without this, `platform.machine()` returns `AMD64` on Windows and
`arm64` on macOS Apple Silicon — both different from the Linux strings
— and a naive lookup will fall through to `ImportError`.

---

## Runtime SIMD Dispatch (Within a Single Binary)

Even with Approach A (one wheel per platform), a user on x86_64 may
have AVX2 or only SSE4. The current Phase 20 SIMD gate is compile-time
(`#[cfg(target_feature = "avx2")]`). For dynamic dispatch within one
binary the options are:

1. **`multiversion` crate** — `#[multiversion(targets(...))]` generates
   multiple codepaths at compile time, dispatches via CPUID at first
   call. Zero Python surface change.
2. **Multiple `features` wheels** — ship `avx2` and `scalar` wheels,
   use `[tool.maturin] features = ["simd"]` per target. Already done in
   the Phase 22 matrix.
3. **`is_x86_feature_detected!` + `std::arch`** — hand-rolled dispatch
   in Rust, already used in tinyquant-core Phase 20 kernels.

Option 3 is already in place. Option 1 is the cleanest long-term path
if Phase 23 introduces AVX-512.

---

## Sources

- PyO3 documentation: https://pyo3.rs/
- Maturin documentation: https://www.maturin.rs/
- PEP 425 — Compatibility Tags for Built Distributions
- PEP 517 / PEP 518 — Build system interface
- `rust/crates/tinyquant-py/Cargo.toml` — current PyO3 stub
- `rust/crates/tinyquant-py/src/lib.rs` — Phase 22 starting point
- `docs/plans/rust/phase-22-pyo3-cabi-release.md` — Phase 22 spec
- `docs/design/rust/ffi-and-bindings` — FFI design notes
