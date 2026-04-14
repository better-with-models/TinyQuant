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
