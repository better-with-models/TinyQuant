#!/usr/bin/env python3
"""Regenerate parity fixtures for the ``@tinyquant/core`` npm package.

The TS parity tests load JSON produced by this script at test time
rather than shelling out to Python. That keeps the TypeScript test
suite hermetic — Python is not required in CI steps that exercise
``node --test``.

Two things anchor the numbers written here:

* The canonical Python reference implementation in
  ``tests/reference/tinyquant_py_reference`` (installed in-tree under
  ``src/tinyquant_cpu/`` via the Phase 24 shim) computes the
  ``config_hash`` via SHA-256 over a fixed canonical string.
* The Rust core implementation in
  ``rust/crates/tinyquant-core/src/codec/codec_config.rs`` matches
  that canonical string byte-for-byte (see the module-level docstring
  there for the format). That makes the Python reference a valid
  oracle for what the Rust + napi-rs path will produce once the TS
  binding is wired through ``tinyquant-core::CodecConfig``.

If the canonical format ever needs to change, update both the Python
reference and the Rust core *and* regenerate this fixture.

Usage
-----
::

    python scripts/packaging/generate_js_parity_fixtures.py \\
        javascript/@tinyquant/core/tests/fixtures

The output directory is created if it does not exist. A ``parity/``
subdirectory is populated with JSON files that the TS test suite
loads relative to ``tests/fixtures``.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Import via the in-tree shim so we do NOT require the Rust-backed
# fat wheel to be installed. The Python reference path is anchored
# to the Rust core's canonical config_hash format — see module
# docstring above.
try:
    from tinyquant_cpu.codec.codec_config import CodecConfig
except ImportError as exc:  # pragma: no cover - environment hint
    sys.stderr.write(
        "error: cannot import tinyquant_cpu.codec.codec_config — "
        "the fixture generator needs the in-tree Python shim.\n"
        "  Install it with `pip install -e .` from the TinyQuant root "
        "(or `maturin develop` if you want the Rust-backed fat wheel).\n"
        f"  Underlying error: {exc}\n",
    )
    raise SystemExit(2) from exc


# Sweep (3 * 5 * 4 * 2 = 120) is overkill for the first TS slice;
# keep it tight but diverse enough to cover every bit-width,
# residual flag, and a spread of dimensions mirroring real embedding
# sizes (OpenAI, sentence-transformers, Cohere, etc.).
_BIT_WIDTHS: tuple[int, ...] = (2, 4, 8)
_SEEDS: tuple[int, ...] = (0, 1, 42, 999, (1 << 64) - 1)
_DIMENSIONS: tuple[int, ...] = (64, 384, 768, 1536)
_RESIDUALS: tuple[bool, ...] = (False, True)


def _cases() -> list[dict[str, object]]:
    out: list[dict[str, object]] = []
    for bit_width in _BIT_WIDTHS:
        for seed in _SEEDS:
            for dimension in _DIMENSIONS:
                for residual_enabled in _RESIDUALS:
                    cfg = CodecConfig(
                        bit_width=bit_width,
                        seed=seed,
                        dimension=dimension,
                        residual_enabled=residual_enabled,
                    )
                    out.append(
                        {
                            "bit_width": bit_width,
                            # Seed is written as a string because u64 values
                            # above 2^53 cannot round-trip through JSON number.
                            # TS parses via `BigInt(seed)` — see tests/parity.test.ts.
                            "seed": str(seed),
                            "dimension": dimension,
                            "residual_enabled": residual_enabled,
                            "config_hash": cfg.config_hash,
                        },
                    )
    return out


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        sys.stderr.write(
            "usage: generate_js_parity_fixtures.py <output-dir>\n"
            "  e.g. javascript/@tinyquant/core/tests/fixtures\n",
        )
        return 2

    out_root = Path(argv[1]).resolve() / "parity"
    out_root.mkdir(parents=True, exist_ok=True)

    cases = _cases()
    target = out_root / "config_hashes.json"
    target.write_text(
        json.dumps(cases, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    sys.stdout.write(f"wrote {len(cases)} cases to {target}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
