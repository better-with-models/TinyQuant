# AGENTS.md — Guide for AI Agents Working in `.github/workflows`

`.github/workflows/` contains the repository's GitHub Actions workflow definitions.

## Layout

```text
.github/workflows/
├── ci.yml
├── docs-lint.yml
├── js-ci.yml
├── js-release.yml
├── python-fatwheel.yml
├── release.yml
├── rust-calibration.yml
├── rust-ci.yml
├── rust-release.yml
└── README.md
```

## Common Workflows

### Update an existing workflow

1. Edit the narrowest workflow file that owns the behavior.
2. Keep path filters, tool versions, and job names aligned with the docs and
   scripts they reference.
3. When a workflow gates a directory-local contract, update that directory's
   docs too.

### Add a new workflow

1. Confirm the behavior does not belong in an existing workflow first.
2. Add the file to `README.md` here.
3. Link it from parent docs if it changes contributor-facing workflow.

## Invariants — Do Not Violate

- `ci.yml` owns non-`docs/` markdown lint, Python CI, and baseline repo gates.
- `docs-lint.yml` is the Obsidian-vault-specific lint workflow.
- Keep workflow file names stable unless the surrounding docs are updated in the
  same change.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../AGENTS.md)
- [README.md](./README.md)
