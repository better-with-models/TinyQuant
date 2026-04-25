#!/usr/bin/env bash
# Local mirror of the calibration-arm64 leg in .github/workflows/rust-calibration.yml.
#
# Targeted for aarch64 hosts (Apple Silicon macOS, aarch64 Linux).  Adds
# --features simd to every group to exercise the NEON dispatch path; the CI
# leg does not pass simd features because GitHub's ubuntu-24.04-arm runner
# picks up the default feature set, but local runs benefit from explicit NEON
# coverage.
#
# macOS PATH note: /opt/homebrew/bin ships Homebrew's own cargo (typically a
# newer version than the workspace MSRV).  This script prepends ~/.cargo/bin
# so rustup's toolchain-pinned cargo (1.81) takes precedence.
#
# Run from any directory — the script locates the rust/ workspace via its own
# path.  Fixtures must be present (git lfs pull if absent).
#
# Usage:
#   bash calibration/run_calibration_arm64.sh
#
# Outputs (written relative to repo root, not this script's location):
#   calibration_arm64_run.log       full test output, one run per group
#   calibration_arm64_summary.txt   pipe-delimited: name|rc|elapsed_s
set -euo pipefail

# Prepend rustup cargo ahead of any Homebrew or system cargo.
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TERM_COLOR=always
export RUSTFLAGS="-D warnings"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUST_DIR="$REPO_ROOT/rust"
LOG="$REPO_ROOT/calibration_arm64_run.log"
SUMMARY="$REPO_ROOT/calibration_arm64_summary.txt"

: > "$LOG"
: > "$SUMMARY"

run_group() {
  local name="$1" filter="$2" pkg="$3" features="$4"
  local start end rc
  start=$(date +%s)
  echo "=== START $name @ $(date -Iseconds) ===" | tee -a "$LOG"
  # shellcheck disable=SC2086
  cargo test --release $pkg $features -- --ignored "$filter" \
    >>"$LOG" 2>&1
  rc=$?
  end=$(date +%s)
  echo "=== END   $name rc=$rc elapsed=$((end - start))s ===" | tee -a "$LOG"
  echo "$name|$rc|$((end - start))" >>"$SUMMARY"
  return $rc
}

cd "$RUST_DIR" || { echo "ERROR: rust/ not found at $RUST_DIR"; exit 99; }

# Groups mirror the calibration-arm64 CI matrix with --features simd added to
# enable NEON dispatch on every group.
#
# tinyquant-bench: --features simd  (bench crate has no std feature)
# tinyquant-core:  --features std,simd  (core integration tests + NEON paths)
run_group "pr-speed"          "pr_speed"             "-p tinyquant-bench" "--features simd"
run_group "full-bw2"          "full_bw2"             "-p tinyquant-bench" "--features simd"
run_group "full-bw4"          "full_bw4"             "-p tinyquant-bench" "--features simd"
run_group "full-bw8"          "full_bw8"             "-p tinyquant-bench" "--features simd"
run_group "core-codebook"     "train_matches_python" "-p tinyquant-core"  "--features std,simd"
run_group "core-codec-parity" "codec"                "-p tinyquant-core"  "--features std,simd"

echo "DONE @ $(date -Iseconds)" | tee -a "$LOG"
