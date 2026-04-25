#!/usr/bin/env bash
# scripts/check_version_consistency.sh
#
# GAP-JS-009: verify that package.json, the workspace Cargo.toml, and (when
# the crate sets an explicit version) rust/crates/tinyquant-js/Cargo.toml all
# carry the same version string.
#
# Workspace-inheriting crates use `version.workspace = true` and do not carry
# a local `version = "…"` line; in that case only the workspace version is
# compared. If the crate ever adds a local override, this script will catch it.
#
# Usage: bash scripts/check_version_consistency.sh
# Exit 0 if all present versions agree; exit 1 with a descriptive message if not.
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

# If tinyquant-js/Cargo.toml carries an explicit local version (not workspace-
# inherited), verify it matches too.  Workspace-inheriting crates omit a
# top-level `version = "…"` line and use `version.workspace = true` instead,
# which does not match the grep below.
CARGO_JS_RAW=$(grep -m1 '^version\s*=\s*"' "$CARGO_JS" || true)
if [[ -n "$CARGO_JS_RAW" ]]; then
  CARGO_JS_VERSION=$(echo "$CARGO_JS_RAW" | sed 's/.*"\([^"]*\)".*/\1/')
  echo "Cargo.toml (crate) ver     : $CARGO_JS_VERSION"
  if [[ "$PKG_VERSION" != "$CARGO_JS_VERSION" ]]; then
    echo ""
    echo "FAIL: crate version mismatch"
    echo "  package.json                     = $PKG_VERSION"
    echo "  rust/crates/tinyquant-js/Cargo.toml = $CARGO_JS_VERSION"
    echo "Both files must carry the same version before merging."
    exit 1
  fi
else
  echo "Cargo.toml (crate)         : inherits workspace version"
fi

echo "PASS: versions match ($PKG_VERSION)"
