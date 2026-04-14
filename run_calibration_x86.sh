#!/usr/bin/env bash
# Local mirror of .github/workflows/rust-calibration.yml x86_64 leg.
set -uo pipefail
export PATH="/c/Users/aaqui/.cargo/bin:$PATH"
export CARGO_TERM_COLOR=always
export RUSTFLAGS="-D warnings"

cd "$(dirname "$0")/rust" || exit 99
LOG="../calibration_x86_run.log"
: > "$LOG"

run_group() {
  local name="$1" filter="$2" pkg="$3" features="$4"
  local start end rc
  start=$(date +%s)
  echo "=== START $name @ $(date -Iseconds) ===" | tee -a "$LOG"
  # shellcheck disable=SC2086
  cargo test --release $pkg $features -- --ignored "$filter" >>"$LOG" 2>&1
  rc=$?
  end=$(date +%s)
  echo "=== END   $name rc=$rc elapsed=$((end-start))s ===" | tee -a "$LOG"
  echo "$name|$rc|$((end-start))" >> ../calibration_x86_summary.txt
  return $rc
}

: > ../calibration_x86_summary.txt
run_group "pr-speed"          "pr_speed"             "-p tinyquant-bench" ""
run_group "full-bw2"          "full_bw2"             "-p tinyquant-bench" ""
run_group "full-bw4"          "full_bw4"             "-p tinyquant-bench" ""
run_group "full-bw8"          "full_bw8"             "-p tinyquant-bench" ""
run_group "core-codebook"     "train_matches_python" "-p tinyquant-core"  "--features std"
run_group "core-codec-parity" "codec"                "-p tinyquant-core"  "--features std"

echo "DONE @ $(date -Iseconds)" | tee -a "$LOG"
