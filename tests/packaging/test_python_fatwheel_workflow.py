"""Dry-run smoke harness for `.github/workflows/python-fatwheel.yml`.

Phase 24.3 acceptance gate. Since the CI-side workflow cannot be fired
from a local machine, this harness simulates the critical invariants
locally so regressions are caught at `pytest` time:

1. **Static validation** of the workflow YAML — every job has
   `runs-on`, no `needs:` reference names a non-existent job,
   `release-gate` is a `needs` of `publish`, the `publish` `if:`
   matches the contract expression byte-for-byte (modulo the same
   whitespace normalisation `cargo xtask check-publish-guards` uses),
   and `workflow_dispatch` carries a `dry_run` input defaulting to
   `true`.
2. **End-to-end assembler smoke** — fabricate 5 per-arch dummy wheels,
   run the real assembler against them, and re-check the output with
   `twine check` and a `wheel unpack`/`wheel pack` round-trip.
3. **Version gate** — ensure the pre-publish guard's `packaging.version`
   comparison correctly rejects a `0.0.0-rc.smoke` dry-run version
   against the required `> 0.1.1` threshold.

See `docs/plans/rust/phase-24-python-fat-wheel-official.md`
§CI workflow and §Acceptance criteria items 6-10.
"""

from __future__ import annotations

import subprocess
import sys
import zipfile
from pathlib import Path
from typing import TYPE_CHECKING, Any, cast

import pytest
import yaml
from packaging.version import Version

if TYPE_CHECKING:
    from collections.abc import Mapping


_REPO_ROOT = Path(__file__).resolve().parents[2]
_WORKFLOW_PATH = _REPO_ROOT / ".github" / "workflows" / "python-fatwheel.yml"
_ASSEMBLER = _REPO_ROOT / "scripts" / "packaging" / "assemble_fat_wheel.py"

# Byte-identical to the contract documented in
# `rust/xtask/src/cmd/guard_sync_python.rs::CONTRACT` and asserted in
# CI via `cargo xtask check-publish-guards`.
_PUBLISH_IF_CONTRACT = (
    "needs.release-gate.outputs.should_publish == 'true' && inputs.dry_run != true"
)

# Platforms fabricated for the assembler smoke — matches
# `scripts/packaging/assemble_fat_wheel.py::PLATFORM_KEY_BY_TAG`.
_PLATFORM_FIXTURES: list[tuple[str, str]] = [
    ("manylinux_2_17_x86_64", "_core.abi3.so"),
    ("manylinux_2_28_aarch64", "_core.abi3.so"),
    ("macosx_10_14_x86_64", "_core.abi3.so"),
    ("macosx_11_0_arm64", "_core.abi3.so"),
    ("win_amd64", "_core.pyd"),
]


# ---------------------------------------------------------------------------
# YAML loader: tolerate the `on:` short-form and GH-Actions expressions.
# ---------------------------------------------------------------------------


def _load_workflow() -> dict[Any, Any]:
    """Parse the workflow with PyYAML, treating `on` as a normal key.

    PyYAML may load the `on:` key as the boolean ``True`` (YAML 1.1
    reserved word), so the returned mapping is typed as ``dict[Any, Any]``
    rather than ``dict[str, Any]``.
    """
    assert _WORKFLOW_PATH.is_file(), f"workflow missing at {_WORKFLOW_PATH}"
    with _WORKFLOW_PATH.open("r", encoding="utf-8") as f:
        return cast(dict[Any, Any], yaml.safe_load(f))


def _normalise_ws(text: str) -> str:
    """Collapse whitespace runs to single spaces (contract compare)."""
    return " ".join(text.split())


# ---------------------------------------------------------------------------
# 1. Static workflow validation
# ---------------------------------------------------------------------------


def test_every_job_has_runs_on() -> None:
    """Every job in the workflow declares `runs-on` (non-reusable)."""
    wf = _load_workflow()
    jobs: Mapping[str, Any] = wf["jobs"]
    offenders = [
        name
        for name, job in jobs.items()
        if not isinstance(job, dict) or "runs-on" not in job
    ]
    assert not offenders, f"jobs missing `runs-on`: {offenders}"


def test_no_orphan_needs_references() -> None:
    """Every `needs:` reference names an existing top-level job."""
    wf = _load_workflow()
    jobs: Mapping[str, Any] = wf["jobs"]
    job_names = set(jobs.keys())
    for name, job in jobs.items():
        needs = job.get("needs")
        if needs is None:
            continue
        needed = [needs] if isinstance(needs, str) else list(needs)
        missing = [n for n in needed if n not in job_names]
        assert not missing, f"job {name!r} needs unknown jobs {missing}"


