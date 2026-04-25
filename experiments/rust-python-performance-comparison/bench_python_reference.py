"""Legacy compatibility wrapper for the cross-surface benchmark orchestrator."""

from __future__ import annotations

from run_benchmark import main as run_benchmark_main


def main() -> int:
    """Run the historical reference-only codec case through the new harness."""
    return run_benchmark_main(
        [
            "--surfaces",
            "py_reference",
            "--suites",
            "codec",
            "--corpora",
            "real_openai_335_d1536",
            "--warmups",
            "1",
            "--reps",
            "5",
        ]
    )


if __name__ == "__main__":
    raise SystemExit(main())
