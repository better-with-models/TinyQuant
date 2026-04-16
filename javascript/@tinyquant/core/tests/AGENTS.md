# AGENTS.md — Guide for AI Agents Working in `javascript/@tinyquant/core/tests`

`tests/` verifies the npm package's runtime, parity, and module-format behavior.

## Layout

```text
javascript/@tinyquant/core/tests/
├── backend.test.ts
├── corpus.test.ts           — lifecycle + GAP-JS-004 policy invariants
├── esm-subpath-smoke.test.ts — GAP-JS-007: sub-path export resolution
├── loader.test.ts            — GAP-JS-006: binaryKey() all platforms + musl
├── parity.test.ts
├── round-trip.test.ts        — dim=128 + GAP-JS-002 dim=768
├── types.test.ts
├── cjs-smoke.test.cjs
├── fixtures/
└── README.md
```

## Common Workflows

### Add or update a test

1. Put the assertion in the narrowest test module that owns the behavior.
2. Keep parity expectations aligned with generated fixtures.
3. Always keep the CommonJS smoke test passing when export or build logic moves.

## Invariants — Do Not Violate

- `cjs-smoke.test.cjs` guards the shipped CJS runtime surface.
- Fixture-backed tests must stay aligned with generated parity data.
- Tests here validate package behavior, not Rust internal implementation detail.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../AGENTS.md)
- [scripts/packaging/AGENTS.md](../../../../scripts/packaging/AGENTS.md)
