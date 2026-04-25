# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-cli/scripts`

`scripts/` contains smoke-test helpers for the `tinyquant` CLI.

## Layout

```text
rust/crates/tinyquant-cli/scripts/
├── cli-smoke.ps1
├── cli-smoke.sh
└── README.md
```

## Common Workflows

### Update a smoke script

1. Keep PowerShell and shell behavior aligned where both scripts cover the same
   surface.
2. Prefer small assertion changes over rewriting the script shape.
3. Re-run the relevant smoke script after editing it.

## Invariants — Do Not Violate

- These scripts validate operator-facing basics, not exhaustive CLI behavior.
- Cross-platform parity between the smoke scripts matters.
- If flags or output expectations change here, update CLI docs or tests too.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../../AGENTS.md)
