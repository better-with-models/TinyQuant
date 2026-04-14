---
title: "Phase 24: Python Fat-Wheel Official Distribution"
tags:
  - plans
  - rust
  - phase-24
  - python
  - distribution
  - fat-wheel
  - packaging
date-created: 2026-04-13
status: in-progress
category: planning
---

# Phase 24: Python Fat-Wheel Official Distribution

> [!info] Goal
> Replace the shipped `tinyquant-cpu` PyPI package with a **fat wheel**
> that bundles Rust-compiled extensions for every Tier-1 target and
> selects the correct binary at import time. The public import surface
> and module layout of `tinyquant_cpu` are preserved bit-for-bit so
> downstream code requires **no source-level changes** and no rename.
> The pure-Python implementation demoted by [[plans/rust/phase-23-python-reference-demotion|Phase 23]] becomes
> the parity oracle for this phase, and the per-arch maturin wheels
> published by [[plans/rust/phase-22-pyo3-cabi-release|Phase 22]]
> become the build inputs.

> [!note] Design reference
> See [[research/python-rust-wrapper-distribution|Distribution Research]]
> §Approach B for the full trade-off analysis. That document concluded
> that fat wheels are *not* recommended as the primary PyPI mechanism
> for greenfield packages — but the `tinyquant-cpu` name is already
> taken and already in use by downstream consumers, and the
> zero-rename requirement forces the fat-wheel shape. The research
> §Cons list in that document reappears here as §Risks.

> [!warning] Prerequisites
> - **Phase 22 complete.** `tinyquant-rs` per-arch abi3 wheels are
>   green on all 5 Tier-1 runners
>   ({linux-x86_64, linux-aarch64, macos-x86_64, macos-arm64, win-amd64})
>   and published to PyPI under the transitional `tinyquant-rs` name.
> - **Phase 23 complete.** `src/tinyquant_cpu/` is empty of
>   implementation code; the reference moved to
>   `tests/reference/tinyquant_py_reference/`. A `build-package-does-not-leak-reference`
>   CI job exists.
> - **PyPI trusted publishing** configured for `tinyquant-cpu`
>   (OIDC; no stored API token).
> - **`tinyquant_rs._core`** extension exports exactly the name set
>   enumerated in §Shim-layer contract. Any drift breaks the shim.
> - `tests/parity/test_cross_impl.py` (scaffolded in Phase 23) is
>   wired to an `rs` fixture that currently points at `tinyquant_rs`;
>   this phase flips the fixture to the fat wheel once assembled.

## Distribution surface — before vs after

| Artifact | Before (v0.1.1) | After (v0.2.0) |
|---|---|---|
| PyPI name | `tinyquant-cpu` | `tinyquant-cpu` (unchanged) |
| Wheel tag | `py3-none-any` (pure-Python) | `py3-none-any` (fat — 5 binaries) |
| Backing impl | Pure-Python (numpy only) | Rust via `_core.{so,dylib,pyd}` selected at import |
| Install size | ~80 KB | ~10–25 MB (estimated; PyPI hard cap 100 MB) |
| Top-level import | `import tinyquant_cpu` | `import tinyquant_cpu` (byte-identical public names) |
| Sub-packages | `codec/`, `corpus/`, `backend/` (Python source) | `codec/`, `corpus/`, `backend/` (shim re-exports from `_core`) |
| Python requirement | `>=3.12` | `>=3.12` (stable ABI `abi3-py312`) |
| NumPy dependency | `numpy>=1.26` | `numpy>=1.26` (zero-copy via `PyReadonlyArray`) |
| Source-build fallback | N/A (pure Python) | sdist includes build script + Rust workspace pointer |
| Coexists with v0.1.x | N/A | Yes — v0.1.1 stays on PyPI; v0.2.0 is additive |
| Bundles pure-Python reference | Yes (it *is* the package) | **No** — reference lives in `tests/reference/` only |

> [!note] Why keep `py3-none-any`?
> Per [[research/python-rust-wrapper-distribution|Distribution Research]],
> a platform-neutral wheel tag is what makes one-file-covers-all possible.
> The trade-off is that pip's platform-tag resolver no longer guards
> against mismatched binaries — the runtime selector in `_selector.py`
> becomes the load-bearing component and must fail **loudly** on
> unsupported hosts (see §Runtime selector implementation).

## Fat wheel anatomy

Exact on-disk layout of `tinyquant_cpu-0.2.0-py3-none-any.whl`
(decompressed view — the `.whl` itself is a zip):

```
tinyquant_cpu-0.2.0-py3-none-any.whl
├── tinyquant_cpu/
│   ├── __init__.py                              (~2 KB)   runtime selector entry point
│   ├── _selector.py                             (~6 KB)   platform detection + loader
│   ├── _lib/
│   │   ├── linux_x86_64_gnu/
│   │   │   └── _core.abi3.so                    (~3.5 MB) PyO3 extension, manylinux_2_17
│   │   ├── linux_x86_64_musl/                              [optional; see Open questions]
│   │   │   └── _core.abi3.so                    (~3.8 MB) musllinux_1_2, built on alpine
│   │   ├── linux_aarch64_gnu/
│   │   │   └── _core.abi3.so                    (~3.4 MB) manylinux_2_28
│   │   ├── macos_x86_64/
│   │   │   └── _core.abi3.so                    (~3.2 MB) macOS x86_64, `.so` suffix on darwin
│   │   ├── macos_arm64/
│   │   │   └── _core.abi3.so                    (~3.0 MB) macOS Apple Silicon
│   │   └── win_amd64/
│   │       └── _core.pyd                        (~3.3 MB) Windows x86_64
│   ├── codec/
│   │   └── __init__.py                          (~1 KB)   re-exports from _core.codec
│   ├── corpus/
│   │   └── __init__.py                          (~1 KB)   re-exports from _core.corpus
│   ├── backend/
│   │   └── __init__.py                          (~1 KB)   re-exports from _core.backend
│   └── py.typed                                 (0 B)     PEP 561 marker
├── tinyquant_cpu-0.2.0.dist-info/
│   ├── METADATA                                 (~4 KB)   PEP 621-rendered project metadata
│   ├── RECORD                                   (~3 KB)   file list + sha256 + size
│   ├── WHEEL                                    (~0.2 KB) build-tool tag, wheel format version
│   ├── LICENSE                                  (~11 KB)  Apache-2.0 full text
│   └── entry_points.txt                         (empty)   reserved for future CLI hooks
└── [zip central directory]

total uncompressed: ~17 MB   (5 binaries plus ~25 KB of Python)
total compressed:   ~12 MB   (zip DEFLATE; binaries already stripped)
```

> [!warning] PyPI file-size ceiling
> PyPI imposes a **100 MB hard limit** per uploaded file; files over
> 60 MB emit warnings and may need an explicit waiver. With five
> stripped abi3 extensions at ~3.5 MB each we are comfortably under,
> but AVX-512 or a `fat` SIMD variant would push closer to the cap.
> See §Open questions §SIMD dispatch for the escape hatch.

### `METADATA` (dist-info)

