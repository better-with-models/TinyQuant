# AGENTS.md — Guide for AI Agents Working in `.githooks`

`.githooks/` stores versioned Git hooks for this repository.

## Layout

```text
.githooks/
├── pre-commit   versioned hook wrapper
└── README.md    human-facing install notes
```

## Common Workflows

### Update hook behavior

1. Keep hook logic thin in this directory.
2. Put real verification logic in `scripts/`, then call it from the hook.
3. Re-test installation with `git config core.hooksPath .githooks`.

## Invariants — Do Not Violate

- `.githooks/pre-commit` is a wrapper, not the home for complex policy.
- Hook behavior must stay aligned with `scripts/pre-commit.ps1`.
- If the hook entrypoint changes, update `.githooks/README.md` in the same
  change.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [scripts/AGENTS.md](../scripts/AGENTS.md)
- [README.md](./README.md)
