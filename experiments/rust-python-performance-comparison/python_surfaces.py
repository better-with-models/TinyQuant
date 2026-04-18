"""In-process runners for the Python benchmark surfaces."""

from __future__ import annotations

import importlib
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np

from benchmark_cases import (
    DEFAULT_CODEC_CONFIG,
    QUERY_COUNT,
    CorpusArtifact,
    ResultRow,
    SurfaceStatus,
    result_row_from_samples,
)


@dataclass(frozen=True)
class SurfaceModules:
    """Loaded module handles for one Python benchmark surface."""

    surface_id: str
    codec_module: Any
    backend_module: Any


def _repo_root() -> Path:
    """Return the repository root path."""
    return Path(__file__).resolve().parents[2]


def _ensure_sys_path(path: Path) -> None:
    """Prepend a path to sys.path exactly once."""
    raw = str(path)
    if raw not in sys.path:
        sys.path.insert(0, raw)


def probe_surface(surface_id: str) -> SurfaceStatus:
    """Return availability for a Python surface without running benchmarks."""
    try:
        _ = load_surface(surface_id)
    except ImportError as exc:
        return SurfaceStatus(
            surface_id=surface_id,
            surface_label=_surface_label(surface_id),
            status="skipped",
            reason=str(exc),
        )
    except Exception as exc:  # pragma: no cover - defensive probe path
        return SurfaceStatus(
            surface_id=surface_id,
            surface_label=_surface_label(surface_id),
            status="failed",
            reason=str(exc),
        )
    return SurfaceStatus(
        surface_id=surface_id,
        surface_label=_surface_label(surface_id),
        status="ok",
    )


def load_surface(surface_id: str) -> SurfaceModules:
    """Import the Python modules that back one surface."""
    repo_root = _repo_root()
    if surface_id == "py_reference":
        _ensure_sys_path(repo_root / "tests" / "reference")
        codec_module = importlib.import_module("tinyquant_py_reference.codec")
        backend_module = importlib.import_module("tinyquant_py_reference.backend")
        return SurfaceModules(surface_id, codec_module, backend_module)

    if surface_id == "py_shim":
        _ensure_sys_path(repo_root / "src")
        try:
            codec_module = importlib.import_module("tinyquant_cpu.codec")
            backend_module = importlib.import_module("tinyquant_cpu.backend")
        except ImportError as exc:
            raise ImportError(
                "tinyquant_rs not installed; build with maturin develop"
            ) from exc
        return SurfaceModules(surface_id, codec_module, backend_module)

    if surface_id == "py_direct":
        try:
            root_module = importlib.import_module("tinyquant_rs")
        except ImportError as exc:
            raise ImportError(
                "tinyquant_rs not installed; build with maturin develop"
            ) from exc
        return SurfaceModules(surface_id, root_module.codec, root_module.backend)

    raise ValueError(f"unsupported Python surface: {surface_id}")


def run_codec_case(
    surface_id: str,
    corpus: np.ndarray,
    corpus_artifact: CorpusArtifact,
    *,
    warmups: int,
    reps: int,
) -> list[ResultRow]:
    """Run the codec benchmark suite for one Python surface."""
    modules = load_surface(surface_id)
    codec = modules.codec_module.Codec()
    config = modules.codec_module.CodecConfig(**DEFAULT_CODEC_CONFIG)
    train_codebook = lambda: modules.codec_module.Codebook.train(corpus, config)

    _run_warmups(train_codebook, warmups)
    setup_samples = _measure_simple(train_codebook, reps)
    codebook = train_codebook()

    compress_batch = lambda: codec.compress_batch(corpus, config, codebook)
    _run_warmups(compress_batch, warmups)
    compress_samples = _measure_simple(compress_batch, reps)
    compressed = compress_batch()

    decompress_batch = lambda: codec.decompress_batch(compressed, config, codebook)
    _run_warmups(decompress_batch, warmups)
    decompress_samples = _measure_simple(decompress_batch, reps)

    return [
        result_row_from_samples(
            suite="codec",
            surface_id=surface_id,
            corpus=corpus_artifact,
            phase="codec_setup",
            samples=setup_samples,
            warmups=warmups,
            reps=reps,
            notes="codebook training only",
        ),
        result_row_from_samples(
            suite="codec",
            surface_id=surface_id,
            corpus=corpus_artifact,
            phase="codec_compress_batch",
            samples=compress_samples,
            warmups=warmups,
            reps=reps,
            notes="full-batch compress_batch on prepared codebook",
        ),
        result_row_from_samples(
            suite="codec",
            surface_id=surface_id,
            corpus=corpus_artifact,
            phase="codec_decompress_batch",
            samples=decompress_samples,
            warmups=warmups,
            reps=reps,
            notes="full-batch decompress_batch from pre-compressed payload",
        ),
    ]


