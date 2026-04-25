# Scripts

Repository automation, verification helpers, and packaging tools.

## Root scripts

| File | Purpose |
| --- | --- |
| `pre-commit.ps1` | PowerShell entrypoint for the versioned Git pre-commit hook |
| `verify_pre_commit.py` | Documentation verification logic run by the pre-commit hook and CI |
| `generate_rust_fixtures.py` | Generate Rust test fixtures from the Python reference implementation |
| `ci_local_simulate.sh` | Simulate key CI steps locally before pushing |

## Subdirectories

| Directory | Purpose |
| --- | --- |
| `packaging/` | Fat-wheel assembler, dummy-wheel fabricator, and JS parity fixture generator |
| `calibration/` | OpenAI-sample generator for calibration fixtures used by `tinyquant-bench` |

## Usage

Run the pre-commit verification suite (same checks as the Git hook):

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\pre-commit.ps1
```

Or invoke the Python verifier directly:

```bash
python scripts/verify_pre_commit.py
```

Generate Rust test fixtures from the Python reference:

```bash
python scripts/generate_rust_fixtures.py --help
```

## See also

- [Local AGENTS.md](./AGENTS.md)
- [Root README](../README.md)
