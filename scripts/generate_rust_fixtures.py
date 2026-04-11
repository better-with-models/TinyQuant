#!/usr/bin/env python3
"""Generate Rust test fixtures from the Python reference implementation.

Subcommands
-----------

* ``hashes``   — Emit the canonical 120-triple ``config_hashes.json`` file by
  instantiating ``tinyquant_cpu.codec.CodecConfig`` for every triple and
  recording its ``config_hash``. This is the only fixture that anchors Rust
  to Python byte-for-byte (SHA-256 of a deterministic canonical string).
* ``list``     — Print the 120 triples (debug helper).

Rotation fixtures (``.f64.bin`` files) are NOT produced here: the canonical
Rust path uses a ChaCha20 + Box-Muller pipeline that is not present in the
production Python codec, so the fixtures are generated Rust-side by the
``dump_rotation_fixture`` example binary and frozen into Git LFS. See
``docs/plans/rust/phase-13-rotation-numerics.md`` and the project plan file
for rationale.

Usage
-----

``python scripts/generate_rust_fixtures.py hashes``

Run from the repository root so imports of ``tinyquant_cpu`` resolve.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

# Canonical fixture sweep: 3 * 5 * 4 * 2 = 120 triples.
_BIT_WIDTHS: tuple[int, ...] = (2, 4, 8)
_SEEDS: tuple[int, ...] = (0, 1, 42, 999, (1 << 64) - 1)
_DIMENSIONS: tuple[int, ...] = (1, 32, 768, 1536)
_RESIDUALS: tuple[bool, ...] = (False, True)

_FIXTURE_RELPATH = Path("rust/crates/tinyquant-core/tests/fixtures/config_hashes.json")


def _triples() -> list[tuple[int, int, int, bool]]:
    return [
        (b, s, d, r)
        for b in _BIT_WIDTHS
        for s in _SEEDS
        for d in _DIMENSIONS
        for r in _RESIDUALS
    ]


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _cmd_hashes(_args: argparse.Namespace) -> int:
    repo_root = _repo_root()
    sys.path.insert(0, str(repo_root / "src"))

    from tinyquant_cpu.codec.codec_config import CodecConfig  # noqa: PLC0415

    entries: list[dict[str, Any]] = []
    for bit_width, seed, dimension, residual_enabled in _triples():
        cfg = CodecConfig(
            bit_width=bit_width,
            seed=seed,
            dimension=dimension,
            residual_enabled=residual_enabled,
        )
        entries.append(
            {
                "bit_width": bit_width,
                "seed": seed,
                "dimension": dimension,
                "residual_enabled": residual_enabled,
                "config_hash": cfg.config_hash,
            }
        )

    out_path = repo_root / _FIXTURE_RELPATH
    out_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {"schema": 1, "entries": entries}
    out_path.write_text(
        json.dumps(payload, indent=2, sort_keys=False) + "\n",
        encoding="utf-8",
    )
    print(f"wrote {len(entries)} entries -> {out_path.relative_to(repo_root)}")
    return 0


def _cmd_list(_args: argparse.Namespace) -> int:
    for triple in _triples():
        print(triple)
    return 0


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    p_hashes = sub.add_parser("hashes", help="Write config_hashes.json")
    p_hashes.set_defaults(func=_cmd_hashes)

    p_list = sub.add_parser("list", help="Print the 120 canonical triples")
    p_list.set_defaults(func=_cmd_list)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
