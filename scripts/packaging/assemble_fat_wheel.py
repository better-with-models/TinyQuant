#!/usr/bin/env python3
"""Fat wheel assembler for ``tinyquant-cpu`` (Phase 24).

Consumes per-arch maturin wheels produced by ``rust-release.yml`` and
emits a single ``py3-none-any`` wheel that bundles all 5 Tier-1
extensions plus the runtime selector. The on-disk layout and metadata
shapes are defined by
``docs/plans/rust/phase-24-python-fat-wheel-official.md`` — see
§Fat wheel anatomy (lines 79-119), §METADATA (lines 130-154),
§WHEEL (lines 156-163), §RECORD (lines 170-188), and
§Fat wheel assembler script (lines 562-817).

Implementation contract:

- **No subprocess / shell-out.** Everything is in-process
  ``zipfile`` + ``hashlib``, per plan §"Do not shell out from the
  assembler" (lines 826-830). This keeps byte output deterministic
  across CI runners and avoids security-hook friction.
- **PEP 376 / PEP 427 RECORD encoding.** The ``sha256`` field is
  url-safe base64 with ``=`` padding stripped (NOT hex). The RECORD
  entry for ``RECORD`` itself uses empty hash and size fields. A
  naive repack that re-hashes RECORD produces an unparseable wheel.
- **Byte-reproducibility.** Every zip entry uses the fixed mtime
  ``(2026, 4, 13, 0, 0, 0)`` so re-running the assembler against the
  same inputs yields the same bytes.

The script must run without ``tinyquant_rs`` installed; it only
uses the stdlib plus the shim templates checked in under
``scripts/packaging/templates/``.
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
from datetime import UTC, datetime
from pathlib import Path

__all__ = [
    "EXT_BY_KEY",
    "PLATFORM_KEY_BY_TAG",
    "REQUIRED_PLATFORM_KEYS",
    "WHEEL_NAME_RE",
    "SourceWheel",
    "build_fat_wheel",
    "discover_inputs",
    "extract_core_extension",
    "main",
]

# Version-mismatch exit code per plan §CLI contract (line 586).
_EXIT_VERSION_MISMATCH = 3
_ASSEMBLER_VERSION = "0.1.0"
# Deterministic zip-entry mtime (plan §Fat wheel assembler, lines 730-731).
_DETERMINISTIC_MTIME: tuple[int, int, int, int, int, int] = (
    2026, 4, 13, 0, 0, 0,
)

WHEEL_NAME_RE = re.compile(
    r"^tinyquant_rs-(?P<ver>[^-]+)-"
    r"cp\d+-abi3-(?P<plat>[^.]+)\.whl$"
)

# Platform tag on the input wheel filename -> _lib/<key>/ directory name.
# musllinux is optional per plan §Open questions §musl fallback; all
# others are required.
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

REQUIRED_PLATFORM_KEYS: frozenset[str] = frozenset({
    "linux_x86_64_gnu",
    "linux_aarch64_gnu",
    "macos_x86_64",
    "macos_arm64",
    "win_amd64",
})


@dataclass(frozen=True)
class SourceWheel:
    """One validated per-arch input wheel."""

    path: Path
    version: str
    platform_key: str
    sha256: str
    size_bytes: int


def _sha256(data: bytes) -> str:
    """Encode ``data`` as ``sha256=<urlsafe-b64-nopad>`` per PEP 376.

    PEP 376 RECORD uses url-safe base64 with ``=`` padding stripped,
    NOT hex. ``pip`` rejects hex-encoded sha256 fields.
    """
    return "sha256=" + base64.urlsafe_b64encode(
        hashlib.sha256(data).digest()
    ).rstrip(b"=").decode("ascii")


def discover_inputs(
    input_dir: Path, expected_version: str
) -> list[SourceWheel]:
    """Scan ``input_dir`` for per-arch wheels and validate them.

    Glob ``tinyquant_rs-*.whl``, parse each via ``WHEEL_NAME_RE``,
    validate the version matches ``expected_version``, map the platform
    tag to a canonical key, and confirm every required platform key is
    present. Raises :class:`SystemExit` on any failure.
    """
    out: list[SourceWheel] = []
    for whl in sorted(input_dir.glob("tinyquant_rs-*.whl")):
        m = WHEEL_NAME_RE.match(whl.name)
        if not m:
            continue
        ver = m.group("ver")
        if ver != expected_version:
            # Plan §CLI contract: version mismatch -> exit code 3.
            msg = (
                f"version mismatch: {whl.name} has {ver!r}, "
                f"expected {expected_version!r}"
            )
            print(msg)
            raise SystemExit(_EXIT_VERSION_MISMATCH)
        tag = m.group("plat")
        key = PLATFORM_KEY_BY_TAG.get(tag)
        if key is None:
            raise SystemExit(f"unknown platform tag {tag!r} in {whl.name}")
        blob = whl.read_bytes()
        out.append(SourceWheel(
            path=whl,
            version=ver,
            platform_key=key,
            sha256=_sha256(blob),
            size_bytes=len(blob),
        ))

    have = {w.platform_key for w in out}
    missing = REQUIRED_PLATFORM_KEYS - have
    if missing:
        raise SystemExit(
            f"missing required platform wheels: {sorted(missing)}"
        )
    return out


def extract_core_extension(src_whl: Path, platform_key: str) -> bytes:
    """Read the ``_core.{abi3.so,pyd}`` blob out of a per-arch wheel.

    The extension name is determined by ``EXT_BY_KEY[platform_key]``;
    we also accept any archive member whose basename starts with
    ``_core.`` as a fallback for maturin variants that may ship a
    differently-suffixed extension (e.g. ``_core.cpython-312-*.so``
    on non-abi3 builds — not expected for Tier-1 but tolerated).
    """
    target_name = EXT_BY_KEY[platform_key]
    with zipfile.ZipFile(src_whl) as zf:
        # Prefer exact match first.
        for info in zf.infolist():
            basename = info.filename.rsplit("/", 1)[-1]
            if basename == target_name:
                return zf.read(info)
        # Fallback: any `_core.` blob.
        for info in zf.infolist():
            basename = info.filename.rsplit("/", 1)[-1]
            if basename.startswith("_core."):
                return zf.read(info)
    raise SystemExit(
        f"{src_whl}: no _core extension found inside "
        f"(expected basename {target_name!r})"
    )


def _deterministic_zipinfo(arcname: str) -> zipfile.ZipInfo:
    """Build a ``ZipInfo`` with the fixed mtime + 0644 permissions."""
    zi = zipfile.ZipInfo(arcname, date_time=_DETERMINISTIC_MTIME)
    zi.compress_type = zipfile.ZIP_DEFLATED
    zi.external_attr = (0o644 << 16)
    return zi


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
    """Assemble the fat wheel at ``output`` and return (sha256-hex, size).

    Layout follows plan §Fat wheel anatomy (lines 79-119). RECORD is
    emitted last and its own line has empty hash / size fields per
    PEP 376.
    """
    dist_info = f"tinyquant_cpu-{version}.dist-info"
    record_entries: list[tuple[str, str, int]] = []

    def add(zf: zipfile.ZipFile, arcname: str, blob: bytes) -> None:
        zf.writestr(_deterministic_zipinfo(arcname), blob)
        record_entries.append((arcname, _sha256(blob), len(blob)))

    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as zf:
        # Package tree.
        add(zf, "tinyquant_cpu/__init__.py", init_src)
        add(zf, "tinyquant_cpu/_selector.py", selector_src)
        add(zf, "tinyquant_cpu/py.typed", py_typed_src)
        add(zf, "tinyquant_cpu/codec/__init__.py", codec_src)
        add(zf, "tinyquant_cpu/corpus/__init__.py", corpus_src)
        add(zf, "tinyquant_cpu/backend/__init__.py", backend_src)

        # Per-arch _core binaries. Sort by platform key for a stable
        # byte-reproducible order.
        for src in sorted(sources, key=lambda s: s.platform_key):
            ext_blob = extract_core_extension(src.path, src.platform_key)
            ext_name = EXT_BY_KEY[src.platform_key]
            add(
                zf,
                f"tinyquant_cpu/_lib/{src.platform_key}/{ext_name}",
                ext_blob,
            )

        # dist-info.
        add(zf, f"{dist_info}/METADATA", metadata_src)
        add(zf, f"{dist_info}/WHEEL", wheel_src)
        add(zf, f"{dist_info}/LICENSE", license_src)

        # RECORD last; its own entry has empty hash/size per PEP 376.
        record_lines = [
            f"{arc},{sha},{size}" for arc, sha, size in record_entries
        ]
        record_lines.append(f"{dist_info}/RECORD,,")
        record_blob = ("\n".join(record_lines) + "\n").encode("ascii")
        zf.writestr(
            _deterministic_zipinfo(f"{dist_info}/RECORD"),
            record_blob,
        )

    blob = buf.getvalue()
    output.write_bytes(blob)
    digest = hashlib.sha256(blob).hexdigest()
    return digest, len(blob)


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    ap = argparse.ArgumentParser(
        prog="assemble_fat_wheel.py",
        description=(
            "Assemble a tinyquant-cpu fat wheel from 5 per-arch "
            "tinyquant-rs maturin wheels. See "
            "docs/plans/rust/phase-24-python-fat-wheel-official.md "
            "§Fat wheel assembler script."
        ),
    )
    ap.add_argument("--input-dir", type=Path, required=True)
    ap.add_argument("--version", required=True)
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--skip-verify", action="store_true")
    return ap.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    """Assembler CLI entry point. Returns 0 on success."""
    args = _parse_args(argv)

    sources = discover_inputs(args.input_dir, args.version)

    # Shim sources come verbatim from scripts/packaging/templates/
    # (checked in during Phase 24.1). LICENSE lives at the repo root.
    tpl = Path(__file__).parent / "templates"
    license_path = Path(__file__).parents[2] / "LICENSE"

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
        license_src=license_path.read_bytes(),
        py_typed_src=b"",
        output=args.output,
    )

    manifest = {
        "version": args.version,
        "source_wheels": [
            {
                "path": s.path.name,
                "sha256": s.sha256,
                "size_bytes": s.size_bytes,
                "platform_key": s.platform_key,
            }
            for s in sources
        ],
        "fat_wheel_sha256": digest,
        "fat_wheel_size_bytes": size,
        "assembler_version": _ASSEMBLER_VERSION,
        "built_at": datetime.now(UTC).isoformat(),
    }
    manifest_path = args.output.with_suffix(
        args.output.suffix + ".manifest.json"
    )
    manifest_path.write_text(json.dumps(manifest, indent=2))

    print(
        f"wrote {args.output} ({size:,} bytes, sha256={digest[:12]}...)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
