#!/usr/bin/env python3
"""Generate Rust test fixtures from the Python reference implementation.

Subcommands
-----------

* ``hashes``   — Emit the canonical 120-triple ``config_hashes.json`` file by
  instantiating ``tinyquant_py_reference.codec.CodecConfig`` for every triple and
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
* ``residual`` — Emit a 1 000 × 64 f32 original corpus, a matching
  reconstructed corpus, and the byte-for-byte fp16 residual produced by
  ``(original − reconstructed).astype(np.float16)``. Phase 15 uses these
  to prove byte parity on ``compute_residual``.
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
``python scripts/generate_rust_fixtures.py residual --seed 19 --rows 1000 --cols 64``

Run from the repository root so imports of ``tinyquant_py_reference`` resolve.
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
_RESIDUAL_FIXTURE_DIR = Path("rust/crates/tinyquant-core/tests/fixtures/residual")
_CODEC_FIXTURE_DIR = Path("rust/crates/tinyquant-core/tests/fixtures/codec")
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

    from tinyquant_py_reference.codec.codec_config import CodecConfig  # noqa: PLC0415

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

    from tinyquant_py_reference.codec.codebook import Codebook  # noqa: PLC0415
    from tinyquant_py_reference.codec.codec_config import CodecConfig  # noqa: PLC0415

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

    from tinyquant_py_reference.codec.codebook import Codebook  # noqa: PLC0415

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


def _cmd_residual(args: argparse.Namespace) -> int:
    repo_root = _repo_root()

    import numpy as np  # noqa: PLC0415

    seed = int(args.seed)
    n = int(args.rows)
    d = int(args.cols)

    out_dir = repo_root / _RESIDUAL_FIXTURE_DIR
    out_dir.mkdir(parents=True, exist_ok=True)

    rng = np.random.default_rng(seed)
    original = rng.standard_normal((n * d,)).astype(np.float32)
    reconstructed = rng.standard_normal((n * d,)).astype(np.float32)
    diff = original - reconstructed
    expected_residual = diff.astype(np.float16).tobytes()

    (out_dir / f"original_n{n}_d{d}_seed{seed}.f32.bin").write_bytes(original.tobytes())
    print(
        f"wrote original corpus -> "
        f"{(out_dir / f'original_n{n}_d{d}_seed{seed}.f32.bin').relative_to(repo_root)}"
    )
    (out_dir / f"reconstructed_n{n}_d{d}_seed{seed}.f32.bin").write_bytes(
        reconstructed.tobytes()
    )
    print(
        f"wrote reconstructed corpus -> "
        f"{(out_dir / f'reconstructed_n{n}_d{d}_seed{seed}.f32.bin').relative_to(repo_root)}"
    )
    (out_dir / f"expected_residual_seed{seed}.bin").write_bytes(expected_residual)
    print(
        f"wrote expected residual -> "
        f"{(out_dir / f'expected_residual_seed{seed}.bin').relative_to(repo_root)}"
    )
    return 0


def _cmd_codec(args: argparse.Namespace) -> int:
    """Generate the codec fidelity manifest using the Python reference codec.

    Binary fixture files (indices, residual, decompressed) are generated by
    the ``dump_codec_fixture`` Rust example binary, because Rust and Python
    use different RNG algorithms for the rotation matrix (ChaCha20 vs PCG64)
    and therefore cannot produce byte-identical codec outputs.

    This subcommand only writes ``fidelity_manifest.json`` — the quality
    thresholds (MSE, Pearson ρ) derived from the Python reference codec.
    """
    repo_root = _repo_root()
    sys.path.insert(0, str(repo_root / "src"))

    import json  # noqa: PLC0415
    import numpy as np  # noqa: PLC0415

    from tinyquant_py_reference.codec.codec import Codec  # noqa: PLC0415
    from tinyquant_py_reference.codec.codec_config import CodecConfig  # noqa: PLC0415
    from tinyquant_py_reference.codec.codebook import Codebook  # noqa: PLC0415

    input_seed = int(args.input_seed)
    codec_seed = int(args.codec_seed)
    rows = int(args.rows)
    cols = int(args.cols)

    codebook_dir = repo_root / _CODEBOOK_FIXTURE_DIR
    out_dir = repo_root / _CODEC_FIXTURE_DIR
    out_dir.mkdir(parents=True, exist_ok=True)

    # Input corpus using Python RNG (for fidelity measurement; binary fixtures
    # use the Rust RNG via dump_codec_fixture).
    input_corpus = (
        np.random.default_rng(input_seed).standard_normal((rows, cols)).astype(np.float32)
    )

    # Load training corpus (reuse Phase 14 fixture).
    training_path = codebook_dir / "training_n10000_d64.f32.bin"
    if not training_path.exists():
        print(
            f"error: training fixture {training_path.relative_to(repo_root)} is missing; "
            "run `codebook` first.",
            file=sys.stderr,
        )
        return 1
    training = np.frombuffer(training_path.read_bytes(), dtype=np.float32).reshape(10000, 64).copy()

    thresholds: dict[str, dict[str, float]] = {}
    codec = Codec()

    for bit_width in _SUPPORTED_BIT_WIDTHS:
        config = CodecConfig(
            bit_width=bit_width,
            seed=codec_seed,
            dimension=cols,
            residual_enabled=True,
        )
        codebook = Codebook.train(training, config)

        all_decompressed = []
        for row_vec in input_corpus:
            cv = codec.compress(row_vec, config, codebook)
            dec = codec.decompress(cv, config, codebook)
            all_decompressed.append(dec)

        dec_arr = np.array(all_decompressed, dtype=np.float32)

        # Compute fidelity metrics for the manifest.
        orig = input_corpus
        rec = dec_arr
        mse_per_row = ((orig - rec) ** 2).mean(axis=1)
        mse_max = float(mse_per_row.max())

        # Pearson rho on pairwise cosine similarities (sample 200 pairs for speed).
        rng2 = np.random.default_rng(codec_seed + bit_width)
        sample = min(rows, 200)
        idx_a = rng2.integers(0, rows, size=sample)
        idx_b = rng2.integers(0, rows, size=sample)
        same = idx_a == idx_b
        idx_b[same] = (idx_b[same] + 1) % rows

        def _cosine(a: np.ndarray, b: np.ndarray) -> float:
            return float(np.dot(a, b) / (np.linalg.norm(a) * np.linalg.norm(b) + 1e-12))

        orig_cos = np.array([_cosine(orig[a], orig[b]) for a, b in zip(idx_a, idx_b)])
        rec_cos = np.array([_cosine(rec[a], rec[b]) for a, b in zip(idx_a, idx_b)])
        rho_val = np.corrcoef(orig_cos, rec_cos)[0, 1]
        rho = float(rho_val) if not np.isnan(rho_val) else 0.0

        thresholds[f"bw{bit_width}"] = {
            "mse_max": mse_max,
            "rho_min": max(0.0, rho - 0.05),  # 5% safety margin below observed
        }
        print(f"bw={bit_width}: mse_max={mse_max:.4f}, observed_rho={rho:.4f}")

    manifest = {
        "seed": input_seed,
        "codec_seed": codec_seed,
        "rows": rows,
        "cols": cols,
        "thresholds": thresholds,
    }
    manifest_path = out_dir / "fidelity_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"wrote fidelity manifest -> {manifest_path.relative_to(repo_root)}")
    return 0


def _cmd_serialization(_args: argparse.Namespace) -> int:
    """Delegate to the dedicated serialization fixture generator."""
    repo_root = _repo_root()
    sys.path.insert(0, str(repo_root / "src"))
    from tinyquant_py_reference.tools.dump_serialization import main as _ser_main  # noqa: PLC0415

    return int(_ser_main())


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

    p_residual = sub.add_parser(
        "residual",
        help="Write original + reconstructed f32 corpora and expected fp16 residual bytes",
    )
    p_residual.add_argument("--seed", type=int, default=19)
    p_residual.add_argument("--rows", type=int, default=1_000)
    p_residual.add_argument("--cols", type=int, default=64)
    p_residual.set_defaults(func=_cmd_residual)

    p_codec = sub.add_parser(
        "codec",
        help="Write end-to-end codec byte-parity fixtures (indices, residual, decompressed)",
    )
    p_codec.add_argument("--input-seed", type=int, default=11)
    p_codec.add_argument("--codec-seed", type=int, default=42)
    p_codec.add_argument("--rows", type=int, default=1_000)
    p_codec.add_argument("--cols", type=int, default=64)
    p_codec.set_defaults(func=_cmd_codec)

    p_serialization = sub.add_parser(
        "serialization",
        help="Write CompressedVector byte-parity fixtures for tinyquant-io tests",
    )
    p_serialization.set_defaults(func=_cmd_serialization)

    p_list = sub.add_parser("list", help="Print the 120 canonical triples")
    p_list.set_defaults(func=_cmd_list)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
