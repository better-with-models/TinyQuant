# TinyQuant

TinyQuant is a documentation-first repository with an LLM-maintained knowledge
base under `docs/`.

The repository uses two documentation modes:

- `docs/` is an Obsidian wiki that follows the LLM Wiki pattern described in
  [`docs/research/llm-wiki.md`](docs/research/llm-wiki.md)
- markdown outside `docs/` stays portable and must pass strict markdownlint
  checks

## Repository layout

| Path | Purpose |
| --- | --- |
| `docs/` | compiled knowledge layer and Obsidian vault |
| `docs/research/` | raw research inputs and idea files |
| `scripts/` | repository automation and verification helpers |
| `.githooks/` | versioned Git hooks for local enforcement |
| `AGENTS.md` | agent operating rules for the repository |
| `CONCEPTS.md` | glossary for repository-specific documentation terms |
| `CLAUDE.md` | Claude-facing stub that points back to `AGENTS.md` |

## Documentation policy

- Use rich Obsidian-flavored markdown only inside `docs/`
- Use ordinary markdown everywhere else
- Keep repository docs aligned with the actual code and file layout
- Treat `docs/` as the compiled wiki and the rest of the repo as raw evidence
  when synthesizing knowledge pages

## Pre-commit verification

This repository uses a versioned pre-commit hook in `.githooks/pre-commit`.

The hook currently verifies:

- required root documentation files exist
- markdown outside `docs/` passes markdownlint
- Obsidian-specific markdown does not leak outside `docs/`
- wiki pages under `docs/` follow the required frontmatter conventions

Install or refresh the local hook path with:

```powershell
git config core.hooksPath .githooks
```

You can run the same checks manually with:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\pre-commit.ps1
```

## Related files

- [AGENTS.md](AGENTS.md)
- [CONCEPTS.md](CONCEPTS.md)
- [docs/README.md](docs/README.md)
- [scripts/README.md](scripts/README.md)
- [.githooks/README.md](.githooks/README.md)
