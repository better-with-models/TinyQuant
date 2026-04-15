# AGENTS.md — Guide for AI Agents Working in `javascript/@tinyquant/core/tests`

`tests/` verifies the npm package's runtime, parity, and module-format behavior.

## Layout

```text
javascript/@tinyquant/core/tests/
├── backend.test.ts
├── corpus.test.ts
├── parity.test.ts
├── round-trip.test.ts
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
