"""Quantization benchmark: TinyQuant vs common storage baselines.

Compares storage size, compression ratio, fidelity (Pearson rho, MSE,
top-k recall), and compress/decompress throughput across:

  1. FP32 (uncompressed baseline)
  2. FP16 (half-precision)
  3. uint8 min-max scalar quantization
  4. Product Quantization (PQ) simulation
  5. TinyQuant 8-bit (no residual)
  6. TinyQuant 4-bit (no residual)
  7. TinyQuant 4-bit + FP16 residual
  8. TinyQuant 2-bit (no residual)
  9. TinyQuant 2-bit + FP16 residual
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, cast

import numpy as np

from tinyquant_cpu.codec import Codebook, Codec, CodecConfig

if TYPE_CHECKING:
    from collections.abc import Callable

    from numpy.typing import NDArray

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

DATA_DIR = Path(__file__).parent / "data"
RESULTS_DIR = Path(__file__).parent / "results"
SEED = 42
NUM_QUERIES = 20
TOP_K = 5


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def cosine_similarities(
    corpus: NDArray[np.float32], query: NDArray[np.float32]
) -> NDArray[np.float32]:
    """Cosine similarity of query against every row in corpus."""
    norms = np.linalg.norm(corpus, axis=1) * np.linalg.norm(query)
    norms = np.maximum(norms, 1e-12)
    return cast("NDArray[np.float32]", (corpus @ query / norms).astype(np.float32))


def pearson_rho(
    original: NDArray[np.float32],
    reconstructed: NDArray[np.float32],
    queries: NDArray[np.float32],
    num_queries: int = NUM_QUERIES,
) -> float:
    """Mean Pearson correlation of cosine similarity rankings."""
    rhos = []
    for q in queries[:num_queries]:
        orig_sims = cosine_similarities(original, q)
        approx_sims = cosine_similarities(reconstructed, q)
        rho = float(np.corrcoef(orig_sims, approx_sims)[0, 1])
        rhos.append(rho)
    return float(np.mean(rhos))


def top_k_recall(
    original: NDArray[np.float32],
    reconstructed: NDArray[np.float32],
    queries: NDArray[np.float32],
    k: int = TOP_K,
    num_queries: int = NUM_QUERIES,
) -> float:
    """Mean top-k recall across queries."""
    recalls = []
    for q in queries[:num_queries]:
        orig_sims = cosine_similarities(original, q)
        approx_sims = cosine_similarities(reconstructed, q)
        orig_topk = set(np.argsort(orig_sims)[-k:])
        approx_topk = set(np.argsort(approx_sims)[-k:])
        recalls.append(len(orig_topk & approx_topk) / k)
    return float(np.mean(recalls))


def mse(original: NDArray[np.float32], reconstructed: NDArray[np.float32]) -> float:
    """Mean squared error between original and reconstructed vectors."""
    return float(np.mean((original - reconstructed) ** 2))


# ---------------------------------------------------------------------------
# Result container
# ---------------------------------------------------------------------------


@dataclass
class MethodResult:
    """Results for a single compression method."""

    name: str
    bits_per_dim: float
    bytes_per_vector: float
    total_bytes: float
    compression_ratio: float
    pearson_rho: float
    mse: float
    top_k_recall: float
    compress_time_ms: float
    decompress_time_ms: float
    notes: str = ""


# ---------------------------------------------------------------------------
# Baseline methods
# ---------------------------------------------------------------------------


def benchmark_fp32(
    embeddings: NDArray[np.float32], queries: NDArray[np.float32]
) -> MethodResult:
    """FP32 baseline — no compression."""
    dim = embeddings.shape[1]
    bpv = dim * 4
    return MethodResult(
        name="FP32 (baseline)",
        bits_per_dim=32.0,
        bytes_per_vector=bpv,
        total_bytes=embeddings.nbytes,
        compression_ratio=1.0,
        pearson_rho=1.0,
        mse=0.0,
        top_k_recall=1.0,
        compress_time_ms=0.0,
        decompress_time_ms=0.0,
        notes="Uncompressed reference",
    )


def benchmark_fp16(
    embeddings: NDArray[np.float32], queries: NDArray[np.float32]
) -> MethodResult:
    """FP16 half-precision truncation."""
    dim = embeddings.shape[1]
    t0 = time.perf_counter()
    compressed = embeddings.astype(np.float16)
    ct = (time.perf_counter() - t0) * 1000

    t0 = time.perf_counter()
    reconstructed = compressed.astype(np.float32)
    dt = (time.perf_counter() - t0) * 1000

    bpv = dim * 2
    return MethodResult(
        name="FP16",
        bits_per_dim=16.0,
        bytes_per_vector=bpv,
        total_bytes=compressed.nbytes,
        compression_ratio=embeddings.nbytes / compressed.nbytes,
        pearson_rho=pearson_rho(embeddings, reconstructed, queries),
        mse=mse(embeddings, reconstructed),
        top_k_recall=top_k_recall(embeddings, reconstructed, queries),
        compress_time_ms=ct,
        decompress_time_ms=dt,
        notes="Direct float16 cast",
    )


def benchmark_uint8_scalar(
    embeddings: NDArray[np.float32], queries: NDArray[np.float32]
) -> MethodResult:
    """uint8 min-max scalar quantization (per-vector)."""
    dim = embeddings.shape[1]
    n = embeddings.shape[0]

    t0 = time.perf_counter()
    mins = embeddings.min(axis=1, keepdims=True)
    maxs = embeddings.max(axis=1, keepdims=True)
    ranges = np.maximum(maxs - mins, 1e-12)
    quantized = np.round((embeddings - mins) / ranges * 255).astype(np.uint8)
    ct = (time.perf_counter() - t0) * 1000

    t0 = time.perf_counter()
    reconstructed = ((quantized.astype(np.float32) / 255.0) * ranges + mins).astype(
        np.float32
    )
    dt = (time.perf_counter() - t0) * 1000

    # Storage: indices + 2 floats per vector (min, max)
    bpv = dim * 1 + 8  # 8 bytes for min/max per vector
    return MethodResult(
        name="uint8 scalar",
        bits_per_dim=8.0 + 64.0 / dim,  # ~8 bits + overhead
        bytes_per_vector=bpv,
        total_bytes=n * bpv,
        compression_ratio=embeddings.nbytes / (n * bpv),
        pearson_rho=pearson_rho(embeddings, reconstructed, queries),
        mse=mse(embeddings, reconstructed),
        top_k_recall=top_k_recall(embeddings, reconstructed, queries),
        compress_time_ms=ct,
        decompress_time_ms=dt,
        notes="Per-vector min/max normalization",
    )


def benchmark_pq_simulated(
    embeddings: NDArray[np.float32], queries: NDArray[np.float32]
) -> MethodResult:
    """Simulated Product Quantization (M=96 sub-vectors, 256 centroids each).

    This is a simplified PQ simulation — real PQ uses k-means on sub-vectors.
    We approximate by using per-subvector uint8 scalar quantization, which
    gives a reasonable approximation of PQ's storage and fidelity profile.
    """
    dim = embeddings.shape[1]
    n = embeddings.shape[0]
    # N806: `M` matches standard PQ literature notation (Jégou et al. 2011).
    M = 96  # noqa: N806 — number of sub-quantizers
    sub_dim = dim // M

    t0 = time.perf_counter()
    # Reshape into sub-vectors and quantize each
    reshaped = embeddings.reshape(n, M, sub_dim)
    sub_mins = reshaped.min(axis=2, keepdims=True)
    sub_maxs = reshaped.max(axis=2, keepdims=True)
    sub_ranges = np.maximum(sub_maxs - sub_mins, 1e-12)
    codes = np.round((reshaped - sub_mins) / sub_ranges * 255).astype(np.uint8)
    ct = (time.perf_counter() - t0) * 1000

    t0 = time.perf_counter()
    reconstructed = (
        ((codes.astype(np.float32) / 255.0) * sub_ranges + sub_mins)
        .reshape(n, dim)
        .astype(np.float32)
    )
    dt = (time.perf_counter() - t0) * 1000

    # Storage: M bytes per vector (codes) + M*8 bytes codebook overhead (shared)
    bpv = M  # 1 byte per sub-quantizer
    codebook_overhead = M * 256 * sub_dim * 4  # shared codebook, amortized
    total = n * bpv + codebook_overhead
    return MethodResult(
        name="PQ (M=96, simulated)",
        bits_per_dim=M * 8 / dim,
        bytes_per_vector=bpv,
        total_bytes=total,
        compression_ratio=embeddings.nbytes / total,
        pearson_rho=pearson_rho(embeddings, reconstructed, queries),
        mse=mse(embeddings, reconstructed),
        top_k_recall=top_k_recall(embeddings, reconstructed, queries),
        compress_time_ms=ct,
        decompress_time_ms=dt,
        notes=f"M={M} sub-quantizers, 256 centroids, sub_dim={sub_dim}",
    )


# ---------------------------------------------------------------------------
# TinyQuant methods
# ---------------------------------------------------------------------------


def benchmark_tinyquant(
    embeddings: NDArray[np.float32],
    queries: NDArray[np.float32],
    bit_width: int,
    residual: bool,
    codebook: Codebook,
    codec: Codec,
) -> MethodResult:
    """Benchmark TinyQuant at a given bit width and residual setting."""
    dim = embeddings.shape[1]
    n = embeddings.shape[0]
    config = CodecConfig(
        bit_width=bit_width, seed=SEED, dimension=dim, residual_enabled=residual
    )
    # Need a matching codebook
    cb = Codebook.train(embeddings, config)

    t0 = time.perf_counter()
    compressed = codec.compress_batch(embeddings, config, cb)
    ct = (time.perf_counter() - t0) * 1000

    t0 = time.perf_counter()
    reconstructed = codec.decompress_batch(compressed, config, cb)
    dt = (time.perf_counter() - t0) * 1000

    total_payload = sum(cv.size_bytes for cv in compressed)
    bpv = total_payload / n

    label = f"TinyQuant {bit_width}-bit"
    if residual:
        label += " + residual"

    return MethodResult(
        name=label,
        bits_per_dim=bpv * 8 / dim,
        bytes_per_vector=bpv,
        total_bytes=total_payload,
        compression_ratio=embeddings.nbytes / total_payload,
        pearson_rho=pearson_rho(embeddings, reconstructed, queries),
        mse=mse(embeddings, reconstructed),
        top_k_recall=top_k_recall(embeddings, reconstructed, queries),
        compress_time_ms=ct,
        decompress_time_ms=dt,
        notes=f"seed={SEED}, residual={'yes' if residual else 'no'}",
    )


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    """Run the full benchmark suite."""
    RESULTS_DIR.mkdir(exist_ok=True)

    # Load data. passages.json is not needed here — the benchmark operates on
    # the embedding vectors; the source texts live next to generate_embeddings.py.
    embeddings = np.load(DATA_DIR / "embeddings.npy")
    with open(DATA_DIR / "metadata.json", encoding="utf-8") as f:
        metadata = json.load(f)

    print(f"Loaded {embeddings.shape[0]} embeddings, dim={embeddings.shape[1]}")
    print(f"Model: {metadata['model']}")

    # Use a subset of embeddings as queries
    rng = np.random.default_rng(SEED)
    query_idx = rng.choice(len(embeddings), size=NUM_QUERIES, replace=False)
    queries = embeddings[query_idx]

    codec = Codec()
    dim = embeddings.shape[1]
    dummy_config = CodecConfig(bit_width=4, seed=SEED, dimension=dim)
    dummy_cb = Codebook.train(embeddings, dummy_config)

    # Run benchmarks
    results: list[MethodResult] = []

    print("\n--- Running benchmarks ---\n")

    def tq(bits: int, *, residual: bool) -> MethodResult:
        """Run TinyQuant at the given bit-width with or without FP16 residual correction."""
        return benchmark_tinyquant(embeddings, queries, bits, residual, dummy_cb, codec)

    suite: list[tuple[str, Callable[[], MethodResult]]] = [
        ("FP32", lambda: benchmark_fp32(embeddings, queries)),
        ("FP16", lambda: benchmark_fp16(embeddings, queries)),
        ("uint8 scalar", lambda: benchmark_uint8_scalar(embeddings, queries)),
        ("PQ simulated", lambda: benchmark_pq_simulated(embeddings, queries)),
        ("TQ 8-bit", lambda: tq(8, residual=False)),
        ("TQ 4-bit", lambda: tq(4, residual=False)),
        ("TQ 4-bit+res", lambda: tq(4, residual=True)),
        ("TQ 2-bit", lambda: tq(2, residual=False)),
        ("TQ 2-bit+res", lambda: tq(2, residual=True)),
    ]
    for name, func in suite:
        print(f"  {name}...", end=" ", flush=True)
        r = func()
        results.append(r)
        print(f"done (rho={r.pearson_rho:.4f}, ratio={r.compression_ratio:.1f}x)")

    # Save results as JSON
    results_data = []
    for r in results:
        results_data.append(
            {
                "name": r.name,
                "bits_per_dim": round(r.bits_per_dim, 3),
                "bytes_per_vector": round(r.bytes_per_vector, 1),
                "total_bytes": round(r.total_bytes, 0),
                "compression_ratio": round(r.compression_ratio, 2),
                "pearson_rho": round(r.pearson_rho, 6),
                "mse": round(r.mse, 8),
                "top_k_recall": round(r.top_k_recall, 4),
                "compress_time_ms": round(r.compress_time_ms, 2),
                "decompress_time_ms": round(r.decompress_time_ms, 2),
                "notes": r.notes,
            }
        )

    with open(RESULTS_DIR / "benchmark_results.json", "w", encoding="utf-8") as f:
        json.dump(
            {
                "metadata": metadata,
                "config": {
                    "seed": SEED,
                    "num_queries": NUM_QUERIES,
                    "top_k": TOP_K,
                    "num_vectors": embeddings.shape[0],
                    "dimension": embeddings.shape[1],
                },
                "results": results_data,
            },
            f,
            indent=2,
        )

    # Print summary table
    print("\n" + "=" * 120)
    header = (
        f"{'Method':<28} {'Bits/dim':>9} {'B/vec':>8} {'Ratio':>7} "
        f"{'Rho':>8} {'MSE':>12} {'R@5':>7} {'Enc ms':>8} {'Dec ms':>8}"
    )
    print(header)
    print("-" * 120)
    for r in results:
        print(
            f"{r.name:<28} {r.bits_per_dim:>9.2f} {r.bytes_per_vector:>8.0f} "
            f"{r.compression_ratio:>7.1f}x {r.pearson_rho:>8.4f} {r.mse:>12.8f} "
            f"{r.top_k_recall:>7.2%} {r.compress_time_ms:>8.1f} "
            f"{r.decompress_time_ms:>8.1f}"
        )
    print("=" * 120)

    print(f"\nResults saved to {RESULTS_DIR}/")


if __name__ == "__main__":
    main()
