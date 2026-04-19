"""Unit tests for the fat-wheel runtime selector.

These tests exercise `scripts/packaging/templates/_selector.py` in
isolation — the template file is *not* installed into the dev
environment, and this test deliberately loads it by absolute path so
that it exercises the same source that the Phase 24.2 assembler will
stamp into the fat wheel.

The selector is covered for:

- All Tier-1 `(sys.platform, platform.machine())` tuples.
- The two glibc/musl variants on Linux x86_64.
- One unsupported-host path (`linux/ppc64le`) to verify
  `UnsupportedPlatformError` is raised with a diagnostic message.

See `docs/plans/rust/phase-24-python-fat-wheel-official.md`
§Runtime selector implementation for the authoritative contract.
"""

from __future__ import annotations

import importlib.util
import sys
import types
from pathlib import Path

import pytest

# Absolute path to the selector *template* — NOT installed, so we load
# it as a free-standing module to avoid depending on the fat wheel.
_REPO_ROOT = Path(__file__).resolve().parents[2]
_SELECTOR_PATH = _REPO_ROOT / "scripts" / "packaging" / "templates" / "_selector.py"


def _load_selector() -> types.ModuleType:
    """Import `_selector.py` by absolute path into a fresh module."""
    assert _SELECTOR_PATH.is_file(), (
        f"selector template missing at {_SELECTOR_PATH}; "
        f"Phase 24.1 should have created it."
    )
    spec = importlib.util.spec_from_file_location(
        "_tinyquant_cpu_selector_under_test",
        str(_SELECTOR_PATH),
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


@pytest.fixture()
def selector() -> types.ModuleType:
    """Fresh import of the selector template per test."""
    return _load_selector()


@pytest.mark.parametrize(
    ("plat", "machine", "libc", "expected_key"),
    [
        ("linux", "x86_64", "gnu", "linux_x86_64_gnu"),
        ("linux", "x86_64", "musl", "linux_x86_64_musl"),
        ("linux", "aarch64", "gnu", "linux_aarch64_gnu"),
        ("darwin", "x86_64", "", "macos_x86_64"),
        ("darwin", "arm64", "", "macos_arm64"),
        ("win32", "AMD64", "", "win_amd64"),
    ],
)
def test_detect_platform_key_supported_tuples(
    monkeypatch: pytest.MonkeyPatch,
    selector: types.ModuleType,
    plat: str,
    machine: str,
    libc: str,
    expected_key: str,
) -> None:
    """Supported Tier-1 tuples resolve to the expected `_lib/<key>/` name.

    `_detect_libc` is stubbed directly because its probe stack
    (platform.libc_ver / /etc/alpine-release / SOABI) is exercised by
    its own dedicated tests; here we just want to pin the branch.
    """
    monkeypatch.setattr(selector.sys, "platform", plat, raising=False)
    monkeypatch.setattr(selector.platform, "machine", lambda: machine, raising=False)
    monkeypatch.setattr(selector, "_detect_libc", lambda: libc, raising=True)

    assert selector.detect_platform_key() == expected_key


def test_detect_platform_key_unsupported_machine(
    monkeypatch: pytest.MonkeyPatch,
    selector: types.ModuleType,
) -> None:
    """An unknown `machine()` raises with a diagnostic message."""
    monkeypatch.setattr(selector.sys, "platform", "linux", raising=False)
    monkeypatch.setattr(selector.platform, "machine", lambda: "ppc64le", raising=False)
    monkeypatch.setattr(
        selector.platform, "libc_ver", lambda: ("glibc", ""), raising=False
    )

    with pytest.raises(selector.UnsupportedPlatformError) as excinfo:
        selector.detect_platform_key()

    message = str(excinfo.value)
    assert "ppc64le" in message
    assert "linux" in message
    # The message must point at the source-build escape hatch.
    assert "github.com/better-with-models/TinyQuant" in message


def test_detect_platform_key_musl_aarch64_rejected(
    monkeypatch: pytest.MonkeyPatch,
    selector: types.ModuleType,
) -> None:
    """Musllinux aarch64 is not bundled; must fail with sdist guidance."""
    monkeypatch.setattr(selector.sys, "platform", "linux", raising=False)
    monkeypatch.setattr(selector.platform, "machine", lambda: "aarch64", raising=False)
    monkeypatch.setattr(selector, "_detect_libc", lambda: "musl", raising=True)

    with pytest.raises(selector.UnsupportedPlatformError) as excinfo:
        selector.detect_platform_key()

    assert (
        "musllinux" in str(excinfo.value).lower()
        or "musl" in str(excinfo.value).lower()
    )
    assert "--no-binary" in str(excinfo.value)


def test_selector_public_surface(selector: types.ModuleType) -> None:
    """`__all__` declares exactly the expected public API."""
    assert set(selector.__all__) == {
        "load_core",
        "detect_platform_key",
        "UnsupportedPlatformError",
    }
    assert issubclass(selector.UnsupportedPlatformError, ImportError)


def test_ext_filename_unknown_platform_raises(
    selector: types.ModuleType,
) -> None:
    """An unknown `sys.platform` on the suffix lookup fails loud."""
    with pytest.raises(selector.UnsupportedPlatformError):
        selector._ext_filename("plan9")


def _expected_ext_suffix() -> str:
    """The suffix the currently-running interpreter expects."""
    _suffixes: dict[str, str] = {
        "linux": ".abi3.so",
        "darwin": ".abi3.so",
        "win32": ".pyd",
    }
    suffix = _suffixes.get(sys.platform)
    if suffix is not None:
        return suffix
    raise pytest.skip.Exception(f"host platform {sys.platform!r} is not Tier-1")


def test_ext_filename_for_host(selector: types.ModuleType) -> None:
    """Host suffix matches the canonical table."""
    assert selector._ext_filename(sys.platform) == "_core" + _expected_ext_suffix()


def test_detect_libc_non_linux_returns_empty(
    monkeypatch: pytest.MonkeyPatch, selector: types.ModuleType
) -> None:
    """Outside Linux, libc detection is a no-op (returns '')."""
    monkeypatch.setattr(selector.sys, "platform", "darwin", raising=False)
    assert selector._detect_libc() == ""
    monkeypatch.setattr(selector.sys, "platform", "win32", raising=False)
    assert selector._detect_libc() == ""


def test_detect_libc_glibc(
    monkeypatch: pytest.MonkeyPatch, selector: types.ModuleType
) -> None:
    """`platform.libc_ver() == ('glibc', ...)` resolves to 'gnu'."""
    monkeypatch.setattr(selector.sys, "platform", "linux", raising=False)
    monkeypatch.setattr(
        selector.platform,
        "libc_ver",
        lambda: ("glibc", "2.39"),
        raising=False,
    )
    assert selector._detect_libc() == "gnu"


def test_detect_libc_soabi_fallback_to_musl(
    monkeypatch: pytest.MonkeyPatch, selector: types.ModuleType
) -> None:
    """When libc_ver is empty and alpine-release is absent, SOABI decides."""
    monkeypatch.setattr(selector.sys, "platform", "linux", raising=False)
    monkeypatch.setattr(
        selector.platform,
        "libc_ver",
        lambda: ("", ""),
        raising=False,
    )
    # Force the alpine probe negative so SOABI is the deciding signal.
    monkeypatch.setattr(
        selector.Path,
        "exists",
        lambda self: False,
        raising=False,
    )
    monkeypatch.setattr(
        selector.sysconfig,
        "get_config_var",
        lambda key: "cpython-312-x86_64-linux-musl" if key == "SOABI" else None,
        raising=False,
    )
    assert selector._detect_libc() == "musl"


def test_detect_libc_default_to_gnu(
    monkeypatch: pytest.MonkeyPatch, selector: types.ModuleType
) -> None:
    """With all probes silent, the selector defaults to glibc."""
    monkeypatch.setattr(selector.sys, "platform", "linux", raising=False)
    monkeypatch.setattr(
        selector.platform,
        "libc_ver",
        lambda: ("", ""),
        raising=False,
    )
    monkeypatch.setattr(
        selector.Path,
        "exists",
        lambda self: False,
        raising=False,
    )
    monkeypatch.setattr(
        selector.sysconfig,
        "get_config_var",
        lambda key: "" if key == "SOABI" else None,
        raising=False,
    )
    assert selector._detect_libc() == "gnu"