def test_release_gate_is_publish_dependency() -> None:
    """`publish` job must declare `release-gate` in its `needs:`."""
    wf = _load_workflow()
    publish = wf["jobs"]["publish"]
    needs = publish["needs"]
    needed = [needs] if isinstance(needs, str) else list(needs)
    assert "release-gate" in needed, (
        f"publish.needs must include 'release-gate', got {needed}"
    )


def test_publish_if_matches_contract_byte_for_byte() -> None:
    """`publish.if` matches the two-clause contract after normalisation."""
    wf = _load_workflow()
    guard = wf["jobs"]["publish"]["if"]
    assert isinstance(guard, str), f"publish.if must be a string, got {type(guard)}"
    assert _normalise_ws(guard) == _PUBLISH_IF_CONTRACT, (
        f"publish.if drifted from contract.\n"
        f"expected: {_PUBLISH_IF_CONTRACT}\n"
        f"actual:   {_normalise_ws(guard)}"
    )


def test_dry_run_input_defaults_true() -> None:
    """`workflow_dispatch.inputs.dry_run` must default to `true`."""
    wf = _load_workflow()
    # PyYAML may load `on` as the boolean True since it's a YAML 1.1
    # reserved word. Accept either the string key or the True key.
    on_block: Any = None
    if "on" in wf:
        on_block = wf["on"]
    elif True in wf:
        on_block = wf[True]
    assert on_block is not None, "workflow has no trigger block"
    dispatch = on_block["workflow_dispatch"]
    inputs = dispatch["inputs"]
    assert inputs["dry_run"]["default"] is True, (
        "dry_run input must default to True to prevent accidental publishes"
    )


def test_concurrency_cancel_in_progress_false() -> None:
    """Concurrency group must NOT cancel in-progress runs."""
    wf = _load_workflow()
    concurrency = wf["concurrency"]
    assert concurrency["cancel-in-progress"] is False, (
        "cancel-in-progress must be false so a late release tag does "
        "not interrupt an in-flight upload"
    )


def test_release_gate_uses_py_v_regex() -> None:
    """`release-gate` shell regex must match `py-v` prefix exclusively."""
    wf = _load_workflow()
    gate = wf["jobs"]["release-gate"]
    steps = gate["steps"]
    # Find the `evaluate` step and inspect its `run:` body.
    evaluate = next(s for s in steps if s.get("id") == "evaluate")
    body = evaluate["run"]
    assert "^py-v[0-9]+\\.[0-9]+\\.[0-9]+$" in body, (
        f"release-gate must regex-match `py-v<SEMVER>` tags exactly, got body: {body!r}"
    )
    # Workflow_dispatch must force should_publish=false.
    assert "workflow_dispatch" in body and "should_publish=false" in body, (
        "release-gate must short-circuit workflow_dispatch runs"
    )


# ---------------------------------------------------------------------------
# 2. Assembler smoke — fabricate 5 dummy wheels, run the real assembler.
# ---------------------------------------------------------------------------


def _make_dummy_arch_wheel(
    tmp_path: Path,
    version: str,
    platform_tag: str,
    ext_name: str,
    ext_blob: bytes,
) -> Path:
    """Fabricate a minimal per-arch wheel matching the Phase 22 name format.

    Inline copy of `tests/packaging/test_assemble_fat_wheel.py::_make_dummy_arch_wheel`
    — duplicated rather than imported so this harness stands alone when
    the assembler suite is skipped.
    """
    name = f"tinyquant_rs-{version}-cp312-abi3-{platform_tag}.whl"
    path = tmp_path / name
    dist_info = f"tinyquant_rs-{version}.dist-info"
    metadata = (
        f"Metadata-Version: 2.1\nName: tinyquant-rs\nVersion: {version}\n"
    ).encode("ascii")
    wheel_meta = (
        b"Wheel-Version: 1.0\nGenerator: test\nRoot-Is-Purelib: false\n"
        b"Tag: cp312-abi3-" + platform_tag.encode("ascii") + b"\n"
    )
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("tinyquant_rs/__init__.py", b"# dummy\n")
        zf.writestr(f"tinyquant_rs/{ext_name}", ext_blob)
        zf.writestr(f"{dist_info}/METADATA", metadata)
        zf.writestr(f"{dist_info}/WHEEL", wheel_meta)
        zf.writestr(f"{dist_info}/RECORD", b"")
    return path


@pytest.fixture
def dry_run_version() -> str:
    """Synthetic pre-release version used for the dry-run smoke."""
    # We use PEP 440 semantics: `0.0.0rc.smoke` is not a legal PEP 440
    # version (`rc.` is not a valid separator). `packaging.Version`
    # normalises `0.0.0rc0` correctly; we need something that parses
    # and that is definitively NOT greater than `0.1.1`.
    return "0.0.0rc0"


