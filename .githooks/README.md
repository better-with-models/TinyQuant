# Git Hooks

This directory stores versioned Git hooks for TinyQuant.

## Current hook

`pre-commit` runs the following checks before a commit is created:

| Check | Scope | Requirement |
| --- | --- | --- |
| Required root files | always | all listed files must exist |
| CLAUDE.md stub | always | must reference AGENTS.md |
| Obsidian boundary | always | wikilinks/callouts confined to `docs/` |
| Wiki frontmatter | always | `title`, `tags`, `date-created` present |
| markdownlint | always | no markdown lint errors |
| `cargo fmt --all -- --check` | staged `*.rs` files only | Rust code must be formatted |

## Install

Configure this clone to use the versioned hooks:

```powershell
git config core.hooksPath .githooks
```

## Notes

- the hook delegates to [`scripts/pre-commit.ps1`](../scripts/pre-commit.ps1)
- keep hook logic thin in this directory and place real verification code in
  `scripts/`
