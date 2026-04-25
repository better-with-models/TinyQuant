#!/usr/bin/env bash

set -euo pipefail

# Header
echo "CI Local Simulate — TinyQuant parallel chunks"
echo ""

# Detect repo root and navigate there
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
echo "Working directory: $REPO_ROOT"
echo ""

# Clean stale coverage files
echo "Cleaning stale coverage files..."
rm -f .coverage .coverage.*
echo ""

# Run 7 pytest chunks sequentially with dedicated coverage files

echo "Running chunk 1: codec"
COVERAGE_FILE=.coverage.codec python -m pytest tests/codec/ -x --tb=short --cov=tinyquant_py_reference --cov-report=
echo ""

echo "Running chunk 2: corpus"
COVERAGE_FILE=.coverage.corpus python -m pytest tests/corpus/ -x --tb=short --cov=tinyquant_py_reference --cov-report=
echo ""

echo "Running chunk 3: backend"
COVERAGE_FILE=.coverage.backend python -m pytest tests/backend/ tests/test_smoke.py -x --tb=short --cov=tinyquant_py_reference --cov-report=
echo ""

echo "Running chunk 4: architecture"
COVERAGE_FILE=.coverage.arch python -m pytest tests/architecture/ -x --tb=short --cov=tinyquant_py_reference --cov-report=
echo ""

echo "Running chunk 5: integration (excluding pgvector)"
COVERAGE_FILE=.coverage.intloc python -m pytest tests/integration/ --ignore=tests/integration/test_pgvector.py -x --tb=short --cov=tinyquant_py_reference --cov-report=
echo ""

echo "Running chunk 6: integration pgvector"
if [[ -z "${PGVECTOR_TEST_DSN:-}" ]]; then
  echo "SKIP pgvector chunk: PGVECTOR_TEST_DSN not set"
else
  COVERAGE_FILE=.coverage.intpg python -m pytest tests/integration/test_pgvector.py -x --tb=short --cov=tinyquant_py_reference --cov-report=
fi
echo ""

echo "Running chunk 7: e2e"
COVERAGE_FILE=.coverage.e2e python -m pytest tests/e2e/ -x --tb=short --cov=tinyquant_py_reference --cov-report=
echo ""

# Combine coverage reports
echo "Combining coverage reports..."
python -m coverage combine
echo ""

# Enforce coverage gates
echo "Enforcing coverage gates..."
python -m coverage report --fail-under=90
python -m coverage xml
python -m coverage report --include="*/tinyquant_py_reference/codec/*" --fail-under=94
python -m coverage report --include="*/tinyquant_py_reference/corpus/*" --fail-under=90
echo ""

# Phase 23: exercise the wheel-leak guard locally.
echo "Running wheel-leak guard: build-package-does-not-leak-reference"
rm -rf dist/
python -m build --wheel --outdir dist/ 2>&1 | tee /tmp/phase23-build.log || true
if compgen -G 'dist/*.whl' > /dev/null; then
  for whl in dist/*.whl; do
    echo "  Inspecting $whl"
    if python -m zipfile -l "$whl" | grep -E 'tinyquant_py_reference|tests/reference/' > /dev/null; then
      echo "FAIL: $whl contains reference paths — this is a Phase 23 regression."
      exit 1
    fi
  done
  echo "  Wheels present but do not contain reference paths (hatchling produced an empty wheel with bypass-selection=true)."
else
  echo "  No wheels produced — build contract satisfied trivially."
fi
echo ""

echo "All gates passed."
