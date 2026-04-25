# AGENTS.md — Guide for AI Agents Working in `tests/reference`

`tests/reference/` holds test-only reference assets, not shipped product code.

## Layout

```text
tests/reference/
├── tinyquant_py_reference/
└── README.md
```

## Common Workflows

### Update the frozen oracle

1. Confirm the change is justified by an explicit rollout or migration plan.
2. Keep the change narrowly scoped to parity or fixture preservation needs.
3. Update the README here if the role of the oracle changes.

## Invariants — Do Not Violate

- Nothing under `tests/reference/` is a supported runtime surface.
- The reference package exists to preserve historical behavior, not to grow new
  features.
- Keep docs explicit that this subtree is test-only.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../AGENTS.md)
- [tinyquant_py_reference/AGENTS.md](./tinyquant_py_reference/AGENTS.md)