```text
Metadata-Version: 2.3
Name: tinyquant-cpu
Version: 0.2.0
Summary: CPU-only vector quantization codec for embedding storage compression (Rust-backed)
Author-email: Better With Models <ops@betterwithmodels.example>
License-Expression: Apache-2.0
Requires-Python: >=3.12
Classifier: Development Status :: 4 - Beta
Classifier: Programming Language :: Python :: 3
Classifier: Programming Language :: Python :: 3 :: Only
Classifier: Programming Language :: Python :: 3.12
Classifier: Programming Language :: Python :: 3.13
Classifier: Programming Language :: Rust
Classifier: License :: OSI Approved :: Apache Software License
Classifier: Operating System :: POSIX :: Linux
Classifier: Operating System :: MacOS
Classifier: Operating System :: Microsoft :: Windows
Classifier: Topic :: Scientific/Engineering :: Artificial Intelligence
Classifier: Typing :: Typed
Requires-Dist: numpy>=1.26
Project-URL: Homepage, https://github.com/better-with-models/TinyQuant
Project-URL: Changelog, https://github.com/better-with-models/TinyQuant/blob/main/CHANGELOG.md
Description-Content-Type: text/markdown
```

### `WHEEL` (dist-info)

```text
Wheel-Version: 1.0
Generator: tinyquant-fat-wheel-assembler 0.1.0
Root-Is-Purelib: true
Tag: py3-none-any
```

> [!note] `Root-Is-Purelib: true` is technically inaccurate (we ship
> compiled binaries) but is required because the `py3-none-any` tag
> declares the wheel as purelib. This is the documented trade-off for
> fat wheels. `pip` routes the install to `site-packages/` either way.

### `RECORD` (dist-info)

Line format (PEP 376 + PEP 427):

```text
tinyquant_cpu/__init__.py,sha256=<b64>,<bytes>
tinyquant_cpu/_selector.py,sha256=<b64>,<bytes>
tinyquant_cpu/_lib/linux_x86_64_gnu/_core.abi3.so,sha256=<b64>,<bytes>
...
tinyquant_cpu-0.2.0.dist-info/METADATA,sha256=<b64>,<bytes>
tinyquant_cpu-0.2.0.dist-info/RECORD,,
```

The `RECORD` file itself is listed with empty hash and size fields —
that is the PEP 376 convention. Naive repack scripts that recompute
sha256 for every line including `RECORD` will produce an unparseable
wheel: `pip install` fails with `BadZipFile` or silently installs with
a broken `pip show --files` listing. See §Fat wheel assembler script
for the correct algorithm.

## Runtime selector implementation

### `tinyquant_cpu/_selector.py` — full module

```python
"""Platform detection and _core extension loader for the fat wheel.

This module is imported exactly once, by `tinyquant_cpu/__init__.py`,
before any user code touches `tinyquant_cpu.codec`, `.corpus`, or
`.backend`. Its job is to detect the running host, load the matching
pre-built extension from `_lib/<key>/`, and register it as
`tinyquant_cpu._core` in `sys.modules` so the sub-package shims can
re-export from it.

Every failure path raises `ImportError` with a diagnostic message
naming the detected `(sys.platform, machine, libc)` tuple and pointing
at the sdist source-build instructions.
"""

from __future__ import annotations

import hashlib
import importlib.util
import platform
import sys
import sysconfig
import types
from pathlib import Path

__all__ = ["load_core", "detect_platform_key", "UnsupportedPlatformError"]


class UnsupportedPlatformError(ImportError):
    """Raised when no pre-built binary exists for the running host."""


# Canonical platform keys used as _lib/<key>/ directory names.
_LINUX_GNU_X86_64 = "linux_x86_64_gnu"
_LINUX_MUSL_X86_64 = "linux_x86_64_musl"
_LINUX_GNU_AARCH64 = "linux_aarch64_gnu"
_MACOS_X86_64 = "macos_x86_64"
_MACOS_ARM64 = "macos_arm64"
_WIN_AMD64 = "win_amd64"

# Normalise the wildly inconsistent machine() strings across OSes.
# Windows returns "AMD64"; macOS Apple Silicon returns "arm64";
# Linux returns "aarch64". All three map to the same binary family.
_ARCH_ALIASES: dict[str, str] = {
    "x86_64": "x86_64",
    "amd64": "x86_64",
    "AMD64": "x86_64",
    "i686": "x86_64",   # 32-bit hosts are NOT supported; falls through below
    "aarch64": "aarch64",
    "arm64": "aarch64",
}

# Extension suffix per OS. Note: macOS PyO3 wheels use `.so` (NOT .dylib)
# because CPython's importer looks for `.so` on all POSIX platforms.
_EXT_SUFFIX: dict[str, str] = {
    "linux": ".abi3.so",
    "darwin": ".abi3.so",
    "win32": ".pyd",
}


def _detect_libc() -> str:
    """Return 'gnu' or 'musl' on Linux, empty string elsewhere.

    Detection order (first match wins):
      1. `platform.libc_ver()` returns a non-empty tuple for glibc.
      2. `/etc/alpine-release` exists -> musl.
      3. The `SOABI` sysconfig value contains 'musl' -> musl.
      4. Fallback: 'gnu'.
    """
    if sys.platform != "linux":
        return ""
    libc_name, _libc_ver = platform.libc_ver()
    if libc_name == "glibc":
        return "gnu"
    if Path("/etc/alpine-release").exists():
        return "musl"
    # Fallback probe via the CPython config; auditwheel tags the
    # interpreter itself with the libc family on manylinux/musllinux.
    soabi = sysconfig.get_config_var("SOABI") or ""
    if "musl" in soabi:
        return "musl"
    return "gnu"


def detect_platform_key() -> str:
    """Return the `_lib/<key>/` directory name for the running host.

    Raises UnsupportedPlatformError if no key matches.
    """
    plat = sys.platform
    raw_machine = platform.machine()
    machine = _ARCH_ALIASES.get(raw_machine)

    if machine is None:
        raise UnsupportedPlatformError(
            f"tinyquant_cpu: no pre-built binary for machine "
            f"{raw_machine!r} on {plat!r}. Supported machines: "
            f"{sorted(set(_ARCH_ALIASES.values()))}. "
            f"Build from source: "
            f"https://github.com/better-with-models/TinyQuant#building-from-source"
        )

    if plat == "linux":
        libc = _detect_libc()
        if machine == "x86_64":
            return _LINUX_MUSL_X86_64 if libc == "musl" else _LINUX_GNU_X86_64
        if machine == "aarch64":
            if libc == "musl":
                raise UnsupportedPlatformError(
                    "tinyquant_cpu: musllinux aarch64 is not in the fat "
                    "wheel. Install from sdist with `pip install "
                    "--no-binary tinyquant-cpu tinyquant-cpu`."
                )
            return _LINUX_GNU_AARCH64

    if plat == "darwin":
        if machine == "x86_64":
            return _MACOS_X86_64
        if machine == "aarch64":
            return _MACOS_ARM64

    if plat == "win32":
        if machine == "x86_64":
            return _WIN_AMD64

    raise UnsupportedPlatformError(
        f"tinyquant_cpu: no pre-built binary for "
        f"(platform={plat!r}, machine={raw_machine!r}). "
        f"Supported tuples: linux/x86_64 (gnu,musl), linux/aarch64 (gnu), "
        f"darwin/x86_64, darwin/arm64, win32/amd64."
    )


