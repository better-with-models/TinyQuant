# AGENTS.md — Guide for AI Agents Working in `calibration`

`calibration/` holds the local runner scripts that mirror the
`rust-calibration.yml` workflow legs. These scripts are run manually before a
release or after a codec-algorithm change, not by CI. Each script logs to a
plain-text file at the repo root and appends a pipe-delimited summary.

## What this area contains

- primary responsibility: local mirrors of the GitHub Actions calibration legs —
  run the same `--release` test groups against the same LFS-backed fixture corpus
  used in CI, without needing the full GitHub runner environment.
- main entrypoints:
  - `run_calibration_arm64.sh` — mirrors the `calibration-arm64` CI leg;
    enables `--features simd` on all groups to exercise NEON dispatch.
  - `run_calibration_x86.sh` — mirrors the `calibration-x86` CI leg;
    runs scalar (no-SIMD) test groups to match the self-hosted docker runner.
- outputs: `calibration_<arch>_run.log` and `calibration_<arch>_summary.txt`
  written at the repo root.
- fixture generator lives separately at `scripts/calibration/gen_openai_sample.py`.

## Layout

```text
calibration/
├── run_calibration_arm64.sh
├── run_calibration_x86.sh
├── AGENTS.md
└── CLAUDE.md
```

## Common workflows

### Run calibration on this machine

```bash
# ARM64 (Apple Silicon, aarch64 Linux):
bash calibration/run_calibration_arm64.sh

# x86_64 (Linux):
bash calibration/run_calibration_x86.sh
```

Both scripts must be run from the repo root or any directory — they use
`$(dirname "$0")` to locate the `rust/` workspace regardless of invocation path.
Fixtures must be present under `rust/crates/tinyquant-bench/fixtures/`; pull
with `git lfs pull` if absent.

### Add or modify a test group

1. Mirror the change in the corresponding CI matrix in
   `.github/workflows/rust-calibration.yml` at the same time.
2. Update both `run_calibration_arm64.sh` and `run_calibration_x86.sh` if the
   group applies to both legs (all current groups do).

## Invariants — Do Not Violate

- keep these scripts in sync with the corresponding CI matrix in
  `.github/workflows/rust-calibration.yml`; silent divergence defeats the
  purpose of the local mirrors.
- do not add groups here that are not also in the CI matrix, or vice versa,
  without a corresponding update to both surfaces.
- arm64 groups use `--features simd` to exercise NEON dispatch; x86 groups
  mirror CI exactly (no simd features).
- fixture generation is not this directory's job — use
  `scripts/calibration/gen_openai_sample.py`.

## See Also

- [Root AGENTS.md](../AGENTS.md)
- [CI workflow](../.github/workflows/rust-calibration.yml)
- [Fixture generator](../scripts/calibration/gen_openai_sample.py)
