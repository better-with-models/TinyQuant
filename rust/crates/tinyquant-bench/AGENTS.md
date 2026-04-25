# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-bench`

This crate provides the Criterion benchmark harness for TinyQuant and the calibration quality-gate helpers (neighbor recall and Pearson ρ) used to enforce codec fidelity in CI. CI quality-gate jobs depend on this crate to verify that quantization does not degrade search recall or correlation below defined thresholds. Changes here most often involve adding Criterion bench groups for new codec paths, adjusting recall or Pearson ρ thresholds in the calibration helpers, or adding fixture data under `baselines/`.

## What this area contains

- primary responsibility: Criterion benchmark harness for TinyQuant codec performance and calibration quality-gate helpers (neighbor recall, Pearson ρ) consumed by CI quality-gate jobs
- main entrypoints: `src/lib.rs` (crate root, exposes `pub mod calibration`), `benches/` (Criterion entry points), `tests/` (calibration quality-gate integration tests)
- common changes: adding Criterion bench groups for new codec variants, adjusting calibration thresholds, adding baseline fixture data

## Layout

```text
tinyquant-bench/
├── baselines/
├── benches/
├── src/
├── tests/
├── Cargo.toml
└── README.md
```

## Common workflows

### Update existing behavior

1. Read the local README and the files you will touch before editing.
2. Follow the local invariants before introducing new files or abstractions.
3. Update nearby docs when the change affects layout, commands, or invariants.
4. Run the narrowest useful verification first, then the broader project gate.

### Add a new file or module

1. Confirm the new file belongs in this directory rather than a sibling.
2. Update the layout section if the structure changes in a way another agent must notice.
3. Add or refine local docs when the new file introduces a new boundary or invariant.

## Invariants — Do Not Violate

- keep this directory focused on its stated responsibility
- do not invent APIs, workflows, or invariants that the code does not support
- update this file when structure or safe-editing rules change

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../AGENTS.md)
