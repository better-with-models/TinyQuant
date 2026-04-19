"""Cross-surface benchmark orchestrator for Python, Rust CPU, and Rust wgpu."""

from __future__ import annotations

import argparse
import csv
import json
import platform
import shutil
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Iterable

import numpy as np
from benchmark_cases import (
    DEFAULT_CODEC_CONFIG,
    DEFAULT_CORPORA,
    DEFAULT_REPS,
    DEFAULT_SUITES,
    DEFAULT_SURFACES,
    DEFAULT_TOP_K,
    DEFAULT_WARMUPS,
    QUERY_COUNT,
    REAL_CORPUS_ID,
    SUITE_PHASES,
    SURFACES,
    SYNTHETIC_CORPORA,
    CorpusArtifact,
    ResultRow,
    SurfaceStatus,
    phase_names,
    repo_root,
    status_rows,
)
from python_surfaces import probe_surface, run_codec_case, run_search_case


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    """Parse command-line arguments for the benchmark orchestrator."""
    parser = argparse.ArgumentParser(
        description="Cross-surface TinyQuant benchmark runner."
    )
    parser.add_argument(
        "--surfaces",
        default=",".join(DEFAULT_SURFACES),
        help="Comma-separated surface ids.",
    )
    parser.add_argument(
        "--suites",
        default=",".join(DEFAULT_SUITES),
        help="Comma-separated suite ids.",
    )
    parser.add_argument(
        "--corpora",
        default=",".join(DEFAULT_CORPORA),
        help="Comma-separated corpus ids.",
    )
    parser.add_argument("--warmups", type=int, default=DEFAULT_WARMUPS)
    parser.add_argument("--reps", type=int, default=DEFAULT_REPS)
    parser.add_argument("--top-k", type=int, default=DEFAULT_TOP_K)
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=None,
        help="Override the default timestamped output directory.",
    )
    parser.add_argument(
        "--skip-unavailable",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Skip optional unavailable surfaces instead of raising.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    """Run the full benchmark and write JSON, CSV, and Markdown outputs."""
    args = parse_args(argv)
    requested_surfaces = _parse_requested(args.surfaces, SURFACES.keys(), "surface")
    requested_suites = _parse_requested(args.suites, DEFAULT_SUITES, "suite")
    requested_corpora = _parse_requested(
        args.corpora,
        [REAL_CORPUS_ID, *SYNTHETIC_CORPORA.keys()],
        "corpus",
    )
    if args.top_k <= 0:
        raise ValueError("--top-k must be positive")
    if args.warmups < 0:
        raise ValueError("--warmups must be non-negative")
    if args.reps <= 0:
        raise ValueError("--reps must be positive")

    output_dir = _resolve_output_dir(args.output_dir)
    corpora = _prepare_corpora(output_dir, requested_corpora)

    environment = _build_environment_metadata()
    surface_statuses = _probe_python_surfaces(requested_surfaces)
    _enforce_required_python_surfaces(surface_statuses, args.skip_unavailable)

    result_rows: list[ResultRow] = []
    result_rows.extend(
        _run_python_surfaces(
            requested_surfaces=requested_surfaces,
            requested_suites=requested_suites,
            corpora=corpora,
            warmups=args.warmups,
            reps=args.reps,
            top_k=args.top_k,
            surface_statuses=surface_statuses,
        )
    )

    rust_surface_ids = [
        surface_id
        for surface_id in requested_surfaces
        if SURFACES[surface_id].implementation == "rust"
    ]
    if rust_surface_ids:
        rust_spec_path = output_dir / "rust_spec.json"
        rust_out_path = output_dir / "rust_results.json"
        rust_spec_path.write_text(
            json.dumps(
                {
                    "codec_config": DEFAULT_CODEC_CONFIG,
                    "warmups": args.warmups,
                    "reps": args.reps,
                    "top_k": args.top_k,
                    "query_count": QUERY_COUNT,
                    "surfaces": rust_surface_ids,
                    "suites": requested_suites,
                    "corpora": [corpus.to_spec() for corpus in corpora],
                },
                indent=2,
            ),
            encoding="utf-8",
        )
        rust_payload = _run_rust_helper(rust_spec_path, rust_out_path)
        environment["rust_runner"] = rust_payload.get("environment", {})
        surface_statuses.extend(
            SurfaceStatus(**status)
            for status in rust_payload.get("surface_statuses", [])
        )
        result_rows.extend(ResultRow(**row) for row in rust_payload.get("rows", []))

    ordered_statuses = _order_statuses(surface_statuses, requested_surfaces)
    ordered_rows = _order_rows(result_rows, requested_surfaces, requested_corpora)
    results_payload = {
        "metadata": {
            "generated_at": environment["generated_at"],
            "environment": environment,
            "codec_config": DEFAULT_CODEC_CONFIG,
            "requested_surfaces": requested_surfaces,
            "requested_suites": requested_suites,
            "requested_corpora": requested_corpora,
            "warmups": args.warmups,
            "reps": args.reps,
            "top_k": args.top_k,
            "query_count": QUERY_COUNT,
            "skip_unavailable": args.skip_unavailable,
        },
        "corpora": [corpus.to_spec() for corpus in corpora],
        "surface_statuses": [status.to_dict() for status in ordered_statuses],
        "rows": [row.to_dict() for row in ordered_rows],
    }

    (output_dir / "results.json").write_text(
        json.dumps(results_payload, indent=2),
        encoding="utf-8",
    )
    _write_csv(output_dir / "results.csv", ordered_rows)
    (output_dir / "SUMMARY.md").write_text(
        _build_summary(
            environment=environment,
            corpora=corpora,
            surface_statuses=ordered_statuses,
            rows=ordered_rows,
            requested_surfaces=requested_surfaces,
        ),
        encoding="utf-8",
    )

    print(f"Benchmark results written to {output_dir}")
    return 0


def _parse_requested(raw: str, allowed: Iterable[str], noun: str) -> list[str]:
    """Parse and validate a comma-separated CLI list."""
    allowed_set = set(allowed)
    values = [part.strip() for part in raw.split(",") if part.strip()]
    if not values:
        raise ValueError(f"no {noun}s requested")
    unknown = [value for value in values if value not in allowed_set]
    if unknown:
        raise ValueError(f"unknown {noun}(s): {', '.join(unknown)}")
    return values


def _resolve_output_dir(raw_output_dir: Path | None) -> Path:
    """Create the output directory for this run."""
    if raw_output_dir is not None:
        output_dir = raw_output_dir
    else:
        timestamp = datetime.now().astimezone().strftime("%Y%m%d-%H%M%S")
        output_dir = Path(__file__).resolve().parents[1] / "results" / timestamp
    output_dir.mkdir(parents=True, exist_ok=True)
    return output_dir


def _prepare_corpora(
    output_dir: Path, requested_corpora: list[str]
) -> list[CorpusArtifact]:
    """Load the real corpus, generate synthetic corpora, and persist artifacts."""
    artifacts_dir = output_dir / "corpora"
    artifacts_dir.mkdir(parents=True, exist_ok=True)
    corpora: list[CorpusArtifact] = []
    real_embeddings = _load_real_corpus()

    for corpus_id in requested_corpora:
        if corpus_id == REAL_CORPUS_ID:
            array = real_embeddings
            source = "real fixture"
            label = "OpenAI 335x1536"
        else:
            spec = SYNTHETIC_CORPORA[corpus_id]
            array = _generate_synthetic_corpus(spec.rows, spec.dim, spec.seed)
            source = f"synthetic seed={spec.seed}"
            label = f"Synthetic {spec.rows}x{spec.dim}"

        if array.ndim != 2:
            raise ValueError(f"{corpus_id} must be 2-D, got shape {array.shape!r}")
        rows, dim = array.shape
        if rows < QUERY_COUNT:
            raise ValueError(
                f"{corpus_id} has only {rows} rows; need at least {QUERY_COUNT} queries"
            )
        codec_path = artifacts_dir / f"{corpus_id}.codec.npy"
        search_path = artifacts_dir / f"{corpus_id}.search.npy"
        np.save(codec_path, array)
        np.save(search_path, _normalize_rows(array))
        query_indices = _select_query_indices(rows)
        corpora.append(
            CorpusArtifact(
                corpus_id=corpus_id,
                label=label,
                rows=rows,
                dim=dim,
                source=source,
                codec_path=codec_path,
                search_path=search_path,
                query_indices=query_indices,
            )
        )
    return corpora


def _load_real_corpus() -> np.ndarray:
    """Load the required real benchmark corpus."""
    embeddings_path = (
        repo_root()
        / "experiments"
        / "quantization-benchmark"
        / "data"
        / "embeddings.npy"
    )
    if not embeddings_path.exists():
        raise FileNotFoundError(
            f"missing real corpus fixture: {embeddings_path}. "
            "Run experiments/quantization-benchmark/generate_embeddings.py first."
        )
    embeddings = np.load(embeddings_path).astype(np.float32, copy=False)
    return embeddings


def _generate_synthetic_corpus(rows: int, dim: int, seed: int) -> np.ndarray:
    """Generate a deterministic synthetic FP32 corpus."""
    rng = np.random.default_rng(seed)
    return rng.standard_normal((rows, dim), dtype=np.float32)


def _normalize_rows(matrix: np.ndarray) -> np.ndarray:
    """Return a row-normalized copy of a matrix."""
    norms = np.linalg.norm(matrix, axis=1, keepdims=True)
    safe_norms = np.maximum(norms, 1e-12)
    return (matrix / safe_norms).astype(np.float32, copy=False)


def _select_query_indices(rows: int) -> list[int]:
    """Select deterministic query rows from a corpus."""
    rng = np.random.default_rng(DEFAULT_CODEC_CONFIG["seed"])
    selected = rng.choice(rows, size=QUERY_COUNT, replace=False)
    return [int(index) for index in selected.tolist()]


def _build_environment_metadata() -> dict[str, Any]:
    """Collect run environment details for results and summary output."""
    return {
        "generated_at": datetime.now().astimezone().isoformat(),
        "host_os": platform.platform(),
        "python_version": sys.version.replace("\n", " "),
        "python_executable": sys.executable,
        "rust_toolchain": _capture_rustc_version(),
    }


def _capture_rustc_version() -> str:
    """Capture the rustc version string, returning a placeholder on failure."""
    rustc = shutil.which("rustc")
    if rustc is None:
        return "unavailable (rustc not found on PATH)"
    try:
        completed = subprocess.run(
            [rustc, "+1.87.0", "--version"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        return f"unavailable ({exc})"
    return completed.stdout.strip()


def _probe_python_surfaces(requested_surfaces: list[str]) -> list[SurfaceStatus]:
    """Probe availability for requested Python surfaces."""
    statuses: list[SurfaceStatus] = []
    for surface_id in requested_surfaces:
        if SURFACES[surface_id].implementation == "python":
            statuses.append(probe_surface(surface_id))
    return statuses


def _enforce_required_python_surfaces(
    statuses: list[SurfaceStatus],
    skip_unavailable: bool,
) -> None:
    """Raise when a requested Python surface is unavailable and skipping is disabled."""
    if skip_unavailable:
        return
    bad_statuses = [status for status in statuses if status.status != "ok"]
    if bad_statuses:
        reasons = "; ".join(
            f"{status.surface_id}: {status.reason}" for status in bad_statuses
        )
        raise RuntimeError(reasons)


def _run_python_surfaces(
    *,
    requested_surfaces: list[str],
    requested_suites: list[str],
    corpora: list[CorpusArtifact],
    warmups: int,
    reps: int,
    top_k: int,
    surface_statuses: list[SurfaceStatus],
) -> list[ResultRow]:
    """Execute all requested Python benchmark surfaces in-process."""
    rows: list[ResultRow] = []
    status_map = {status.surface_id: status for status in surface_statuses}
    for surface_id in requested_surfaces:
        if SURFACES[surface_id].implementation != "python":
            continue
        status = status_map[surface_id]
        if status.status != "ok":
            rows.extend(
                _unavailable_surface_rows(
                    surface_id=surface_id,
                    corpora=corpora,
                    requested_suites=requested_suites,
                    warmups=warmups,
                    reps=reps,
                    status=status.status,
                    reason=status.reason or "unavailable",
                )
            )
            continue
        rows.extend(
            _run_surface_corpora(
                surface_id=surface_id,
                requested_suites=requested_suites,
                corpora=corpora,
                warmups=warmups,
                reps=reps,
                top_k=top_k,
            )
        )
    return rows


def _unavailable_surface_rows(
    *,
    surface_id: str,
    corpora: list[CorpusArtifact],
    requested_suites: list[str],
    warmups: int,
    reps: int,
    status: str,
    reason: str,
) -> list[ResultRow]:
    """Emit placeholder rows for every corpus/suite when a surface is unavailable."""
    rows: list[ResultRow] = []
    for corpus in corpora:
        for suite in requested_suites:
            rows.extend(
                status_rows(
                    suite=suite,
                    surface_id=surface_id,
                    corpus=corpus,
                    warmups=warmups,
                    reps=reps,
                    status=status,
                    reason=reason,
                )
            )
    return rows


def _run_surface_corpora(
    *,
    surface_id: str,
    requested_suites: list[str],
    corpora: list[CorpusArtifact],
    warmups: int,
    reps: int,
    top_k: int,
) -> list[ResultRow]:
    """Run all suite/corpus combinations for a single available Python surface."""
    rows: list[ResultRow] = []
    for corpus in corpora:
        codec_corpus = np.load(corpus.codec_path).astype(np.float32, copy=False)
        search_corpus = np.load(corpus.search_path).astype(np.float32, copy=False)
        queries = search_corpus[np.asarray(corpus.query_indices)]
        for suite in requested_suites:
            try:
                if suite == "codec":
                    rows.extend(
                        run_codec_case(
                            surface_id,
                            codec_corpus,
                            corpus,
                            warmups=warmups,
                            reps=reps,
                        )
                    )
                else:
                    rows.extend(
                        run_search_case(
                            surface_id,
                            search_corpus,
                            queries,
                            corpus,
                            warmups=warmups,
                            reps=reps,
                            top_k=top_k,
                        )
                    )
            except Exception as exc:
                rows.extend(
                    status_rows(
                        suite=suite,
                        surface_id=surface_id,
                        corpus=corpus,
                        warmups=warmups,
                        reps=reps,
                        status="failed",
                        reason=str(exc),
                    )
                )
    return rows


def _run_rust_helper(spec_path: Path, out_path: Path) -> dict[str, Any]:
    """Run the standalone Rust helper and return its parsed JSON payload."""
    cargo = shutil.which("cargo")
    if cargo is None:
        raise RuntimeError("cargo not found on PATH; cannot run Rust helper")
    manifest = str(Path(__file__).resolve().parent / "rust_runner" / "Cargo.toml")
    completed = subprocess.run(
        [
            cargo,
            "+1.87.0",
            "run",
            "--release",
            "--manifest-path",
            manifest,
            "--",
            "--spec",
            str(spec_path),
            "--out",
            str(out_path),
        ],
        cwd=repo_root(),
        capture_output=True,
        text=True,
        check=False,  # returncode checked explicitly below
    )
    if completed.returncode != 0:
        raise RuntimeError(
            "Rust helper failed.\n"
            f"stdout:\n{completed.stdout}\n"
            f"stderr:\n{completed.stderr}"
        )
    if not out_path.exists():
        raise RuntimeError("Rust helper completed without producing rust_results.json")
    return json.loads(out_path.read_text(encoding="utf-8"))


def _order_statuses(
    statuses: list[SurfaceStatus],
    requested_surfaces: list[str],
) -> list[SurfaceStatus]:
    """Sort surface availability rows to match the requested order."""
    order = {surface_id: index for index, surface_id in enumerate(requested_surfaces)}
    return sorted(statuses, key=lambda status: order.get(status.surface_id, len(order)))


def _order_rows(
    rows: list[ResultRow],
    requested_surfaces: list[str],
    requested_corpora: list[str],
) -> list[ResultRow]:
    """Sort result rows into a stable, readable order."""
    suite_order = {"codec": 0, "search": 1}
    corpus_order = {
        corpus_id: index for index, corpus_id in enumerate(requested_corpora)
    }
    surface_order = {
        surface_id: index for index, surface_id in enumerate(requested_surfaces)
    }
    phase_order = {
        suite: {phase: index for index, phase in enumerate(phase_names(suite))}
        for suite in SUITE_PHASES
    }
    return sorted(
        rows,
        key=lambda row: (
            suite_order[row.suite],
            corpus_order[row.corpus_id],
            surface_order[row.surface_id],
            phase_order[row.suite][row.phase],
        ),
    )


def _write_csv(path: Path, rows: list[ResultRow]) -> None:
    """Write flat result rows to CSV."""
    fieldnames = (
        list(rows[0].to_dict().keys())
        if rows
        else list(
            ResultRow(
                suite="codec",
                surface_id="py_reference",
                surface_label="Python reference",
                corpus_id=REAL_CORPUS_ID,
                rows=0,
                dim=0,
                phase="codec_setup",
                metric_unit="ms",
                median=None,
                min=None,
                max=None,
            )
            .to_dict()
            .keys()
        )
    )
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            payload = row.to_dict()
            payload["samples"] = json.dumps(payload["samples"])
            writer.writerow(payload)


def _build_summary(
    *,
    environment: dict[str, Any],
    corpora: list[CorpusArtifact],
    surface_statuses: list[SurfaceStatus],
    rows: list[ResultRow],
    requested_surfaces: list[str],
) -> str:
    """Render the Markdown benchmark summary."""
    row_index = {
        (row.suite, row.corpus_id, row.surface_id, row.phase): row for row in rows
    }
    lines: list[str] = []
    lines.extend(_summary_header(environment, corpora, surface_statuses))
    lines.extend(_summary_codec_table(corpora, requested_surfaces, row_index))
    lines.extend(_summary_search_table(corpora, requested_surfaces, row_index))
    notes = _collect_notes(surface_statuses, rows)
    lines.extend(["", "## Notes"])
    if notes:
        for note in notes:
            lines.append(f"- {note}")
    else:
        lines.append("- No skip or failure notes recorded.")
    return "\n".join(lines) + "\n"


def _summary_header(
    environment: dict[str, Any],
    corpora: list[CorpusArtifact],
    surface_statuses: list[SurfaceStatus],
) -> list[str]:
    """Render the environment, corpus matrix, and surface availability sections."""
    lines: list[str] = [
        "# Cross-Surface Performance Benchmark",
        "",
        "## Environment",
        f"- Generated at: {environment['generated_at']}",
        f"- Host OS: {environment['host_os']}",
        f"- Python version: `{environment['python_version']}`",
        f"- Python executable: `{environment['python_executable']}`",
        f"- Rust toolchain: `{environment['rust_toolchain']}`",
    ]
    rust_runner_env = environment.get("rust_runner", {})
    adapter_name = rust_runner_env.get("wgpu_adapter_name")
    lines.append(
        f"- GPU adapter: `{adapter_name}`"
        if adapter_name
        else "- GPU adapter: unavailable"
    )
    lines.extend(
        [
            "",
            "## Corpus Matrix",
            "| Corpus | Rows | Dim | Source | Query count |",
            "| --- | ---: | ---: | --- | ---: |",
        ]
    )
    for corpus in corpora:
        n_queries = len(corpus.query_indices)
        lines.append(
            f"| `{corpus.corpus_id}` | {corpus.rows} | {corpus.dim}"
            f" | {corpus.source} | {n_queries} |"
        )
    lines.extend(
        [
            "",
            "## Surface Availability",
            "| Surface | Label | Status | Reason |",
            "| --- | --- | --- | --- |",
        ]
    )
    for status in surface_statuses:
        lines.append(
            f"| `{status.surface_id}` | {status.surface_label}"
            f" | {status.status} | {status.reason or ''} |"
        )
    return lines


def _summary_codec_table(
    corpora: list[CorpusArtifact],
    requested_surfaces: list[str],
    row_index: dict[tuple[str, str, str, str], ResultRow],
) -> list[str]:
    """Render the codec results table section."""
    lines: list[str] = [
        "",
        "## Codec Results",
        "| Corpus | Surface | Status | Setup ms | Compress ms"
        + " | Compress vec/s | Decompress ms | Decompress vec/s |",
        "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for corpus in corpora:
        for surface_id in requested_surfaces:
            codec_setup = row_index.get(
                ("codec", corpus.corpus_id, surface_id, "codec_setup")
            )
            codec_compress = row_index.get(
                ("codec", corpus.corpus_id, surface_id, "codec_compress_batch")
            )
            codec_decompress = row_index.get(
                ("codec", corpus.corpus_id, surface_id, "codec_decompress_batch")
            )
            status = _combined_status(codec_setup, codec_compress, codec_decompress)
            lines.append(
                f"| `{corpus.corpus_id}` | {SURFACES[surface_id].label} | {status}"
                f" | {_fmt_metric(codec_setup)}"
                f" | {_fmt_metric(codec_compress)}"
                f" | {_fmt_rate(corpus.rows, codec_compress)}"
                f" | {_fmt_metric(codec_decompress)}"
                f" | {_fmt_rate(corpus.rows, codec_decompress)} |"
            )
    return lines


def _summary_search_table(
    corpora: list[CorpusArtifact],
    requested_surfaces: list[str],
    row_index: dict[tuple[str, str, str, str], ResultRow],
) -> list[str]:
    """Render the search results table section."""
    lines: list[str] = [
        "",
        "## Search Results",
        "| Corpus | Surface | Status | Setup ms | Query batch ms"
        + " | Queries/s | Query p50 ms | Query p95 ms |",
        "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for corpus in corpora:
        for surface_id in requested_surfaces:
            search_setup = row_index.get(
                ("search", corpus.corpus_id, surface_id, "search_setup")
            )
            query_batch = row_index.get(
                ("search", corpus.corpus_id, surface_id, "search_query_batch")
            )
            query_p50 = row_index.get(
                ("search", corpus.corpus_id, surface_id, "search_query_p50")
            )
            query_p95 = row_index.get(
                ("search", corpus.corpus_id, surface_id, "search_query_p95")
            )
            status = _combined_status(search_setup, query_batch, query_p50, query_p95)
            n_queries = len(corpus.query_indices)
            lines.append(
                f"| `{corpus.corpus_id}` | {SURFACES[surface_id].label} | {status}"
                f" | {_fmt_metric(search_setup)}"
                f" | {_fmt_metric(query_batch)}"
                f" | {_fmt_rate(n_queries, query_batch)}"
                f" | {_fmt_metric(query_p50)}"
                f" | {_fmt_metric(query_p95)} |"
            )
    return lines


def _combined_status(*rows: ResultRow | None) -> str:
    """Collapse phase-level statuses into one summary cell."""
    real_rows = [row for row in rows if row is not None]
    if not real_rows:
        return "not requested"
    if any(row.status == "failed" for row in real_rows):
        return "failed"
    if any(row.status == "skipped" for row in real_rows):
        return "skipped"
    return "ok"


def _fmt_metric(row: ResultRow | None) -> str:
    """Format a median metric value for Markdown tables."""
    if row is None or row.median is None:
        return ""
    return f"{row.median:.3f}"


def _fmt_rate(dividend: int, row: ResultRow | None) -> str:
    """Format derived throughput from a timing row."""
    if row is None or row.median in (None, 0.0):
        return ""
    return f"{dividend / (row.median / 1000.0):.1f}"


def _collect_notes(
    surface_statuses: list[SurfaceStatus], rows: list[ResultRow]
) -> list[str]:
    """Collect unique skip and failure notes for the summary footer."""
    notes: list[str] = []
    for status in surface_statuses:
        if status.reason:
            notes.append(f"{status.surface_id}: {status.reason}")
    for row in rows:
        if row.status != "ok" and row.skip_reason:
            note = f"{row.surface_id} {row.corpus_id} {row.phase}: {row.skip_reason}"
            if note not in notes:
                notes.append(note)
    return notes


if __name__ == "__main__":
    raise SystemExit(main())
