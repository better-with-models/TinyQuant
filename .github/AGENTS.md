# AGENTS.md — Guide for AI Agents Working in `.github`

`.github/` holds GitHub-facing assets and automation for TinyQuant.

## Layout

```text
.github/
├── README.md             rich GitHub landing page for the repository
├── workflows/            CI, docs lint, packaging, and release workflows
└── release-drafter.yml   release note categorization rules
```

## Common Workflows

### Update the landing page

1. Treat `.github/README.md` as the GitHub-specific companion to the root
   `README.md`, not as a local directory manual.
2. Keep repository identity, benchmark headlines, and user-facing install
   guidance aligned with the root docs.

### Update GitHub automation

1. Make workflow edits in `.github/workflows/`.
2. Keep workflow names, trigger paths, and documentation references aligned
   with the directories they gate.
3. Update `release-drafter.yml` when release-note categories or labels change.

## Invariants — Do Not Violate

- `.github/README.md` is a repo landing page, not a directory README.
- Workflow changes that affect linting, packaging, or release behavior must
  stay aligned with `scripts/`, `tests/packaging/`, and root docs.
- Keep GitHub-specific Markdown or HTML in `.github/README.md`; do not copy
  those patterns into ordinary markdown outside `.github/` and `docs/`.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [workflows/AGENTS.md](./workflows/AGENTS.md)
- [scripts/AGENTS.md](../scripts/AGENTS.md)
