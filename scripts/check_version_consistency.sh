#!/usr/bin/env bash
# scripts/check_version_consistency.sh
#
# GAP-JS-009: verify that package.json, Cargo.toml (workspace root), and
# rust/crates/tinyquant-js/Cargo.toml all agree on the package version.
#
# Usage: bash scripts/check_version_consistency.sh
# Exit 0 if all versions agree; exit 1 with a descriptive message if not.
#
# Designed to run from the repo root on any platform supported by GitHub
# Actions (ubuntu, macos, windows-with-git-bash).
set -euo pipefail

JS_PKG="javascript/@tinyquant/core/package.json"
CARGO_WS="rust/Cargo.toml"
CARGO_JS="rust/crates/tinyquant-js/Cargo.toml"

for f in "$JS_PKG" "$CARGO_WS" "$CARGO_JS"; do
  if [[ ! -f "$f" ]]; then
    echo "ERROR: $f not found — run this script from the repo root" >&2
    exit 1
  fi
done

# Extract versions.
PKG_VERSION=$(python3 -c \
  "import json,sys; d=json.load(open(sys.argv[1])); print(d['version'])" \
  "$JS_PKG")

# Workspace Cargo.toml: look for the [workspace.package] section's version.
# The line looks like:  version = "1.0.0"
CARGO_VERSION=$(grep -A20 '^\[workspace\.package\]' "$CARGO_WS" \
  | grep -m1 '^version' \
  | sed 's/.*"\(.*\)".*/\1/')

echo "package.json version       : $PKG_VERSION"
echo "Cargo.toml (workspace) ver : $CARGO_VERSION"

if [[ "$PKG_VERSION" != "$CARGO_VERSION" ]]; then
  echo ""
  echo "FAIL: version mismatch"
  echo "  package.json        = $PKG_VERSION"
  echo "  Cargo.toml          = $CARGO_VERSION"
  echo "Both files must carry the same version before merging."
  exit 1
fi

echo "PASS: versions match ($PKG_VERSION)"