def _ext_filename(plat: str) -> str:
    try:
        return "_core" + _EXT_SUFFIX[plat]
    except KeyError as exc:
        raise UnsupportedPlatformError(
            f"tinyquant_cpu: unknown extension suffix for sys.platform={plat!r}"
        ) from exc


def _verify_magic(path: Path) -> None:
    """Lightweight corruption guard: check the first bytes match the
    expected binary magic for the platform. Not cryptographic - just
    catches the class of partial downloads and truncated wheels."""
    with path.open("rb") as fh:
        head = fh.read(4)
    if sys.platform == "linux":
        if not head.startswith(b"\x7fELF"):
            raise UnsupportedPlatformError(
                f"tinyquant_cpu: {path} is not a valid ELF binary "
                f"(head={head!r}). Reinstall: "
                f"`pip install --force-reinstall tinyquant-cpu`."
            )
    elif sys.platform == "darwin":
        # Mach-O magic: 0xfeedface / 0xfeedfacf / 0xcafebabe (fat)
        if head not in (
            b"\xfe\xed\xfa\xce", b"\xce\xfa\xed\xfe",
            b"\xfe\xed\xfa\xcf", b"\xcf\xfa\xed\xfe",
            b"\xca\xfe\xba\xbe",
        ):
            raise UnsupportedPlatformError(
                f"tinyquant_cpu: {path} is not a valid Mach-O binary "
                f"(head={head!r})."
            )
    elif sys.platform == "win32":
        if not head.startswith(b"MZ"):
            raise UnsupportedPlatformError(
                f"tinyquant_cpu: {path} is not a valid PE binary "
                f"(head={head!r})."
            )


def load_core() -> types.ModuleType:
    """Detect the platform, locate the matching extension, load it,
    and register it as `tinyquant_cpu._core` in `sys.modules`.

    Returns the loaded module. Idempotent: subsequent calls return the
    already-loaded module instance.
    """
    already = sys.modules.get("tinyquant_cpu._core")
    if already is not None:
        return already

    key = detect_platform_key()
    ext = _ext_filename(sys.platform)
    here = Path(__file__).resolve().parent
    lib_path = here / "_lib" / key / ext

    if not lib_path.is_file():
        raise UnsupportedPlatformError(
            f"tinyquant_cpu: detected platform key {key!r} but the "
            f"expected binary {lib_path} is missing. The fat wheel "
            f"may have been repackaged or the install is corrupt. "
            f"Reinstall with `pip install --force-reinstall tinyquant-cpu`."
        )

    _verify_magic(lib_path)

    spec = importlib.util.spec_from_file_location(
        "tinyquant_cpu._core",
        str(lib_path),
        submodule_search_locations=None,
    )
    if spec is None or spec.loader is None:
        raise UnsupportedPlatformError(
            f"tinyquant_cpu: importlib could not create a spec for {lib_path}"
        )
    module = importlib.util.module_from_spec(spec)
    sys.modules["tinyquant_cpu._core"] = module
    try:
        spec.loader.exec_module(module)
    except Exception:
        # Roll back on failure so a retry sees a clean sys.modules.
        sys.modules.pop("tinyquant_cpu._core", None)
        raise
    return module
```

### `tinyquant_cpu/__init__.py` — new top of package

```python
"""TinyQuant: CPU-only vector quantization codec (Rust-backed fat wheel)."""

from tinyquant_cpu._selector import load_core as _load_core

_core = _load_core()     # side effect: registers tinyquant_cpu._core

__version__ = _core.__version__

__all__ = ["__version__"]
```

> [!info] Contract
> `_load_core()` is the **only** import-time work done by
> `tinyquant_cpu/__init__.py`. All sub-packages then re-export from
> `tinyquant_cpu._core`. If the selector fails, `import tinyquant_cpu`
> raises `ImportError` before any sub-module can be referenced —
> matching the behavior documented in the [[research/python-rust-wrapper-distribution|Distribution Research]]
> §Approach B "loud fail" principle.

## Shim-layer contract

These three shim files must re-export **exactly** the names exposed by
the pure-Python reference in [[plans/rust/phase-23-python-reference-demotion|Phase 23]].
Any drift is caught by the parity suite. The canonical name set
(enumerated against `src/tinyquant_cpu/{codec,corpus,backend}/__init__.py`
as of v0.1.1):

### `tinyquant_cpu/codec/__init__.py`

```python
"""TinyQuant codec: compression and decompression primitives (Rust-backed)."""

import sys
# Ensure the extension is loaded before we pull sub-attrs off it.
import tinyquant_cpu  # noqa: F401  -- triggers _selector.load_core()

_core_codec = sys.modules["tinyquant_cpu._core"].codec

CodebookIncompatibleError = _core_codec.CodebookIncompatibleError
ConfigMismatchError       = _core_codec.ConfigMismatchError
DimensionMismatchError    = _core_codec.DimensionMismatchError
DuplicateVectorError      = _core_codec.DuplicateVectorError
Codebook                  = _core_codec.Codebook
Codec                     = _core_codec.Codec
CodecConfig               = _core_codec.CodecConfig
CompressedVector          = _core_codec.CompressedVector
RotationMatrix            = _core_codec.RotationMatrix
compress                  = _core_codec.compress
decompress                = _core_codec.decompress

__all__ = [
    "Codebook",
    "CodebookIncompatibleError",
    "Codec",
    "CodecConfig",
    "CompressedVector",
    "ConfigMismatchError",
    "DimensionMismatchError",
    "DuplicateVectorError",
    "RotationMatrix",
    "compress",
    "decompress",
]
```

### `tinyquant_cpu/corpus/__init__.py`

```python
"""TinyQuant corpus: aggregate root and vector lifecycle (Rust-backed)."""

import sys
import tinyquant_cpu  # noqa: F401

_core_corpus = sys.modules["tinyquant_cpu._core"].corpus

CompressionPolicy                     = _core_corpus.CompressionPolicy
CompressionPolicyViolationDetected    = _core_corpus.CompressionPolicyViolationDetected
Corpus                                = _core_corpus.Corpus
CorpusCreated                         = _core_corpus.CorpusCreated
CorpusDecompressed                    = _core_corpus.CorpusDecompressed
VectorEntry                           = _core_corpus.VectorEntry
VectorsInserted                       = _core_corpus.VectorsInserted

__all__ = [
    "CompressionPolicy",
    "CompressionPolicyViolationDetected",
    "Corpus",
    "CorpusCreated",
    "CorpusDecompressed",
    "VectorEntry",
    "VectorsInserted",
]
```

### `tinyquant_cpu/backend/__init__.py`

```python
"""TinyQuant backend: search protocol and adapters (Rust-backed)."""

import sys
import tinyquant_cpu  # noqa: F401

_core_backend = sys.modules["tinyquant_cpu._core"].backend

