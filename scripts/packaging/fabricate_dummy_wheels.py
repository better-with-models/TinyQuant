#!/usr/bin/env python3
"""Fabricate dummy per-arch wheels for `python-fatwheel.yml` dry-run mode.

Workflow-only helper. On real tag-push runs the `fetch-inputs` job
pulls the 5 per-arch `tinyquant_rs-*.whl` artefacts from the matching
`rust-v<version>` release produced by `rust-release.yml` (Phase 22).
On `workflow_dispatch` / dry-run invocations against `main`, the
upstream release may not exist yet; this script fabricates five
minimal wheels matching the Phase 22 name format so downstream jobs
(`assemble`, `install-test`) can exercise end-to-end without touching
a real release.

The fabricated layout mirrors
``tests/packaging/test_assemble_fat_wheel.py::_make_dummy_arch_wheel``
so the assembler's ``discover_inputs`` validator accepts the outputs.

Usage::

    python scripts/packaging/fabricate_dummy_wheels.py <output-dir> <version>

Both arguments are optional; defaults are ``wheels/`` and ``0.0.0rc1``
respectively. This script must never run on a real ``push`` event —
see the workflow's step-level ``if: github.event_name != 'push'``.
"""

from __future__ import annotations

import sys
import zipfile
from pathlib import Path

__all__ = ["PLATFORMS", "fabricate", "main"]

# Platform tag -> core extension basename. Matches
# `scripts/packaging/assemble_fat_wheel.py::PLATFORM_KEY_BY_TAG` keys
# and `EXT_BY_KEY` values (the 5 Tier-1 targets).
PLATFORMS: list[tuple[str, str]] = [
    ("manylinux_2_17_x86_64",  "_core.abi3.so"),
    ("manylinux_2_28_aarch64", "_core.abi3.so"),
    ("macosx_10_14_x86_64",    "_core.abi3.so"),
    ("macosx_11_0_arm64",      "_core.abi3.so"),
    ("win_amd64",              "_core.pyd"),
]


def fabricate(output_dir: Path, version: str) -> list[Path]:
    """Write the 5 dummy per-arch wheels into ``output_dir``.

    Returns the list of wheel paths that were created.
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    dist_info = f"tinyquant_rs-{version}.dist-info"
    written: list[Path] = []
    for tag, ext in PLATFORMS:
        name = f"tinyquant_rs-{version}-cp312-abi3-{tag}.whl"
        path = output_dir / name
        with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as zf:
            zf.writestr(
                "tinyquant_rs/__init__.py",
                b"# dry-run\n",
            )
            zf.writestr(
                f"tinyquant_rs/{ext}",
                f"dry-{tag}".encode("ascii") + b"\0" * 64,
            )
            zf.writestr(
                f"{dist_info}/METADATA",
                (
                    f"Metadata-Version: 2.1\n"
                    f"Name: tinyquant-rs\n"
                    f"Version: {version}\n"
                ).encode("ascii"),
            )
            zf.writestr(
                f"{dist_info}/WHEEL",
                (
                    f"Wheel-Version: 1.0\n"
                    f"Generator: dry-run\n"
                    f"Root-Is-Purelib: false\n"
                    f"Tag: cp312-abi3-{tag}\n"
                ).encode("ascii"),
            )
            zf.writestr(f"{dist_info}/RECORD", b"")
        written.append(path)
    return written


def main(argv: list[str] | None = None) -> int:
    """CLI entry point. Returns 0 on success."""
    args = list(sys.argv[1:] if argv is None else argv)
    output_dir = Path(args[0]) if len(args) >= 1 else Path("wheels")
    version = args[1] if len(args) >= 2 else "0.0.0rc1"

    written = fabricate(output_dir, version)
    for path in written:
        print(f"fabricated {path}")
    print(
        f"wrote {len(written)} dry-run wheel(s) to {output_dir} "
        f"(version={version})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
