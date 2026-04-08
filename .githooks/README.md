# Git Hooks

This directory stores versioned Git hooks for TinyQuant.

## Current hook

- `pre-commit` runs the repository documentation checks before a commit is
  created

## Install

Configure this clone to use the versioned hooks:

```powershell
git config core.hooksPath .githooks
```

## Notes

- the hook delegates to [`scripts/pre-commit.ps1`](../scripts/pre-commit.ps1)
- keep hook logic thin in this directory and place real verification code in
  `scripts/`
