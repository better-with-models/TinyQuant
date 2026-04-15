"""Throughput benchmark for tinyquant_py_reference.

Measures encode and decode throughput across common CodecConfig variants on a
335-vector x 1536-dimension embedding corpus. Run from the repository root:

    python experiments/rust-python-performance-comparison/bench_python_reference.py

The script writes a summary table to stdout. No output files are produced.
"""

from __future__ import annotations

import statistics
import sys
import time
from typing import TYPE_CHECKING, Any, cast

if TYPE_CHECKING:
    from collections.abc import Callable

# Use the reference implementation directly -- tinyquant_cpu requires the
# compiled tinyquant_rs Rust extension which is not available without maturin.
sys.path.insert(0, "tests/reference")

import numpy as np
import numpy.typing as npt
from tinyquant_py_reference.codec.codebook import Codebook
from tinyquant_py_reference.codec.codec import Codec
from tinyquant_py_reference.codec.codec_config import CodecConfig

CORPUS_PATH = "tests/reference/fixtures/openai_335_d1536.npy"
REPS = 5
SEED = 42


def load_corpus(path: str) -> npt.NDArray[np.float32]:
    """Load the embedding corpus, falling back to a random matrix if absent."""
    try:
        vectors = np.load(path)
        print(f"Loaded corpus: {vectors.shape} from {path}")
        return cast("npt.NDArray[np.float32]", vectors.astype(np.float32))
    except FileNotFoundError:
        n, d = 335, 1536
        rng = np.random.default_rng(SEED)
        vectors = rng.standard_normal((n, d)).astype(np.float32)
        # Normalise to unit sphere so cosine similarity is well-defined.
        norms = np.linalg.norm(vectors, axis=1, keepdims=True)
        vectors /= norms
        print(f"Corpus file not found; using {n}x{d} random unit vectors")
        return cast("npt.NDArray[np.float32]", vectors)


def bench(
    label: str, fn: Callable[[], Any], reps: int = REPS
) -> tuple[float, list[float]]:
    """Run fn() reps times and return (median_s, all_s)."""
    times = []
    for _ in range(reps):
        t0 = time.perf_counter()
        fn()
        times.append(time.perf_counter() - t0)
    return statistics.median(times), times


def run_config(
    codec: Codec,
    vectors: npt.NDArray[np.float32],
    config: CodecConfig,
    codebook: Codebook,
    label: str,
) -> None:
    """Benchmark encode and decode for one CodecConfig and print results."""
    n = len(vectors)
    compressed = [codec.compress(v, config, codebook) for v in vectors]

    def encode() -> list[Any]:
        return [codec.compress(v, config, codebook) for v in vectors]

    def decode() -> list[Any]:
        return [codec.decompress(c, config, codebook) for c in compressed]

    enc_s, _enc_all = bench(label + "/encode", encode)
    dec_s, _dec_all = bench(label + "/decode", decode)

    enc_ms = enc_s * 1000
    dec_ms = dec_s * 1000
    enc_vps = n / enc_s
    dec_vps = n / dec_s

    print(
        f"  {label:<30}  enc {enc_ms:7.1f} ms  ({enc_vps:6.0f} vec/s)"
        f"  |  dec {dec_ms:7.1f} ms  ({dec_vps:6.0f} vec/s)"
    )


def main() -> None:
    """Run the benchmark across all standard configs and print results."""
    vectors = load_corpus(CORPUS_PATH)
    n, d = vectors.shape
    codec = Codec()

    print(f"\nBenchmark: n={n}, d={d}, reps={REPS}\n")

    configs = [
        (
            "4-bit + residual",
            CodecConfig(bit_width=4, seed=SEED, dimension=d, residual_enabled=True),
        ),
        (
            "4-bit no-residual",
            CodecConfig(bit_width=4, seed=SEED, dimension=d, residual_enabled=False),
        ),
        (
            "2-bit + residual",
            CodecConfig(bit_width=2, seed=SEED, dimension=d, residual_enabled=True),
        ),
        (
            "8-bit + residual",
            CodecConfig(bit_width=8, seed=SEED, dimension=d, residual_enabled=True),
        ),
    ]

    for label, config in configs:
        codebook = Codebook.train(vectors, config)
        run_config(codec, vectors, config, codebook, label)

    print()


if __name__ == "__main__":
    main()