BruteForceBackend = _core_backend.BruteForceBackend
SearchBackend     = _core_backend.SearchBackend
SearchResult      = _core_backend.SearchResult

__all__ = [
    "BruteForceBackend",
    "SearchBackend",
    "SearchResult",
]
```

### Name-parity audit

A CI job `fatwheel-shim-parity` walks the Phase 23 reference and the
Phase 24 shim in lockstep:

```python
# tests/packaging/test_shim_parity.py
import importlib
import pytest

SUBPKGS = ["codec", "corpus", "backend"]


@pytest.mark.parametrize("subpkg", SUBPKGS)
def test_shim_exports_match_reference(subpkg: str) -> None:
    shim = importlib.import_module(f"tinyquant_cpu.{subpkg}")
    ref  = importlib.import_module(f"tinyquant_py_reference.{subpkg}")
    shim_names = set(shim.__all__)
    ref_names  = set(ref.__all__)
    missing    = ref_names - shim_names
    extra      = shim_names - ref_names
    assert not missing, f"{subpkg}: shim missing names {missing}"
    assert not extra,   f"{subpkg}: shim has extra names {extra}"
```

This is the load-bearing guard against accidental name drift when new
symbols are added to `_core` — both sides of the parity contract must
move together.

## Fat wheel assembler script

Location: `scripts/packaging/assemble_fat_wheel.py`. Runs once, on the
Linux assembly job in `python-fatwheel.yml`. Reuses artifacts produced
by the Phase 22 `rust-release.yml` matrix — **no rebuild** happens
here; this script is pure repackaging.

### CLI contract

```text
usage: assemble_fat_wheel.py [-h]
    --input-dir INPUT_DIR
    --version VERSION
    --output OUTPUT
    [--skip-verify]

required:
  --input-dir   Directory containing exactly 5 per-arch wheel files:
                tinyquant_rs-<ver>-*-linux_x86_64.whl
                tinyquant_rs-<ver>-*-linux_aarch64.whl
                tinyquant_rs-<ver>-*-macosx_*_x86_64.whl
                tinyquant_rs-<ver>-*-macosx_*_arm64.whl
                tinyquant_rs-<ver>-*-win_amd64.whl
  --version     Expected version string (e.g. 0.2.0). Every input
                wheel's version must match; mismatch exits with code 3.
  --output      Destination path for the fat wheel.

optional:
  --skip-verify Skip post-assembly `twine check`. Default: run it.
