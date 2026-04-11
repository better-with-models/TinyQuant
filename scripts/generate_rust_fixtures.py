#!/usr/bin/env python3
"""Generate Rust test fixtures from the Python reference implementation.

Subcommands
-----------

* ``hashes``   — Emit the canonical 120-triple ``config_hashes.json`` file by
  instantiating ``tinyquant_cpu.codec.CodecConfig`` for every triple and
  recording its ``config_hash``. This is the only fixture that anchors Rust
  to Python byte-for-byte (SHA-256 of a deterministic canonical string).
* ``codebook`` — Emit the 10 000 × 64 f32 training corpus and the trained
  ``Codebook.entries`` byte-for-byte output from the Python reference for
  every supported bit width. Phase 14 uses these to prove byte parity on
  ``Codebook::train``. See
  ``docs/plans/rust/phase-14-codebook-quantize.md``.
* ``quantize`` — Emit a 10 000-value f32 corpus (``seed=7``) and, for each
  supported bit width, the ``u8`` index output produced by
  ``Codebook.quantize`` against the matching frozen codebook.
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
``python scripts/generate_rust_fixtures.py codebook --seed 42 --rows 10000 --cols 64``
``python scripts/generate_rust_fixtures.py quantize --seed 7 --count 10000 --codebook-seed 42``

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
_CODEBOOK_FIXTURE_DIR = Path("rust/crates/tinyquant-core/tests/fixtures/codebook")
_QUANTIZE_FIXTURE_DIR = Path("rust/crates/tinyquant-core/tests/fixtures/quantize")
_SUPPORTED_BIT_WIDTHS: tuple[int, ...] = (2, 4, 8)


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


def _cmd_codebook(args: argparse.Namespace) -> int:
    repo_root = _repo_root()
    sys.path.insert(0, str(repo_root / "src"))

    import numpy as np  # noqa: PLC0415

    from tinyquant_cpu.codec.codebook import Codebook  # noqa: PLC0415
    from tinyquant_cpu.codec.codec_config import CodecConfig  # noqa: PLC0415

    seed = int(args.seed)
    rows = int(args.rows)
    cols = int(args.cols)

    out_dir = repo_root / _CODEBOOK_FIXTURE_DIR
    out_dir.mkdir(parents=True, exist_ok=True)

    rng = np.random.default_rng(seed)
    training = rng.standard_normal((rows, cols)).astype(np.float32)
    training_path = out_dir / f"training_n{rows}_d{cols}.f32.bin"
    training_path.write_bytes(training.tobytes())
    print(f"wrote training corpus -> {training_path.relative_to(repo_root)}")

    for bit_width in _SUPPORTED_BIT_WIDTHS:
        cfg = CodecConfig(
            bit_width=bit_width,
            seed=seed,
            dimension=cols,
            residual_enabled=False,
        )
        cb = Codebook.train(training, cfg)
        expected_path = out_dir / f"expected_bw{bit_width}_seed{seed}.f32.bin"
        expected_path.write_bytes(cb.entries.tobytes())
        print(
            f"wrote bw={bit_width} codebook -> "
            f"{expected_path.relative_to(repo_root)}"
        )

    return 0


def _cmd_quantize(args: argparse.Namespace) -> int:
    repo_root = _repo_root()
    sys.path.insert(0, str(repo_root / "src"))

    import numpy as np  # noqa: PLC0415

    from tinyquant_cpu.codec.codebook import Codebook  # noqa: PLC0415

    seed = int(args.seed)
    count = int(args.count)
    codebook_seed = int(args.codebook_seed)

    quantize_dir = repo_root / _QUANTIZE_FIXTURE_DIR
    codebook_dir = repo_root / _CODEBOOK_FIXTURE_DIR
    quantize_dir.mkdir(parents=True, exist_ok=True)

    values = (
        np.random.default_rng(seed).standard_normal(count).astype(np.float32)
    )
    values_path = quantize_dir / f"values_n{count}.f32.bin"
    values_path.write_bytes(values.tobytes())
    print(f"wrote quantize corpus -> {values_path.relative_to(repo_root)}")

    for bit_width in _SUPPORTED_BIT_WIDTHS:
        cb_path = (
            codebook_dir / f"expected_bw{bit_width}_seed{codebook_seed}.f32.bin"
        )
        if not cb_path.exists():
            print(
                f"error: codebook fixture {cb_path.relative_to(repo_root)} is "
                f"missing; run `codebook` first.",
                file=sys.stderr,
            )
            return 1
        entries = np.frombuffer(cb_path.read_bytes(), dtype=np.float32).copy()
        cb = Codebook(entries=entries, bit_width=bit_width)
        indices = cb.quantize(values)
        idx_path = (
            quantize_dir / f"expected_bw{bit_width}_seed{codebook_seed}.u8.bin"
        )
        idx_path.write_bytes(indices.tobytes())
        print(
            f"wrote bw={bit_width} indices -> "
            f"{idx_path.relative_to(repo_root)}"
        )

    return 0


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    p_hashes = sub.add_parser("hashes", help="Write config_hashes.json")
    p_hashes.set_defaults(func=_cmd_hashes)

    p_codebook = sub.add_parser(
        "codebook",
        help="Write training corpus + Python-trained codebook entries",
    )
    p_codebook.add_argument("--seed", type=int, default=42)
    p_codebook.add_argument("--rows", type=int, default=10_000)
    p_codebook.add_argument("--cols", type=int, default=64)
    p_codebook.set_defaults(func=_cmd_codebook)

    p_quantize = sub.add_parser(
        "quantize",
        help="Write quantize value corpus + expected u8 indices per bw",
    )
    p_quantize.add_argument("--seed", type=int, default=7)
    p_quantize.add_argument("--count", type=int, default=10_000)
    p_quantize.add_argument("--codebook-seed", type=int, default=42)
    p_quantize.set_defaults(func=_cmd_quantize)

    p_list = sub.add_parser("list", help="Print the 120 canonical triples")
    p_list.set_defaults(func=_cmd_list)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
