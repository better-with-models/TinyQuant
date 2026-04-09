"""Generate publication-quality plots from benchmark results."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import matplotlib
import matplotlib.pyplot as plt
import numpy as np

# Non-interactive backend — safe to switch after pyplot import because no
# figures have been created yet.
matplotlib.use("Agg")

RESULTS_DIR = Path(__file__).parent / "results"
PLOTS_DIR = RESULTS_DIR / "plots"


def load_results() -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """Load benchmark results JSON."""
    with open(RESULTS_DIR / "benchmark_results.json", encoding="utf-8") as f:
        data = json.load(f)
    return data["config"], data["results"]


def setup_style() -> None:
    """Configure matplotlib for publication-quality figures."""
    plt.rcParams.update(
        {
            "figure.figsize": (10, 6),
            "figure.dpi": 150,
            "font.size": 11,
            "axes.titlesize": 13,
            "axes.labelsize": 12,
            "xtick.labelsize": 10,
            "ytick.labelsize": 10,
            "legend.fontsize": 9,
            "figure.facecolor": "white",
            "axes.grid": True,
            "grid.alpha": 0.3,
        }
    )


def get_colors(results: list[dict[str, Any]]) -> list[str]:
    """Assign colors: baselines in grey/blue, TinyQuant in warm colors."""
    # Ordered (substring, color) pairs — first match wins, so residual
    # variants must appear before their non-residual counterparts.
    color_map: list[tuple[str, str]] = [
        ("baseline", "#888888"),
        ("FP16", "#4A90D9"),
        ("uint8", "#50B848"),
        ("PQ", "#9B59B6"),
        ("8-bit", "#E67E22"),
        ("4-bit + residual", "#E74C3C"),
        ("4-bit", "#F39C12"),
        ("2-bit + residual", "#C0392B"),
        ("2-bit", "#D35400"),
    ]
    default = "#34495E"
    return [next((c for k, c in color_map if k in r["name"]), default) for r in results]


def _marker_for(name: str) -> str:
    """Choose a marker shape per method category."""
    if "TinyQuant" in name:
        return "o"  # circle
    if "PQ" in name:
        return "^"  # triangle
    if "uint8" in name:
        return "s"  # square
    if "FP16" in name:
        return "D"  # diamond
    return "P"  # plus (FP32 baseline)


def plot_compression_vs_fidelity(
    results: list[dict[str, Any]], colors: list[str]
) -> None:
    """Scatter plot: compression ratio vs Pearson rho.

    The cluster of methods near (ratio < 5, rho ≈ 1.0) is identified
    via a legend rather than overlapping inline labels. Well-separated
    outliers (TinyQuant 4-bit and 2-bit) are annotated directly.
    """
    fig, ax = plt.subplots(figsize=(11, 7))

    # Plot each method as its own series so it appears in the legend.
    for i, r in enumerate(results):
        ax.scatter(
            r["compression_ratio"],
            r["pearson_rho"],
            s=180,
            c=colors[i],
            edgecolors="black",
            linewidths=0.7,
            marker=_marker_for(r["name"]),
            label=r["name"],
            zorder=5,
        )

    # Annotate only the well-separated outliers with leader lines.
    annotate_names = {"TinyQuant 4-bit", "TinyQuant 2-bit"}
    for r in results:
        if r["name"] not in annotate_names:
            continue
        ax.annotate(
            r["name"],
            xy=(r["compression_ratio"], r["pearson_rho"]),
            xytext=(-60, 30),
            textcoords="offset points",
            fontsize=10,
            fontweight="bold",
            ha="center",
            arrowprops={
                "arrowstyle": "->",
                "color": "black",
                "lw": 0.8,
                "shrinkA": 0,
                "shrinkB": 8,
            },
        )

    # Threshold reference lines
    ax.axhline(
        y=0.995,
        color="green",
        linestyle="--",
        alpha=0.5,
        linewidth=1.0,
        label="rho = 0.995 (target)",
    )
    ax.axhline(
        y=0.99,
        color="orange",
        linestyle="--",
        alpha=0.5,
        linewidth=1.0,
        label="rho = 0.99",
    )

    ax.set_xlabel("Compression Ratio (x)")
    ax.set_ylabel("Pearson Correlation (rho)")
    ax.set_title(
        "Compression Ratio vs. Similarity Fidelity\n"
        "(higher rho = better ranking preservation)"
    )

    rho_min = min(r["pearson_rho"] for r in results)
    ax.set_ylim(bottom=rho_min - 0.01, top=1.005)
    ax.set_xlim(left=0, right=max(r["compression_ratio"] for r in results) * 1.1)

    # Legend outside the plot area on the right to avoid covering data points.
    ax.legend(
        loc="center left",
        bbox_to_anchor=(1.02, 0.5),
        fontsize=9,
        framealpha=0.95,
        title="Method",
        title_fontsize=10,
    )

    fig.tight_layout()
    fig.savefig(PLOTS_DIR / "compression_vs_fidelity.png", bbox_inches="tight")
    plt.close(fig)
    print("  Saved compression_vs_fidelity.png")


def plot_storage_comparison(results: list[dict[str, Any]], colors: list[str]) -> None:
    """Horizontal bar chart: bytes per vector."""
    fig, ax = plt.subplots(figsize=(10, 6))

    names = [r["name"] for r in results]
    bytes_per_vec = [r["bytes_per_vector"] for r in results]

    y_pos = np.arange(len(names))
    bars = ax.barh(y_pos, bytes_per_vec, color=colors, edgecolor="black", linewidth=0.5)

    ax.set_yticks(y_pos)
    ax.set_yticklabels(names)
    ax.set_xlabel("Bytes per Vector")
    ax.set_title("Storage Cost per Vector by Method")
    ax.invert_yaxis()

    # Add value labels
    for bar, bpv, r in zip(bars, bytes_per_vec, results, strict=True):
        ratio = r["compression_ratio"]
        ax.text(
            bar.get_width() + 20,
            bar.get_y() + bar.get_height() / 2,
            f"{bpv:.0f} B ({ratio:.1f}x)",
            va="center",
            fontsize=9,
        )

    ax.set_xlim(right=max(bytes_per_vec) * 1.3)
    fig.tight_layout()
    fig.savefig(PLOTS_DIR / "storage_comparison.png", bbox_inches="tight")
    plt.close(fig)
    print("  Saved storage_comparison.png")


def plot_fidelity_metrics(results: list[dict[str, Any]], colors: list[str]) -> None:
    """Grouped bar chart: Pearson rho and top-k recall side by side."""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    names = [r["name"].replace(" (baseline)", "\n(baseline)") for r in results]
    rhos = [r["pearson_rho"] for r in results]
    recalls = [r["top_k_recall"] for r in results]

    x = np.arange(len(names))

    # Pearson rho
    ax1.bar(x, rhos, color=colors, edgecolor="black", linewidth=0.5)
    ax1.set_xticks(x)
    ax1.set_xticklabels(names, rotation=45, ha="right", fontsize=8)
    ax1.set_ylabel("Pearson rho")
    ax1.set_title("Similarity Ranking Preservation")
    ax1.set_ylim(bottom=min(rhos) - 0.02, top=1.005)
    ax1.axhline(y=0.995, color="green", linestyle="--", alpha=0.5, linewidth=0.8)

    # Top-k recall
    ax2.bar(x, recalls, color=colors, edgecolor="black", linewidth=0.5)
    ax2.set_xticks(x)
    ax2.set_xticklabels(names, rotation=45, ha="right", fontsize=8)
    ax2.set_ylabel(f"Top-{results[0].get('top_k', 5)} Recall")
    ax2.set_title("Neighbor Retrieval Accuracy")
    ax2.set_ylim(bottom=min(recalls) - 0.05, top=1.05)
    ax2.axhline(y=0.80, color="orange", linestyle="--", alpha=0.5, linewidth=0.8)

    fig.tight_layout()
    fig.savefig(PLOTS_DIR / "fidelity_metrics.png", bbox_inches="tight")
    plt.close(fig)
    print("  Saved fidelity_metrics.png")


def plot_pareto_frontier(results: list[dict[str, Any]], colors: list[str]) -> None:
    """Pareto chart: bits per dimension vs MSE (log scale)."""
    fig, ax = plt.subplots(figsize=(10, 7))

    for i, r in enumerate(results):
        mse_val = r["mse"] if r["mse"] > 0 else 1e-12
        ax.scatter(
            r["bits_per_dim"],
            mse_val,
            s=180,
            c=colors[i],
            edgecolors="black",
            linewidths=0.5,
            zorder=5,
            label=r["name"],
        )

    ax.set_xlabel("Bits per Dimension")
    ax.set_ylabel("Mean Squared Error (log scale)")
    ax.set_title("Rate-Distortion: Bits per Dimension vs. Reconstruction Error")
    ax.set_yscale("log")
    ax.legend(loc="upper right", fontsize=8, ncol=2)

    fig.tight_layout()
    fig.savefig(PLOTS_DIR / "pareto_rate_distortion.png", bbox_inches="tight")
    plt.close(fig)
    print("  Saved pareto_rate_distortion.png")


def plot_throughput(results: list[dict[str, Any]], colors: list[str]) -> None:
    """Bar chart: encode and decode throughput."""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # Filter out FP32 baseline (0 time)
    filtered = [
        (r, c)
        for r, c in zip(results, colors, strict=True)
        if r["compress_time_ms"] > 0
    ]
    names = [r["name"] for r, _ in filtered]
    enc_times = [r["compress_time_ms"] for r, _ in filtered]
    dec_times = [r["decompress_time_ms"] for r, _ in filtered]
    fcolors = [c for _, c in filtered]

    x = np.arange(len(names))

    ax1.bar(x, enc_times, color=fcolors, edgecolor="black", linewidth=0.5)
    ax1.set_xticks(x)
    ax1.set_xticklabels(names, rotation=45, ha="right", fontsize=8)
    ax1.set_ylabel("Time (ms)")
    ax1.set_title("Compression Time (entire corpus)")

    ax2.bar(x, dec_times, color=fcolors, edgecolor="black", linewidth=0.5)
    ax2.set_xticks(x)
    ax2.set_xticklabels(names, rotation=45, ha="right", fontsize=8)
    ax2.set_ylabel("Time (ms)")
    ax2.set_title("Decompression Time (entire corpus)")

    fig.tight_layout()
    fig.savefig(PLOTS_DIR / "throughput.png", bbox_inches="tight")
    plt.close(fig)
    print("  Saved throughput.png")


def main() -> None:
    """Generate all plots."""
    PLOTS_DIR.mkdir(parents=True, exist_ok=True)
    setup_style()

    _config, results = load_results()
    colors = get_colors(results)

    print(f"Generating plots for {len(results)} methods...\n")

    plot_compression_vs_fidelity(results, colors)
    plot_storage_comparison(results, colors)
    plot_fidelity_metrics(results, colors)
    plot_pareto_frontier(results, colors)
    plot_throughput(results, colors)

    print(f"\nAll plots saved to {PLOTS_DIR}/")


if __name__ == "__main__":
    main()
