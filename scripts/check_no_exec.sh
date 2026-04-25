#!/usr/bin/env bash
# scripts/check_no_exec.sh
#
# GAP-JS-010: scan compiled JS/TS output for imports or requires of
# subprocess-spawning Node.js APIs. The TinyQuant loader must not invoke
# child processes — all codec work happens in the native .node binding.
#
# Usage: bash scripts/check_no_exec.sh <directory>
# Exit 0 if no forbidden patterns found; exit 1 with matches listed.
set -euo pipefail

TARGET="${1:?Usage: $0 <directory>}"

if [[ ! -d "$TARGET" ]]; then
  echo "ERROR: target directory '$TARGET' does not exist" >&2
  exit 1
fi

# Patterns that indicate subprocess invocation in compiled JS/TS output.
PATTERNS=(
  'require("child_process")'
  "require('child_process')"
  'from "child_process"'
  "from 'child_process'"
  'spawnSync'
  'execSync'
  'execFileSync'
)

FOUND=0
for PATTERN in "${PATTERNS[@]}"; do
  # grep returns exit 1 when there are no matches; suppress that.
  MATCHES=$(grep -r --include="*.js" --include="*.mjs" --include="*.cjs" \
    -l "$PATTERN" "$TARGET" 2>/dev/null || true)
  if [[ -n "$MATCHES" ]]; then
    echo "FAIL: found '$PATTERN' in:"
    echo "$MATCHES" | sed 's/^/  /'
    FOUND=1
  fi
done

if [[ "$FOUND" -eq 1 ]]; then
  echo ""
  echo "Loader must not use child_process or synchronous spawn/exec APIs."
  exit 1
fi

echo "PASS: no subprocess-spawning patterns found in $TARGET"
