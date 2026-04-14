#!/usr/bin/env python3
"""Regenerate parity fixtures for the ``@tinyquant/core`` npm package.

The TS parity tests load JSON produced by this script at test time
rather than shelling out to Python. That keeps the TypeScript test
suite hermetic — Python is not required in CI steps that exercise
``node --test``.

Three fixture buckets are produced under the target directory's
``parity/`` subdirectory:

* ``config_hashes.json`` — 120 codec-config cases whose
  ``config_hash`` must match byte-for-byte between Python and TS
  (Phase 25.2).
* ``corpus_scenarios.json`` — 5 corpus scenarios covering insert,
  contains, decompress, vector_count, and pending_events, across
  each compression policy (Phase 25.3).
* ``backend_scenarios.json`` — 3 brute-force-backend scenarios
  covering ingest + topK=3 cosine-similarity search (Phase 25.3).

If the canonical config-hash format ever needs to change, update both
the Python reference and the Rust core *and* regenerate this fixture.

Usage
-----
::

    python scripts/packaging/generate_js_parity_fixtures.py \\
        javascript/@tinyquant/core/tests/fixtures
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

try:
    from tinyquant_cpu.backend import BruteForceBackend
    from tinyquant_cpu.codec import Codebook, CodecConfig
    from tinyquant_cpu.corpus import CompressionPolicy, Corpus
except ImportError as exc:  # pragma: no cover - environment hint
    sys.stderr.write(
        "error: cannot import tinyquant_cpu — "
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


def _config_hash_cases() -> list[dict[str, object]]:
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


def _round_vec(vec: np.ndarray) -> list[float]:
    # Cast to float32 then to Python float to keep JSON output stable
    # across numpy versions.
    return [float(v) for v in vec.astype(np.float32)]


def _corpus_scenarios() -> list[dict[str, object]]:
    """Emit 5 corpus scenarios — one per policy plus edge cases.

    The Python reference is used as an oracle for (vector_count,
    contains, event types). Round-trip float comparisons are left to
    the TS round-trip test.
    """
    scenarios: list[dict[str, object]] = []
    rng = np.random.default_rng(seed=20260414)

    policy_cases: list[tuple[str, object]] = [
        ("insert-3-vectors-policy-compress", CompressionPolicy.COMPRESS),
        ("insert-3-vectors-policy-passthrough", CompressionPolicy.PASSTHROUGH),
        ("insert-3-vectors-policy-fp16", CompressionPolicy.FP16),
    ]
    for scenario_id, policy in policy_cases:
        dim = 16
        bit_width = 4
        cfg = CodecConfig(bit_width=bit_width, seed=42, dimension=dim, residual_enabled=False)
        calibration = rng.standard_normal((256, dim)).astype(np.float32)
        codebook = Codebook.train(calibration, cfg)
        corpus = Corpus(
            corpus_id="fixtures",
            codec_config=cfg,
            codebook=codebook,
            compression_policy=policy,
        )
        vectors: dict[str, list[float]] = {}
        for i in range(3):
            vec = rng.standard_normal(dim).astype(np.float32)
            corpus.insert(f"v{i}", vec)
            vectors[f"v{i}"] = _round_vec(vec)
        events = corpus.pending_events()
        scenarios.append(
            {
                "scenario_id": scenario_id,
                "config": {
                    "bit_width": bit_width,
                    "seed": "42",
                    "dimension": dim,
                    "residual_enabled": False,
                },
                "policy": policy.value,
                "vectors": vectors,
                "expected_events": [type(e).__name__ for e in events],
                "expected_vector_count": corpus.vector_count,
                "expected_contains": list(vectors.keys()),
            },
        )

    # Empty scenario — insert zero vectors, confirm only CorpusCreated event.
    dim = 8
    cfg = CodecConfig(bit_width=8, seed=7, dimension=dim, residual_enabled=False)
    calibration = rng.standard_normal((512, dim)).astype(np.float32)
    codebook = Codebook.train(calibration, cfg)
    corpus = Corpus(
        corpus_id="fixtures",
        codec_config=cfg,
        codebook=codebook,
        compression_policy=CompressionPolicy.COMPRESS,
    )
    events = corpus.pending_events()
    scenarios.append(
        {
            "scenario_id": "empty-corpus-only-created-event",
            "config": {
                "bit_width": 8,
                "seed": "7",
                "dimension": dim,
                "residual_enabled": False,
            },
            "policy": CompressionPolicy.COMPRESS.value,
            "vectors": {},
            "expected_events": [type(e).__name__ for e in events],
            "expected_vector_count": 0,
            "expected_contains": [],
        },
    )

    # Batch insert scenario — insertBatch with 5 vectors, single
    # VectorsInserted event expected.
    dim = 32
    cfg = CodecConfig(bit_width=4, seed=123, dimension=dim, residual_enabled=False)
    calibration = rng.standard_normal((1024, dim)).astype(np.float32)
    codebook = Codebook.train(calibration, cfg)
    corpus = Corpus(
        corpus_id="fixtures",
        codec_config=cfg,
        codebook=codebook,
        compression_policy=CompressionPolicy.COMPRESS,
    )
    batch_vectors: dict[str, list[float]] = {}
    for i in range(5):
        vec = rng.standard_normal(dim).astype(np.float32)
        batch_vectors[f"b{i}"] = _round_vec(vec)
    corpus.insert_batch(
        {k: np.asarray(v, dtype=np.float32) for k, v in batch_vectors.items()},
    )
    events = corpus.pending_events()
    scenarios.append(
        {
            "scenario_id": "insert-batch-5-single-event",
            "config": {
                "bit_width": 4,
                "seed": "123",
                "dimension": dim,
                "residual_enabled": False,
            },
            "policy": CompressionPolicy.COMPRESS.value,
            "vectors": batch_vectors,
            "use_batch": True,
            "expected_events": [type(e).__name__ for e in events],
            "expected_vector_count": corpus.vector_count,
            "expected_contains": list(batch_vectors.keys()),
        },
    )

    return scenarios


def _backend_scenarios() -> list[dict[str, object]]:
    """Emit 3 brute-force-backend scenarios.

    Each ingests 10 random vectors and records three top-k=3
    cosine-similarity queries. The TS test compares result ordering
    and score floats (within 1e-6) against the Python oracle.
    """
    scenarios: list[dict[str, object]] = []
    rng = np.random.default_rng(seed=20260501)

    for scenario_id, dim in (
        ("bruteforce-dim16-topk3", 16),
        ("bruteforce-dim64-topk3", 64),
        ("bruteforce-dim4-topk3", 4),
    ):
        backend = BruteForceBackend()
        corpus_vectors: dict[str, list[float]] = {}
        ingest_map: dict[str, np.ndarray] = {}
        for i in range(10):
            vec = rng.standard_normal(dim).astype(np.float32)
            corpus_vectors[f"v{i}"] = _round_vec(vec)
            ingest_map[f"v{i}"] = vec
        backend.ingest(ingest_map)

        queries: list[dict[str, object]] = []
        for q in range(3):
            query = rng.standard_normal(dim).astype(np.float32)
            results = backend.search(query, top_k=3)
            queries.append(
                {
                    "query": _round_vec(query),
                    "top_k": 3,
                    "expected": [
                        {"vector_id": r.vector_id, "score": float(r.score)}
                        for r in results
                    ],
                },
            )

        scenarios.append(
            {
                "scenario_id": scenario_id,
                "dimension": dim,
                "vectors": corpus_vectors,
                "queries": queries,
            },
        )

    return scenarios


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        sys.stderr.write(
            "usage: generate_js_parity_fixtures.py <output-dir>\n"
            "  e.g. javascript/@tinyquant/core/tests/fixtures\n",
        )
        return 2

    out_root = Path(argv[1]).resolve() / "parity"
    out_root.mkdir(parents=True, exist_ok=True)

    cases = _config_hash_cases()
    target = out_root / "config_hashes.json"
    target.write_text(
        json.dumps(cases, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    sys.stdout.write(f"wrote {len(cases)} config-hash cases to {target}\n")

    corpus_scenarios = _corpus_scenarios()
    corpus_target = out_root / "corpus_scenarios.json"
    corpus_target.write_text(
        json.dumps(corpus_scenarios, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    sys.stdout.write(
        f"wrote {len(corpus_scenarios)} corpus scenarios to {corpus_target}\n",
    )

    backend_scenarios = _backend_scenarios()
    backend_target = out_root / "backend_scenarios.json"
    backend_target.write_text(
        json.dumps(backend_scenarios, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    sys.stdout.write(
        f"wrote {len(backend_scenarios)} backend scenarios to {backend_target}\n",
    )

    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