def test_assembler_produces_valid_fat_wheel(
    tmp_path: Path,
    dry_run_version: str,
) -> None:
    """Fabricate 5 dummy inputs -> assembler -> twine check passes."""
    input_dir = tmp_path / "input"
    input_dir.mkdir()
    for plat_tag, ext_name in _PLATFORM_FIXTURES:
        blob = f"fake-core-{plat_tag}".encode() + b"\x00" * 32
        _make_dummy_arch_wheel(input_dir, dry_run_version, plat_tag, ext_name, blob)

    output = tmp_path / f"tinyquant_cpu-{dry_run_version}-py3-none-any.whl"
    result = subprocess.run(  # noqa: S603 -- trusted stdlib + assembler invocation
        [
            sys.executable,
            str(_ASSEMBLER),
            "--input-dir",
            str(input_dir),
            "--version",
            dry_run_version,
            "--output",
            str(output),
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, (
        f"assembler failed: stdout={result.stdout!r} stderr={result.stderr!r}"
    )
    assert output.is_file(), f"fat wheel missing at {output}"

    # twine check validates METADATA + RECORD sha256 encoding.
    twine = subprocess.run(  # noqa: S603 -- trusted stdlib + twine invocation
        [sys.executable, "-m", "twine", "check", str(output)],
        check=False,
        capture_output=True,
        text=True,
    )
    assert twine.returncode == 0, (
        f"twine check failed: stdout={twine.stdout} stderr={twine.stderr}"
    )
    assert "PASSED" in twine.stdout, f"twine output unexpected: {twine.stdout}"


def test_wheel_unpack_pack_round_trip(
    tmp_path: Path,
    dry_run_version: str,
) -> None:
    """`wheel unpack` + `wheel pack` on our fat wheel round-trips cleanly."""
    input_dir = tmp_path / "input"
    input_dir.mkdir()
    for plat_tag, ext_name in _PLATFORM_FIXTURES:
        blob = f"fake-core-{plat_tag}".encode() + b"\x00" * 32
        _make_dummy_arch_wheel(input_dir, dry_run_version, plat_tag, ext_name, blob)

    output = tmp_path / f"tinyquant_cpu-{dry_run_version}-py3-none-any.whl"
    assemble = subprocess.run(  # noqa: S603 -- trusted stdlib + assembler invocation
        [
            sys.executable,
            str(_ASSEMBLER),
            "--input-dir",
            str(input_dir),
            "--version",
            dry_run_version,
            "--output",
            str(output),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    assert assemble.returncode == 0

    unpack_dir = tmp_path / "unpacked"
    unpack_dir.mkdir()
    unpack = subprocess.run(  # noqa: S603 -- trusted stdlib + wheel invocation
        [
            sys.executable,
            "-m",
            "wheel",
            "unpack",
            "--dest",
            str(unpack_dir),
            str(output),
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    assert unpack.returncode == 0, (
        f"wheel unpack failed: stdout={unpack.stdout} stderr={unpack.stderr}"
    )

    # The unpack directory contains a single `<dist>-<ver>/` subdirectory.
    children = [p for p in unpack_dir.iterdir() if p.is_dir()]
    assert len(children) == 1, f"expected 1 unpacked dir, got {children}"
    unpacked_root = children[0]

    repack_dir = tmp_path / "repacked"
    repack_dir.mkdir()
    pack = subprocess.run(  # noqa: S603 -- trusted stdlib + wheel invocation
        [
            sys.executable,
            "-m",
            "wheel",
            "pack",
            "--dest-dir",
            str(repack_dir),
            str(unpacked_root),
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    assert pack.returncode == 0, (
        f"wheel pack failed: stdout={pack.stdout} stderr={pack.stderr}"
    )
    repacked = list(repack_dir.glob("*.whl"))
    assert len(repacked) == 1, f"expected 1 repacked wheel, got {repacked}"


# ---------------------------------------------------------------------------
# 3. Version gate — confirms the pre-publish guard rejects dry-run versions.
# ---------------------------------------------------------------------------


def test_version_gate_rejects_dry_run_version(dry_run_version: str) -> None:
    """A dry-run version must NOT clear the `> 0.1.1` publish guard."""
    v = Version(dry_run_version)
    assert not (v > Version("0.1.1")), (
        f"dry-run version {v} unexpectedly > 0.1.1 — would wrongly "
        f"pass the publish gate"
    )


def test_version_gate_accepts_real_release_version() -> None:
    """A real `0.2.0` tag DOES clear the `> 0.1.1` publish guard."""
    v = Version("0.2.0")
    assert v > Version("0.1.1"), (
        "0.2.0 must be accepted by the publish gate — otherwise "
        "the first real release would fail to upload"
    )
