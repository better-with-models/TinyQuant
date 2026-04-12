#!/usr/bin/env python3
"""Generate synthetic calibration fixtures for tinyquant-bench.

Produces Gaussian unit-sphere vectors (L2-normalised) as a stand-in for
real OpenAI text-embedding vectors.  The distribution approximates the
statistics of embedding models: near-zero mean, unit-length rows.

Usage
-----
    python scripts/calibration/gen_openai_sample.py \
        --seed 42 \
        --out rust/crates/tinyquant-bench/fixtures/calibration/

Outputs (relative to repo root)
--------------------------------
- ``fixtures/calibration/openai_10k_d1536.f32.bin``   10 000 × 1536 f32 LE
- ``fixtures/calibration/openai_1k_d768.f32.bin``      1 000 × 768  f32 LE (PR-speed)
- ``fixtures/calibration/manifest.json``               SHA-256 per file

The fixtures are advisory-only: humans regenerate them; CI reads them.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import struct
import sys
from pathlib import Path
from typing import TYPE_CHECKING, Sequence

if TYPE_CHECKING:
    import numpy as np
    import numpy.typing as npt


def l2_normalise(arr: npt.NDArray[np.float32]) -> npt.NDArray[np.float32]:
    """Row-wise L2 normalisation (avoids numpy norm for compat)."""
    import numpy as _np
    norms = _np.linalg.norm(arr, axis=1, keepdims=True)
    norms = _np.where(norms == 0, 1.0, norms)
    return arr / norms  # type: ignore[return-value]


def generate(rng: np.random.Generator, rows: int, cols: int) -> npt.NDArray[np.float32]:
    """Return an (rows, cols) float32 array of unit-sphere Gaussians."""
    import numpy as _np
    data = rng.standard_normal((rows, cols)).astype(_np.float32)
    return l2_normalise(data)


def write_f32_bin(path: Path, arr: npt.NDArray[np.float32]) -> str:
    """Write float32 array to little-endian binary; return hex SHA-256."""
    raw = arr.astype("<f4").tobytes()
    path.write_bytes(raw)
    digest = hashlib.sha256(raw).hexdigest()
    print(f"  wrote {path}  ({len(raw):,} bytes, sha256={digest[:16]}...)")
    return digest


def main(argv: Sequence[str] | None = None) -> None:
    import numpy as np

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument(
        "--out",
        default="rust/crates/tinyquant-bench/fixtures/calibration",
        help="output directory (relative to repo root or absolute)",
    )
    args = parser.parse_args(argv)

    rng = np.random.default_rng(args.seed)

    # Find repo root: walk up from this script looking for AGENTS.md or Cargo.lock
    script_dir = Path(__file__).resolve().parent
    repo_root = script_dir
    for _ in range(6):
        if (repo_root / "AGENTS.md").exists() or (repo_root / "rust" / "Cargo.toml").exists():
            break
        repo_root = repo_root.parent
    else:
        sys.exit("Could not find repo root (no AGENTS.md or rust/Cargo.toml)")

    out_dir = Path(args.out)
    if not out_dir.is_absolute():
        out_dir = repo_root / out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    manifest: dict[str, str] = {}

    # Full fixture: 10 000 × 1536
    print("Generating openai_10k_d1536 …")
    arr_10k = generate(rng, rows=10_000, cols=1536)
    manifest["openai_10k_d1536.f32.bin"] = write_f32_bin(
        out_dir / "openai_10k_d1536.f32.bin", arr_10k
    )

    # PR-speed fixture: 1 000 × 768
    print("Generating openai_1k_d768 …")
    arr_1k = generate(rng, rows=1_000, cols=768)
    manifest["openai_1k_d768.f32.bin"] = write_f32_bin(
        out_dir / "openai_1k_d768.f32.bin", arr_1k
    )

    # SHA-256 manifest (drift detection gate)
    manifest_path = out_dir / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"  wrote {manifest_path}")

    print("Done.")


if __name__ == "__main__":
    main()