```

### Outputs

- The fat wheel at `OUTPUT`.
- `OUTPUT.manifest.json` next to it, containing:
  ```json
  {
    "version": "0.2.0",
    "source_wheels": [
      {"path": "tinyquant_rs-0.2.0-cp312-abi3-manylinux_2_17_x86_64.whl",
       "sha256": "<hex>", "size_bytes": 3578112, "platform_key": "linux_x86_64_gnu"}
    ],
    "fat_wheel_sha256": "<hex>",
    "fat_wheel_size_bytes": 17301504,
    "assembler_version": "0.1.0",
    "built_at": "2026-04-13T18:04:11Z"
  }
  ```

### Reference implementation sketch

```python
#!/usr/bin/env python3
"""Fat wheel assembler for tinyquant-cpu (Phase 24).

Consumes per-arch maturin wheels produced by rust-release.yml and
emits a single py3-none-any wheel that bundles all 5 extensions plus
the runtime selector. Produces a correct RECORD (see PEP 376).
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import io
import json
import re
import zipfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

WHEEL_NAME_RE = re.compile(
    r"^tinyquant_rs-(?P<ver>[^-]+)-"
    r"cp\d+-abi3-(?P<plat>[^.]+)\.whl$"
)
PLATFORM_KEY_BY_TAG: dict[str, str] = {
    "manylinux_2_17_x86_64":  "linux_x86_64_gnu",
    "manylinux_2_28_aarch64": "linux_aarch64_gnu",
    "musllinux_1_2_x86_64":   "linux_x86_64_musl",
    "macosx_10_14_x86_64":    "macos_x86_64",
    "macosx_11_0_arm64":      "macos_arm64",
    "win_amd64":              "win_amd64",
}
EXT_BY_KEY: dict[str, str] = {
    "linux_x86_64_gnu":  "_core.abi3.so",
    "linux_x86_64_musl": "_core.abi3.so",
    "linux_aarch64_gnu": "_core.abi3.so",
    "macos_x86_64":      "_core.abi3.so",
    "macos_arm64":       "_core.abi3.so",
    "win_amd64":         "_core.pyd",
}


@dataclass(frozen=True)
class SourceWheel:
    path: Path
    version: str
    platform_key: str
    sha256: str
    size_bytes: int


def _sha256(data: bytes) -> str:
    return "sha256=" + base64.urlsafe_b64encode(
        hashlib.sha256(data).digest()
    ).rstrip(b"=").decode("ascii")


def discover_inputs(input_dir: Path, expected_version: str) -> list[SourceWheel]:
    out: list[SourceWheel] = []
    for whl in sorted(input_dir.glob("tinyquant_rs-*.whl")):
        m = WHEEL_NAME_RE.match(whl.name)
        if not m:
            continue
        ver = m.group("ver")
        if ver != expected_version:
            raise SystemExit(
                f"version mismatch: {whl.name} has {ver!r}, "
                f"expected {expected_version!r}"
            )
        key = PLATFORM_KEY_BY_TAG.get(m.group("plat"))
        if key is None:
            raise SystemExit(f"unknown platform tag in {whl.name}")
        blob = whl.read_bytes()
        out.append(SourceWheel(
            path=whl, version=ver, platform_key=key,
            sha256=_sha256(blob), size_bytes=len(blob),
        ))
    required = {"linux_x86_64_gnu", "linux_aarch64_gnu",
                "macos_x86_64", "macos_arm64", "win_amd64"}
    have = {w.platform_key for w in out}
    missing = required - have
    if missing:
        raise SystemExit(f"missing platform wheels: {sorted(missing)}")
    return out


def extract_core_extension(src_whl: Path, platform_key: str) -> bytes:
    """Read the _core.{abi3.so,pyd} blob out of a per-arch wheel."""
    target_name = EXT_BY_KEY[platform_key]
    with zipfile.ZipFile(src_whl) as zf:
        for info in zf.infolist():
            name = info.filename.rsplit("/", 1)[-1]
            if name == target_name or name.startswith("_core."):
                return zf.read(info)
    raise SystemExit(f"{src_whl}: no _core extension found inside")


def build_fat_wheel(
    sources: list[SourceWheel],
    version: str,
    selector_src: bytes,
    init_src: bytes,
    codec_src: bytes,
    corpus_src: bytes,
    backend_src: bytes,
    metadata_src: bytes,
    wheel_src: bytes,
    license_src: bytes,
    py_typed_src: bytes,
    output: Path,
) -> tuple[str, int]:
    dist_info = f"tinyquant_cpu-{version}.dist-info"
    record_entries: list[tuple[str, str, int]] = []

    def add(zf: zipfile.ZipFile, arcname: str, blob: bytes) -> None:
        # `ZipInfo` with deterministic mtime so the fat wheel is
        # byte-reproducible across CI runs with the same inputs.
        zi = zipfile.ZipInfo(arcname, date_time=(2026, 4, 13, 0, 0, 0))
        zi.compress_type = zipfile.ZIP_DEFLATED
        zi.external_attr = (0o644 << 16)
        zf.writestr(zi, blob)
        record_entries.append((arcname, _sha256(blob), len(blob)))

    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as zf:
        add(zf, "tinyquant_cpu/__init__.py",        init_src)
        add(zf, "tinyquant_cpu/_selector.py",       selector_src)
        add(zf, "tinyquant_cpu/py.typed",           py_typed_src)
        add(zf, "tinyquant_cpu/codec/__init__.py",  codec_src)
        add(zf, "tinyquant_cpu/corpus/__init__.py", corpus_src)
        add(zf, "tinyquant_cpu/backend/__init__.py", backend_src)
        for src in sources:
            ext_blob = extract_core_extension(src.path, src.platform_key)
            ext_name = EXT_BY_KEY[src.platform_key]
            add(zf, f"tinyquant_cpu/_lib/{src.platform_key}/{ext_name}", ext_blob)
        add(zf, f"{dist_info}/METADATA", metadata_src)
        add(zf, f"{dist_info}/WHEEL",    wheel_src)
        add(zf, f"{dist_info}/LICENSE",  license_src)

        # RECORD is last; its own entry has empty hash/size per PEP 376.
        record_lines = [
            f"{arc},{sha},{size}" for arc, sha, size in record_entries
        ]
        record_lines.append(f"{dist_info}/RECORD,,")
        record_blob = ("\n".join(record_lines) + "\n").encode("ascii")
        zi = zipfile.ZipInfo(f"{dist_info}/RECORD",
                             date_time=(2026, 4, 13, 0, 0, 0))
        zi.compress_type = zipfile.ZIP_DEFLATED
        zi.external_attr = (0o644 << 16)
        zf.writestr(zi, record_blob)

    blob = buf.getvalue()
    output.write_bytes(blob)
    digest = hashlib.sha256(blob).hexdigest()
    return digest, len(blob)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--input-dir", type=Path, required=True)
    ap.add_argument("--version", required=True)
    ap.add_argument("--output",  type=Path, required=True)
    ap.add_argument("--skip-verify", action="store_true")
    args = ap.parse_args()

    sources = discover_inputs(args.input_dir, args.version)
    # Load shim sources from scripts/packaging/templates/ (checked in).
    tpl = Path(__file__).parent / "templates"
    digest, size = build_fat_wheel(
        sources=sources,
        version=args.version,
        selector_src=(tpl / "_selector.py").read_bytes(),
        init_src=(tpl / "__init__.py").read_bytes(),
        codec_src=(tpl / "codec__init__.py").read_bytes(),
        corpus_src=(tpl / "corpus__init__.py").read_bytes(),
        backend_src=(tpl / "backend__init__.py").read_bytes(),
        metadata_src=(tpl / "METADATA").read_bytes(),
        wheel_src=(tpl / "WHEEL").read_bytes(),
        license_src=(Path(__file__).parents[2] / "LICENSE").read_bytes(),
        py_typed_src=b"",
        output=args.output,
    )

    manifest = {
        "version": args.version,
        "source_wheels": [
            {"path": s.path.name, "sha256": s.sha256,
             "size_bytes": s.size_bytes, "platform_key": s.platform_key}
            for s in sources
        ],
        "fat_wheel_sha256": digest,
        "fat_wheel_size_bytes": size,
        "assembler_version": "0.1.0",
        "built_at": datetime.now(timezone.utc).isoformat(),
    }
    (args.output.with_suffix(args.output.suffix + ".manifest.json")
     ).write_text(json.dumps(manifest, indent=2))
    print(f"wrote {args.output} ({size:,} bytes, sha256={digest[:12]}...)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

> [!warning] RECORD gotcha
> The sha256 field in `RECORD` uses **url-safe base64 with `=` padding
> stripped** (PEP 376), not hex. A naive `hashlib.sha256(b).hexdigest()`
> produces a `RECORD` that `pip` refuses to verify. The helper
> `_sha256()` above is the correct encoding. The `RECORD` entry for
> `RECORD` itself must have empty hash and size fields.

> [!warning] Do **not** shell out from the assembler
> The whole pipeline is in-process `zipfile` + `hashlib` — no external
> process launches, no shell invocations. Reasons: (a) determinism
> across runners, (b) auditability of the fat wheel contents,
> (c) avoiding security-hook friction in the repo's release CI.

## CI workflow — `.github/workflows/python-fatwheel.yml`

```yaml
name: python-fatwheel

on:
  workflow_dispatch:
    inputs:
      version:
        description: "Version to assemble (must match tag, e.g. 0.2.0)"
        required: true
  push:
    tags:
      - "py-v*"

permissions:
  contents: read
  id-token: write   # for PyPI OIDC trusted publishing

concurrency:
  group: python-fatwheel-${{ github.ref }}
  cancel-in-progress: false

jobs:

  # -------------------------------------------------------------------
  # 1. Fetch the 5 per-arch wheels published by rust-release.yml.
  # -------------------------------------------------------------------
  fetch-inputs:
    runs-on: ubuntu-22.04
    outputs:
      version: ${{ steps.resolve.outputs.version }}
    steps:
      - uses: actions/checkout@v4
      - id: resolve
        name: Resolve version
        run: |
          if [[ "${{ github.event_name }}" == "push" ]]; then
            echo "version=${GITHUB_REF_NAME#py-v}" >> "$GITHUB_OUTPUT"
          else
            echo "version=${{ inputs.version }}" >> "$GITHUB_OUTPUT"
          fi
      - name: Download per-arch wheels from prior release
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          mkdir -p dist/input
          gh release download "rust-v${{ steps.resolve.outputs.version }}" \
              --repo "${{ github.repository }}" \
              --dir dist/input \
              --pattern 'tinyquant_rs-*.whl'
      - uses: actions/upload-artifact@v4
        with:
          name: input-wheels
          path: dist/input/

  # -------------------------------------------------------------------
  # 2. Assemble the fat wheel (single Linux runner).
  # -------------------------------------------------------------------
  assemble:
    needs: fetch-inputs
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      - uses: actions/download-artifact@v4
        with:
          name: input-wheels
          path: dist/input/
      - name: Install assembler deps
        run: python -m pip install --upgrade pip twine
      - name: Assemble
        run: |
          python scripts/packaging/assemble_fat_wheel.py \
            --input-dir dist/input \
            --version "${{ needs.fetch-inputs.outputs.version }}" \
            --output  "dist/tinyquant_cpu-${{ needs.fetch-inputs.outputs.version }}-py3-none-any.whl"
      - name: Size gate (< 50 MB)
        run: |
          size=$(stat -c '%s' dist/tinyquant_cpu-*.whl)
          echo "fat wheel size: $size bytes"
          test "$size" -lt 52428800 || { echo "fat wheel exceeds 50 MB"; exit 1; }
      - name: Binary-count gate (exactly 5)
        run: |
          count=$(python -m zipfile -l dist/tinyquant_cpu-*.whl \
                  | grep -cE '_core\.(abi3\.so|pyd)$')
          echo "bundled binaries: $count"
          test "$count" -eq 5 || { echo "expected 5 binaries, got $count"; exit 1; }
      - name: twine check
        run: twine check dist/tinyquant_cpu-*.whl
      - uses: actions/upload-artifact@v4
        with:
          name: fat-wheel
          path: dist/tinyquant_cpu-*.whl
      - uses: actions/upload-artifact@v4
        with:
          name: fat-wheel-manifest
          path: dist/tinyquant_cpu-*.manifest.json

  # -------------------------------------------------------------------
  # 3. Install-and-test on all 5 Tier-1 runners.
  # -------------------------------------------------------------------
  install-test:
    needs: assemble
    strategy:
      fail-fast: true
      matrix:
        include:
          - { os: ubuntu-22.04,     key: linux_x86_64_gnu  }
          - { os: ubuntu-22.04-arm, key: linux_aarch64_gnu }
          - { os: macos-13,         key: macos_x86_64      }
          - { os: macos-14,         key: macos_arm64       }
          - { os: windows-2022,     key: win_amd64         }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      - uses: actions/download-artifact@v4
        with:
          name: fat-wheel
          path: dist/
      - name: Install fat wheel into fresh venv
        shell: bash
        run: |
          python -m venv .venv
          source .venv/bin/activate || source .venv/Scripts/activate
          pip install --upgrade pip
          pip install dist/tinyquant_cpu-*-py3-none-any.whl numpy pytest hypothesis
      - name: Smoke import + key detection
        shell: bash
        run: |
          source .venv/bin/activate || source .venv/Scripts/activate
          python -c "
          import tinyquant_cpu
          from tinyquant_cpu._selector import detect_platform_key
          assert detect_platform_key() == '${{ matrix.key }}', detect_platform_key()
          print('ok:', tinyquant_cpu.__version__)
          "
      - name: Run parity suite against reference
        shell: bash
        run: |
          source .venv/bin/activate || source .venv/Scripts/activate
          pip install -e .[dev]   # pulls in tinyquant_py_reference
          pytest tests/parity/test_cross_impl.py -v

  # -------------------------------------------------------------------
  # 4. Publish to PyPI via OIDC (guarded).
  # -------------------------------------------------------------------
  publish:
    needs: install-test
    runs-on: ubuntu-22.04
    environment: pypi
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: fat-wheel
          path: dist/
      - name: Verify version > 0.1.1
        run: |
          wheel=$(ls dist/tinyquant_cpu-*.whl | head -n1)
          ver=$(basename "$wheel" | sed 's/tinyquant_cpu-\([^-]*\).*/\1/')
          python - <<'PY' "$ver"
          import sys
          from packaging.version import Version
          v = Version(sys.argv[1])
          assert v > Version("0.1.1"), f"fat wheel version {v} must be > 0.1.1"
          print("version gate ok:", v)
          PY
      - uses: pypa/gh-action-pypi-publish@release/v1
        with:
          skip-existing: false
          packages-dir: dist/
```

## Parity test expansion — `tests/parity/test_cross_impl.py`

Phase 23 seeded this file with one trivial test. Phase 24 populates it
with the following parametrized tests, wired to an `rs` fixture that
imports `tinyquant_cpu` (which at runtime is now Rust-backed):

```python
"""Cross-implementation parity: pure-Python reference vs Rust fat wheel.

