# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-bench/tests`

This directory contains the calibration quality-gate integration tests. All heavy tests are marked `#[ignore]` by default so they do not run in the standard `cargo test` pass; they are activated explicitly in CI quality-gate jobs. Changes here most often involve adding new calibration assertions, adjusting thresholds, or wiring new codec variants into the existing test harness.

## What this area contains

- primary responsibility: calibration quality-gate tests — `calibration.rs` (neighbor recall and Pearson ρ assertions against codec output), `smoke.rs` (fast sanity check that the calibration helpers compile and return plausible values)
- main entrypoints: `calibration.rs` (the primary quality gate), `smoke.rs` (always-run sanity check)
- common changes: adjusting recall or Pearson ρ thresholds, adding test cases for new codec configurations, marking new tests `#[ignore]` appropriately

## Layout

```text
tests/
├── calibration.rs
├── README.md
└── smoke.rs
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
- [Root AGENTS.md](../../../../AGENTS.md)
