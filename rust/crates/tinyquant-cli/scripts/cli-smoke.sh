#!/usr/bin/env bash
# End-to-end CLI smoke chain: info -> codec train -> compress -> decompress
# -> verify -> corpus search. Designed to run against a fresh `cargo build
# --release` output without any extra test fixtures — we synthesize the
# training matrix on the fly via /dev/urandom.
#
# Usage: scripts/cli-smoke.sh [path/to/tinyquant]
# Exit: 0 on success, non-zero on any failure (set -e).
#
# MSE check uses a tiny Python shim (stdlib only). If `python3` is not on
# the path the script degrades to a byte-length comparison.

set -euo pipefail

BIN="${1:-${TINYQUANT_BIN:-target/release/tinyquant}}"
if [[ ! -x "$BIN" && ! -x "$BIN.exe" ]]; then
    # Fall back to Windows-style path if caller handed us a UNIX-style one.
    if [[ -x "${BIN}.exe" ]]; then BIN="${BIN}.exe"; fi
fi

ROWS=1024
COLS=32
BIT=4
SEED=7

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

INPUT="$TMP/train.f32.bin"
CODEBOOK="$TMP/codebook.tqcb"
CONFIG="$TMP/config.json"
CORPUS="$TMP/corpus.tqcv"
DECOMP="$TMP/decompressed.f32.bin"
QUERY="$TMP/query.f32.bin"

# Deterministic well-formed FP32 training data — random bytes from
# /dev/urandom produce NaN / Inf bit patterns that poison the MSE
# check downstream. Use a tiny Python generator instead.
python3 - "$INPUT" "$ROWS" "$COLS" 0 <<'PY'
import struct, sys, random
path, rows, cols, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
rng = random.Random(seed)
with open(path, "wb") as f:
    f.write(struct.pack(f"{rows * cols}f", *[rng.gauss(0.0, 1.0) for _ in range(rows * cols)]))
PY
python3 - "$QUERY" 1 "$COLS" 1 <<'PY'
import struct, sys, random
path, rows, cols, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
rng = random.Random(seed)
with open(path, "wb") as f:
    f.write(struct.pack(f"{rows * cols}f", *[rng.gauss(0.0, 1.0) for _ in range(rows * cols)]))
PY

echo "== info =="
"$BIN" info

echo "== codec train =="
"$BIN" codec train \
    --input "$INPUT" --rows "$ROWS" --cols "$COLS" \
    --bit-width "$BIT" --seed "$SEED" --residual \
    --format f32 --output "$CODEBOOK" --config-out "$CONFIG"

echo "== codec compress =="
"$BIN" codec compress \
    --input "$INPUT" --rows "$ROWS" --cols "$COLS" \
    --config-json "$CONFIG" --codebook "$CODEBOOK" \
    --output "$CORPUS" --format f32

echo "== codec decompress =="
"$BIN" codec decompress \
    --input "$CORPUS" --config-json "$CONFIG" --codebook "$CODEBOOK" \
    --output "$DECOMP" --format f32

echo "== verify codebook + corpus =="
"$BIN" verify "$CODEBOOK"
"$BIN" verify "$CORPUS"

echo "== corpus search =="
"$BIN" corpus search \
    --corpus "$CORPUS" --query "$QUERY" \
    --codebook "$CODEBOOK" --config-json "$CONFIG" \
    --top-k 3 --format json

echo "== MSE check =="
if command -v python3 >/dev/null 2>&1; then
    python3 - "$INPUT" "$DECOMP" <<'PY'
import struct, sys
def load(path):
    with open(path, "rb") as f:
        data = f.read()
    return struct.unpack(f"{len(data) // 4}f", data)
orig = load(sys.argv[1])
back = load(sys.argv[2])
assert len(orig) == len(back), f"length mismatch {len(orig)} vs {len(back)}"
sq = sum((a - b) ** 2 for a, b in zip(orig, back))
mse = sq / len(orig)
print(f"mse={mse:.6f}")
# Release gate threshold — pinned by
# docs/plans/rust/phase-22-pyo3-cabi-release.md §CLI smoke test matrix.
# The seed / dataset combination (rows=1024, cols=32, bit_width=4, seed=7
# with residual) is chosen to stay well under this bound; Phase 22.A
# parity tests observed MSE ~3e-4 cross-impl, and the CLI drives the
# exact same codec.
assert mse < 1e-2, f"mse {mse} too large"
PY
else
    # Degraded path: byte-length parity.
    s1="$(wc -c < "$INPUT")"
    s2="$(wc -c < "$DECOMP")"
    if [[ "$s1" != "$s2" ]]; then
        echo "byte-length mismatch: $s1 != $s2" >&2
        exit 1
    fi
    echo "byte-length check passed (python3 not available for MSE)"
fi

echo "== smoke ok =="
