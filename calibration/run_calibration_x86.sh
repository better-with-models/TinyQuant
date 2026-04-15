#!/usr/bin/env bash
# Local mirror of the calibration-x86 leg in .github/workflows/rust-calibration.yml.
#
# Run from any directory — the script locates the rust/ workspace via its own
# path.  Fixtures must be present (git lfs pull if absent).
#
# Usage:
#   bash calibration/run_calibration_x86.sh
#
# Outputs (written relative to repo root, not this script's location):
#   calibration_x86_run.log       full test output, one run per group
#   calibration_x86_summary.txt   pipe-delimited: name|rc|elapsed_s
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TERM_COLOR=always
export RUSTFLAGS="-D warnings"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUST_DIR="$REPO_ROOT/rust"
LOG="$REPO_ROOT/calibration_x86_run.log"
SUMMARY="$REPO_ROOT/calibration_x86_summary.txt"

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

# Groups mirror the calibration-x86 CI matrix exactly.
# tinyquant-bench has no std feature (only simd); features field is "".
# tinyquant-core integration tests require --features std.
run_group "pr-speed"          "pr_speed"             "-p tinyquant-bench" ""
run_group "full-bw2"          "full_bw2"             "-p tinyquant-bench" ""
run_group "full-bw4"          "full_bw4"             "-p tinyquant-bench" ""
run_group "full-bw8"          "full_bw8"             "-p tinyquant-bench" ""
run_group "core-codebook"     "train_matches_python" "-p tinyquant-core"  "--features std"
run_group "core-codec-parity" "codec"                "-p tinyquant-core"  "--features std"

echo "DONE @ $(date -Iseconds)" | tee -a "$LOG"