The `ref` fixture points at the Phase 23 pure-Python reference.
The `rs`  fixture points at the Phase 24 fat wheel - exposed as
`tinyquant_cpu` at the import name level, so downstream apps observe
no difference.
"""

import numpy as np
import pytest

import tinyquant_cpu as rs                       # fat wheel (Rust)
import tinyquant_py_reference as ref             # pure-Python oracle


@pytest.fixture(params=[2, 4, 8], ids=lambda bw: f"bw{bw}")
def bit_width(request):
    return request.param


@pytest.fixture(params=[(64,), (384,), (768,), (1536,)],
                ids=lambda d: f"dim{d[0]}")
def dim(request):
    return request.param[0]


@pytest.fixture
def rng():
    return np.random.default_rng(20260413)


# ---------------------------------------------------------------- 1
def test_config_hash_parity(bit_width, dim):
    """CodecConfig hashes must match bit-for-bit across impls."""
    ref_cfg = ref.codec.CodecConfig(bit_width=bit_width, seed=42,
                                    dimension=dim)
    rs_cfg  = rs.codec.CodecConfig(bit_width=bit_width,  seed=42,
                                    dimension=dim)
    assert ref_cfg.config_hash == rs_cfg.config_hash


# ---------------------------------------------------------------- 2
def test_round_trip_byte_equality(bit_width, dim, rng):
    """Compress on ref, decompress on rs (and vice-versa)."""
    vec = rng.standard_normal(dim, dtype=np.float32)

    ref_cfg = ref.codec.CodecConfig(bit_width=bit_width, seed=7,
                                    dimension=dim)
    ref_cb  = ref.codec.Codebook.train(vec.reshape(1, -1), ref_cfg)
    rs_cfg  = rs.codec.CodecConfig(bit_width=bit_width,  seed=7,
                                    dimension=dim)
    rs_cb   = rs.codec.Codebook.from_bytes(ref_cb.to_bytes())

    ref_cv  = ref.codec.compress(vec, ref_cfg, ref_cb)
    rs_cv   = rs.codec.CompressedVector.from_bytes(ref_cv.to_bytes())

    assert ref_cv.to_bytes() == rs_cv.to_bytes()


# ---------------------------------------------------------------- 3
@pytest.mark.parametrize("n_vectors", [1, 16, 256])
def test_batch_compress_parity(n_vectors, bit_width, dim, rng):
    """compress_batch produces identical byte streams on both sides."""
    batch = rng.standard_normal((n_vectors, dim), dtype=np.float32)
    ref_cfg = ref.codec.CodecConfig(bit_width=bit_width, seed=0,
                                    dimension=dim)
    ref_cb  = ref.codec.Codebook.train(batch, ref_cfg)
    rs_cfg  = rs.codec.CodecConfig(bit_width=bit_width,  seed=0,
                                    dimension=dim)
    rs_cb   = rs.codec.Codebook.from_bytes(ref_cb.to_bytes())

    ref_cvs = [ref.codec.compress(v, ref_cfg, ref_cb) for v in batch]
    rs_cvs  = [rs.codec.compress(v, rs_cfg, rs_cb)    for v in batch]

    for r, s in zip(ref_cvs, rs_cvs, strict=True):
        assert r.to_bytes() == s.to_bytes()


# ---------------------------------------------------------------- 4
def test_corpus_lifecycle_parity(rng):
    """100-vector corpus insert + decompress byte equality."""
    batch = rng.standard_normal((100, 384), dtype=np.float32)
    ref_cfg = ref.codec.CodecConfig(bit_width=4, seed=1, dimension=384)
    ref_cb  = ref.codec.Codebook.train(batch, ref_cfg)
    rs_cfg  = rs.codec.CodecConfig(bit_width=4, seed=1, dimension=384)
    rs_cb   = rs.codec.Codebook.from_bytes(ref_cb.to_bytes())

    ref_corpus = ref.corpus.Corpus(config=ref_cfg, codebook=ref_cb)
    rs_corpus  = rs.corpus.Corpus(config=rs_cfg,  codebook=rs_cb)
    for i, v in enumerate(batch):
        ref_corpus.insert(ref.corpus.VectorEntry(id=str(i), vector=v))
        rs_corpus.insert(rs.corpus.VectorEntry(id=str(i),  vector=v))

    ref_dec = np.stack([ref_corpus.decompress(str(i)) for i in range(100)])
    rs_dec  = np.stack([rs_corpus.decompress(str(i))  for i in range(100)])
    np.testing.assert_array_equal(ref_dec, rs_dec)


# ---------------------------------------------------------------- 5
def test_exception_hierarchy_parity():
    """Every exception class in the reference has a same-named cousin
    in the fat wheel with the same bases (modulo Rust-side exception
    object identity)."""
    for subpkg in ("codec", "corpus", "backend"):
        ref_mod = getattr(ref, subpkg)
        rs_mod  = getattr(rs,  subpkg)
        ref_errs = {n for n in ref_mod.__all__ if n.endswith("Error")}
        rs_errs  = {n for n in rs_mod.__all__  if n.endswith("Error")}
        assert ref_errs == rs_errs, (subpkg, ref_errs ^ rs_errs)
```

## Release procedure

Linear steps to ship `tinyquant-cpu v0.2.0`:

1. **Version bump (PR).**
   - Edit `pyproject.toml` `version = "0.2.0"`.
   - Edit `rust/crates/tinyquant-py/Cargo.toml` `version = "0.2.0"`.
   - Edit `CHANGELOG.md` with a `## [0.2.0] - 2026-04-13` section:
     - "Rust-backed fat wheel. Pure-Python implementation moved to
       the test-only reference in `tests/reference/`."
     - "Import surface unchanged; no source-level changes required
       downstream."
     - "New: `tinyquant_cpu._selector.detect_platform_key()` public
       debug helper."
   - Run `python scripts/lint_skills.py` (or project-level lint) and
     ensure `pytest tests/parity/ -v` passes locally.
2. **PR review.** Two-reviewer gate on the `phase-24-python-fat-wheel`
   branch; `python-fatwheel.yml` must be green on a dispatch run with
   `inputs.version=0.2.0`.
3. **Merge.** Squash merge to `main`. Tag `rust-v0.2.0` is already
   produced by [[plans/rust/phase-22-pyo3-cabi-release|Phase 22]]'s
   `rust-release.yml`; do not create it twice.
4. **Tag `py-v0.2.0`.**
   - `git tag -a py-v0.2.0 -m "Python fat wheel v0.2.0"`
   - `git push origin py-v0.2.0`
   - This triggers `python-fatwheel.yml` which runs `fetch-inputs`,
     `assemble`, `install-test`, and `publish` in sequence.
5. **Monitor.** The `publish` job uses PyPI OIDC — no API token. If
   the job fails after `install-test` passed but before `publish`
   completed, the fat-wheel artifact is still available from the
   workflow run and can be uploaded manually via `twine upload` with
   a rescue token.
6. **GitHub Release notes.**
   - Edit the auto-created `py-v0.2.0` release.
   - Attach the fat wheel + its `.manifest.json`.
   - Body: changelog excerpt + "Install: `pip install
     tinyquant-cpu==0.2.0`".
7. **Post-release verification.**
   - On each of the 5 Tier-1 platforms, in a clean venv:
     `pip install tinyquant-cpu==0.2.0 && python -c "import tinyquant_cpu; print(tinyquant_cpu.__version__)"`.
   - Check `pypi-stats` download count begins rising within 6 h.
   - Close the Phase 24 tracking issue.

## Rollback plan

If a critical defect surfaces after publish:

1. **Yank (not delete).**
   - `twine yank --version 0.2.0 tinyquant-cpu -m "binary mismatch on <platform>"`.
   - Yanked versions stay resolvable by exact pin but are not picked
     by `pip install tinyquant-cpu` without a version.
2. **Tag revert.** `git tag -d py-v0.2.0 && git push --delete origin py-v0.2.0`.
   This does not unpublish the PyPI artifact (yank does), but prevents
   the workflow from re-running on the tag.
3. **Downstream pinning guidance.** Post a GitHub Release note saying
   "Pin `tinyquant-cpu==0.1.1` (pure Python) until v0.2.1 ships."
   Downstream CI pins can add `tinyquant-cpu!=0.2.0` temporarily.
4. **Root-cause fix.** The defect is almost always in one of
   (a) `_selector.py` (wrong key detection for a host we forgot),
   (b) a single per-arch wheel's `_core` symbol mismatch, or
   (c) a RECORD-sha256 bug that `twine check` missed. Fix, land a
   patch PR, and release `0.2.1` by re-running the tag workflow on
   `py-v0.2.1`.

> [!note] Why we yank instead of delete
> PyPI **does not** support hard-delete of a published version. The
> file is permanently in the index; yank is the only available
> escape hatch. Downstream consumers with exact pins continue to work.

## Open questions

- **SIMD dispatch within a single binary.** Phase 20 SIMD kernels are
  compile-time gated via `#[cfg(target_feature = "avx2")]`. Inside a
  fat wheel we ship one binary per `(OS, arch)` tuple, which means an
  x86_64 Linux user with AVX-512 runs the same `manylinux_2_17_x86_64`
  binary as a user with only SSE4.2. Three paths forward, to decide
  before v0.3.0:
  1. **`multiversion` crate** — `#[multiversion(targets(...))]`
     compiles several codepaths into one binary and dispatches via
     CPUID at first call. Bloats binary by ~1.5 MB; zero Python
     change. **Recommended default.**
  2. **Variant wheels.** Ship `tinyquant-cpu[avx2]`,
     `tinyquant-cpu[avx512]` as optional-dependencies-selected
     extras. Downside: fragments downstream pins; pip has no native
     CPU-feature resolver.
  3. **Source build for advanced users.** `--no-binary tinyquant-cpu`
     rebuilds with `RUSTFLAGS='-C target-cpu=native'`. Documented as
     "advanced" in the README. This is already available with no
     extra work.
- **musl fallback.** `linux_x86_64_musl` is in the platform-key table
  but **not** in the initial v0.2.0 matrix because Phase 22 does not
  yet build a `musllinux_1_2` variant. Decision: either (a) add
  musllinux to Phase 22's matrix and include it here, or (b) ship
  v0.2.0 without musl and document `pip install --no-binary` as the
  Alpine path. Current plan: (b) for v0.2.0, (a) for v0.3.0.
- **AVX-512 future path.** Whenever `tinyquant-core` grows an AVX-512
  kernel set, the fat wheel binary size increases by ~1 MB per
  x86_64 variant (since `multiversion` compiles one path per target).
  If the fat wheel approaches 50 MB, split into
  `tinyquant-cpu-avx512` as a separate PyPI package that shares the
  same `tinyquant_cpu` import namespace (yes, that is legal; pip
  resolves by Project-URL).
- **Wheel size vs PyPI policy.** PyPI's default file cap is 100 MB.
  The [PyPI file-size help page][pypi-limits] lists the process for
  requesting an increase (email `admin@pypi.org` with justification).
  We do not expect to need this before v0.5, but the option is
  documented.
- **Stable-ABI ceiling.** `abi3-py312` means the wheel serves CPython
  3.12 and later. If CPython 3.15 breaks the stable ABI (historical
  precedent: never, but PEP 703 threading work introduces risk),
  the wheel can be rebuilt to `abi3-py315` without rename.

[pypi-limits]: https://pypi.org/help/#file-size-limit

## Acceptance criteria

All of the following must be true before `publish` is allowed to run:

1. `pip install tinyquant-cpu==0.2.0` on each of
   {linux-x86_64-gnu, linux-aarch64-gnu, macos-x86_64, macos-arm64,
   win-amd64} installs exactly one wheel from PyPI (the fat wheel)
   and `python -c "import tinyquant_cpu"` exits 0.
2. On every Tier-1 runner, `detect_platform_key()` returns the
   expected key for that runner (enforced by the `install-test` job).
3. On an unsupported host (e.g., `linux-ppc64le`), `import tinyquant_cpu`
   raises `UnsupportedPlatformError` whose message names the detected
   `(sys.platform, machine())` tuple and a source-build URL.
4. `tests/parity/test_cross_impl.py` passes: **zero** byte diffs
   between the pure-Python reference and the fat wheel on the 20
   parametrised `(bit_width, dim, seed)` tuples.
5. `tests/packaging/test_shim_parity.py` asserts that
   `tinyquant_cpu.{codec,corpus,backend}.__all__` is equal to the
   reference's `__all__` for the corresponding sub-package.
6. The fat wheel is `< 50 MB` (half the PyPI hard limit).
7. The fat wheel contains **exactly five** `_core` binaries (four
   `.abi3.so`, one `.pyd`). The `binary-count gate` step enforces.
8. `twine check dist/tinyquant_cpu-*.whl` passes.
9. The fat wheel's `RECORD` sha256 entries validate: running
   `python -m wheel unpack dist/tinyquant_cpu-*.whl` and then
   `python -m wheel pack <unpacked>` produces an identical-content
   wheel (byte-identical modulo zip timestamp noise).
10. The fat wheel's version (parsed from filename) is strictly greater
    than `0.1.1`.
11. No file under `tests/reference/` appears in the fat wheel — the
    Phase 23 `build-package-does-not-leak-reference` guard passes.
12. The sdist at `dist/tinyquant_cpu-0.2.0.tar.gz` contains a
    `pyproject.toml` with a `[tool.maturin]` fallback block so
    `pip install --no-binary tinyquant-cpu tinyquant-cpu` works on
    unsupported platforms (musl, FreeBSD, 32-bit).

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| **`py3-none-any` tag bypasses pip's platform check.** Pip will happily install the fat wheel on an `s390x` box; the runtime selector is the only guard. | High (by design) | Import-time `UnsupportedPlatformError` — loud fail, no silent load | §Runtime selector raises with the detected tuple and a source-build URL; covered by CI test on a synthetic `machine() = ppc64le` monkeypatch |
| **PyPI 100 MB per-file hard cap.** Adding musllinux + an AVX-512 variant could push past 50 MB, and future ARM variants closer to the cap. | Medium (arises around v0.4) | Release blocked until size fixed | §Open questions §SIMD dispatch documents the `multiversion` crate escape; CI gate fails at `< 50 MB` so we notice well before the PyPI cap |
| **Naive RECORD repack breaks sha256 verification.** Anyone using `zip` + `zipfile.writestr` without the PEP 376 base64 encoder produces a wheel that pip installs but `pip show --files` reports as tampered. | High if assembler is edited carelessly | Silent corruption, downstream trust loss | §Fat wheel assembler encodes sha256 as `sha256=<urlsafe-b64-nopad>`; `RECORD` for itself has empty hash/size fields; `twine check` catches the worst cases in CI |
| **`platform.machine()` inconsistency: `AMD64` on Windows, `arm64` on macOS, `aarch64` on Linux.** A naive dict lookup fails on at least two of the three. | Certain (OS-level behavior) | `ImportError` on macOS arm64 or Windows x86_64 | `_ARCH_ALIASES` table normalises all three to canonical `x86_64` / `aarch64`; verified by the per-runner `detect_platform_key()` assertion in `install-test` |
| **musl vs glibc detection mis-classifies Alpine hosts.** `platform.libc_ver()` returns `('', '')` on Alpine; a naive fallback to "gnu" loads a glibc-linked `.so` on musl → `libc.so.6: not found` at runtime. | Medium (anyone on Alpine) | Crash on `_core` load | Multi-probe `_detect_libc()`: `libc_ver()` → `/etc/alpine-release` → SOABI substring → "gnu" default; musl key raises with sdist guidance if the musllinux variant is not in the wheel |
| **Stable ABI assumption.** `abi3-py312` means one wheel serves 3.12+, but a future CPython release could technically break the stable ABI. | Low (historical precedent: near-zero) | All wheels need rebuild on new Python | Rebuild under `abi3-py31N` and publish a patch release; no import-surface change required for downstream |
| **Unicode / long-path issues in `_lib/` on Windows.** Path longer than 260 chars plus the legacy non-`\\?\` limit could block `zipfile` extraction on some Windows installations. | Low (MAX_PATH disabled on Win 10+) | Install failure on locked-down Windows | Keep `_lib/<key>/` directory names short (already done); test on `windows-2022` which has long-path enabled by default |

## See also

- [[plans/rust/phase-22-pyo3-cabi-release|Phase 22: Pyo3, C ABI, Release]] — upstream source of per-arch wheels consumed by this phase.
- [[plans/rust/phase-23-python-reference-demotion|Phase 23: Python Reference Demotion]] — prerequisite; defines the parity oracle.
- [[plans/rust/phase-25-typescript-npm-package|Phase 25: TypeScript / Bun npm Package]] — downstream; same runtime-selector pattern applied to npm.
- [[research/python-rust-wrapper-distribution|Python Wrapper Over Rust Core: Distribution Strategies]] — design rationale (§Approach B).
- [[design/rust/ffi-and-bindings|FFI and Bindings]] — PyO3 `#[pyclass]` patterns re-exported by the shim.
- [[design/rust/release-strategy|Release and Versioning]] — repo-wide semver + yank policy.
- [[design/rust/ci-cd|CI/CD]] — workflow runner + secret conventions.

### External references

- PEP 376 — Database of Installed Python Distributions (RECORD format).
- PEP 425 — Compatibility Tags for Built Distributions.
- PEP 427 — The Wheel Binary Package Format 1.0.
- PEP 513 / PEP 599 / PEP 600 — manylinux specifications.
- PEP 656 — musllinux specification.
- PEP 703 — Making the GIL optional (stable-ABI impact).
- PyPI Trusted Publishing (OIDC): <https://docs.pypi.org/trusted-publishers/>
- PyPI file size help: <https://pypi.org/help/#file-size-limit>
