"""Generate publication-quality plots from benchmark results."""

from __future__ import annotations

import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")  # Non-interactive backend
import matplotlib.pyplot as plt
import numpy as np

RESULTS_DIR = Path(__file__).parent / "results"
PLOTS_DIR = RESULTS_DIR / "plots"


def load_results() -> tuple[dict, list[dict]]:
    """Load benchmark results JSON."""
    with open(RESULTS_DIR / "benchmark_results.json", encoding="utf-8") as f:
        data = json.load(f)
    return data["config"], data["results"]


def setup_style() -> None:
    """Configure matplotlib for publication-quality figures."""
    plt.rcParams.update({
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
    })


def get_colors(results: list[dict]) -> list[str]:
    """Assign colors: baselines in grey/blue, TinyQuant in warm colors."""
    colors = []
    for r in results:
        name = r["name"]
        if "baseline" in name:
            colors.append("#888888")
        elif "FP16" in name:
            colors.append("#4A90D9")
        elif "uint8" in name:
            colors.append("#50B848")
        elif "PQ" in name:
            colors.append("#9B59B6")
        elif "8-bit" in name:
            colors.append("#E67E22")
        elif "4-bit + residual" in name:
            colors.append("#E74C3C")
        elif "4-bit" in name:
            colors.append("#F39C12")
        elif "2-bit + residual" in name:
            colors.append("#C0392B")
        elif "2-bit" in name:
            colors.append("#D35400")
        else:
            colors.append("#34495E")
    return colors


def plot_compression_vs_fidelity(results: list[dict], colors: list[str]) -> None:
    """Scatter plot: compression ratio vs Pearson rho."""
    fig, ax = plt.subplots(figsize=(10, 7))

    for i, r in enumerate(results):
        ax.scatter(
            r["compression_ratio"],
            r["pearson_rho"],
            s=150,
            c=colors[i],
            edgecolors="black",
            linewidths=0.5,
            zorder=5,
        )
        # Label offset to avoid overlaps
        offset_x = 0.15
        offset_y = -0.003
        if "2-bit + residual" in r["name"]:
            offset_y = 0.004
        elif "PQ" in r["name"]:
            offset_x = -0.3
        ax.annotate(
            r["name"],
            (r["compression_ratio"], r["pearson_rho"]),
            textcoords="offset points",
            xytext=(8, -4),
            fontsize=8,
            ha="left",
        )

    ax.set_xlabel("Compression Ratio (x)")
    ax.set_ylabel("Pearson Correlation (rho)")
    ax.set_title("Compression Ratio vs. Similarity Fidelity\n(higher rho = better ranking preservation)")
    ax.axhline(y=0.995, color="green", linestyle="--", alpha=0.5, label="rho = 0.995 target")
    ax.axhline(y=0.99, color="orange", linestyle="--", alpha=0.5, label="rho = 0.99")
    ax.legend(loc="lower left")
    ax.set_ylim(bottom=min(r["pearson_rho"] for r in results) - 0.01, top=1.002)

    fig.tight_layout()
    fig.savefig(PLOTS_DIR / "compression_vs_fidelity.png", bbox_inches="tight")
    plt.close(fig)
    print("  Saved compression_vs_fidelity.png")


def plot_storage_comparison(results: list[dict], colors: list[str]) -> None:
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
    for bar, bpv, r in zip(bars, bytes_per_vec, results):
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


def plot_fidelity_metrics(results: list[dict], colors: list[str]) -> None:
    """Grouped bar chart: Pearson rho and top-k recall side by side."""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    names = [r["name"].replace(" (baseline)", "\n(baseline)") for r in results]
    rhos = [r["pearson_rho"] for r in results]
    recalls = [r["top_k_recall"] for r in results]

    x = np.arange(len(names))

    # Pearson rho
    bars1 = ax1.bar(x, rhos, color=colors, edgecolor="black", linewidth=0.5)
    ax1.set_xticks(x)
    ax1.set_xticklabels(names, rotation=45, ha="right", fontsize=8)
    ax1.set_ylabel("Pearson rho")
    ax1.set_title("Similarity Ranking Preservation")
    ax1.set_ylim(bottom=min(rhos) - 0.02, top=1.005)
    ax1.axhline(y=0.995, color="green", linestyle="--", alpha=0.5, linewidth=0.8)

    # Top-k recall
    bars2 = ax2.bar(x, recalls, color=colors, edgecolor="black", linewidth=0.5)
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


def plot_pareto_frontier(results: list[dict], colors: list[str]) -> None:
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


def plot_throughput(results: list[dict], colors: list[str]) -> None:
    """Bar chart: encode and decode throughput."""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # Filter out FP32 baseline (0 time)
    filtered = [(r, c) for r, c in zip(results, colors) if r["compress_time_ms"] > 0]
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

    config, results = load_results()
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
