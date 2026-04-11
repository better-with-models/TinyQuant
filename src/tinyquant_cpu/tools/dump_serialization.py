"""Fixture generator for tinyquant-io serialization byte-parity tests.

Usage (from repo root)::

    python src/tinyquant_cpu/tools/dump_serialization.py

Or via the generate_rust_fixtures.py dispatcher::

    python scripts/generate_rust_fixtures.py serialization
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

import numpy as np

# ---------------------------------------------------------------------------
# Canonical fixture table
# | id       | bit_width | dim  | residual | seed |
# |----------|-----------|------|----------|------|
# | case_01  | 4         | 768  | True     | 42   |
# | case_02  | 2         | 768  | False    | 42   |
# | case_03  | 8         | 768  | False    | 42   |
# | case_04  | 4         | 1    | False    | 42   |
# | case_05  | 2         | 17   | True     | 42   |
# | case_06  | 4         | 15   | False    | 42   |
# | case_07  | 8         | 1536 | True     | 42   |
# | case_08  | 4         | 768  | False    | 42   |
# | case_09  | 2         | 16   | False    | 42   |
# | case_10  | 4         | 16   | True     | 42   |
# ---------------------------------------------------------------------------

_CASES: list[dict] = [
    {"id": "case_01", "bit_width": 4, "dim": 768, "residual": True, "seed": 42},
    {"id": "case_02", "bit_width": 2, "dim": 768, "residual": False, "seed": 42},
    {"id": "case_03", "bit_width": 8, "dim": 768, "residual": False, "seed": 42},
    {"id": "case_04", "bit_width": 4, "dim": 1, "residual": False, "seed": 42},
    {"id": "case_05", "bit_width": 2, "dim": 17, "residual": True, "seed": 42},
    {"id": "case_06", "bit_width": 4, "dim": 15, "residual": False, "seed": 42},
    {"id": "case_07", "bit_width": 8, "dim": 1536, "residual": True, "seed": 42},
    {"id": "case_08", "bit_width": 4, "dim": 768, "residual": False, "seed": 42},
    {"id": "case_09", "bit_width": 2, "dim": 16, "residual": False, "seed": 42},
    {"id": "case_10", "bit_width": 4, "dim": 16, "residual": True, "seed": 42},
]


def _repo_root() -> Path:
    """Return the repository root (two levels above this file's src/tinyquant_cpu/tools/)."""
    return Path(__file__).resolve().parents[3]


def _serialization_fixture_dir(repo_root: Path) -> Path:
    return repo_root / "rust" / "crates" / "tinyquant-io" / "tests" / "fixtures"


def _config_hash_for(bit_width: int, seed: int, dim: int, residual: bool) -> str:
    """Compute the canonical config hash matching CodecConfig.config_hash."""
    canonical = (
        f"CodecConfig("
        f"bit_width={bit_width},"
        f"seed={seed},"
        f"dimension={dim},"
        f"residual_enabled={residual})"
    )
    return hashlib.sha256(canonical.encode(), usedforsecurity=False).hexdigest()


def run_case(case: dict, out_dir: Path, repo_root: Path) -> dict:
    """Generate one fixture case directory and return its manifest entry."""
    from tinyquant_cpu.codec.compressed_vector import CompressedVector  # noqa: PLC0415

    bit_width: int = case["bit_width"]
    dim: int = case["dim"]
    has_residual: bool = case["residual"]
    seed: int = case["seed"]
    case_id: str = case["id"]

    case_dir = out_dir / case_id
    case_dir.mkdir(parents=True, exist_ok=True)

    rng = np.random.default_rng(seed)
    indices = rng.integers(0, 1 << bit_width, size=dim, dtype=np.uint8)
    residual_bytes: bytes | None = rng.bytes(dim * 2) if has_residual else None

    config_hash = _config_hash_for(bit_width, seed, dim, has_residual)

    # Build Python CompressedVector — indices is NDArray[np.uint8]
    cv = CompressedVector(
        indices=indices,
        residual=residual_bytes,
        config_hash=config_hash,
        dimension=dim,
        bit_width=bit_width,
    )
    expected: bytes = cv.to_bytes()

    # Write fixture files
    (case_dir / "indices.u8.bin").write_bytes(indices.tobytes())
    if residual_bytes is not None:
        (case_dir / "residual.u8.bin").write_bytes(residual_bytes)
    (case_dir / "config_hash.txt").write_text(config_hash, encoding="utf-8")
    (case_dir / "expected.bin").write_bytes(expected)

    sha256 = hashlib.sha256(expected).hexdigest()
    print(
        f"  {case_id}: bw={bit_width} dim={dim} residual={has_residual} "
        f"len={len(expected)} sha256={sha256[:16]}..."
    )

    return {
        "id": case_id,
        "bit_width": bit_width,
        "dim": dim,
        "residual": has_residual,
        "seed": seed,
        "config_hash": config_hash,
        "expected_sha256": sha256,
        "expected_len": len(expected),
    }


def main() -> int:
    """Generate all serialization fixtures."""
    repo_root = _repo_root()
    sys.path.insert(0, str(repo_root / "src"))

    out_dir = _serialization_fixture_dir(repo_root)
    out_dir.mkdir(parents=True, exist_ok=True)

    print(f"Generating {len(_CASES)} serialization fixtures -> {out_dir.relative_to(repo_root)}")
    manifest_entries: list[dict] = []
    for case in _CASES:
        entry = run_case(case, out_dir, repo_root)
        manifest_entries.append(entry)

    manifest = {"schema": 1, "cases": manifest_entries}
    manifest_path = out_dir / "manifest.json"
    manifest_path.write_text(
        json.dumps(manifest, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"wrote manifest -> {manifest_path.relative_to(repo_root)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
