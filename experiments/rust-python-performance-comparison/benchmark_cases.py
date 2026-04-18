"""Shared benchmark definitions for cross-surface performance comparison."""

from __future__ import annotations

from dataclasses import asdict, dataclass, field
from pathlib import Path
from statistics import median
from typing import Any

REAL_CORPUS_ID = "real_openai_335_d1536"
DEFAULT_SURFACES = [
    "py_reference",
    "py_shim",
    "py_direct",
    "rust_cpu",
    "rust_wgpu",
]
DEFAULT_SUITES = ["codec", "search"]
DEFAULT_CORPORA = [
    REAL_CORPUS_ID,
    "synthetic_1000_d1536",
    "synthetic_10000_d1536",
]
DEFAULT_WARMUPS = 1
DEFAULT_REPS = 5
DEFAULT_TOP_K = 10
QUERY_COUNT = 20
METRIC_UNIT_MS = "ms"

SUITE_PHASES = {
    "codec": [
        "codec_setup",
        "codec_compress_batch",
        "codec_decompress_batch",
    ],
    "search": [
        "search_setup",
        "search_query_batch",
        "search_query_p50",
        "search_query_p95",
    ],
}

DEFAULT_CODEC_CONFIG = {
    "bit_width": 4,
    "seed": 42,
    "dimension": 1536,
    "residual_enabled": False,
}


@dataclass(frozen=True)
class SurfaceCase:
    """Static metadata for one benchmark surface."""

    surface_id: str
    label: str
    implementation: str


SURFACES = {
    "py_reference": SurfaceCase(
        surface_id="py_reference",
        label="Python reference",
        implementation="python",
    ),
    "py_shim": SurfaceCase(
        surface_id="py_shim",
        label="Python shim",
        implementation="python",
    ),
    "py_direct": SurfaceCase(
        surface_id="py_direct",
        label="Python direct",
        implementation="python",
    ),
    "rust_cpu": SurfaceCase(
        surface_id="rust_cpu",
        label="Rust CPU",
        implementation="rust",
    ),
    "rust_wgpu": SurfaceCase(
        surface_id="rust_wgpu",
        label="Rust wgpu",
        implementation="rust",
    ),
}


@dataclass(frozen=True)
class SyntheticCorpusSpec:
    """Deterministic synthetic corpus definition."""

    corpus_id: str
    rows: int
    dim: int
    seed: int


SYNTHETIC_CORPORA = {
    "synthetic_1000_d1536": SyntheticCorpusSpec(
        corpus_id="synthetic_1000_d1536",
        rows=1000,
        dim=1536,
        seed=1042,
    ),
    "synthetic_10000_d1536": SyntheticCorpusSpec(
        corpus_id="synthetic_10000_d1536",
        rows=10000,
        dim=1536,
        seed=10042,
    ),
}


@dataclass(frozen=True)
class CorpusArtifact:
    """Prepared corpus paths and metadata for one benchmark case."""

    corpus_id: str
    label: str
    rows: int
    dim: int
    source: str
    codec_path: Path
    search_path: Path
    query_indices: list[int]

    def to_spec(self) -> dict[str, Any]:
        """Serialize for the Rust helper spec JSON."""
        return {
            "id": self.corpus_id,
            "label": self.label,
            "rows": self.rows,
            "dim": self.dim,
            "source": self.source,
            "codec_path": str(self.codec_path),
            "search_path": str(self.search_path),
            "query_indices": self.query_indices,
        }


@dataclass
class ResultRow:
    """Flat benchmark row emitted by both Python and Rust surfaces."""

    suite: str
    surface_id: str
    surface_label: str
    corpus_id: str
    rows: int
    dim: int
    phase: str
    metric_unit: str
    median: float | None
    min: float | None
    max: float | None
    samples: list[float] = field(default_factory=list)
    warmups: int = DEFAULT_WARMUPS
    reps: int = DEFAULT_REPS
    status: str = "ok"
    skip_reason: str | None = None
    notes: str = ""

    def to_dict(self) -> dict[str, Any]:
        """Convert to a JSON-serializable dict."""
        return asdict(self)


@dataclass(frozen=True)
class SurfaceStatus:
    """Availability record for one surface."""

    surface_id: str
    surface_label: str
    status: str
    reason: str | None = None
    details: str = ""

    def to_dict(self) -> dict[str, Any]:
        """Convert to a JSON-serializable dict."""
        return asdict(self)


def repo_root() -> Path:
    """Return the repository root path."""
    return Path(__file__).resolve().parents[2]


def phase_names(suite: str) -> list[str]:
    """Return the configured phase names for a suite."""
    try:
        return SUITE_PHASES[suite]
    except KeyError as exc:  # pragma: no cover - defensive configuration guard
        raise ValueError(f"unknown suite: {suite}") from exc


def summarize_samples(
    samples: list[float],
) -> tuple[float | None, float | None, float | None]:
    """Return median/min/max for a non-empty sample list."""
    if not samples:
        return None, None, None
    return float(median(samples)), float(min(samples)), float(max(samples))


def result_row_from_samples(
    *,
    suite: str,
    surface_id: str,
    corpus: CorpusArtifact,
    phase: str,
    samples: list[float],
    warmups: int,
    reps: int,
    notes: str = "",
) -> ResultRow:
    """Build a successful result row from measured samples."""
    row_median, row_min, row_max = summarize_samples(samples)
    return ResultRow(
        suite=suite,
        surface_id=surface_id,
        surface_label=SURFACES[surface_id].label,
        corpus_id=corpus.corpus_id,
        rows=corpus.rows,
        dim=corpus.dim,
        phase=phase,
        metric_unit=METRIC_UNIT_MS,
        median=row_median,
        min=row_min,
        max=row_max,
        samples=[float(value) for value in samples],
        warmups=warmups,
        reps=reps,
        status="ok",
        skip_reason=None,
        notes=notes,
    )


def status_rows(
    *,
    suite: str,
    surface_id: str,
    corpus: CorpusArtifact,
    warmups: int,
    reps: int,
    status: str,
    reason: str,
    notes: str = "",
) -> list[ResultRow]:
    """Build skipped or failed rows for every phase in a suite."""
    return [
        ResultRow(
            suite=suite,
            surface_id=surface_id,
            surface_label=SURFACES[surface_id].label,
            corpus_id=corpus.corpus_id,
            rows=corpus.rows,
            dim=corpus.dim,
            phase=phase,
            metric_unit=METRIC_UNIT_MS,
            median=None,
            min=None,
            max=None,
            samples=[],
            warmups=warmups,
            reps=reps,
            status=status,
            skip_reason=reason,
            notes=notes,
        )
        for phase in phase_names(suite)
    ]