def run_search_case(
    surface_id: str,
    search_corpus: np.ndarray,
    queries: np.ndarray,
    corpus_artifact: CorpusArtifact,
    *,
    warmups: int,
    reps: int,
    top_k: int,
) -> list[ResultRow]:
    """Run the search benchmark suite for one Python surface."""
    modules = load_surface(surface_id)
    vector_map = {str(index): search_corpus[index] for index in range(search_corpus.shape[0])}

    build_backend = lambda: _build_python_backend(modules.backend_module, vector_map)

    _run_warmups(build_backend, warmups)
    setup_samples = _measure_simple(build_backend, reps)
    backend = build_backend()

    _run_warmups(lambda: _search_batch(backend, queries, top_k), warmups)
    batch_samples, p50_samples, p95_samples = _measure_search_batch(
        backend,
        queries,
        top_k,
        reps=reps,
    )

    query_notes = f"query_count={QUERY_COUNT}, top_k={top_k}"
    return [
        result_row_from_samples(
            suite="search",
            surface_id=surface_id,
            corpus=corpus_artifact,
            phase="search_setup",
            samples=setup_samples,
            warmups=warmups,
            reps=reps,
            notes="backend construction plus ingest",
        ),
        result_row_from_samples(
            suite="search",
            surface_id=surface_id,
            corpus=corpus_artifact,
            phase="search_query_batch",
            samples=batch_samples,
            warmups=warmups,
            reps=reps,
            notes=query_notes,
        ),
        result_row_from_samples(
            suite="search",
            surface_id=surface_id,
            corpus=corpus_artifact,
            phase="search_query_p50",
            samples=p50_samples,
            warmups=warmups,
            reps=reps,
            notes=query_notes,
        ),
        result_row_from_samples(
            suite="search",
            surface_id=surface_id,
            corpus=corpus_artifact,
            phase="search_query_p95",
            samples=p95_samples,
            warmups=warmups,
            reps=reps,
            notes=query_notes,
        ),
    ]


def _surface_label(surface_id: str) -> str:
    """Return the display label used in surface availability rows."""
    labels = {
        "py_reference": "Python reference",
        "py_shim": "Python shim",
        "py_direct": "Python direct",
    }
    return labels[surface_id]


def _run_warmups(fn: Any, warmups: int) -> None:
    """Execute unmeasured warmup iterations."""
    for _ in range(warmups):
        fn()


def _measure_simple(fn: Any, reps: int) -> list[float]:
    """Measure a no-argument callable in milliseconds."""
    samples: list[float] = []
    for _ in range(reps):
        t0 = time.perf_counter_ns()
        fn()
        samples.append((time.perf_counter_ns() - t0) / 1_000_000.0)
    return samples


def _build_python_backend(backend_module: Any, vectors: dict[str, np.ndarray]) -> Any:
    """Create and ingest a Python search backend."""
    backend = backend_module.BruteForceBackend()
    backend.ingest(vectors)
    return backend


def _search_batch(backend: Any, queries: np.ndarray, top_k: int) -> list[float]:
    """Run the full query batch and return per-query timings in milliseconds."""
    per_query_ms: list[float] = []
    for query in queries:
        t0 = time.perf_counter_ns()
        backend.search(query, top_k)
        per_query_ms.append((time.perf_counter_ns() - t0) / 1_000_000.0)
    return per_query_ms


def _measure_search_batch(
    backend: Any,
    queries: np.ndarray,
    top_k: int,
    *,
    reps: int,
) -> tuple[list[float], list[float], list[float]]:
    """Measure search batch, per-repetition p50, and per-repetition p95."""
    batch_samples: list[float] = []
    p50_samples: list[float] = []
    p95_samples: list[float] = []
    for _ in range(reps):
        batch_start = time.perf_counter_ns()
        per_query_ms = _search_batch(backend, queries, top_k)
        batch_samples.append((time.perf_counter_ns() - batch_start) / 1_000_000.0)
        p50_samples.append(float(np.percentile(per_query_ms, 50)))
        p95_samples.append(float(np.percentile(per_query_ms, 95)))
    return batch_samples, p50_samples, p95_samples
