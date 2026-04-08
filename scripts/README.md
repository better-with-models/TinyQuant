# Scripts

This directory holds repository automation and verification helpers.

## Files

| File | Purpose |
| --- | --- |
| `pre-commit.ps1` | PowerShell entrypoint used by the versioned Git pre-commit hook |
| `verify_pre_commit.py` | Repository documentation verification logic |

## Usage

Run the same checks used by the Git hook:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\pre-commit.ps1
```

## Scope

The current checks focus on repository documentation policy:

- required root docs exist
- non-`docs/` markdown stays lint-clean and ordinary
- `docs/` wiki pages keep the expected frontmatter contract
