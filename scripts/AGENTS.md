# AGENTS.md — Guide for AI Agents Working in `scripts`

`scripts/` holds repository automation that runs outside the product
source tree: pre-commit verification, local CI simulation, and Rust
fixture regeneration.

## What this area contains

- primary responsibility: reproducible repo-wide checks that gate commits
  and CI, plus the fixture generator that keeps Python and Rust in parity.
- main entrypoints:
  - `pre-commit.ps1` — PowerShell entrypoint used by the versioned Git
    pre-commit hook (`.githooks/pre-commit`).
  - `verify_pre_commit.py` — documentation + lint policy enforcement.
  - `ci_local_simulate.sh` — reproduces the CI pipeline locally.
  - `generate_rust_fixtures.py` — refreshes the golden fixtures shared
    between Python and Rust parity tests.
  - `calibration/gen_openai_sample.py` — generates synthetic LFS-backed
    calibration fixtures for `tinyquant-bench`.
- common changes: extend pre-commit checks when new policy lands, adjust
  fixture generation when codec byte layout changes, keep CI simulation
  aligned with `.github/workflows/`.
- calibration run scripts (local mirrors of the CI calibration legs) live in
  the top-level `calibration/` directory, not here.

## Layout

```text
scripts/
├── ci_local_simulate.sh
├── generate_rust_fixtures.py
├── pre-commit.ps1
├── verify_pre_commit.py
└── README.md
```

## Common workflows

### Extend pre-commit verification

1. Add the new rule inside `verify_pre_commit.py` and re-run the hook
   locally via `pwsh -File scripts/pre-commit.ps1`.
2. Document the new expectation in the root `AGENTS.md` under
   `## Pre-commit verification` so contributors see it.
3. Mirror the change in CI (`.github/workflows/ci.yml` or
   `docs-lint.yml`) when the check also belongs in the gated pipeline.

### Refresh Rust fixtures

1. Run `python scripts/generate_rust_fixtures.py` after any codec change
   that affects byte layout.
2. Commit the regenerated fixtures under
   `rust/crates/tinyquant-core/tests/fixtures/` alongside the behavior
   change that motivated them.

## Invariants — Do Not Violate

- every `.py` entrypoint in this tree opens with a module docstring per
  `$python-docstrings`.
- pre-commit logic here must stay consistent with `.pre-commit-config.yaml`
  and the CI pipeline; do not add a check to one surface without mirroring
  it in the other.
- do not hide opt-in bypasses (`--no-verify`, skipped hooks) behind these
  scripts; failures must surface.

## See Also

- [Root AGENTS.md](../AGENTS.md)
- [Pre-commit config](../.pre-commit-config.yaml)
- [CI workflows](../.github/workflows/)
