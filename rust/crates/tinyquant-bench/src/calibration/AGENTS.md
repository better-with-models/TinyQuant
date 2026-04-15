# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-bench/src/calibration`

`calibration/` implements quality metrics reused by benchmarks and calibration tests.

## Layout

```text
rust/crates/tinyquant-bench/src/calibration/
├── neighbor_recall.rs
├── pearson.rs
├── mod.rs
└── README.md
```

## Common Workflows

### Update a calibration metric

1. Keep the metric definition and its thresholds conceptually separate.
2. Change the narrowest metric module possible.
3. Re-run the benchmark crate tests after changing these helpers.

## Invariants — Do Not Violate

- `mod.rs` is the module registry.
- Metrics here are reused by quality-gate tests, so behavior changes are
  user-visible at the release level.
- Keep statistical meaning and naming stable unless the surrounding docs move
  too.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../../AGENTS.md)
